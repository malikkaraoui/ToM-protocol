/**
 * Message status lifecycle tracking for delivery confirmation and read receipts.
 *
 * Status flow: pending → sent → relayed → delivered → read
 *
 * - pending: Message created, not yet handed to transport
 * - sent: Message handed to transport layer
 * - relayed: Relay ACK received (message forwarded by relay)
 * - delivered: Recipient ACK received (message arrived at recipient node)
 * - read: Read receipt received (recipient viewed the message)
 *
 * @see Story 4.2 - Delivery Confirmation & Read Receipts
 * @see architecture.md#ADR-003 - Wire Format
 */

export type MessageStatus = 'pending' | 'sent' | 'relayed' | 'delivered' | 'read';

/** Maximum number of messages to track (prevents DoS) */
const MAX_TRACKED_MESSAGES = 10000;

/** Maximum age for stuck messages (not read) before forced cleanup (24 hours) */
const MAX_STUCK_MESSAGE_AGE_MS = 24 * 60 * 60 * 1000;

export interface MessageStatusTimestamps {
  pending?: number;
  sent?: number;
  relayed?: number;
  delivered?: number;
  read?: number;
}

export interface MessageStatusEntry {
  messageId: string;
  to: string;
  status: MessageStatus;
  timestamps: MessageStatusTimestamps;
}

export interface MessageTrackerEvents {
  onStatusChanged: (messageId: string, previousStatus: MessageStatus, newStatus: MessageStatus) => void;
}

/** Status order for preventing regression */
const STATUS_ORDER: Record<MessageStatus, number> = {
  pending: 0,
  sent: 1,
  relayed: 2,
  delivered: 3,
  read: 4,
};

export class MessageTracker {
  private messages = new Map<string, MessageStatusEntry>();
  private events: MessageTrackerEvents;

  constructor(events: MessageTrackerEvents) {
    this.events = events;
  }

  /**
   * Start tracking a new message.
   * Initial status is 'pending'.
   * Returns false if message is already being tracked (prevents state regression).
   */
  track(messageId: string, to: string): boolean {
    // Prevent overwriting existing state (fixes race condition)
    if (this.messages.has(messageId)) {
      return false;
    }

    // Evict oldest messages if at capacity (DoS protection)
    if (this.messages.size >= MAX_TRACKED_MESSAGES) {
      this.evictOldest();
    }

    const entry: MessageStatusEntry = {
      messageId,
      to,
      status: 'pending',
      timestamps: {
        pending: Date.now(),
      },
    };
    this.messages.set(messageId, entry);
    return true;
  }

  /**
   * Evict the oldest message to make room for new ones.
   * Prefers evicting 'read' messages, then by oldest timestamp.
   */
  private evictOldest(): void {
    let oldestReadId: string | null = null;
    let oldestReadTime = Number.POSITIVE_INFINITY;
    let oldestAnyId: string | null = null;
    let oldestAnyTime = Number.POSITIVE_INFINITY;

    for (const [id, entry] of this.messages) {
      const time = entry.timestamps.pending ?? Number.POSITIVE_INFINITY;

      if (entry.status === 'read') {
        if (time < oldestReadTime) {
          oldestReadTime = time;
          oldestReadId = id;
        }
      }

      if (time < oldestAnyTime) {
        oldestAnyTime = time;
        oldestAnyId = id;
      }
    }

    // Prefer evicting read messages
    const toEvict = oldestReadId ?? oldestAnyId;
    if (toEvict) {
      this.messages.delete(toEvict);
    }
  }

  /**
   * Get the current status of a tracked message.
   */
  getStatus(messageId: string): MessageStatusEntry | undefined {
    return this.messages.get(messageId);
  }

  /**
   * Mark message as sent (handed to transport layer).
   */
  markSent(messageId: string): void {
    this.updateStatus(messageId, 'sent');
  }

  /**
   * Mark message as relayed (relay ACK received).
   */
  markRelayed(messageId: string): void {
    this.updateStatus(messageId, 'relayed');
  }

  /**
   * Mark message as delivered (recipient ACK received).
   */
  markDelivered(messageId: string): void {
    this.updateStatus(messageId, 'delivered');
  }

  /**
   * Mark message as read (read receipt received).
   */
  markRead(messageId: string): void {
    this.updateStatus(messageId, 'read');
  }

  /**
   * Remove a message from tracking.
   */
  remove(messageId: string): void {
    this.messages.delete(messageId);
  }

  /**
   * Check if a message has reached a specific status.
   */
  hasReachedStatus(messageId: string, status: MessageStatus): boolean {
    const entry = this.messages.get(messageId);
    if (!entry) return false;
    return STATUS_ORDER[entry.status] >= STATUS_ORDER[status];
  }

  /**
   * Clean up messages older than the specified age (in milliseconds).
   * Removes:
   * - Messages that have reached 'read' status (after maxAgeMs from read time)
   * - Messages stuck in non-read status (after MAX_STUCK_MESSAGE_AGE_MS from pending time)
   * @returns Number of messages removed
   */
  cleanupOldMessages(maxAgeMs: number): number {
    const now = Date.now();
    let removed = 0;

    for (const [messageId, entry] of this.messages) {
      // Clean up messages that are fully read
      if (entry.status === 'read' && entry.timestamps.read) {
        if (now - entry.timestamps.read > maxAgeMs) {
          this.messages.delete(messageId);
          removed++;
          continue;
        }
      }

      // Clean up stuck messages (not read after MAX_STUCK_MESSAGE_AGE_MS)
      // This prevents memory leaks from messages that never get read receipts
      if (entry.status !== 'read' && entry.timestamps.pending) {
        if (now - entry.timestamps.pending > MAX_STUCK_MESSAGE_AGE_MS) {
          this.messages.delete(messageId);
          removed++;
        }
      }
    }

    return removed;
  }

  /**
   * Get the number of tracked messages.
   */
  get size(): number {
    return this.messages.size;
  }

  /**
   * Update message status with validation.
   * Prevents status regression (e.g., delivered → relayed).
   */
  private updateStatus(messageId: string, newStatus: MessageStatus): void {
    const entry = this.messages.get(messageId);
    if (!entry) {
      return;
    }

    const currentOrder = STATUS_ORDER[entry.status];
    const newOrder = STATUS_ORDER[newStatus];

    // Prevent status regression
    if (newOrder <= currentOrder) {
      return;
    }

    const previousStatus = entry.status;
    entry.status = newStatus;
    entry.timestamps[newStatus] = Date.now();

    this.events.onStatusChanged(messageId, previousStatus, newStatus);
  }
}
