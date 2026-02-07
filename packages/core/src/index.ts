export const TOM_PROTOCOL_VERSION = '0.0.1';

export {
  IdentityManager,
  generateKeypair,
  signData,
  verifySignature,
  publicKeyToNodeId,
  MemoryStorage,
  LocalStorageAdapter,
  FileStorageAdapter,
} from './identity/index.js';
export type { NodeIdentity, NodeId, IdentityStorage } from './identity/index.js';

export type { MessageEnvelope } from './types/index.js';
export type { TomEventMap } from './types/index.js';

// Metrics types (Golden path measurement - inversion property validation)
export type { MessageLatency, MessagePath, NetworkMetrics, MetricsEvents } from './types/index.js';

// Path visualization (Story 4.3 - FR14)
export type { PathInfo } from './types/index.js';
export { extractPathInfo, formatLatency } from './types/index.js';

export { TomError } from './errors/index.js';
export type { TomErrorCode } from './errors/index.js';

export { TransportLayer, DirectPathManager } from './transport/index.js';
export type {
  PeerConnection,
  TransportEvents,
  SignalingClient,
  DirectPathEvents,
  ConnectionType,
} from './transport/index.js';

export {
  Router,
  RelaySelector,
  RelayStats,
  MessageTracker,
  OfflineDetector,
  ACK_TYPE,
  READ_RECEIPT_TYPE,
} from './routing/index.js';
export type {
  RouterEvents,
  SignatureVerifier,
  RelaySelectionResult,
  RelaySelectorOptions,
  RelayStatsData,
  RelayStatsEvents,
  RelayStatsOptions,
  MessageStatus,
  MessageStatusEntry,
  MessageStatusTimestamps,
  MessageTrackerEvents,
  AckType,
  AckPayload,
  OfflineDetectorEvents,
  OfflinePeerInfo,
} from './routing/index.js';

export { NetworkTopology } from './discovery/index.js';
export type { PeerInfo, PeerStatus, NodeRole } from './discovery/index.js';
export { HeartbeatManager } from './discovery/index.js';
export type { HeartbeatEvents, HeartbeatSender } from './discovery/index.js';
// Peer Gossip (Story 7.1 - Bootstrap Fade)
export { PeerGossip, isPeerGossipMessage } from './discovery/index.js';
export type {
  GossipPeerInfo,
  PeerGossipMessage,
  PeerGossipEvents,
  PeerGossipConfig,
} from './discovery/index.js';
// Ephemeral Subnets (Story 7.2 - Sliding Genesis)
export { EphemeralSubnetManager } from './discovery/index.js';
export type {
  SubnetInfo,
  CommunicationEdge,
  SubnetEvents,
  SubnetConfig,
} from './discovery/index.js';

export { RoleManager } from './roles/index.js';
export type { RoleAssignment, RoleManagerEvents, NodeMetrics } from './roles/index.js';

// Backup module (ADR-009 - Message Backup & Survival)
export {
  BackupStore,
  MAX_TTL_MS,
  DEFAULT_TTL_MS,
  MessageViability,
  REPLICATION_THRESHOLD,
  DELETION_THRESHOLD,
  BackupReplicator,
  BACKUP_REPLICATION_TYPE,
  BACKUP_REPLICATION_ACK_TYPE,
  BackupCoordinator,
  PENDING_QUERY_TYPE,
  PENDING_RESPONSE_TYPE,
  RECEIVED_CONFIRMATION_TYPE,
} from './backup/index.js';
export type {
  BackedUpMessage,
  BackupStoreEvents,
  BackupStoreOptions,
  ViabilityFactors,
  MessageViabilityEvents,
  ReplicationPayload,
  ReplicationAckPayload,
  BackupReplicatorEvents,
  PendingQueryPayload,
  PendingResponsePayload,
  ReceivedConfirmationPayload,
  BackupCoordinatorEvents,
} from './backup/index.js';

// Bootstrap module (ADR-002) — interface boundary for future DHT replacement
export type {
  BootstrapPeer,
  BootstrapEvents,
  BootstrapConfig,
  BootstrapMechanism,
  BootstrapFactory,
} from './bootstrap/index.js';

// Group module (Story 4.6 - Group Messaging via Relay Hub)
export { GroupManager, GroupHub } from './groups/index.js';
export type {
  GroupId,
  GroupInfo,
  GroupMember,
  GroupPayload,
  GroupPayloadBase,
  GroupCreatePayload,
  GroupCreatedPayload,
  GroupInvitePayload,
  GroupInviteAckPayload,
  GroupJoinPayload,
  GroupMemberJoinedPayload,
  GroupLeavePayload,
  GroupMemberLeftPayload,
  GroupMessagePayload,
  GroupSyncPayload,
  GroupHubMigrationPayload,
  GroupHubHeartbeatPayload,
  GroupDeliveryAckPayload,
  GroupReadReceiptPayload,
  GroupAnnouncementPayload,
  GroupManagerEvents,
  GroupManagerOptions,
  GroupHubEvents,
  GroupHubOptions,
  PublicGroupInfo,
} from './groups/index.js';
export {
  isGroupPayload,
  isGroupMessage,
  isGroupInvite,
  isGroupInviteAck,
  isGroupCreate,
  isGroupSync,
  isGroupHubMigration,
  isGroupHubHeartbeat,
  isGroupAnnouncement,
  DEFAULT_MAX_GROUP_MEMBERS,
  MAX_SYNC_MESSAGES,
  GROUP_RATE_LIMIT_PER_SECOND,
  HUB_HEARTBEAT_INTERVAL_MS,
  HUB_FAILURE_THRESHOLD,
  INVITE_TTL_MS,
  INVITE_MAX_RETRIES,
  INVITE_RETRY_DELAY_MS,
  // Security
  generateNonce,
  NonceTracker,
  signGroupMessage,
  verifyGroupMessageSignature,
  GROUP_SECURITY_DEFAULTS,
} from './groups/index.js';
export type { GroupMigrationData, GroupSecurityConfig } from './groups/index.js';

// Crypto module (Story 6.1 - End-to-End Encryption)
export {
  generateEncryptionKeypair,
  encryptionKeyToHex,
  hexToEncryptionKey,
  encryptPayload,
  decryptPayload,
  isEncryptedPayload,
  storeEncryptionKeypair,
  loadEncryptionKeypair,
  getOrCreateEncryptionKeypair,
} from './crypto/index.js';
export type { EncryptionKeypair, EncryptedPayload } from './crypto/index.js';
