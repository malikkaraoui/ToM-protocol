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
   */
  track(messageId: string, to: string): void {
    const entry: MessageStatusEntry = {
      messageId,
      to,
      status: 'pending',
      timestamps: {
        pending: Date.now(),
      },
    };
    this.messages.set(messageId, entry);
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
   * Only removes messages that have reached 'read' status.
   * @returns Number of messages removed
   */
  cleanupOldMessages(maxAgeMs: number): number {
    const now = Date.now();
    let removed = 0;

    for (const [messageId, entry] of this.messages) {
      // Only clean up messages that are fully read
      if (entry.status === 'read' && entry.timestamps.read) {
        if (now - entry.timestamps.read > maxAgeMs) {
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
