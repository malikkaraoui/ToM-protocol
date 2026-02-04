import type { NodeId } from '../identity/index.js';
import type { MessageEnvelope } from '../types/envelope.js';
import type { BackedUpMessage, BackupStore } from './backup-store.js';

/** Envelope type for backup replication requests */
export const BACKUP_REPLICATION_TYPE = 'backup-replication';

/** Envelope type for backup replication acknowledgment */
export const BACKUP_REPLICATION_ACK_TYPE = 'backup-replication-ack';

/** Replication request payload */
export interface ReplicationPayload {
  /** The original message envelope */
  envelope: MessageEnvelope;
  /** Original recipient */
  recipientId: NodeId;
  /** Absolute expiration timestamp (ms since epoch) - prevents TTL drift */
  expiresAt: number;
  /** Current viability score */
  viabilityScore: number;
  /** Nodes that already have copies */
  replicatedTo: string[];
}

/** Timeout for pending replication acknowledgments (30 seconds) */
const PENDING_REPLICATION_TIMEOUT_MS = 30 * 1000;

/** Interval for purging expired pending replications (10 seconds) */
const PENDING_PURGE_INTERVAL_MS = 10 * 1000;

/** Pending replication entry with timestamp */
interface PendingReplicationEntry {
  targetNodeId: NodeId;
  startedAt: number;
}

/** Replication acknowledgment payload */
export interface ReplicationAckPayload {
  /** The message ID that was replicated */
  messageId: string;
  /** Whether replication was accepted */
  accepted: boolean;
  /** Reason if not accepted */
  reason?: string;
}

export interface BackupReplicatorEvents {
  /** Emitted when a message is successfully replicated to another node */
  onMessageReplicated: (messageId: string, targetNodeId: NodeId) => void;
  /** Emitted when replication fails */
  onReplicationFailed: (messageId: string, targetNodeId: NodeId, reason: string) => void;
  /** Request to send an envelope to a peer */
  sendEnvelope: (targetNodeId: NodeId, envelope: MessageEnvelope) => void;
}

/**
 * BackupReplicator - Handles proactive message replication between backup nodes.
 *
 * Per ADR-009 "Virus Metaphor":
 * - Replication happens in parallel (fire-and-forget)
 * - Track which nodes have copies
 * - Messages replicate to better hosts proactively
 */
export class BackupReplicator {
  private events: BackupReplicatorEvents;
  private backupStore: BackupStore;
  private selfNodeId: NodeId;
  /** Map<messageId, Map<targetNodeId, PendingReplicationEntry>> */
  private pendingReplications = new Map<string, Map<NodeId, PendingReplicationEntry>>();
  private purgeInterval: ReturnType<typeof setInterval> | null = null;

  constructor(events: BackupReplicatorEvents, backupStore: BackupStore, selfNodeId: NodeId) {
    this.events = events;
    this.backupStore = backupStore;
    this.selfNodeId = selfNodeId;
  }

  /** Start the pending replication purge timer */
  start(): void {
    if (this.purgeInterval) return;
    this.purgeInterval = setInterval(() => {
      this.purgeExpiredPending();
    }, PENDING_PURGE_INTERVAL_MS);
  }

  /** Stop the pending replication purge timer */
  stop(): void {
    if (this.purgeInterval) {
      clearInterval(this.purgeInterval);
      this.purgeInterval = null;
    }
  }

  /** Purge expired pending replications to prevent memory leak */
  private purgeExpiredPending(): void {
    const now = Date.now();
    for (const [messageId, pending] of this.pendingReplications) {
      for (const [targetNodeId, entry] of pending) {
        if (now - entry.startedAt > PENDING_REPLICATION_TIMEOUT_MS) {
          pending.delete(targetNodeId);
          console.log(
            `[BackupReplicator] Pending replication of ${messageId.slice(0, 8)} to ${targetNodeId.slice(0, 8)} timed out`,
          );
          this.events.onReplicationFailed(messageId, targetNodeId, 'timeout');
        }
      }
      if (pending.size === 0) {
        this.pendingReplications.delete(messageId);
      }
    }
  }

  /** Cancel all pending replications for a message (e.g., when delivered) */
  cancelPendingReplications(messageId: string): void {
    this.pendingReplications.delete(messageId);
  }

  /**
   * Replicate a message to another backup node.
   * This is fire-and-forget - returns immediately without waiting for ACK.
   */
  replicateTo(messageId: string, targetNodeId: NodeId): void {
    const msg = this.backupStore.getMessage(messageId);
    if (!msg) {
      console.log(`[BackupReplicator] Cannot replicate ${messageId.slice(0, 8)} - not in store`);
      return;
    }

    // Don't replicate to self
    if (targetNodeId === this.selfNodeId) {
      return;
    }

    // Don't replicate if target already has it
    if (msg.replicatedTo.has(targetNodeId)) {
      console.log(
        `[BackupReplicator] Skip replicate ${messageId.slice(0, 8)} to ${targetNodeId.slice(0, 8)} - already has copy`,
      );
      return;
    }

    // Track pending replication
    let pending = this.pendingReplications.get(messageId);
    if (!pending) {
      pending = new Map();
      this.pendingReplications.set(messageId, pending);
    }

    if (pending.has(targetNodeId)) {
      // Already replicating to this node
      return;
    }
    pending.set(targetNodeId, { targetNodeId, startedAt: Date.now() });

    // Calculate absolute expiration timestamp (prevents TTL drift on multi-hop)
    const remainingTtl = this.backupStore.getRemainingTtl(messageId);
    const expiresAt = Date.now() + remainingTtl;

    // Create replication request envelope
    const payload: ReplicationPayload = {
      envelope: msg.envelope,
      recipientId: msg.recipientId,
      expiresAt,
      viabilityScore: msg.viabilityScore,
      replicatedTo: Array.from(msg.replicatedTo),
    };

    const replicationEnvelope: MessageEnvelope = {
      id: `repl-${messageId}-${Date.now()}`,
      from: this.selfNodeId,
      to: targetNodeId,
      via: [],
      type: BACKUP_REPLICATION_TYPE,
      payload,
      timestamp: Date.now(),
      signature: '', // Will be signed by transport layer
    };

    console.log(`[BackupReplicator] Replicating ${messageId.slice(0, 8)} to ${targetNodeId.slice(0, 8)}`);

    // Fire-and-forget
    this.events.sendEnvelope(targetNodeId, replicationEnvelope);
  }

  /**
   * Handle incoming replication request.
   * Returns true if the message was accepted.
   */
  handleReplicationRequest(envelope: MessageEnvelope): boolean {
    if (envelope.type !== BACKUP_REPLICATION_TYPE) {
      return false;
    }

    const payload = envelope.payload as ReplicationPayload;
    const originalEnvelope = payload.envelope;

    // Reject if message is already expired (prevents TTL drift attacks)
    const now = Date.now();
    if (payload.expiresAt <= now) {
      console.log(`[BackupReplicator] Rejecting replication of ${originalEnvelope.id.slice(0, 8)} - already expired`);
      this.sendReplicationAck(envelope.from, originalEnvelope.id, false, 'expired');
      return false;
    }

    // Check if we already have this message
    if (this.backupStore.hasMessage(originalEnvelope.id)) {
      console.log(`[BackupReplicator] Rejecting replication of ${originalEnvelope.id.slice(0, 8)} - already have it`);
      this.sendReplicationAck(envelope.from, originalEnvelope.id, false, 'already-stored');
      return false;
    }

    // Calculate remaining TTL from absolute expiration (prevents drift)
    const ttlMs = Math.max(0, payload.expiresAt - now);

    // Store the message
    this.backupStore.storeForRecipient(payload.recipientId, originalEnvelope, ttlMs);

    // Update viability score
    this.backupStore.updateViabilityScore(originalEnvelope.id, payload.viabilityScore);

    // Record existing replications
    for (const nodeId of payload.replicatedTo) {
      this.backupStore.recordReplication(originalEnvelope.id, nodeId);
    }

    // Also record the sender as having a copy
    this.backupStore.recordReplication(originalEnvelope.id, envelope.from);

    console.log(
      `[BackupReplicator] Accepted replication of ${originalEnvelope.id.slice(0, 8)} from ${envelope.from.slice(0, 8)}`,
    );

    // Send acknowledgment
    this.sendReplicationAck(envelope.from, originalEnvelope.id, true);
    return true;
  }

  /**
   * Handle incoming replication acknowledgment.
   */
  handleReplicationAck(envelope: MessageEnvelope): void {
    if (envelope.type !== BACKUP_REPLICATION_ACK_TYPE) {
      return;
    }

    const payload = envelope.payload as ReplicationAckPayload;
    const { messageId, accepted, reason } = payload;

    // Clear pending state
    const pending = this.pendingReplications.get(messageId);
    if (pending) {
      pending.delete(envelope.from);
      if (pending.size === 0) {
        this.pendingReplications.delete(messageId);
      }
    }

    // Check if message still exists (race condition: delivery vs replication)
    if (!this.backupStore.hasMessage(messageId)) {
      console.log(`[BackupReplicator] ACK for ${messageId.slice(0, 8)} but message no longer in store`);
      return;
    }

    if (accepted) {
      // Record successful replication
      this.backupStore.recordReplication(messageId, envelope.from);
      console.log(
        `[BackupReplicator] Replication of ${messageId.slice(0, 8)} to ${envelope.from.slice(0, 8)} confirmed`,
      );
      this.events.onMessageReplicated(messageId, envelope.from);
    } else {
      console.log(
        `[BackupReplicator] Replication of ${messageId.slice(0, 8)} to ${envelope.from.slice(0, 8)} rejected: ${reason}`,
      );
      this.events.onReplicationFailed(messageId, envelope.from, reason || 'unknown');
    }
  }

  /**
   * Replicate a message to multiple backup nodes in parallel.
   */
  replicateToMultiple(messageId: string, targetNodeIds: NodeId[]): void {
    for (const nodeId of targetNodeIds) {
      this.replicateTo(messageId, nodeId);
    }
  }

  /**
   * Get the nodes a message has been replicated to.
   */
  getReplicatedNodes(messageId: string): NodeId[] {
    return this.backupStore.getReplicatedNodes(messageId);
  }

  /**
   * Check if a message has been replicated to a specific node.
   */
  isReplicatedTo(messageId: string, nodeId: NodeId): boolean {
    return this.backupStore.getReplicatedNodes(messageId).includes(nodeId);
  }

  private sendReplicationAck(targetNodeId: NodeId, messageId: string, accepted: boolean, reason?: string): void {
    const payload: ReplicationAckPayload = {
      messageId,
      accepted,
      reason,
    };

    const ackEnvelope: MessageEnvelope = {
      id: `repl-ack-${messageId}-${Date.now()}`,
      from: this.selfNodeId,
      to: targetNodeId,
      via: [],
      type: BACKUP_REPLICATION_ACK_TYPE,
      payload,
      timestamp: Date.now(),
      signature: '',
    };

    this.events.sendEnvelope(targetNodeId, ackEnvelope);
  }
}
