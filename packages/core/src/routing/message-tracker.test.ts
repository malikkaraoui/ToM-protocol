import { beforeEach, describe, expect, it, vi } from 'vitest';
import {
  type MessageStatus,
  type MessageStatusEntry,
  MessageTracker,
  type MessageTrackerEvents,
} from './message-tracker.js';

describe('MessageTracker', () => {
  let tracker: MessageTracker;
  let events: MessageTrackerEvents;

  beforeEach(() => {
    events = {
      onStatusChanged: vi.fn(),
    };
    tracker = new MessageTracker(events);
  });

  describe('status tracking', () => {
    it('should track a new message with pending status', () => {
      tracker.track('msg-1', 'recipient-1');

      const status = tracker.getStatus('msg-1');
      expect(status).toBeDefined();
      expect(status?.status).toBe('pending');
      expect(status?.to).toBe('recipient-1');
    });

    it('should return undefined for unknown message', () => {
      const status = tracker.getStatus('unknown-msg');
      expect(status).toBeUndefined();
    });

    it('should transition from pending to sent', () => {
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');

      const status = tracker.getStatus('msg-1');
      expect(status?.status).toBe('sent');
      expect(events.onStatusChanged).toHaveBeenCalledWith('msg-1', 'pending', 'sent');
    });

    it('should transition from sent to relayed', () => {
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');
      tracker.markRelayed('msg-1');

      const status = tracker.getStatus('msg-1');
      expect(status?.status).toBe('relayed');
      expect(events.onStatusChanged).toHaveBeenCalledWith('msg-1', 'sent', 'relayed');
    });

    it('should transition from relayed to delivered', () => {
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');
      tracker.markRelayed('msg-1');
      tracker.markDelivered('msg-1');

      const status = tracker.getStatus('msg-1');
      expect(status?.status).toBe('delivered');
      expect(events.onStatusChanged).toHaveBeenCalledWith('msg-1', 'relayed', 'delivered');
    });

    it('should transition from delivered to read', () => {
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');
      tracker.markRelayed('msg-1');
      tracker.markDelivered('msg-1');
      tracker.markRead('msg-1');

      const status = tracker.getStatus('msg-1');
      expect(status?.status).toBe('read');
      expect(events.onStatusChanged).toHaveBeenCalledWith('msg-1', 'delivered', 'read');
    });
  });

  describe('timestamp tracking', () => {
    it('should record timestamps for each transition', () => {
      vi.useFakeTimers();
      const baseTime = Date.now();

      tracker.track('msg-1', 'recipient-1');
      expect(tracker.getStatus('msg-1')?.timestamps.pending).toBe(baseTime);

      vi.advanceTimersByTime(100);
      tracker.markSent('msg-1');
      expect(tracker.getStatus('msg-1')?.timestamps.sent).toBe(baseTime + 100);

      vi.advanceTimersByTime(200);
      tracker.markRelayed('msg-1');
      expect(tracker.getStatus('msg-1')?.timestamps.relayed).toBe(baseTime + 300);

      vi.advanceTimersByTime(150);
      tracker.markDelivered('msg-1');
      expect(tracker.getStatus('msg-1')?.timestamps.delivered).toBe(baseTime + 450);

      vi.advanceTimersByTime(1000);
      tracker.markRead('msg-1');
      expect(tracker.getStatus('msg-1')?.timestamps.read).toBe(baseTime + 1450);

      vi.useRealTimers();
    });
  });

  describe('edge cases', () => {
    it('should not emit event if message not tracked', () => {
      tracker.markSent('unknown');
      expect(events.onStatusChanged).not.toHaveBeenCalled();
    });

    it('should allow skipping intermediate statuses (direct path)', () => {
      // Direct path skips relayed status
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');
      tracker.markDelivered('msg-1'); // Skip relayed

      const status = tracker.getStatus('msg-1');
      expect(status?.status).toBe('delivered');
      expect(status?.timestamps.relayed).toBeUndefined();
    });

    it('should not regress status', () => {
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');
      tracker.markRelayed('msg-1');
      tracker.markDelivered('msg-1');

      // Try to go back to relayed
      tracker.markRelayed('msg-1');

      const status = tracker.getStatus('msg-1');
      expect(status?.status).toBe('delivered'); // Should stay at delivered
    });

    it('should handle multiple messages independently', () => {
      tracker.track('msg-1', 'recipient-1');
      tracker.track('msg-2', 'recipient-2');

      tracker.markSent('msg-1');
      tracker.markSent('msg-2');
      tracker.markRelayed('msg-1');

      expect(tracker.getStatus('msg-1')?.status).toBe('relayed');
      expect(tracker.getStatus('msg-2')?.status).toBe('sent');
    });
  });

  describe('cleanup', () => {
    it('should allow removing tracked messages', () => {
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');

      tracker.remove('msg-1');

      expect(tracker.getStatus('msg-1')).toBeUndefined();
    });

    it('should clean up old read messages', () => {
      vi.useFakeTimers();
      const baseTime = Date.now();

      // Create and read a message
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');
      tracker.markRead('msg-1');

      // Advance time past cleanup threshold
      vi.advanceTimersByTime(15 * 60 * 1000); // 15 minutes

      // Cleanup messages older than 10 minutes
      const removed = tracker.cleanupOldMessages(10 * 60 * 1000);

      expect(removed).toBe(1);
      expect(tracker.getStatus('msg-1')).toBeUndefined();

      vi.useRealTimers();
    });

    it('should not clean up messages that are not read', () => {
      vi.useFakeTimers();

      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');
      tracker.markDelivered('msg-1');

      vi.advanceTimersByTime(15 * 60 * 1000);

      const removed = tracker.cleanupOldMessages(10 * 60 * 1000);

      expect(removed).toBe(0);
      expect(tracker.getStatus('msg-1')).toBeDefined();

      vi.useRealTimers();
    });

    it('should report correct size', () => {
      expect(tracker.size).toBe(0);

      tracker.track('msg-1', 'recipient-1');
      expect(tracker.size).toBe(1);

      tracker.track('msg-2', 'recipient-2');
      expect(tracker.size).toBe(2);

      tracker.remove('msg-1');
      expect(tracker.size).toBe(1);
    });
  });

  describe('status checks', () => {
    it('should check if message has reached status', () => {
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');
      tracker.markDelivered('msg-1');

      expect(tracker.hasReachedStatus('msg-1', 'pending')).toBe(true);
      expect(tracker.hasReachedStatus('msg-1', 'sent')).toBe(true);
      expect(tracker.hasReachedStatus('msg-1', 'delivered')).toBe(true);
      expect(tracker.hasReachedStatus('msg-1', 'read')).toBe(false);
    });

    it('should return false for unknown message', () => {
      expect(tracker.hasReachedStatus('unknown', 'sent')).toBe(false);
    });
  });

  describe('best-effort delivery (AC#3)', () => {
    it('should not corrupt status if read receipt fails to send', () => {
      // Simulate: message delivered, but read receipt send fails
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');
      tracker.markDelivered('msg-1');

      // Status should stay at delivered (read receipt never sent/received)
      expect(tracker.getStatus('msg-1')?.status).toBe('delivered');

      // Attempting to mark as read without a receipt would only happen
      // if the receipt was successfully received - so status stays delivered
      // This validates AC#3: "message remains in 'delivered' status"
      expect(tracker.hasReachedStatus('msg-1', 'read')).toBe(false);
    });

    it('should not allow false read status without proper receipt', () => {
      tracker.track('msg-1', 'recipient-1');
      tracker.markSent('msg-1');
      tracker.markDelivered('msg-1');

      // Without markRead being called (simulating failed receipt),
      // status must remain at delivered
      const status = tracker.getStatus('msg-1');
      expect(status?.status).toBe('delivered');
      expect(status?.timestamps.read).toBeUndefined();
    });
  });
});
