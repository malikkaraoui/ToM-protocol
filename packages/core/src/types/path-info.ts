import type { NodeId } from '../identity/index.js';
import type { MessageEnvelope } from './envelope.js';

/**
 * Information about the path a message took through the network.
 * Derived from envelope metadata — no extra network requests required.
 */
export interface PathInfo {
  /** Route type: 'direct' if no relay hops, 'relay' if via relays */
  routeType: 'direct' | 'relay';
  /** NodeId prefixes of relays used (empty if direct) */
  relayHops: string[];
  /** Unix timestamp when message was sent */
  sentAt: number;
  /** Unix timestamp when message was received/delivered */
  deliveredAt: number;
  /** Latency in milliseconds (deliveredAt - sentAt) */
  latencyMs: number;
}

/**
 * Extract path information from a received message envelope.
 * Uses existing envelope fields — no network requests.
 *
 * @param envelope - The received message envelope
 * @param receivedAt - Optional timestamp when message was received (defaults to Date.now())
 * @returns PathInfo with routing details
 */
export function extractPathInfo(envelope: MessageEnvelope, receivedAt?: number): PathInfo {
  const deliveredAt = receivedAt ?? Date.now();
  const sentAt = envelope.timestamp;
  const latencyMs = Math.max(0, deliveredAt - sentAt);

  // Determine route type from envelope
  const hasRelays = envelope.via && envelope.via.length > 0;
  const routeType: 'direct' | 'relay' = envelope.routeType ?? (hasRelays ? 'relay' : 'direct');

  // Get relay hops (show first 8 chars of each nodeId for privacy)
  const relayHops = (envelope.via ?? []).map((nodeId: NodeId) => nodeId.slice(0, 8));

  return {
    routeType,
    relayHops,
    sentAt,
    deliveredAt,
    latencyMs,
  };
}

/**
 * Format latency for human-readable display.
 *
 * @param latencyMs - Latency in milliseconds
 * @returns Formatted string (e.g., "42ms", "<1s", "1.2s")
 */
export function formatLatency(latencyMs: number): string {
  if (latencyMs < 1000) {
    return `${Math.round(latencyMs)}ms`;
  }
  if (latencyMs < 10000) {
    return `${(latencyMs / 1000).toFixed(1)}s`;
  }
  return `${Math.round(latencyMs / 1000)}s`;
}
