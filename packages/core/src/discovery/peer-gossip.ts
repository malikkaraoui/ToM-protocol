/**
 * Peer Gossip Module (Story 7.1 - Bootstrap Fade)
 *
 * Enables autonomous peer discovery through gossip protocol.
 * Nodes exchange their known peers, reducing dependency on the signaling server.
 *
 * Flow:
 * 1. Node connects to signaling server (bootstrap) → discovers initial peers
 * 2. Node asks connected peers "who do you know?" (gossip)
 * 3. Node can connect to newly discovered peers directly
 * 4. Signaling server becomes less critical over time
 *
 * @see architecture.md#ADR-002 for bootstrap elimination roadmap
 */

import { secureId } from '../crypto/secure-random.js';
import type { NodeId } from '../identity/keypair.js';

// ============================================
// Types
// ============================================

export interface GossipPeerInfo {
  nodeId: NodeId;
  username: string;
  /** Encryption public key for E2E (Story 6.1) */
  encryptionKey?: string;
  /** How this peer was discovered */
  discoverySource: 'bootstrap' | 'gossip';
  /** When we first learned about this peer */
  discoveredAt: number;
  /** Roles if known */
  roles?: string[];
}

export interface PeerGossipMessage {
  type: 'peer-list-request' | 'peer-list-response';
  /** Requesting node's info (for request) */
  from?: GossipPeerInfo;
  /** Known peers (for response) */
  peers?: GossipPeerInfo[];
  /** Request ID for correlation */
  requestId: string;
}

export interface PeerGossipEvents {
  /** Called when new peers are discovered via gossip */
  onPeersDiscovered: (peers: GossipPeerInfo[], via: NodeId) => void;
  /** Called when a peer requests our peer list */
  onPeerListRequested: (from: NodeId, requestId: string) => void;
}

export interface PeerGossipConfig {
  /** How often to request peer lists from connected peers (ms) */
  gossipIntervalMs?: number;
  /** Maximum peers to include in a gossip response */
  maxPeersPerResponse?: number;
  /** Minimum time between requests to the same peer (ms) */
  minRequestIntervalMs?: number;
}

// ============================================
// Peer Gossip Manager
// ============================================

const DEFAULT_CONFIG: Required<PeerGossipConfig> = {
  gossipIntervalMs: 30000, // Every 30 seconds
  maxPeersPerResponse: 20,
  minRequestIntervalMs: 10000, // Don't ask same peer more than once per 10s
};

export class PeerGossip {
  private selfNodeId: NodeId;
  private selfUsername: string;
  private selfEncryptionKey?: string;
  private events: PeerGossipEvents;
  private config: Required<PeerGossipConfig>;

  /** All known peers (from any source) */
  private knownPeers = new Map<NodeId, GossipPeerInfo>();
  /** Peers discovered via gossip (subset) */
  private gossipDiscoveredPeers = new Set<NodeId>();
  /** Last time we requested from each peer */
  private lastRequestTime = new Map<NodeId, number>();
  /** Pending requests waiting for response */
  private pendingRequests = new Map<string, { peerId: NodeId; timestamp: number }>();
  /** Gossip interval timer */
  private gossipInterval: ReturnType<typeof setInterval> | null = null;
  /** Connected peers we can gossip with */
  private connectedPeers = new Set<NodeId>();

  constructor(selfNodeId: NodeId, selfUsername: string, events: PeerGossipEvents, config?: PeerGossipConfig) {
    this.selfNodeId = selfNodeId;
    this.selfUsername = selfUsername;
    this.events = events;
    this.config = { ...DEFAULT_CONFIG, ...config };
  }

  /**
   * Set the self encryption key (for gossip responses)
   */
  setSelfEncryptionKey(key: string): void {
    this.selfEncryptionKey = key;
  }

  /**
   * Start the gossip protocol
   */
  start(): void {
    if (this.gossipInterval) return;

    this.gossipInterval = setInterval(() => {
      this.performGossipRound();
    }, this.config.gossipIntervalMs);
  }

  /**
   * Stop the gossip protocol
   */
  stop(): void {
    if (this.gossipInterval) {
      clearInterval(this.gossipInterval);
      this.gossipInterval = null;
    }
  }

  /**
   * Register a peer discovered via bootstrap (signaling server)
   */
  addBootstrapPeer(peer: Omit<GossipPeerInfo, 'discoverySource' | 'discoveredAt'>): void {
    if (peer.nodeId === this.selfNodeId) return;

    const existing = this.knownPeers.get(peer.nodeId);
    if (existing) {
      // Update info but keep discovery source
      this.knownPeers.set(peer.nodeId, {
        ...existing,
        ...peer,
        discoverySource: existing.discoverySource,
        discoveredAt: existing.discoveredAt,
      });
    } else {
      this.knownPeers.set(peer.nodeId, {
        ...peer,
        discoverySource: 'bootstrap',
        discoveredAt: Date.now(),
      });
    }
  }

  /**
   * Mark a peer as connected (can gossip with them)
   */
  markConnected(nodeId: NodeId): void {
    this.connectedPeers.add(nodeId);
  }

  /**
   * Mark a peer as disconnected
   */
  markDisconnected(nodeId: NodeId): void {
    this.connectedPeers.delete(nodeId);
  }

  /**
   * Remove a peer from known peers
   */
  removePeer(nodeId: NodeId): void {
    this.knownPeers.delete(nodeId);
    this.gossipDiscoveredPeers.delete(nodeId);
    this.connectedPeers.delete(nodeId);
    this.lastRequestTime.delete(nodeId);
  }

  /**
   * Create a peer list request message
   */
  createPeerListRequest(): PeerGossipMessage {
    const requestId = this.generateRequestId();
    return {
      type: 'peer-list-request',
      from: {
        nodeId: this.selfNodeId,
        username: this.selfUsername,
        encryptionKey: this.selfEncryptionKey,
        discoverySource: 'bootstrap', // Not relevant for self
        discoveredAt: 0,
      },
      requestId,
    };
  }

  /**
   * Create a peer list response message
   */
  createPeerListResponse(requestId: string): PeerGossipMessage {
    // Select peers to share (excluding self and very stale peers)
    const peersToShare: GossipPeerInfo[] = [];
    const maxAge = 5 * 60 * 1000; // 5 minutes

    for (const [nodeId, peer] of this.knownPeers) {
      if (nodeId === this.selfNodeId) continue;
      if (Date.now() - peer.discoveredAt > maxAge) continue;
      if (peersToShare.length >= this.config.maxPeersPerResponse) break;

      peersToShare.push(peer);
    }

    return {
      type: 'peer-list-response',
      peers: peersToShare,
      requestId,
    };
  }

  /**
   * Handle incoming gossip message
   */
  handleMessage(message: PeerGossipMessage, fromNodeId: NodeId): PeerGossipMessage | null {
    if (message.type === 'peer-list-request') {
      // Someone is asking for our peer list
      this.events.onPeerListRequested(fromNodeId, message.requestId);

      // Also add the requester to our known peers if we got their info
      if (message.from && message.from.nodeId !== this.selfNodeId) {
        this.addPeerFromGossip(message.from, fromNodeId);
      }

      return this.createPeerListResponse(message.requestId);
    }

    if (message.type === 'peer-list-response') {
      // We received a peer list response
      const pending = this.pendingRequests.get(message.requestId);
      if (pending) {
        this.pendingRequests.delete(message.requestId);
      }

      // Process discovered peers
      if (message.peers && message.peers.length > 0) {
        const newPeers: GossipPeerInfo[] = [];

        for (const peer of message.peers) {
          if (peer.nodeId === this.selfNodeId) continue;

          const existing = this.knownPeers.get(peer.nodeId);
          if (!existing) {
            const newPeer: GossipPeerInfo = {
              ...peer,
              discoverySource: 'gossip',
              discoveredAt: Date.now(),
            };
            this.knownPeers.set(peer.nodeId, newPeer);
            this.gossipDiscoveredPeers.add(peer.nodeId);
            newPeers.push(newPeer);
          }
        }

        if (newPeers.length > 0) {
          this.events.onPeersDiscovered(newPeers, fromNodeId);
        }
      }

      return null;
    }

    return null;
  }

  /**
   * Add a peer discovered through gossip
   */
  private addPeerFromGossip(peer: GossipPeerInfo, viaNodeId: NodeId): void {
    if (peer.nodeId === this.selfNodeId) return;

    const existing = this.knownPeers.get(peer.nodeId);
    if (!existing) {
      const newPeer: GossipPeerInfo = {
        ...peer,
        discoverySource: 'gossip',
        discoveredAt: Date.now(),
      };
      this.knownPeers.set(peer.nodeId, newPeer);
      this.gossipDiscoveredPeers.add(peer.nodeId);
      this.events.onPeersDiscovered([newPeer], viaNodeId);
    }
  }

  /**
   * Perform a round of gossip (ask connected peers for their peer lists)
   */
  private performGossipRound(): void {
    const now = Date.now();

    for (const peerId of this.connectedPeers) {
      const lastRequest = this.lastRequestTime.get(peerId) ?? 0;
      if (now - lastRequest < this.config.minRequestIntervalMs) {
        continue; // Too soon to ask again
      }

      // Create and track request
      const request = this.createPeerListRequest();
      this.pendingRequests.set(request.requestId, {
        peerId,
        timestamp: now,
      });
      this.lastRequestTime.set(peerId, now);

      // The caller is responsible for sending this request
      // We just emit an event that a request was created
    }

    // Clean up old pending requests (>30s)
    for (const [requestId, data] of this.pendingRequests) {
      if (now - data.timestamp > 30000) {
        this.pendingRequests.delete(requestId);
      }
    }
  }

  /**
   * Get peers to gossip with (connected peers we haven't asked recently)
   */
  getPeersToGossipWith(): NodeId[] {
    const now = Date.now();
    const result: NodeId[] = [];

    for (const peerId of this.connectedPeers) {
      const lastRequest = this.lastRequestTime.get(peerId) ?? 0;
      if (now - lastRequest >= this.config.minRequestIntervalMs) {
        result.push(peerId);
      }
    }

    return result;
  }

  /**
   * Mark that we're requesting from a peer
   */
  markRequestSent(peerId: NodeId, requestId: string): void {
    this.lastRequestTime.set(peerId, Date.now());
    this.pendingRequests.set(requestId, {
      peerId,
      timestamp: Date.now(),
    });
  }

  /**
   * Get statistics about peer discovery
   */
  getStats(): {
    totalPeers: number;
    bootstrapPeers: number;
    gossipPeers: number;
    connectedPeers: number;
  } {
    return {
      totalPeers: this.knownPeers.size,
      bootstrapPeers: this.knownPeers.size - this.gossipDiscoveredPeers.size,
      gossipPeers: this.gossipDiscoveredPeers.size,
      connectedPeers: this.connectedPeers.size,
    };
  }

  /**
   * Get all known peers
   */
  getKnownPeers(): GossipPeerInfo[] {
    return Array.from(this.knownPeers.values());
  }

  /**
   * Check if a peer was discovered via gossip
   */
  isGossipDiscovered(nodeId: NodeId): boolean {
    return this.gossipDiscoveredPeers.has(nodeId);
  }

  private generateRequestId(): string {
    return secureId('gossip', 8);
  }
}

/**
 * Type guard for gossip messages
 */
export function isPeerGossipMessage(payload: unknown): payload is PeerGossipMessage {
  if (typeof payload !== 'object' || payload === null) return false;
  const p = payload as Record<string, unknown>;
  return (p.type === 'peer-list-request' || p.type === 'peer-list-response') && typeof p.requestId === 'string';
}
