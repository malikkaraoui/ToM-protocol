import type { NodeId } from '../identity/index.js';

export type NodeRole = 'client' | 'relay' | 'observer' | 'bootstrap' | 'backup';

/** Maximum clock drift tolerance (5 minutes into future) */
const MAX_FUTURE_DRIFT_MS = 5 * 60 * 1000;

/** Maximum age for lastSeen (1 hour - beyond this, peer is definitely offline) */
const MAX_PAST_LASTSEEN_MS = 60 * 60 * 1000;

export interface PeerInfo {
  nodeId: NodeId;
  username: string;
  publicKey: string;
  reachableVia: NodeId[];
  lastSeen: number;
  roles: NodeRole[];
}

export type PeerStatus = 'online' | 'stale' | 'offline';

export class NetworkTopology {
  private peers = new Map<NodeId, PeerInfo>();
  private staleThresholdMs: number;

  constructor(staleThresholdMs = 10000) {
    this.staleThresholdMs = staleThresholdMs;
  }

  /**
   * Add a peer to the topology.
   * Clamps lastSeen timestamp to prevent manipulation.
   * @security Prevents forged timestamps from new peers
   */
  addPeer(info: PeerInfo): void {
    const now = Date.now();
    let safeLastSeen = info.lastSeen || now;
    // Clamp: not too far in the future
    safeLastSeen = Math.min(safeLastSeen, now + MAX_FUTURE_DRIFT_MS);
    // Clamp: not too far in the past
    safeLastSeen = Math.max(safeLastSeen, now - MAX_PAST_LASTSEEN_MS);
    this.peers.set(info.nodeId, { ...info, lastSeen: safeLastSeen });
  }

  removePeer(nodeId: NodeId): boolean {
    return this.peers.delete(nodeId);
  }

  getPeer(nodeId: NodeId): PeerInfo | undefined {
    return this.peers.get(nodeId);
  }

  /**
   * Update lastSeen timestamp for a peer.
   * Clamps timestamp to prevent manipulation:
   * - Not more than MAX_FUTURE_DRIFT_MS in the future
   * - Not more than MAX_PAST_LASTSEEN_MS in the past
   * @security Prevents forged timestamps from manipulating relay selection
   */
  updateLastSeen(nodeId: NodeId, timestamp?: number): void {
    const peer = this.peers.get(nodeId);
    if (peer) {
      const now = Date.now();
      let safeTimestamp = timestamp ?? now;
      // Clamp: not too far in the future (clock drift tolerance)
      safeTimestamp = Math.min(safeTimestamp, now + MAX_FUTURE_DRIFT_MS);
      // Clamp: not too far in the past (would make peer appear offline)
      safeTimestamp = Math.max(safeTimestamp, now - MAX_PAST_LASTSEEN_MS);
      peer.lastSeen = safeTimestamp;
    }
  }

  getPeerStatus(nodeId: NodeId): PeerStatus {
    const peer = this.peers.get(nodeId);
    if (!peer) return 'offline';
    const elapsed = Date.now() - peer.lastSeen;
    if (elapsed < this.staleThresholdMs) return 'online';
    if (elapsed < this.staleThresholdMs * 2) return 'stale';
    return 'offline';
  }

  getReachablePeers(): PeerInfo[] {
    return Array.from(this.peers.values());
  }

  getDirectPeers(): PeerInfo[] {
    return Array.from(this.peers.values()).filter((p) => p.reachableVia.length === 0);
  }

  getIndirectPeers(): PeerInfo[] {
    return Array.from(this.peers.values()).filter((p) => p.reachableVia.length > 0);
  }

  getOnlinePeers(): PeerInfo[] {
    return Array.from(this.peers.values()).filter((p) => this.getPeerStatus(p.nodeId) !== 'offline');
  }

  getRelayNodes(): PeerInfo[] {
    return Array.from(this.peers.values()).filter((p) => p.roles.includes('relay'));
  }

  getBackupNodes(): PeerInfo[] {
    return Array.from(this.peers.values()).filter((p) => p.roles.includes('backup'));
  }

  getNodesByRole(role: NodeRole): PeerInfo[] {
    return Array.from(this.peers.values()).filter((p) => p.roles.includes(role));
  }

  size(): number {
    return this.peers.size;
  }

  clear(): void {
    this.peers.clear();
  }
}
