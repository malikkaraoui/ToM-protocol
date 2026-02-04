import type { NodeId } from '../identity/index.js';
import type { MessageEnvelope } from '../types/envelope.js';
import type { BackupStore } from './backup-store.js';

/** Envelope type for pending messages query */
export const PENDING_QUERY_TYPE = 'pending-messages-query';

/** Envelope type for pending messages response */
export const PENDING_RESPONSE_TYPE = 'pending-messages-response';

/** Envelope type for received confirmation (clears backups) */
export const RECEIVED_CONFIRMATION_TYPE = 'received-confirmation';

/** Timeout for query responses (30 seconds) */
const QUERY_TIMEOUT_MS = 30 * 1000;

/** Query payload */
export interface PendingQueryPayload {
  /** Unique query ID for correlation */
  queryId: string;
  /** The recipient requesting their pending messages */
  recipientId: NodeId;
}

/** Response payload */
export interface PendingResponsePayload {
  /** Query ID for correlation (must match the original query) */
  queryId: string;
  /** The recipient these messages are for */
  recipientId: NodeId;
  /** Messages pending for the recipient */
  messages: MessageEnvelope[];
  /** IDs of all messages (for deduplication) */
  messageIds: string[];
}

/** Active query tracking */
interface ActiveQuery {
  queryId: string;
  recipientId: NodeId;
  startedAt: number;
}

/** Confirmation payload */
export interface ReceivedConfirmationPayload {
  /** Message IDs that were received */
  messageIds: string[];
  /** The recipient who received them */
  recipientId: NodeId;
}

export interface BackupCoordinatorEvents {
  /** Request to send an envelope to a peer */
  sendEnvelope: (targetNodeId: NodeId, envelope: MessageEnvelope) => void;
  /** Request to broadcast to all backup nodes */
  broadcastToBackups: (envelope: MessageEnvelope, excludeNodeId?: NodeId) => void;
  /** Emitted when pending messages are received */
  onPendingMessagesReceived: (messages: MessageEnvelope[]) => void;
  /** Emitted when a message backup is cleared due to delivery confirmation */
  onBackupCleared: (messageId: string) => void;
}

/**
 * BackupCoordinator - Coordinates message backup, query, and confirmation across the network.
 *
 * Responsibilities:
 * - Query for pending messages when a recipient comes online
 * - Respond to queries from other nodes
 * - Propagate received confirmations to clear backups
 * - Deduplicate messages from multiple backup sources
 */
export class BackupCoordinator {
  private events: BackupCoordinatorEvents;
  private backupStore: BackupStore;
  private selfNodeId: NodeId;
  private receivedMessageIds = new Set<string>();
  private pendingQueries = new Map<NodeId, number>(); // recipient -> last query timestamp
  private activeQueries = new Map<string, ActiveQuery>(); // queryId -> ActiveQuery
  private queryCounter = 0;

  constructor(events: BackupCoordinatorEvents, backupStore: BackupStore, selfNodeId: NodeId) {
    this.events = events;
    this.backupStore = backupStore;
    this.selfNodeId = selfNodeId;
  }

  /** Generate a unique query ID */
  private generateQueryId(): string {
    return `${this.selfNodeId.slice(0, 8)}-${Date.now()}-${++this.queryCounter}`;
  }

  /** Clean up expired active queries */
  private cleanupExpiredQueries(): void {
    const now = Date.now();
    for (const [queryId, query] of this.activeQueries) {
      if (now - query.startedAt > QUERY_TIMEOUT_MS) {
        this.activeQueries.delete(queryId);
        console.log(`[BackupCoordinator] Query ${queryId} timed out`);
      }
    }
  }

  /**
   * Query the network for pending messages for a recipient.
   * Called when a previously-offline peer comes back online.
   *
   * SECURITY: Only the recipient themselves should call this to query their own messages.
   * The recipientId in the query is validated against envelope.from in handlePendingQuery.
   */
  queryPendingMessages(recipientId: NodeId): string | null {
    // Clean up any expired queries first
    this.cleanupExpiredQueries();

    // Debounce queries (don't query more than once per 5 seconds)
    const lastQuery = this.pendingQueries.get(recipientId) || 0;
    if (Date.now() - lastQuery < 5000) {
      return null;
    }
    this.pendingQueries.set(recipientId, Date.now());

    const queryId = this.generateQueryId();
    const payload: PendingQueryPayload = { queryId, recipientId };

    // Track active query for correlation
    this.activeQueries.set(queryId, {
      queryId,
      recipientId,
      startedAt: Date.now(),
    });

    const queryEnvelope: MessageEnvelope = {
      id: `pending-query-${recipientId}-${Date.now()}`,
      from: this.selfNodeId,
      to: '', // Broadcast
      via: [],
      type: PENDING_QUERY_TYPE,
      payload,
      timestamp: Date.now(),
      signature: '',
    };

    console.log(`[BackupCoordinator] Querying pending messages for ${recipientId.slice(0, 8)} (queryId=${queryId})`);
    this.events.broadcastToBackups(queryEnvelope);
    return queryId;
  }

  /**
   * Handle incoming pending messages query.
   * If we have messages for the recipient, send them.
   *
   * SECURITY: Basic auth check - only respond if the requester (envelope.from)
   * matches the recipientId in the query. This prevents arbitrary nodes from
   * querying messages for other recipients (metadata exfiltration).
   */
  handlePendingQuery(envelope: MessageEnvelope): void {
    if (envelope.type !== PENDING_QUERY_TYPE) return;

    const payload = envelope.payload as PendingQueryPayload;
    const { queryId, recipientId } = payload;

    // SECURITY: Verify requester is the recipient (prevents exfiltration)
    if (envelope.from !== recipientId) {
      console.log(
        `[BackupCoordinator] Rejecting query from ${envelope.from.slice(0, 8)} for ${recipientId.slice(0, 8)} - auth mismatch`,
      );
      return;
    }

    // Get messages we have stored for this recipient
    const storedMessages = this.backupStore.getMessagesForRecipient(recipientId);
    if (storedMessages.length === 0) {
      return;
    }

    // Send the messages back with correlation IDs
    const responsePayload: PendingResponsePayload = {
      queryId,
      recipientId,
      messages: storedMessages.map((m) => m.envelope),
      messageIds: storedMessages.map((m) => m.envelope.id),
    };

    const responseEnvelope: MessageEnvelope = {
      id: `pending-response-${recipientId}-${Date.now()}`,
      from: this.selfNodeId,
      to: envelope.from,
      via: [],
      type: PENDING_RESPONSE_TYPE,
      payload: responsePayload,
      timestamp: Date.now(),
      signature: '',
    };

    console.log(
      `[BackupCoordinator] Responding with ${storedMessages.length} pending messages for ${recipientId.slice(0, 8)} (queryId=${queryId})`,
    );
    this.events.sendEnvelope(envelope.from, responseEnvelope);
  }

  /**
   * Handle incoming pending messages response.
   * Deduplicate and deliver to the recipient.
   *
   * SECURITY: Validates that the response correlates to an active query we sent.
   * This prevents replay attacks and injection of messages for other recipients.
   */
  handlePendingResponse(envelope: MessageEnvelope): MessageEnvelope[] {
    if (envelope.type !== PENDING_RESPONSE_TYPE) return [];

    const payload = envelope.payload as PendingResponsePayload;
    const { queryId, recipientId } = payload;

    // Clean up expired queries
    this.cleanupExpiredQueries();

    // SECURITY: Verify this response correlates to an active query we sent
    const activeQuery = this.activeQueries.get(queryId);
    if (!activeQuery) {
      console.log(`[BackupCoordinator] Ignoring response for unknown/expired queryId=${queryId}`);
      return [];
    }

    // SECURITY: Verify the recipientId matches what we queried
    if (activeQuery.recipientId !== recipientId) {
      console.log(
        `[BackupCoordinator] Ignoring response with mismatched recipientId (expected=${activeQuery.recipientId.slice(0, 8)}, got=${recipientId.slice(0, 8)})`,
      );
      return [];
    }

    const newMessages: MessageEnvelope[] = [];

    for (const msg of payload.messages) {
      // SECURITY: Verify message is actually for the recipient we queried
      if (msg.to !== recipientId) {
        console.log(`[BackupCoordinator] Skipping message ${msg.id.slice(0, 8)} - not for queried recipient`);
        continue;
      }

      // Deduplicate - skip if already received
      if (this.receivedMessageIds.has(msg.id)) {
        continue;
      }
      this.receivedMessageIds.add(msg.id);
      newMessages.push(msg);
    }

    if (newMessages.length > 0) {
      console.log(
        `[BackupCoordinator] Received ${newMessages.length} new pending messages (${payload.messages.length - newMessages.length} filtered) for queryId=${queryId}`,
      );
      this.events.onPendingMessagesReceived(newMessages);
    }

    return newMessages;
  }

  /**
   * Broadcast that messages were received, so backup nodes can clear them.
   */
  confirmMessagesReceived(messageIds: string[], recipientId: NodeId): void {
    if (messageIds.length === 0) return;

    const payload: ReceivedConfirmationPayload = {
      messageIds,
      recipientId,
    };

    const confirmEnvelope: MessageEnvelope = {
      id: `received-confirm-${Date.now()}`,
      from: this.selfNodeId,
      to: '', // Broadcast
      via: [],
      type: RECEIVED_CONFIRMATION_TYPE,
      payload,
      timestamp: Date.now(),
      signature: '',
    };

    console.log(`[BackupCoordinator] Broadcasting received confirmation for ${messageIds.length} messages`);
    this.events.broadcastToBackups(confirmEnvelope);
  }

  /**
   * Handle incoming received confirmation.
   * Clear our backup copies of these messages.
   */
  handleReceivedConfirmation(envelope: MessageEnvelope): void {
    if (envelope.type !== RECEIVED_CONFIRMATION_TYPE) return;

    const payload = envelope.payload as ReceivedConfirmationPayload;

    for (const messageId of payload.messageIds) {
      if (this.backupStore.hasMessage(messageId)) {
        this.backupStore.markDelivered(messageId);
        console.log(`[BackupCoordinator] Cleared backup of ${messageId.slice(0, 8)} due to delivery confirmation`);
        this.events.onBackupCleared(messageId);
      }
    }
  }

  /**
   * Handle any incoming envelope related to backup coordination.
   * Returns true if the envelope was handled.
   */
  handleEnvelope(envelope: MessageEnvelope): boolean {
    switch (envelope.type) {
      case PENDING_QUERY_TYPE:
        this.handlePendingQuery(envelope);
        return true;
      case PENDING_RESPONSE_TYPE:
        this.handlePendingResponse(envelope);
        return true;
      case RECEIVED_CONFIRMATION_TYPE:
        this.handleReceivedConfirmation(envelope);
        return true;
      default:
        return false;
    }
  }

  /**
   * Check if an envelope is backup-related.
   */
  isBackupEnvelope(envelope: MessageEnvelope): boolean {
    return [PENDING_QUERY_TYPE, PENDING_RESPONSE_TYPE, RECEIVED_CONFIRMATION_TYPE].includes(envelope.type);
  }

  /**
   * Clear deduplication cache for a recipient (called after some time to free memory).
   */
  clearDeduplicationCache(): void {
    this.receivedMessageIds.clear();
  }

  /**
   * Get count of deduplicated messages (for debugging/stats).
   */
  getDeduplicationCacheSize(): number {
    return this.receivedMessageIds.size;
  }
}
