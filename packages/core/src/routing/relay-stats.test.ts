import { describe, expect, it, vi } from 'vitest';
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
});
