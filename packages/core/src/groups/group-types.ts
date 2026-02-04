/**
 * Group Chat Types (Story 4.6 - Group Messaging)
 *
 * Defines types for relay-based group messaging where
 * a relay node acts as a "temporary hub" for message fanout.
 *
 * @see architecture.md#ADR-010 for group chat design
 */

import type { NodeId } from '../identity/index.js';

// ============================================
// Group Identity
// ============================================

/** Unique identifier for a group */
export type GroupId = string;

/** Group membership info */
export interface GroupMember {
  nodeId: NodeId;
  username: string;
  joinedAt: number;
  /** Role within the group */
  role: 'admin' | 'member';
}

/** Group metadata */
export interface GroupInfo {
  groupId: GroupId;
  name: string;
  /** NodeId of the relay acting as hub */
  hubRelayId: NodeId;
  /** Backup hub in case primary fails */
  backupHubId?: NodeId;
  /** Group members */
  members: GroupMember[];
  /** Creator of the group */
  createdBy: NodeId;
  createdAt: number;
  /** Last activity timestamp */
  lastActivityAt: number;
  /** Max members allowed (prevents DoS) */
  maxMembers: number;
}

// ============================================
// Group Protocol Messages
// ============================================

/** Base group payload type */
export interface GroupPayloadBase {
  type: string;
  groupId: GroupId;
}

/** Create a new group */
export interface GroupCreatePayload extends GroupPayloadBase {
  type: 'group-create';
  name: string;
  /** Initial members to invite (creator is auto-included) */
  initialMembers: { nodeId: NodeId; username: string }[];
  /** Requested hub relay (optional, can be auto-selected) */
  requestedHub?: NodeId;
  maxMembers?: number;
}

/** Hub acknowledges group creation */
export interface GroupCreatedPayload extends GroupPayloadBase {
  type: 'group-created';
  /** Full group info for creator */
  groupInfo: GroupInfo;
}

/** Invite a member to the group */
export interface GroupInvitePayload extends GroupPayloadBase {
  type: 'group-invite';
  /** Who is being invited */
  inviteeId: NodeId;
  inviteeUsername: string;
  /** Who sent the invite */
  inviterId: NodeId;
  inviterUsername: string;
  /** Group name for display */
  groupName: string;
  /** Current member count */
  memberCount: number;
}

/** Accept group invitation */
export interface GroupJoinPayload extends GroupPayloadBase {
  type: 'group-join';
  /** Joiner's info */
  nodeId: NodeId;
  username: string;
}

/** Member joined notification (broadcast to group) */
export interface GroupMemberJoinedPayload extends GroupPayloadBase {
  type: 'group-member-joined';
  member: GroupMember;
}

/** Leave a group */
export interface GroupLeavePayload extends GroupPayloadBase {
  type: 'group-leave';
  nodeId: NodeId;
  reason?: 'voluntary' | 'kicked' | 'timeout';
}

/** Member left notification (broadcast to group) */
export interface GroupMemberLeftPayload extends GroupPayloadBase {
  type: 'group-member-left';
  nodeId: NodeId;
  username: string;
  reason: 'voluntary' | 'kicked' | 'timeout';
}

/** Group chat message */
export interface GroupMessagePayload extends GroupPayloadBase {
  type: 'group-message';
  /** Unique message ID */
  messageId: string;
  /** Sender info */
  senderId: NodeId;
  senderUsername: string;
  /** Message content */
  text: string;
  /** Timestamp when sent */
  sentAt: number;
  /** Ed25519 signature of payload (hex encoded) - for authentication */
  signature?: string;
  /** Random nonce for anti-replay (combined with messageId) */
  nonce?: string;
}

/** Sync group state (for new members or reconnecting) */
export interface GroupSyncPayload extends GroupPayloadBase {
  type: 'group-sync';
  /** Full group info */
  groupInfo: GroupInfo;
  /** Recent messages (last N) */
  recentMessages?: GroupMessagePayload[];
}

/** Hub migration (when current hub is failing) */
export interface GroupHubMigrationPayload extends GroupPayloadBase {
  type: 'group-hub-migration';
  /** New hub relay ID */
  newHubId: NodeId;
  /** Old hub relay ID */
  oldHubId: NodeId;
  /** Reason for migration */
  reason: 'failure' | 'capacity' | 'manual';
}

/** Delivery confirmation for group messages */
export interface GroupDeliveryAckPayload extends GroupPayloadBase {
  type: 'group-delivery-ack';
  /** Original message ID */
  messageId: string;
  /** Who received it */
  receiverId: NodeId;
  /** Delivery timestamp */
  deliveredAt: number;
}

/** Read receipt for group messages */
export interface GroupReadReceiptPayload extends GroupPayloadBase {
  type: 'group-read-receipt';
  /** Message IDs that were read */
  messageIds: string[];
  /** Who read them */
  readerId: NodeId;
  /** Read timestamp */
  readAt: number;
}

// ============================================
// Union Type
// ============================================

export type GroupPayload =
  | GroupCreatePayload
  | GroupCreatedPayload
  | GroupInvitePayload
  | GroupJoinPayload
  | GroupMemberJoinedPayload
  | GroupLeavePayload
  | GroupMemberLeftPayload
  | GroupMessagePayload
  | GroupSyncPayload
  | GroupHubMigrationPayload
  | GroupDeliveryAckPayload
  | GroupReadReceiptPayload
  | GroupHubHeartbeatPayload;

// ============================================
// Type Guards
// ============================================

/** Valid group payload types */
const GROUP_PAYLOAD_TYPES = [
  'group-create',
  'group-created',
  'group-invite',
  'group-join',
  'group-member-joined',
  'group-leave',
  'group-member-left',
  'group-message',
  'group-sync',
  'group-hub-migration',
  'group-delivery-ack',
  'group-read-receipt',
  'group-hub-heartbeat',
] as const;

export function isGroupPayload(payload: unknown): payload is GroupPayload {
  if (!payload || typeof payload !== 'object') return false;
  const p = payload as Record<string, unknown>;
  if (typeof p.type !== 'string') return false;
  if (typeof p.groupId !== 'string' || p.groupId.length === 0) return false;
  return GROUP_PAYLOAD_TYPES.includes(p.type as (typeof GROUP_PAYLOAD_TYPES)[number]);
}

export function isGroupMessage(payload: unknown): payload is GroupMessagePayload {
  if (!isGroupPayload(payload)) return false;
  if (payload.type !== 'group-message') return false;
  const p = payload as GroupMessagePayload;
  return (
    typeof p.messageId === 'string' &&
    typeof p.senderId === 'string' &&
    typeof p.senderUsername === 'string' &&
    typeof p.text === 'string' &&
    typeof p.sentAt === 'number'
  );
}

export function isGroupInvite(payload: unknown): payload is GroupInvitePayload {
  if (!isGroupPayload(payload)) return false;
  if (payload.type !== 'group-invite') return false;
  const p = payload as GroupInvitePayload;
  return typeof p.inviteeId === 'string' && typeof p.inviterId === 'string' && typeof p.groupName === 'string';
}

export function isGroupCreate(payload: unknown): payload is GroupCreatePayload {
  if (!isGroupPayload(payload)) return false;
  if (payload.type !== 'group-create') return false;
  const p = payload as GroupCreatePayload;
  return typeof p.name === 'string' && Array.isArray(p.initialMembers);
}

export function isGroupSync(payload: unknown): payload is GroupSyncPayload {
  if (!isGroupPayload(payload)) return false;
  if (payload.type !== 'group-sync') return false;
  const p = payload as GroupSyncPayload;
  return p.groupInfo !== undefined && typeof p.groupInfo.groupId === 'string';
}

export function isGroupHubMigration(payload: unknown): payload is GroupHubMigrationPayload {
  if (!isGroupPayload(payload)) return false;
  if (payload.type !== 'group-hub-migration') return false;
  const p = payload as GroupHubMigrationPayload;
  return typeof p.newHubId === 'string' && typeof p.oldHubId === 'string' && typeof p.reason === 'string';
}

/** Hub heartbeat message */
export interface GroupHubHeartbeatPayload extends GroupPayloadBase {
  type: 'group-hub-heartbeat';
  /** Hub's current member count for this group */
  memberCount: number;
  /** Hub timestamp */
  timestamp: number;
}

export function isGroupHubHeartbeat(payload: unknown): payload is GroupHubHeartbeatPayload {
  if (!isGroupPayload(payload)) return false;
  if (payload.type !== 'group-hub-heartbeat') return false;
  const p = payload as GroupHubHeartbeatPayload;
  return typeof p.memberCount === 'number' && typeof p.timestamp === 'number';
}

// ============================================
// Constants
// ============================================

/** Default max members per group */
export const DEFAULT_MAX_GROUP_MEMBERS = 50;

/** Max recent messages to sync for new members */
export const MAX_SYNC_MESSAGES = 100;

/** Group message rate limit (messages per second) */
export const GROUP_RATE_LIMIT_PER_SECOND = 5;

/** Hub heartbeat interval (ms) */
export const HUB_HEARTBEAT_INTERVAL_MS = 30_000;

/** Hub failure threshold (missed heartbeats) */
export const HUB_FAILURE_THRESHOLD = 3;
