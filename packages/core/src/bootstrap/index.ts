/**
 * BOOTSTRAP MODULE (ADR-002) — TEMPORARY
 *
 * This module defines the interface boundary for network bootstrap mechanisms.
 * All bootstrap-specific code should implement these interfaces to ensure
 * clean replacement when transitioning to distributed DHT discovery.
 *
 * Current implementation: WebSocket signaling server
 * Target implementation: Distributed DHT (Epic 7, Story 7.1)
 *
 * The bootstrap layer is responsible for:
 * - Initial peer discovery (finding first peers to connect to)
 * - Signaling exchange (WebRTC SDP/ICE coordination)
 * - Presence announcements (join/leave notifications)
 *
 * The bootstrap layer is NOT responsible for:
 * - Message routing (handled by Router)
 * - Role assignment (handled by RoleManager)
 * - Transport connections (handled by TransportLayer)
 *
 * @see architecture.md#ADR-002 for bootstrap elimination roadmap
 * @module bootstrap
 */

import type { NodeId } from '../identity/index.js';

/**
 * Bootstrap peer information received during discovery.
 * Minimal data needed to establish initial connection.
 */
export interface BootstrapPeer {
  /** Unique node identifier (derived from public key) */
  nodeId: NodeId;
  /** Human-readable name for display */
  username: string;
  /** Public key for identity verification */
  publicKey: string;
}

/**
 * Events emitted by bootstrap mechanisms.
 * Any bootstrap implementation must emit these events.
 */
export interface BootstrapEvents {
  /** A new peer was discovered and is available for connection */
  onPeerDiscovered: (peer: BootstrapPeer) => void;
  /** A known peer has left the network */
  onPeerDeparted: (nodeId: NodeId) => void;
  /** Bootstrap connection established successfully */
  onConnected: () => void;
  /** Bootstrap connection lost */
  onDisconnected: () => void;
  /** Error occurred during bootstrap */
  onError: (error: Error) => void;
}

/**
 * Configuration for bootstrap mechanisms.
 */
export interface BootstrapConfig {
  /** Local node's identity */
  nodeId: NodeId;
  /** Local node's username */
  username: string;
  /** Local node's public key */
  publicKey: string;
}

/**
 * Interface that all bootstrap mechanisms must implement.
 *
 * This abstraction allows the SDK to work with any bootstrap mechanism
 * (WebSocket signaling, DHT, hybrid) without code changes.
 *
 * @example Current implementation (WebSocket)
 * ```typescript
 * class WebSocketBootstrap implements BootstrapMechanism {
 *   async connect(endpoint: string): Promise<void> {
 *     // Connect to signaling server
 *   }
 * }
 * ```
 *
 * @example Future implementation (DHT)
 * ```typescript
 * class DHTBootstrap implements BootstrapMechanism {
 *   async connect(topicHash: string): Promise<void> {
 *     // Join DHT swarm by topic
 *   }
 * }
 * ```
 */
export interface BootstrapMechanism {
  /**
   * Connect to the bootstrap network.
   * @param endpoint - Bootstrap endpoint (URL for signaling, topic hash for DHT)
   */
  connect(endpoint: string): Promise<void>;

  /**
   * Disconnect from the bootstrap network.
   */
  disconnect(): void;

  /**
   * Send a signaling message to a specific peer.
   * Used for WebRTC SDP/ICE exchange.
   */
  signal(to: NodeId, payload: unknown): void;

  /**
   * Get list of currently known peers.
   */
  getPeers(): BootstrapPeer[];

  /**
   * Check if connected to bootstrap network.
   */
  isConnected(): boolean;

  // ============================================
  // DHT-READY INTERFACE (Epic 7 preparation)
  // These methods are optional stubs for now.
  // WebSocket implementation returns empty/noop.
  // DHT implementation will provide real behavior.
  // ============================================

  /**
   * Announce this node's presence on a topic.
   * DHT: Publishes to swarm. WebSocket: noop (presence is automatic).
   * @param topic - Topic hash to announce on
   */
  announce?(topic: string): Promise<void>;

  /**
   * Lookup peers on a topic.
   * DHT: Queries swarm for peers. WebSocket: returns getPeers().
   * @param topic - Topic hash to search
   * @returns List of peers found on topic
   */
  lookup?(topic: string): Promise<BootstrapPeer[]>;

  /**
   * Leave/unannounce from a topic.
   * DHT: Removes from swarm. WebSocket: noop.
   * @param topic - Topic hash to leave
   */
  unannounce?(topic: string): Promise<void>;
}

/**
 * Factory function type for creating bootstrap mechanisms.
 * Allows runtime selection of bootstrap strategy.
 */
export type BootstrapFactory = (config: BootstrapConfig, events: BootstrapEvents) => BootstrapMechanism;
