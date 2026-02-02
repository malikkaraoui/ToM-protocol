import type { NodeId } from '../identity/index.js';

export interface HeartbeatEvents {
  onPeerStale: (nodeId: NodeId) => void;
  onPeerDeparted: (nodeId: NodeId) => void;
}

export interface HeartbeatSender {
  sendHeartbeat: (nodeId: NodeId) => void;
  broadcastHeartbeat: () => void;
}

export class HeartbeatManager {
  private intervalMs: number;
  private timeoutMs: number;
  private lastSeen = new Map<NodeId, number>();
  private intervalId: ReturnType<typeof setInterval> | null = null;
  private checkId: ReturnType<typeof setInterval> | null = null;
  private events: HeartbeatEvents;
  private sender: HeartbeatSender;
  private stalePeers = new Set<NodeId>();

  constructor(sender: HeartbeatSender, events: HeartbeatEvents, intervalMs = 5000, timeoutMs = 3000) {
    this.sender = sender;
    this.events = events;
    this.intervalMs = intervalMs;
    this.timeoutMs = timeoutMs;
  }

  start(): void {
    this.stop();
    // Send heartbeats periodically
    this.intervalId = setInterval(() => {
      this.sender.broadcastHeartbeat();
    }, this.intervalMs);

    // Check for stale/departed peers
    this.checkId = setInterval(() => {
      this.checkPeers();
    }, 1000);
  }

  stop(): void {
    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = null;
    }
    if (this.checkId) {
      clearInterval(this.checkId);
      this.checkId = null;
    }
    this.lastSeen.clear();
    this.stalePeers.clear();
  }

  recordHeartbeat(nodeId: NodeId): void {
    this.lastSeen.set(nodeId, Date.now());
    this.stalePeers.delete(nodeId);
  }

  trackPeer(nodeId: NodeId): void {
    this.lastSeen.set(nodeId, Date.now());
  }

  untrackPeer(nodeId: NodeId): void {
    this.lastSeen.delete(nodeId);
    this.stalePeers.delete(nodeId);
  }

  private checkPeers(): void {
    const now = Date.now();
    for (const [nodeId, lastSeen] of this.lastSeen.entries()) {
      const elapsed = now - lastSeen;
      if (elapsed >= this.timeoutMs * 2) {
        // Departed
        this.lastSeen.delete(nodeId);
        this.stalePeers.delete(nodeId);
        this.events.onPeerDeparted(nodeId);
      } else if (elapsed >= this.timeoutMs && !this.stalePeers.has(nodeId)) {
        // Stale
        this.stalePeers.add(nodeId);
        this.events.onPeerStale(nodeId);
      }
    }
  }
}
