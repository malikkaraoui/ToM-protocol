import { describe, expect, it } from 'vitest';
import type { MessageEnvelope } from './envelope.js';
import { extractPathInfo, formatLatency } from './path-info.js';

describe('extractPathInfo', () => {
  const createEnvelope = (overrides: Partial<MessageEnvelope> = {}): MessageEnvelope => ({
    id: 'test-msg-id',
    from: 'sender-node-id-abc123',
    to: 'recipient-node-id-xyz789',
    via: [],
    type: 'chat',
    payload: { text: 'Hello' },
    timestamp: Date.now() - 100, // Sent 100ms ago
    signature: 'test-signature',
    ...overrides,
  });

  it('should extract path info for direct route (empty via)', () => {
    const now = Date.now();
    const envelope = createEnvelope({
      via: [],
      timestamp: now - 50,
      routeType: 'direct',
    });

    const pathInfo = extractPathInfo(envelope, now);

    expect(pathInfo.routeType).toBe('direct');
    expect(pathInfo.relayHops).toEqual([]);
    expect(pathInfo.sentAt).toBe(now - 50);
    expect(pathInfo.deliveredAt).toBe(now);
    expect(pathInfo.latencyMs).toBe(50);
  });

  it('should extract path info for relay route with single hop', () => {
    const now = Date.now();
    const relayId = 'relay-node-id-12345678abcdef';
    const envelope = createEnvelope({
      via: [relayId],
      timestamp: now - 150,
      routeType: 'relay',
    });

    const pathInfo = extractPathInfo(envelope, now);

    expect(pathInfo.routeType).toBe('relay');
    expect(pathInfo.relayHops).toEqual(['relay-no']); // First 8 chars
    expect(pathInfo.latencyMs).toBe(150);
  });

  it('should extract path info for relay route with multiple hops', () => {
    const now = Date.now();
    const relay1 = 'aaaabbbb1111222233334444';
    const relay2 = 'ccccdddd5555666677778888';
    const envelope = createEnvelope({
      via: [relay1, relay2],
      timestamp: now - 200,
      routeType: 'relay',
    });

    const pathInfo = extractPathInfo(envelope, now);

    expect(pathInfo.routeType).toBe('relay');
    expect(pathInfo.relayHops).toEqual(['aaaabbbb', 'ccccdddd']);
    expect(pathInfo.latencyMs).toBe(200);
  });

  it('should infer relay route type when routeType is undefined but via has entries', () => {
    const now = Date.now();
    const envelope = createEnvelope({
      via: ['some-relay-node-id'],
      timestamp: now - 100,
      routeType: undefined, // Not set
    });

    const pathInfo = extractPathInfo(envelope, now);

    expect(pathInfo.routeType).toBe('relay');
  });

  it('should infer direct route type when routeType is undefined and via is empty', () => {
    const now = Date.now();
    const envelope = createEnvelope({
      via: [],
      timestamp: now - 100,
      routeType: undefined, // Not set
    });

    const pathInfo = extractPathInfo(envelope, now);

    expect(pathInfo.routeType).toBe('direct');
  });

  it('should use Date.now() when receivedAt is not provided', () => {
    const sentAt = Date.now() - 100;
    const envelope = createEnvelope({ timestamp: sentAt });

    const pathInfo = extractPathInfo(envelope);

    // Allow 10ms tolerance for test execution time
    expect(pathInfo.latencyMs).toBeGreaterThanOrEqual(100);
    expect(pathInfo.latencyMs).toBeLessThan(110);
  });

  it('should handle negative latency (clock skew) by returning 0', () => {
    const now = Date.now();
    const envelope = createEnvelope({
      timestamp: now + 1000, // Future timestamp (clock skew)
    });

    const pathInfo = extractPathInfo(envelope, now);

    expect(pathInfo.latencyMs).toBe(0);
  });

  it('should handle missing via field gracefully', () => {
    const now = Date.now();
    const envelope = createEnvelope({ timestamp: now - 50 });
    // Simulate missing via field
    (envelope as Record<string, unknown>).via = undefined;
    (envelope as Record<string, unknown>).via = undefined;

    const pathInfo = extractPathInfo(envelope, now);

    expect(pathInfo.routeType).toBe('direct');
    expect(pathInfo.relayHops).toEqual([]);
  });
});

describe('formatLatency', () => {
  it('should format sub-second latency in milliseconds', () => {
    expect(formatLatency(42)).toBe('42ms');
    expect(formatLatency(0)).toBe('0ms');
    expect(formatLatency(999)).toBe('999ms');
  });

  it('should format 1-10 second latency with one decimal', () => {
    expect(formatLatency(1000)).toBe('1.0s');
    expect(formatLatency(1234)).toBe('1.2s');
    expect(formatLatency(5678)).toBe('5.7s');
    expect(formatLatency(9999)).toBe('10.0s');
  });

  it('should format 10+ second latency as whole seconds', () => {
    expect(formatLatency(10000)).toBe('10s');
    expect(formatLatency(15000)).toBe('15s');
    expect(formatLatency(60000)).toBe('60s');
  });

  it('should round milliseconds to nearest whole number', () => {
    expect(formatLatency(42.4)).toBe('42ms');
    expect(formatLatency(42.5)).toBe('43ms');
    expect(formatLatency(42.9)).toBe('43ms');
  });
});
