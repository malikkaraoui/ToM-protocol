/**
 * Group Hub (Story 4.6 - Group Messaging)
 *
 * Runs on relay nodes to handle group message fanout.
 * Acts as a "temporary hub" for group communication.
 *
 * Key responsibilities:
 * - Store group membership
 * - Fan out messages to all members
 * - Handle join/leave requests
 * - Provide sync for new/reconnecting members
 *
 * @see group-types.ts for type definitions
 */

import type { NodeId } from '../identity/index.js';
import {
  GROUP_SECURITY_DEFAULTS,
  type GroupSecurityConfig,
  NonceTracker,
  verifyGroupMessageSignature,
} from './group-security.js';
import {
  DEFAULT_MAX_GROUP_MEMBERS,
  GROUP_RATE_LIMIT_PER_SECOND,
  type GroupCreatePayload,
  type GroupCreatedPayload,
  type GroupDeliveryAckPayload,
  type GroupHubHeartbeatPayload,
  type GroupHubMigrationPayload,
  type GroupId,
  type GroupInfo,
  type GroupJoinPayload,
  type GroupLeavePayload,
  type GroupMember,
  type GroupMemberLeftPayload,
  type GroupMessagePayload,
  type GroupPayload,
  type GroupSyncPayload,
  HUB_HEARTBEAT_INTERVAL_MS,
  MAX_SYNC_MESSAGES,
  isGroupCreate,
  isGroupMessage,
  isGroupPayload,
} from './group-types.js';

/** Events emitted by GroupHub */
export interface GroupHubEvents {
  /** Send a message to a specific node */
  sendToNode: (nodeId: NodeId, payload: GroupPayload, groupId: GroupId) => void;
  /** Send a message to all group members */
  broadcastToGroup: (groupId: GroupId, payload: GroupPayload, excludeNodeId?: NodeId) => void;
  /** Broadcast group announcement to all known peers (for public groups) */
  broadcastAnnouncement?: (payload: GroupPayload) => void;
  /** Log hub activity */
  onHubActivity?: (groupId: GroupId, activity: string, details?: unknown) => void;
  /** Capacity warning */
  onCapacityWarning?: (groupsCount: number, totalMembers: number) => void;
}

/** Options for GroupHub */
export interface GroupHubOptions {
  /** Max groups this hub can manage */
  maxGroups?: number;
  /** Max total members across all groups */
  maxTotalMembers?: number;
  /** Max messages to store per group (for sync) */
  maxMessagesPerGroup?: number;
  /** Max total messages across all groups (memory protection) */
  maxTotalMessages?: number;
  /** Max pending deliveries to track (memory protection) */
  maxPendingDeliveries?: number;
  /** Security configuration */
  security?: GroupSecurityConfig;
}

/** Default limits */
const DEFAULT_MAX_GROUPS = 100;
const DEFAULT_MAX_TOTAL_MEMBERS = 1000;
const DEFAULT_MAX_MESSAGES_PER_GROUP = 200;
const DEFAULT_MAX_TOTAL_MESSAGES = 10000; // Global message cap
const DEFAULT_MAX_PENDING_DELIVERIES = 5000; // Pending delivery cap

/** Rate limit tracking per sender */
interface RateLimitEntry {
  count: number;
  windowStart: number;
}

/** Migration state for transferring to new hub */
export interface GroupMigrationData {
  groupInfo: GroupInfo;
  messageHistory: GroupMessagePayload[];
  pendingDeliveries: Array<{ messageId: string; pendingNodes: NodeId[] }>;
}

/**
 * GroupHub
 *
 * Manages groups on a relay node.
 * Handles fanout of messages to all members.
 */
export class GroupHub {
  private localNodeId: NodeId;
  private events: GroupHubEvents;
  /** Groups managed by this hub: groupId -> GroupInfo */
  private groups = new Map<GroupId, GroupInfo>();
  /** Message history per group: groupId -> messages */
  private messageHistory = new Map<GroupId, GroupMessagePayload[]>();
  /** Pending deliveries: messageId -> Set<nodeId> (who hasn't acked) */
  private pendingDeliveries = new Map<string, Set<NodeId>>();
  /** Rate limiting per sender per group */
  private rateLimits = new Map<string, RateLimitEntry>();
  /** Nonce tracker for anti-replay */
  private nonceTracker: NonceTracker;
  /** Security configuration */
  private securityConfig: Required<GroupSecurityConfig>;

  private maxGroups: number;
  private maxTotalMembers: number;
  private maxMessagesPerGroup: number;
  private maxTotalMessages: number;
  private maxPendingDeliveries: number;
  /** Heartbeat interval timer */
  private heartbeatTimer: ReturnType<typeof setInterval> | null = null;

  constructor(localNodeId: NodeId, events: GroupHubEvents, options: GroupHubOptions = {}) {
    this.localNodeId = localNodeId;
    this.events = events;
    this.maxGroups = options.maxGroups ?? DEFAULT_MAX_GROUPS;
    this.maxTotalMembers = options.maxTotalMembers ?? DEFAULT_MAX_TOTAL_MEMBERS;
    this.maxMessagesPerGroup = options.maxMessagesPerGroup ?? DEFAULT_MAX_MESSAGES_PER_GROUP;
    this.maxTotalMessages = options.maxTotalMessages ?? DEFAULT_MAX_TOTAL_MESSAGES;
    this.maxPendingDeliveries = options.maxPendingDeliveries ?? DEFAULT_MAX_PENDING_DELIVERIES;

    // Initialize security
    this.securityConfig = {
      requireSignatures: options.security?.requireSignatures ?? GROUP_SECURITY_DEFAULTS.requireSignatures,
      requireNonces: options.security?.requireNonces ?? GROUP_SECURITY_DEFAULTS.requireNonces,
      nonceMaxAgeMs: options.security?.nonceMaxAgeMs ?? GROUP_SECURITY_DEFAULTS.nonceMaxAgeMs,
      nonceMaxSize: options.security?.nonceMaxSize ?? GROUP_SECURITY_DEFAULTS.nonceMaxSize,
    };
    this.nonceTracker = new NonceTracker({
      maxAgeMs: this.securityConfig.nonceMaxAgeMs,
      maxSize: this.securityConfig.nonceMaxSize,
    });
  }

  /**
   * Start sending heartbeats to all group members
   */
  startHeartbeats(): void {
    if (this.heartbeatTimer) return;
    this.heartbeatTimer = setInterval(() => {
      this.sendHeartbeats();
    }, HUB_HEARTBEAT_INTERVAL_MS);
  }

  /**
   * Stop sending heartbeats
   */
  stopHeartbeats(): void {
    if (this.heartbeatTimer) {
      clearInterval(this.heartbeatTimer);
      this.heartbeatTimer = null;
    }
  }

  /**
   * Send heartbeat to all members of all groups
   */
  private sendHeartbeats(): void {
    for (const [groupId, group] of this.groups) {
      const heartbeat: GroupHubHeartbeatPayload = {
        type: 'group-hub-heartbeat',
        groupId,
        memberCount: group.members.length,
        timestamp: Date.now(),
      };
      this.events.broadcastToGroup(groupId, heartbeat);
    }
  }

  // ============================================
  // Incoming Message Handler
  // ============================================

  /**
   * Handle incoming group payload from any node.
   * Routes to appropriate handler based on type.
   */
  handlePayload(payload: unknown, fromNodeId: NodeId): void {
    if (!isGroupPayload(payload)) {
      return;
    }

    switch (payload.type) {
      case 'group-create':
        if (isGroupCreate(payload)) {
          this.handleCreate(payload, fromNodeId, payload.creatorUsername);
        }
        break;
      case 'group-join':
        this.handleJoin(payload as GroupJoinPayload, fromNodeId);
        break;
      case 'group-leave':
        this.handleLeave(payload as GroupLeavePayload, fromNodeId);
        break;
      case 'group-message':
        if (isGroupMessage(payload)) {
          this.handleMessage(payload, fromNodeId);
        }
        break;
      case 'group-delivery-ack':
        this.handleDeliveryAck(payload as GroupDeliveryAckPayload, fromNodeId);
        break;
      default:
        // Ignore unknown types
        break;
    }
  }

  // ============================================
  // Group Creation
  // ============================================

  private handleCreate(payload: GroupCreatePayload, fromNodeId: NodeId, creatorUsername?: string): void {
    // Check capacity
    if (this.groups.size >= this.maxGroups) {
      this.events.onHubActivity?.(payload.groupId, 'create-rejected', { reason: 'max-groups' });
      return;
    }

    const totalMembers = this.getTotalMemberCount();
    if (totalMembers + payload.initialMembers.length + 1 > this.maxTotalMembers) {
      this.events.onHubActivity?.(payload.groupId, 'create-rejected', { reason: 'max-members' });
      return;
    }

    const now = Date.now();
    const groupId = payload.groupId;
    const username = creatorUsername ?? 'Creator';

    // Create the group with creator as admin
    const group: GroupInfo = {
      groupId,
      name: payload.name,
      hubRelayId: this.localNodeId,
      members: [
        {
          nodeId: fromNodeId,
          username,
          joinedAt: now,
          role: 'admin',
        },
      ],
      createdBy: fromNodeId,
      createdAt: now,
      lastActivityAt: now,
      maxMembers: payload.maxMembers ?? DEFAULT_MAX_GROUP_MEMBERS,
    };

    this.groups.set(groupId, group);
    this.messageHistory.set(groupId, []);

    // Send confirmation to creator
    const createdPayload: GroupCreatedPayload = {
      type: 'group-created',
      groupId,
      groupInfo: group,
    };
    this.events.sendToNode(fromNodeId, createdPayload, groupId);

    // Send invitations to initial members via direct 1-to-1 channels
    // Note: We don't broadcast public announcements - invitations are personal only
    for (const member of payload.initialMembers) {
      this.sendInvite(groupId, group.name, member.nodeId, member.username, fromNodeId, username);
    }

    this.events.onHubActivity?.(groupId, 'created', { creator: fromNodeId, members: payload.initialMembers.length });
  }

  private sendInvite(
    groupId: GroupId,
    groupName: string,
    inviteeId: NodeId,
    _inviteeUsername: string,
    inviterId: NodeId,
    inviterUsername: string,
  ): void {
    const group = this.groups.get(groupId);
    if (!group) return;

    const invitePayload = {
      type: 'group-invite' as const,
      groupId,
      inviteeId,
      inviteeUsername: _inviteeUsername,
      inviterId,
      inviterUsername,
      groupName,
      hubRelayId: this.localNodeId,
      memberCount: group.members.length,
    };

    this.events.sendToNode(inviteeId, invitePayload, groupId);
  }

  // ============================================
  // Join/Leave
  // ============================================

  private handleJoin(payload: GroupJoinPayload, fromNodeId: NodeId): void {
    const group = this.groups.get(payload.groupId);
    if (!group) {
      this.events.onHubActivity?.(payload.groupId, 'join-rejected', { reason: 'group-not-found' });
      return;
    }

    // Verify the joining node matches the payload
    if (payload.nodeId !== fromNodeId) {
      this.events.onHubActivity?.(payload.groupId, 'join-rejected', { reason: 'node-mismatch' });
      return;
    }

    // Check if already a member
    if (group.members.some((m) => m.nodeId === fromNodeId)) {
      // Already a member, just send sync
      this.sendSync(payload.groupId, fromNodeId);
      return;
    }

    // Check max members
    if (group.members.length >= group.maxMembers) {
      this.events.onHubActivity?.(payload.groupId, 'join-rejected', { reason: 'max-members' });
      return;
    }

    // Add new member
    const newMember: GroupMember = {
      nodeId: fromNodeId,
      username: payload.username,
      joinedAt: Date.now(),
      role: 'member',
    };
    group.members.push(newMember);
    group.lastActivityAt = Date.now();

    // Send sync to the new member so they receive full state/history
    this.sendSync(payload.groupId, fromNodeId);

    // Broadcast join event to existing members (exclude the new member)
    const joinedPayload = {
      type: 'group-member-joined' as const,
      groupId: payload.groupId,
      member: newMember,
    };
    this.events.broadcastToGroup(payload.groupId, joinedPayload, fromNodeId);

    this.events.onHubActivity?.(payload.groupId, 'member-joined', {
      nodeId: fromNodeId,
      memberCount: group.members.length,
    });
  }

  private handleLeave(payload: GroupLeavePayload, fromNodeId: NodeId): void {
    const group = this.groups.get(payload.groupId);
    if (!group) return;

    // Verify the leaving node
    const isAdmin = group.members.some((m) => m.nodeId === fromNodeId && m.role === 'admin');
    const isSelf = payload.nodeId === fromNodeId;
    const isKicking = isAdmin && payload.nodeId !== fromNodeId;

    if (!isSelf && !isKicking) {
      // Non-admin trying to kick someone
      return;
    }

    const leavingMember = group.members.find((m) => m.nodeId === payload.nodeId);
    if (!leavingMember) return;

    // Remove member
    group.members = group.members.filter((m) => m.nodeId !== payload.nodeId);
    group.lastActivityAt = Date.now();

    // Notify all members
    const leftPayload: GroupMemberLeftPayload = {
      type: 'group-member-left',
      groupId: payload.groupId,
      nodeId: payload.nodeId,
      username: leavingMember.username,
      reason: payload.reason ?? 'voluntary',
    };
    this.events.broadcastToGroup(payload.groupId, leftPayload);

    // If no members left, delete the group
    if (group.members.length === 0) {
      this.groups.delete(payload.groupId);
      this.messageHistory.delete(payload.groupId);
      this.events.onHubActivity?.(payload.groupId, 'group-deleted', { reason: 'no-members' });
    }

    this.events.onHubActivity?.(payload.groupId, 'member-left', { nodeId: payload.nodeId });
  }

  // ============================================
  // Messages
  // ============================================

  private handleMessage(payload: GroupMessagePayload, fromNodeId: NodeId): void {
    const group = this.groups.get(payload.groupId);
    if (!group) return;

    // Verify sender is a member
    if (!group.members.some((m) => m.nodeId === fromNodeId)) {
      this.events.onHubActivity?.(payload.groupId, 'message-rejected', { reason: 'not-member' });
      return;
    }

    // Security: Verify sender matches payload
    if (payload.senderId !== fromNodeId) {
      this.events.onHubActivity?.(payload.groupId, 'message-rejected', { reason: 'sender-mismatch' });
      return;
    }

    // Security: Verify signature if required
    if (this.securityConfig.requireSignatures) {
      if (!payload.signature || !verifyGroupMessageSignature(payload, fromNodeId)) {
        this.events.onHubActivity?.(payload.groupId, 'message-rejected', { reason: 'invalid-signature' });
        return;
      }
    }

    // Security: Anti-replay check (nonce)
    if (this.securityConfig.requireNonces) {
      if (!payload.nonce) {
        this.events.onHubActivity?.(payload.groupId, 'message-rejected', { reason: 'missing-nonce' });
        return;
      }
      const nonceKey = `${payload.groupId}:${payload.nonce}`;
      if (!this.nonceTracker.checkAndRecord(nonceKey)) {
        this.events.onHubActivity?.(payload.groupId, 'message-rejected', { reason: 'replay-detected' });
        return;
      }
    }

    // Rate limiting
    if (!this.checkRateLimit(payload.groupId, fromNodeId)) {
      this.events.onHubActivity?.(payload.groupId, 'message-rejected', { reason: 'rate-limited' });
      return;
    }

    // Memory protection: check global limits before storing
    const totalMessages = this.getTotalMessageCount();
    if (totalMessages >= this.maxTotalMessages) {
      // Trim oldest messages from all groups
      this.trimOldestMessages();
      this.events.onCapacityWarning?.(this.groups.size, this.getTotalMemberCount());
    }

    // Store message
    const messages = this.messageHistory.get(payload.groupId) ?? [];
    messages.push(payload);
    if (messages.length > this.maxMessagesPerGroup) {
      messages.shift();
    }
    this.messageHistory.set(payload.groupId, messages);
    group.lastActivityAt = Date.now();

    // Memory protection: limit pending deliveries
    if (this.pendingDeliveries.size >= this.maxPendingDeliveries) {
      // Remove oldest pending deliveries
      const oldest = this.pendingDeliveries.keys().next().value;
      if (oldest) {
        this.pendingDeliveries.delete(oldest);
      }
    }

    // Track pending deliveries
    const pendingNodes = new Set(group.members.map((m) => m.nodeId).filter((id) => id !== fromNodeId));
    if (pendingNodes.size > 0) {
      this.pendingDeliveries.set(payload.messageId, pendingNodes);
    }

    // Fanout to all members except sender
    this.events.broadcastToGroup(payload.groupId, payload, fromNodeId);

    this.events.onHubActivity?.(payload.groupId, 'message-fanout', {
      messageId: payload.messageId,
      recipients: pendingNodes.size,
    });
  }

  private handleDeliveryAck(payload: GroupDeliveryAckPayload, fromNodeId: NodeId): void {
    const pending = this.pendingDeliveries.get(payload.messageId);
    if (!pending) return;

    pending.delete(fromNodeId);

    if (pending.size === 0) {
      this.pendingDeliveries.delete(payload.messageId);
      // Could emit full delivery event here
    }
  }

  // ============================================
  // Sync
  // ============================================

  private sendSync(groupId: GroupId, nodeId: NodeId): void {
    const group = this.groups.get(groupId);
    if (!group) return;

    const messages = this.messageHistory.get(groupId) ?? [];

    const syncPayload: GroupSyncPayload = {
      type: 'group-sync',
      groupId,
      groupInfo: group,
      recentMessages: messages.slice(-MAX_SYNC_MESSAGES),
    };

    this.events.sendToNode(nodeId, syncPayload, groupId);
  }

  // ============================================
  // Rate Limiting
  // ============================================

  private checkRateLimit(groupId: GroupId, nodeId: NodeId): boolean {
    const key = `${groupId}:${nodeId}`;
    const now = Date.now();
    const windowMs = 1000; // 1 second window

    let entry = this.rateLimits.get(key);
    if (!entry || now - entry.windowStart > windowMs) {
      entry = { count: 0, windowStart: now };
      this.rateLimits.set(key, entry);
    }

    entry.count++;
    if (entry.count > GROUP_RATE_LIMIT_PER_SECOND) {
      return false;
    }

    return true;
  }

  // ============================================
  // Admin Functions
  // ============================================

  /**
   * Invite a new member (called by group admin)
   */
  inviteMember(
    groupId: GroupId,
    inviteeId: NodeId,
    inviteeUsername: string,
    inviterId: NodeId,
    inviterUsername: string,
  ): boolean {
    const group = this.groups.get(groupId);
    if (!group) return false;

    // Verify inviter is admin
    if (!group.members.some((m) => m.nodeId === inviterId && m.role === 'admin')) {
      return false;
    }

    this.sendInvite(groupId, group.name, inviteeId, inviteeUsername, inviterId, inviterUsername);
    return true;
  }

  // ============================================
  // Queries
  // ============================================

  /**
   * Get all groups managed by this hub
   */
  getAllGroups(): GroupInfo[] {
    return Array.from(this.groups.values());
  }

  /**
   * Get a specific group
   */
  getGroup(groupId: GroupId): GroupInfo | null {
    return this.groups.get(groupId) ?? null;
  }

  /**
   * Check if a node is a member of a group
   */
  isMember(groupId: GroupId, nodeId: NodeId): boolean {
    const group = this.groups.get(groupId);
    return group?.members.some((m) => m.nodeId === nodeId) ?? false;
  }

  /**
   * Get total member count across all groups
   */
  getTotalMemberCount(): number {
    let total = 0;
    for (const group of this.groups.values()) {
      total += group.members.length;
    }
    return total;
  }

  /**
   * Get total message count across all groups
   */
  getTotalMessageCount(): number {
    let total = 0;
    for (const messages of this.messageHistory.values()) {
      total += messages.length;
    }
    return total;
  }

  /**
   * Trim oldest messages from each group to reduce memory
   */
  private trimOldestMessages(): void {
    for (const [groupId, messages] of this.messageHistory) {
      if (messages.length > 10) {
        // Keep only most recent 80%
        const keepCount = Math.floor(messages.length * 0.8);
        this.messageHistory.set(groupId, messages.slice(-keepCount));
      }
    }
    this.events.onHubActivity?.('global', 'memory-trimmed', {
      totalMessages: this.getTotalMessageCount(),
    });
  }

  /**
   * Get hub statistics
   */
  getStats(): {
    groupCount: number;
    totalMembers: number;
    totalMessages: number;
    pendingDeliveries: number;
    nonceTrackerSize: number;
    rateLimitEntries: number;
  } {
    return {
      groupCount: this.groups.size,
      totalMembers: this.getTotalMemberCount(),
      totalMessages: this.getTotalMessageCount(),
      pendingDeliveries: this.pendingDeliveries.size,
      nonceTrackerSize: this.nonceTracker.size,
      rateLimitEntries: this.rateLimits.size,
    };
  }

  /**
   * Clean up stale rate limit entries
   */
  cleanupRateLimits(): void {
    const now = Date.now();
    const staleThreshold = 60_000; // 1 minute

    for (const [key, entry] of this.rateLimits) {
      if (now - entry.windowStart > staleThreshold) {
        this.rateLimits.delete(key);
      }
    }
  }

  // ============================================
  // Hub Migration
  // ============================================

  /**
   * Export group data for migration to new hub
   */
  exportGroupForMigration(groupId: GroupId): GroupMigrationData | null {
    const group = this.groups.get(groupId);
    if (!group) return null;

    const messages = this.messageHistory.get(groupId) ?? [];
    const pending: Array<{ messageId: string; pendingNodes: NodeId[] }> = [];

    for (const [messageId, nodes] of this.pendingDeliveries) {
      // Only include pending deliveries for this group's messages
      if (messages.some((m) => m.messageId === messageId)) {
        pending.push({ messageId, pendingNodes: Array.from(nodes) });
      }
    }

    return {
      groupInfo: { ...group },
      messageHistory: [...messages],
      pendingDeliveries: pending,
    };
  }

  /**
   * Import group data when becoming new hub (migration)
   */
  importGroupFromMigration(data: GroupMigrationData): boolean {
    if (this.groups.size >= this.maxGroups) {
      return false;
    }

    // Update hub to self
    const group = { ...data.groupInfo, hubRelayId: this.localNodeId };
    this.groups.set(group.groupId, group);
    this.messageHistory.set(group.groupId, [...data.messageHistory]);

    // Restore pending deliveries
    for (const pd of data.pendingDeliveries) {
      this.pendingDeliveries.set(pd.messageId, new Set(pd.pendingNodes));
    }

    this.events.onHubActivity?.(group.groupId, 'migration-imported', {
      members: group.members.length,
      messages: data.messageHistory.length,
    });

    return true;
  }

  /**
   * Initiate migration of a group to a new hub
   */
  initiateHubMigration(groupId: GroupId, newHubId: NodeId, reason: 'failure' | 'capacity' | 'manual'): void {
    const group = this.groups.get(groupId);
    if (!group) return;

    // Notify all members of migration
    const migrationPayload: GroupHubMigrationPayload = {
      type: 'group-hub-migration',
      groupId,
      newHubId,
      oldHubId: this.localNodeId,
      reason,
    };

    this.events.broadcastToGroup(groupId, migrationPayload);

    // Remove group from this hub
    this.groups.delete(groupId);
    this.messageHistory.delete(groupId);

    this.events.onHubActivity?.(groupId, 'migration-initiated', { newHubId, reason });
  }

  /**
   * Cleanup when shutting down - migrate all groups
   */
  shutdown(backupHubId?: NodeId): void {
    this.stopHeartbeats();

    if (backupHubId) {
      for (const groupId of this.groups.keys()) {
        this.initiateHubMigration(groupId, backupHubId, 'manual');
      }
    }
  }
}
