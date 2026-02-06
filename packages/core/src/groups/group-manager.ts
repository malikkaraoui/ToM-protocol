/**
 * Group Manager (Story 4.6 - Group Messaging)
 *
 * Manages group membership and state on each node.
 * Handles group creation, joining, leaving, and state sync.
 *
 * @see group-types.ts for type definitions
 */

import { secureRandomUUID } from '../crypto/secure-random.js';
import type { NodeId } from '../identity/index.js';
import {
  DEFAULT_MAX_GROUP_MEMBERS,
  type GroupId,
  type GroupInfo,
  type GroupMember,
  type GroupMessagePayload,
  HUB_FAILURE_THRESHOLD,
  HUB_HEARTBEAT_INTERVAL_MS,
  MAX_SYNC_MESSAGES,
} from './group-types.js';

/** Public group info (for groups we haven't joined yet) */
export interface PublicGroupInfo {
  groupId: GroupId;
  groupName: string;
  hubRelayId: NodeId;
  memberCount: number;
  createdBy: NodeId;
  creatorUsername: string;
  /** When we received this announcement */
  announcedAt: number;
}

/** Events emitted by GroupManager */
export interface GroupManagerEvents {
  /** New group created (we are a member) */
  onGroupCreated: (group: GroupInfo) => void;
  /** Received invitation to join a group */
  onGroupInvite: (groupId: GroupId, groupName: string, inviterId: NodeId, inviterUsername: string) => void;
  /** A member joined one of our groups */
  onMemberJoined: (groupId: GroupId, member: GroupMember) => void;
  /** A member left one of our groups */
  onMemberLeft: (groupId: GroupId, nodeId: NodeId, username: string, reason: string) => void;
  /** Received a group message */
  onGroupMessage: (groupId: GroupId, message: GroupMessagePayload) => void;
  /** Hub migration occurred */
  onHubMigration: (groupId: GroupId, newHubId: NodeId, oldHubId: NodeId) => void;
  /** Group was deleted or we were removed */
  onGroupRemoved: (groupId: GroupId, reason: string) => void;
  /** Hub failure detected - needs migration */
  onHubFailure: (groupId: GroupId, hubId: NodeId) => void;
  /** Public group announced - available to join */
  onPublicGroupAnnounced: (group: PublicGroupInfo) => void;
}

/** Options for GroupManager */
export interface GroupManagerOptions {
  /** Max groups this node can be in */
  maxGroups?: number;
  /** Max messages to keep per group (for sync) */
  maxMessagesPerGroup?: number;
}

/** Default max groups per node */
const DEFAULT_MAX_GROUPS = 20;

/** Default max messages per group */
const DEFAULT_MAX_MESSAGES_PER_GROUP = 500;

/**
 * GroupManager
 *
 * Tracks all groups this node is a member of.
 * Stores recent messages for sync with new members.
 */
/** Hub health tracking */
interface HubHealthInfo {
  lastHeartbeat: number;
  missedHeartbeats: number;
}

export class GroupManager {
  private localNodeId: NodeId;
  private localUsername: string;
  private events: Partial<GroupManagerEvents>;
  private groups = new Map<GroupId, GroupInfo>();
  /** Recent messages per group (for sync) */
  private messageHistory = new Map<GroupId, GroupMessagePayload[]>();
  /** Pending invitations (groupId -> invite info) */
  private pendingInvites = new Map<
    GroupId,
    { groupName: string; inviterId: NodeId; inviterUsername: string; hubRelayId: NodeId }
  >();
  /** Public groups available to join (not yet joined) */
  private availableGroups = new Map<GroupId, PublicGroupInfo>();
  /** Hub health per group */
  private hubHealth = new Map<GroupId, HubHealthInfo>();
  /** Hub health check timer */
  private healthCheckTimer: ReturnType<typeof setInterval> | null = null;
  private maxGroups: number;
  private maxMessagesPerGroup: number;

  constructor(
    localNodeId: NodeId,
    localUsername: string,
    events: Partial<GroupManagerEvents> = {},
    options: GroupManagerOptions = {},
  ) {
    this.localNodeId = localNodeId;
    this.localUsername = localUsername;
    this.events = events;
    this.maxGroups = options.maxGroups ?? DEFAULT_MAX_GROUPS;
    this.maxMessagesPerGroup = options.maxMessagesPerGroup ?? DEFAULT_MAX_MESSAGES_PER_GROUP;
  }

  // ============================================
  // Hub Health Monitoring
  // ============================================

  /**
   * Start monitoring hub health for all groups
   */
  startHubHealthMonitoring(): void {
    if (this.healthCheckTimer) return;
    this.healthCheckTimer = setInterval(() => {
      this.checkHubHealth();
    }, HUB_HEARTBEAT_INTERVAL_MS);
  }

  /**
   * Stop hub health monitoring
   */
  stopHubHealthMonitoring(): void {
    if (this.healthCheckTimer) {
      clearInterval(this.healthCheckTimer);
      this.healthCheckTimer = null;
    }
  }

  /**
   * Check hub health for all groups
   */
  private checkHubHealth(): void {
    const now = Date.now();
    for (const [groupId, group] of this.groups) {
      const health = this.hubHealth.get(groupId);
      if (!health) {
        // Initialize health tracking
        this.hubHealth.set(groupId, { lastHeartbeat: now, missedHeartbeats: 0 });
        continue;
      }

      const elapsed = now - health.lastHeartbeat;
      if (elapsed > HUB_HEARTBEAT_INTERVAL_MS * 1.5) {
        health.missedHeartbeats++;
        if (health.missedHeartbeats >= HUB_FAILURE_THRESHOLD) {
          // Hub is considered failed
          this.events.onHubFailure?.(groupId, group.hubRelayId);
          // Reset to avoid repeated events
          health.missedHeartbeats = 0;
        }
      }
    }
  }

  /**
   * Handle hub heartbeat
   */
  handleHubHeartbeat(groupId: GroupId, _memberCount: number, _timestamp: number): void {
    const health = this.hubHealth.get(groupId);
    if (health) {
      health.lastHeartbeat = Date.now();
      health.missedHeartbeats = 0;
    } else {
      this.hubHealth.set(groupId, { lastHeartbeat: Date.now(), missedHeartbeats: 0 });
    }
  }

  // ============================================
  // Group Creation
  // ============================================

  /**
   * Create a new group.
   * Returns GroupInfo to be sent to the hub relay.
   */
  createGroup(
    name: string,
    hubRelayId: NodeId,
    initialMembers: { nodeId: NodeId; username: string }[] = [],
  ): GroupInfo | null {
    if (this.groups.size >= this.maxGroups) {
      console.warn('[GroupManager] Max groups reached, cannot create new group');
      return null;
    }

    const now = Date.now();
    const groupId = this.generateGroupId();

    // Creator is always admin
    const members: GroupMember[] = [
      {
        nodeId: this.localNodeId,
        username: this.localUsername,
        joinedAt: now,
        role: 'admin',
      },
    ];

    // Add initial members as regular members (they'll need to accept)
    // For now, just track them as pending

    const group: GroupInfo = {
      groupId,
      name,
      hubRelayId,
      members,
      createdBy: this.localNodeId,
      createdAt: now,
      lastActivityAt: now,
      maxMembers: DEFAULT_MAX_GROUP_MEMBERS,
    };

    this.groups.set(groupId, group);
    this.messageHistory.set(groupId, []);

    return group;
  }

  /**
   * Handle group creation confirmation from hub
   */
  handleGroupCreated(group: GroupInfo): void {
    // Update our local state with hub's confirmed version
    this.groups.set(group.groupId, group);
    if (!this.messageHistory.has(group.groupId)) {
      this.messageHistory.set(group.groupId, []);
    }
    this.events.onGroupCreated?.(group);
  }

  // ============================================
  // Invitations
  // ============================================

  /**
   * Handle incoming group invitation
   */
  handleInvite(
    groupId: GroupId,
    groupName: string,
    inviterId: NodeId,
    inviterUsername: string,
    hubRelayId: NodeId,
  ): void {
    // Skip if already have a pending invite for this group
    if (this.pendingInvites.has(groupId)) {
      return;
    }
    // Skip if already a member
    if (this.groups.has(groupId)) {
      return;
    }
    // Store pending invite
    this.pendingInvites.set(groupId, { groupName, inviterId, inviterUsername, hubRelayId });
    this.events.onGroupInvite?.(groupId, groupName, inviterId, inviterUsername);
  }

  /**
   * Accept a pending invitation
   * Returns true if accepted, false if invite not found
   */
  acceptInvite(groupId: GroupId): boolean {
    if (!this.pendingInvites.has(groupId)) {
      return false;
    }
    if (this.groups.size >= this.maxGroups) {
      console.warn('[GroupManager] Max groups reached, cannot accept invite');
      return false;
    }

    // Note: Don't delete from pendingInvites here - wait for group-sync confirmation
    // This allows retry if the join request fails
    return true;
  }

  /**
   * Decline a pending invitation
   */
  declineInvite(groupId: GroupId): boolean {
    return this.pendingInvites.delete(groupId);
  }

  /**
   * Get pending invitations
   */
  getPendingInvites(): Array<{
    groupId: GroupId;
    groupName: string;
    inviterId: NodeId;
    inviterUsername: string;
    hubRelayId: NodeId;
  }> {
    return Array.from(this.pendingInvites.entries()).map(([groupId, info]) => ({
      groupId,
      ...info,
    }));
  }

  /**
   * Update the hub relay ID for a pending invite.
   * Used when the original hub is no longer available.
   * Returns true if updated, false if invite not found.
   */
  updateInviteHub(groupId: GroupId, newHubRelayId: NodeId): boolean {
    const invite = this.pendingInvites.get(groupId);
    if (!invite) {
      return false;
    }
    invite.hubRelayId = newHubRelayId;
    return true;
  }

  // ============================================
  // Membership
  // ============================================

  /**
   * Handle group sync (joining a group or reconnecting)
   */
  handleGroupSync(group: GroupInfo, recentMessages?: GroupMessagePayload[]): void {
    this.groups.set(group.groupId, group);

    // Remove from pending invites - join is now confirmed
    this.pendingInvites.delete(group.groupId);

    // Store recent messages
    if (recentMessages) {
      const messages = this.messageHistory.get(group.groupId) ?? [];
      // Merge and dedupe by messageId
      const existingIds = new Set(messages.map((m) => m.messageId));
      for (const msg of recentMessages) {
        if (!existingIds.has(msg.messageId)) {
          messages.push(msg);
        }
      }
      // Sort by sentAt and trim to max
      messages.sort((a, b) => a.sentAt - b.sentAt);
      if (messages.length > this.maxMessagesPerGroup) {
        messages.splice(0, messages.length - this.maxMessagesPerGroup);
      }
      this.messageHistory.set(group.groupId, messages);
    } else {
      if (!this.messageHistory.has(group.groupId)) {
        this.messageHistory.set(group.groupId, []);
      }
    }

    this.events.onGroupCreated?.(group);
  }

  /**
   * Handle member joined event
   */
  handleMemberJoined(groupId: GroupId, member: GroupMember): void {
    const group = this.groups.get(groupId);
    if (!group) return;

    // Check if member already exists
    const existingIndex = group.members.findIndex((m) => m.nodeId === member.nodeId);
    if (existingIndex >= 0) {
      group.members[existingIndex] = member;
    } else {
      group.members.push(member);
    }

    group.lastActivityAt = Date.now();
    this.events.onMemberJoined?.(groupId, member);
  }

  /**
   * Handle member left event
   */
  handleMemberLeft(groupId: GroupId, nodeId: NodeId, username: string, reason: string): void {
    const group = this.groups.get(groupId);
    if (!group) return;

    // If it's us leaving, remove the group
    if (nodeId === this.localNodeId) {
      this.groups.delete(groupId);
      this.messageHistory.delete(groupId);
      this.events.onGroupRemoved?.(groupId, reason);
      return;
    }

    // Remove the member
    group.members = group.members.filter((m) => m.nodeId !== nodeId);
    group.lastActivityAt = Date.now();
    this.events.onMemberLeft?.(groupId, nodeId, username, reason);
  }

  /**
   * Leave a group
   */
  leaveGroup(groupId: GroupId): boolean {
    const group = this.groups.get(groupId);
    if (!group) return false;

    // Will be fully removed when we receive member-left confirmation
    // For now, just mark as pending leave
    return true;
  }

  // ============================================
  // Messages
  // ============================================

  /**
   * Handle incoming group message
   */
  handleMessage(message: GroupMessagePayload): void {
    const group = this.groups.get(message.groupId);
    if (!group) {
      console.warn('[GroupManager] Received message for unknown group:', message.groupId);
      return;
    }

    // Store message
    const messages = this.messageHistory.get(message.groupId) ?? [];

    // Dedupe by messageId
    if (messages.some((m) => m.messageId === message.messageId)) {
      return; // Already have this message
    }

    messages.push(message);

    // Trim to max
    if (messages.length > this.maxMessagesPerGroup) {
      messages.shift();
    }

    this.messageHistory.set(message.groupId, messages);
    group.lastActivityAt = Date.now();

    this.events.onGroupMessage?.(message.groupId, message);
  }

  /**
   * Get messages for sync (for new members)
   */
  getMessagesForSync(groupId: GroupId): GroupMessagePayload[] {
    const messages = this.messageHistory.get(groupId) ?? [];
    // Return last N messages for sync
    return messages.slice(-MAX_SYNC_MESSAGES);
  }

  // ============================================
  // Hub Migration
  // ============================================

  /**
   * Handle hub migration
   */
  handleHubMigration(groupId: GroupId, newHubId: NodeId, oldHubId: NodeId): void {
    const group = this.groups.get(groupId);
    if (!group) return;

    group.backupHubId = group.hubRelayId;
    group.hubRelayId = newHubId;
    group.lastActivityAt = Date.now();

    this.events.onHubMigration?.(groupId, newHubId, oldHubId);
  }

  // ============================================
  // Queries
  // ============================================

  /**
   * Get all groups
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
   * Check if we're in a group
   */
  isInGroup(groupId: GroupId): boolean {
    return this.groups.has(groupId);
  }

  /**
   * Get group members
   */
  getGroupMembers(groupId: GroupId): GroupMember[] {
    return this.groups.get(groupId)?.members ?? [];
  }

  /**
   * Get group message history
   */
  getMessageHistory(groupId: GroupId): GroupMessagePayload[] {
    return this.messageHistory.get(groupId) ?? [];
  }

  /**
   * Check if a user is admin of a group
   */
  isAdmin(groupId: GroupId, nodeId?: NodeId): boolean {
    const checkNodeId = nodeId ?? this.localNodeId;
    const group = this.groups.get(groupId);
    if (!group) return false;
    const member = group.members.find((m) => m.nodeId === checkNodeId);
    return member?.role === 'admin';
  }

  // ============================================
  // Public Groups
  // ============================================

  /**
   * Handle a public group announcement
   */
  handleGroupAnnouncement(
    groupId: GroupId,
    groupName: string,
    hubRelayId: NodeId,
    memberCount: number,
    createdBy: NodeId,
    creatorUsername: string,
  ): void {
    // Don't track if we're already a member or if it's our own group
    if (this.groups.has(groupId)) {
      return;
    }

    const publicGroup: PublicGroupInfo = {
      groupId,
      groupName,
      hubRelayId,
      memberCount,
      createdBy,
      creatorUsername,
      announcedAt: Date.now(),
    };

    this.availableGroups.set(groupId, publicGroup);
    this.events.onPublicGroupAnnounced?.(publicGroup);
  }

  /**
   * Get all available public groups (not yet joined)
   */
  getAvailableGroups(): PublicGroupInfo[] {
    return Array.from(this.availableGroups.values());
  }

  /**
   * Remove a group from available list (after joining or if it disappears)
   */
  removeFromAvailable(groupId: GroupId): void {
    this.availableGroups.delete(groupId);
  }

  // ============================================
  // Private Helpers
  // ============================================

  private generateGroupId(): GroupId {
    return `grp-${secureRandomUUID()}`;
  }
}
