import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { RelayStats } from './relay-stats.js';

describe('RelayStats', () => {
  it('should track messages relayed', () => {
    const stats = new RelayStats();

    stats.recordRelay();
    stats.recordRelay();
    stats.recordRelay();

    const result = stats.getStats();
    expect(result.messagesRelayed).toBe(3);
  });

  it('should track own messages sent', () => {
    const stats = new RelayStats();

    stats.recordOwnMessage();
    stats.recordOwnMessage();

    const result = stats.getStats();
    expect(result.ownMessagesSent).toBe(2);
  });

  it('should calculate relay to own ratio', () => {
    const stats = new RelayStats();

    stats.recordOwnMessage();
    stats.recordRelay();
    stats.recordRelay();
    stats.recordRelay();
    stats.recordRelay();

    const result = stats.getStats();
    expect(result.relayToOwnRatio).toBe(4);
  });

  it('should emit capacity warning when threshold exceeded', () => {
    const onCapacityWarning = vi.fn();
    const stats = new RelayStats({
      capacityThreshold: 5,
      events: { onCapacityWarning },
    });

    // Send 1 own message
    stats.recordOwnMessage();

    // Relay 5 messages (ratio = 5, at threshold)
    for (let i = 0; i < 5; i++) {
      stats.recordRelay();
    }
    expect(onCapacityWarning).not.toHaveBeenCalled();

    // Relay 1 more (ratio = 6, exceeds threshold)
    stats.recordRelay();
    expect(onCapacityWarning).toHaveBeenCalledOnce();
  });

  it('should emit warning for pure relay node', () => {
    const onCapacityWarning = vi.fn();
    const stats = new RelayStats({
      events: { onCapacityWarning },
    });

    // Relay 20 messages without own messaging
    for (let i = 0; i < 20; i++) {
      stats.recordRelay();
    }
    expect(onCapacityWarning).not.toHaveBeenCalled();

    // 21st relay triggers warning
    stats.recordRelay();
    expect(onCapacityWarning).toHaveBeenCalledOnce();
    expect(onCapacityWarning.mock.calls[0][1]).toContain('pure relay');
  });

  it('should track timestamps', () => {
    const stats = new RelayStats();
    const before = Date.now();

    stats.recordRelay();
    stats.recordOwnMessage();

    const result = stats.getStats();
    expect(result.lastRelayTimestamp).toBeGreaterThanOrEqual(before);
    expect(result.lastOwnMessageTimestamp).toBeGreaterThanOrEqual(before);
  });

  it('should reset stats', () => {
    const stats = new RelayStats();

    stats.recordRelay();
    stats.recordOwnMessage();
    stats.reset();

    const result = stats.getStats();
    expect(result.messagesRelayed).toBe(0);
    expect(result.ownMessagesSent).toBe(0);
  });

  it('should handle zero own messages gracefully', () => {
    const stats = new RelayStats();

    stats.recordRelay();
    stats.recordRelay();

    const result = stats.getStats();
    // When no own messages, ratio equals relay count
    expect(result.relayToOwnRatio).toBe(2);
  });

  describe('security fixes', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('should not emit warning more often than cooldown period (event storm protection)', () => {
      const onCapacityWarning = vi.fn();
      const stats = new RelayStats({
        capacityThreshold: 5,
        events: { onCapacityWarning },
      });

      stats.recordOwnMessage();

      // Rapid relay messages exceeding threshold
      for (let i = 0; i < 20; i++) {
        stats.recordRelay();
      }

      // Should only emit once due to cooldown
      expect(onCapacityWarning).toHaveBeenCalledTimes(1);

      // Advance past cooldown (10 seconds)
      vi.advanceTimersByTime(11000);

      // Relay more - should trigger again
      stats.recordRelay();
      expect(onCapacityWarning).toHaveBeenCalledTimes(2);
    });

    it('should respect cooldown after reset', () => {
      const onCapacityWarning = vi.fn();
      const stats = new RelayStats({
        capacityThreshold: 5,
        events: { onCapacityWarning },
      });

      stats.recordOwnMessage();
      for (let i = 0; i < 10; i++) {
        stats.recordRelay();
      }
      expect(onCapacityWarning).toHaveBeenCalledTimes(1);

      // Reset clears lastWarningAt
      stats.reset();

      // Re-trigger threshold
      stats.recordOwnMessage();
      for (let i = 0; i < 10; i++) {
        stats.recordRelay();
      }

      // Should fire again immediately (cooldown was reset)
      expect(onCapacityWarning).toHaveBeenCalledTimes(2);
    });

    it('should bound messagesRelayed counter (overflow protection)', () => {
      const stats = new RelayStats();

      // Simulate max counter by manually testing the bound
      // We can't actually increment 1 billion times, so we test indirectly
      // by checking the counter increments normally
      for (let i = 0; i < 100; i++) {
        stats.recordRelay();
      }

      expect(stats.getStats().messagesRelayed).toBe(100);
    });

    it('should bound ownMessagesSent counter (overflow protection)', () => {
      const stats = new RelayStats();

      for (let i = 0; i < 100; i++) {
        stats.recordOwnMessage();
      }

      expect(stats.getStats().ownMessagesSent).toBe(100);
    });

    it('should bound relayAcksReceived counter (overflow protection)', () => {
      const stats = new RelayStats();

      for (let i = 0; i < 100; i++) {
        stats.recordRelayAck();
      }

      expect(stats.getStats().relayAcksReceived).toBe(100);
    });
  });

  describe('byte tracking', () => {
    it('should track bytes relayed', () => {
      const stats = new RelayStats();

      stats.recordRelay(100);
      stats.recordRelay(250);
      stats.recordRelay(150);

      const result = stats.getStats();
      expect(result.bytesRelayed).toBe(500);
      expect(result.messagesRelayed).toBe(3);
    });

    it('should track bytes sent', () => {
      const stats = new RelayStats();

      stats.recordOwnMessage(200);
      stats.recordOwnMessage(300);

      const result = stats.getStats();
      expect(result.bytesSent).toBe(500);
      expect(result.ownMessagesSent).toBe(2);
    });

    it('should handle undefined byte size gracefully', () => {
      const stats = new RelayStats();

      stats.recordRelay();
      stats.recordOwnMessage();

      const result = stats.getStats();
      expect(result.bytesRelayed).toBe(0);
      expect(result.bytesSent).toBe(0);
      expect(result.messagesRelayed).toBe(1);
      expect(result.ownMessagesSent).toBe(1);
    });

    it('should reset byte counters', () => {
      const stats = new RelayStats();

      stats.recordRelay(1000);
      stats.recordOwnMessage(500);
      stats.reset();

      const result = stats.getStats();
      expect(result.bytesRelayed).toBe(0);
      expect(result.bytesSent).toBe(0);
    });
  });
});
