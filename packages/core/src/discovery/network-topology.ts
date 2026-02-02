import type { NodeId } from '../identity/index.js';

export type NodeRole = 'client' | 'relay' | 'bootstrap';

export interface PeerInfo {
  nodeId: NodeId;
  username: string;
  publicKey: string;
  reachableVia: NodeId[];
  lastSeen: number;
  role: NodeRole;
}

export type PeerStatus = 'online' | 'stale' | 'offline';

export class NetworkTopology {
  private peers = new Map<NodeId, PeerInfo>();
  private staleThresholdMs: number;

  constructor(staleThresholdMs = 10000) {
    this.staleThresholdMs = staleThresholdMs;
  }

  addPeer(info: PeerInfo): void {
    this.peers.set(info.nodeId, { ...info, lastSeen: info.lastSeen || Date.now() });
  }

  removePeer(nodeId: NodeId): boolean {
    return this.peers.delete(nodeId);
  }

  getPeer(nodeId: NodeId): PeerInfo | undefined {
    return this.peers.get(nodeId);
  }

  updateLastSeen(nodeId: NodeId, timestamp?: number): void {
    const peer = this.peers.get(nodeId);
    if (peer) {
      peer.lastSeen = timestamp ?? Date.now();
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

  size(): number {
    return this.peers.size;
  }

  clear(): void {
    this.peers.clear();
  }
}
