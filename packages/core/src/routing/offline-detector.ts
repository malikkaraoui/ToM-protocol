import type { NodeId } from '../identity/index.js';

/**
 * Events emitted by OfflineDetector
 */
export interface OfflineDetectorEvents {
  /** Emitted when a peer is detected as offline */
  onPeerOffline: (nodeId: NodeId, lastSeen: number) => void;
  /** Emitted when a previously-offline peer comes back online */
  onPeerOnline: (nodeId: NodeId) => void;
}

/**
 * Information about an offline peer
 */
export interface OfflinePeerInfo {
  nodeId: NodeId;
  /** Timestamp when peer was last seen */
  lastSeen: number;
  /** Timestamp when peer was detected as offline */
  detectedAt: number;
}

/**
 * OfflineDetector tracks peer online/offline status and emits events.
 *
 * Integrates with existing HeartbeatManager and NetworkTopology to detect:
 * - Peer going offline (WebRTC connection closed + no heartbeat)
 * - Peer coming back online (reconnection via signaling)
 *
 * This enables the backup system to store messages for offline recipients.
 */
export class OfflineDetector {
  private events: OfflineDetectorEvents;
  private offlinePeers = new Map<NodeId, OfflinePeerInfo>();
  private peerLastSeen = new Map<NodeId, number>();
  /** Debounce timeout for rapid reconnect/disconnect cycles (ms) */
  private debounceMs: number;
  /** Pending online transitions (for debouncing) */
  private pendingOnline = new Map<NodeId, ReturnType<typeof setTimeout>>();

  constructor(events: OfflineDetectorEvents, debounceMs = 2000) {
    this.events = events;
    this.debounceMs = debounceMs;
  }

  /**
   * Record activity from a peer (heartbeat or message received).
   * If peer was offline, marks them as online.
   */
  recordPeerActivity(nodeId: NodeId): void {
    this.peerLastSeen.set(nodeId, Date.now());

    // If peer was marked offline, they're back online
    if (this.offlinePeers.has(nodeId)) {
      this.handlePeerReconnected(nodeId);
    }
  }

  /**
   * Handle peer departure event from HeartbeatManager.
   * Marks peer as offline and emits event.
   */
  handlePeerDeparted(nodeId: NodeId): void {
    // Cancel any pending online transition
    const pendingTimer = this.pendingOnline.get(nodeId);
    if (pendingTimer) {
      clearTimeout(pendingTimer);
      this.pendingOnline.delete(nodeId);
    }

    // Don't double-emit if already offline
    if (this.offlinePeers.has(nodeId)) {
      return;
    }

    const lastSeen = this.peerLastSeen.get(nodeId) ?? Date.now();
    const offlineInfo: OfflinePeerInfo = {
      nodeId,
      lastSeen,
      detectedAt: Date.now(),
    };

    this.offlinePeers.set(nodeId, offlineInfo);
    this.events.onPeerOffline(nodeId, lastSeen);
  }

  /**
   * Handle peer reconnection (signaling presence update).
   * Uses debouncing to handle rapid reconnect/disconnect cycles.
   */
  private handlePeerReconnected(nodeId: NodeId): void {
    // Cancel any pending online transition
    const existingTimer = this.pendingOnline.get(nodeId);
    if (existingTimer) {
      clearTimeout(existingTimer);
    }

    // Debounce the online transition to handle rapid cycles
    const timer = setTimeout(() => {
      this.pendingOnline.delete(nodeId);

      if (this.offlinePeers.has(nodeId)) {
        this.offlinePeers.delete(nodeId);
        this.events.onPeerOnline(nodeId);
      }
    }, this.debounceMs);

    this.pendingOnline.set(nodeId, timer);
  }

  /**
   * Explicitly mark a peer as online (e.g., from signaling presence update).
   * This can trigger reconnection detection.
   */
  markPeerOnline(nodeId: NodeId): void {
    this.peerLastSeen.set(nodeId, Date.now());

    if (this.offlinePeers.has(nodeId)) {
      this.handlePeerReconnected(nodeId);
    }
  }

  /**
   * Check if a peer is currently considered offline.
   */
  isOffline(nodeId: NodeId): boolean {
    return this.offlinePeers.has(nodeId);
  }

  /**
   * Get information about an offline peer.
   */
  getOfflinePeerInfo(nodeId: NodeId): OfflinePeerInfo | undefined {
    return this.offlinePeers.get(nodeId);
  }

  /**
   * Get all currently offline peers.
   */
  getOfflinePeers(): OfflinePeerInfo[] {
    return Array.from(this.offlinePeers.values());
  }

  /**
   * Get the last seen timestamp for a peer.
   */
  getLastSeen(nodeId: NodeId): number | undefined {
    return this.peerLastSeen.get(nodeId);
  }

  /**
   * Remove a peer from tracking (e.g., when they leave the network permanently).
   */
  removePeer(nodeId: NodeId): void {
    this.offlinePeers.delete(nodeId);
    this.peerLastSeen.delete(nodeId);

    const pendingTimer = this.pendingOnline.get(nodeId);
    if (pendingTimer) {
      clearTimeout(pendingTimer);
      this.pendingOnline.delete(nodeId);
    }
  }

  /**
   * Clean up resources.
   */
  destroy(): void {
    // Clear all pending timers
    for (const timer of this.pendingOnline.values()) {
      clearTimeout(timer);
    }
    this.pendingOnline.clear();
    this.offlinePeers.clear();
    this.peerLastSeen.clear();
  }
}
