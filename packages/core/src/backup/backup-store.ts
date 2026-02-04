import type { NodeId } from '../identity/index.js';
import type { MessageEnvelope } from '../types/envelope.js';

/** Maximum TTL for backed-up messages (24 hours per NFR5) */
export const MAX_TTL_MS = 24 * 60 * 60 * 1000;

/** Default TTL if not specified (24 hours) */
export const DEFAULT_TTL_MS = MAX_TTL_MS;

/** Cleanup interval (1 minute) */
const CLEANUP_INTERVAL_MS = 60 * 1000;

/** Information about a backed-up message */
export interface BackedUpMessage {
  /** The message envelope */
  envelope: MessageEnvelope;
  /** Recipient ID this message is for */
  recipientId: NodeId;
  /** When the message was stored */
  storedAt: number;
  /** TTL in ms (max 24h) */
  ttlMs: number;
  /** Current viability score (0-100) */
  viabilityScore: number;
  /** Node IDs that also have copies of this message */
  replicatedTo: Set<NodeId>;
}

export interface BackupStoreEvents {
  onMessageStored: (messageId: string, recipientId: NodeId) => void;
  onMessageExpired: (messageId: string, recipientId: NodeId) => void;
  onMessageDelivered: (messageId: string, recipientId: NodeId) => void;
  onViabilityChanged: (messageId: string, oldScore: number, newScore: number) => void;
}

export interface BackupStoreOptions {
  /** If true, automatically starts the cleanup timer. Default: true */
  autoStart?: boolean;
}

/**
 * BackupStore - In-memory storage for messages awaiting offline recipients.
 *
 * Per ADR-009 "Virus Metaphor":
 * - Memory-only storage (no disk persistence)
 * - 24h max TTL
 * - Messages track their own viability score
 * - Proactive replication to better hosts
 *
 * IMPORTANT: Call stop() when done to prevent memory leaks from the cleanup timer.
 */
export class BackupStore {
  private messages = new Map<string, BackedUpMessage>();
  private byRecipient = new Map<NodeId, Set<string>>();
  private events: BackupStoreEvents;
  private cleanupTimer: ReturnType<typeof setInterval> | null = null;

  constructor(events: BackupStoreEvents, options: BackupStoreOptions = {}) {
    this.events = events;
    const { autoStart = true } = options;
    if (autoStart) {
      this.start();
    }
  }

  /** Start the background cleanup timer */
  start(): void {
    if (this.cleanupTimer) return;
    this.cleanupTimer = setInterval(() => {
      this.cleanupExpired();
    }, CLEANUP_INTERVAL_MS);
  }

  /** Stop the background cleanup timer */
  stop(): void {
    if (this.cleanupTimer) {
      clearInterval(this.cleanupTimer);
      this.cleanupTimer = null;
    }
  }

  /**
   * Store a message for an offline recipient.
   * Messages are stored encrypted - the envelope should already contain encrypted payload.
   */
  storeForRecipient(recipientId: NodeId, envelope: MessageEnvelope, ttlMs = DEFAULT_TTL_MS): void {
    // Enforce max TTL
    const actualTtl = Math.min(ttlMs, MAX_TTL_MS);

    // Check if message already stored
    if (this.messages.has(envelope.id)) {
      return;
    }

    const backedUp: BackedUpMessage = {
      envelope,
      recipientId,
      storedAt: Date.now(),
      ttlMs: actualTtl,
      viabilityScore: 100, // Start at maximum viability
      replicatedTo: new Set(),
    };

    this.messages.set(envelope.id, backedUp);

    // Track by recipient for efficient lookup
    let recipientMessages = this.byRecipient.get(recipientId);
    if (!recipientMessages) {
      recipientMessages = new Set();
      this.byRecipient.set(recipientId, recipientMessages);
    }
    recipientMessages.add(envelope.id);

    console.log(
      `[BackupStore] Stored message ${envelope.id.slice(0, 8)} for recipient ${recipientId.slice(0, 8)} (TTL=${actualTtl}ms)`,
    );

    this.events.onMessageStored(envelope.id, recipientId);
  }

  /** Get all messages pending for a specific recipient */
  getMessagesForRecipient(recipientId: NodeId): BackedUpMessage[] {
    const messageIds = this.byRecipient.get(recipientId);
    if (!messageIds) return [];

    const result: BackedUpMessage[] = [];
    for (const id of messageIds) {
      const msg = this.messages.get(id);
      if (msg) {
        result.push(msg);
      }
    }
    return result;
  }

  /** Get a specific message by ID */
  getMessage(messageId: string): BackedUpMessage | undefined {
    return this.messages.get(messageId);
  }

  /** Check if a message is stored */
  hasMessage(messageId: string): boolean {
    return this.messages.has(messageId);
  }

  /** Mark a message as delivered and remove it from backup */
  markDelivered(messageId: string): boolean {
    const msg = this.messages.get(messageId);
    if (!msg) return false;

    this.removeMessage(messageId);
    console.log(`[BackupStore] Message ${messageId.slice(0, 8)} delivered, removed from backup`);
    this.events.onMessageDelivered(messageId, msg.recipientId);
    return true;
  }

  /** Update the viability score of a message */
  updateViabilityScore(messageId: string, newScore: number): void {
    const msg = this.messages.get(messageId);
    if (!msg) return;

    const oldScore = msg.viabilityScore;
    msg.viabilityScore = Math.max(0, Math.min(100, newScore));

    if (oldScore !== msg.viabilityScore) {
      this.events.onViabilityChanged(messageId, oldScore, msg.viabilityScore);
    }
  }

  /** Record that a message has been replicated to another node */
  recordReplication(messageId: string, nodeId: NodeId): void {
    const msg = this.messages.get(messageId);
    if (msg) {
      msg.replicatedTo.add(nodeId);
    }
  }

  /** Get nodes that have copies of a message */
  getReplicatedNodes(messageId: string): NodeId[] {
    const msg = this.messages.get(messageId);
    return msg ? Array.from(msg.replicatedTo) : [];
  }

  /** Get all stored messages (for replication/viability checks) */
  getAllMessages(): BackedUpMessage[] {
    return Array.from(this.messages.values());
  }

  /** Get message count */
  get size(): number {
    return this.messages.size;
  }

  /** Get number of messages for a specific recipient */
  getRecipientMessageCount(recipientId: NodeId): number {
    const messageIds = this.byRecipient.get(recipientId);
    return messageIds ? messageIds.size : 0;
  }

  /** Check if a message has expired */
  isExpired(messageId: string): boolean {
    const msg = this.messages.get(messageId);
    if (!msg) return true;

    const elapsed = Date.now() - msg.storedAt;
    return elapsed >= msg.ttlMs;
  }

  /** Get remaining TTL for a message in ms */
  getRemainingTtl(messageId: string): number {
    const msg = this.messages.get(messageId);
    if (!msg) return 0;

    const elapsed = Date.now() - msg.storedAt;
    return Math.max(0, msg.ttlMs - elapsed);
  }

  /** Delete a message (e.g., when viability drops too low) */
  deleteMessage(messageId: string): boolean {
    const msg = this.messages.get(messageId);
    if (!msg) return false;

    this.removeMessage(messageId);
    console.log(`[BackupStore] Message ${messageId.slice(0, 8)} self-deleted (viability too low)`);
    return true;
  }

  /** Clean up expired messages */
  private cleanupExpired(): void {
    const now = Date.now();
    const toRemove: string[] = [];

    for (const [id, msg] of this.messages) {
      const elapsed = now - msg.storedAt;
      if (elapsed >= msg.ttlMs) {
        toRemove.push(id);
      }
    }

    for (const id of toRemove) {
      const msg = this.messages.get(id);
      if (msg) {
        this.removeMessage(id);
        // Log without message content per ADR-009
        console.log(`[BackupStore] Message ${id.slice(0, 8)} expired (TTL exceeded)`);
        this.events.onMessageExpired(id, msg.recipientId);
      }
    }
  }

  /** Remove a message from all tracking structures */
  private removeMessage(messageId: string): void {
    const msg = this.messages.get(messageId);
    if (!msg) return;

    // Remove from recipient index
    const recipientMessages = this.byRecipient.get(msg.recipientId);
    if (recipientMessages) {
      recipientMessages.delete(messageId);
      if (recipientMessages.size === 0) {
        this.byRecipient.delete(msg.recipientId);
      }
    }

    // Remove from main store
    this.messages.delete(messageId);
  }

  /** Clear all messages (for testing) */
  clear(): void {
    this.messages.clear();
    this.byRecipient.clear();
  }
}
