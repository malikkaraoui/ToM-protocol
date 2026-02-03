/**
 * METRICS MODULE — Golden Path Measurement
 *
 * Types for latency tracking and path metrics.
 * Used to validate the "inversion property" (more nodes = faster).
 *
 * @module metrics
 */

import type { NodeId } from '../identity/index.js';

/**
 * Latency measurement for a single message delivery.
 */
export interface MessageLatency {
  /** Message ID */
  messageId: string;
  /** Total end-to-end latency in milliseconds */
  totalLatencyMs: number;
  /** Number of hops the message traversed */
  hopCount: number;
  /** Route type used */
  routeType: 'relay' | 'direct';
  /** Timestamp when sent */
  sentAt: number;
  /** Timestamp when delivered */
  deliveredAt: number;
}

/**
 * Path information for a delivered message.
 */
export interface MessagePath {
  /** Message ID */
  messageId: string;
  /** Sender node ID */
  from: NodeId;
  /** Recipient node ID */
  to: NodeId;
  /** Ordered list of relay nodes traversed */
  via: NodeId[];
  /** Route type */
  routeType: 'relay' | 'direct';
  /** Per-hop timestamps (if available) */
  hopTimestamps?: number[];
}

/**
 * Aggregate network metrics.
 * Used to validate inversion property.
 */
export interface NetworkMetrics {
  /** Average message latency in ms */
  avgLatencyMs: number;
  /** 95th percentile latency in ms */
  p95LatencyMs: number;
  /** Total messages sent */
  messagesSent: number;
  /** Total messages delivered */
  messagesDelivered: number;
  /** Delivery success rate (0-1) */
  deliveryRate: number;
  /** Number of active peers */
  activePeers: number;
  /** Number of relay nodes */
  relayNodes: number;
  /** Timestamp of metrics snapshot */
  timestamp: number;
}

/**
 * Events for metrics collection.
 */
export interface MetricsEvents {
  /** Emitted when a message latency is measured */
  onLatencyMeasured: (latency: MessageLatency) => void;
  /** Emitted when network metrics are updated */
  onMetricsUpdated: (metrics: NetworkMetrics) => void;
}
