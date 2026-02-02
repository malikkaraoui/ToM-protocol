import type { MessageEnvelope } from '../types/envelope.js';

export interface PeerConnection {
  peerId: string;
  send(envelope: MessageEnvelope): void;
  close(): void;
  onMessage: ((envelope: MessageEnvelope) => void) | null;
  onClose: (() => void) | null;
}

export interface TransportEvents {
  onPeerConnected: (peerId: string) => void;
  onPeerDisconnected: (peerId: string) => void;
  onMessage: (envelope: MessageEnvelope) => void;
  onError: (peerId: string, error: Error) => void;
}

export interface SignalingClient {
  send(message: unknown): void;
  onMessage: ((message: unknown) => void) | null;
  close(): void;
}

export class TransportLayer {
  private peers = new Map<string, PeerConnection>();
  private events: TransportEvents;
  private signaling: SignalingClient;
  private localNodeId: string;
  private createPeerConnection: (peerId: string, signaling: SignalingClient, localNodeId: string) => PeerConnection;

  constructor(
    localNodeId: string,
    signaling: SignalingClient,
    events: TransportEvents,
    createPeerConnection: (peerId: string, signaling: SignalingClient, localNodeId: string) => PeerConnection,
  ) {
    this.localNodeId = localNodeId;
    this.signaling = signaling;
    this.events = events;
    this.createPeerConnection = createPeerConnection;
  }

  async connectToPeer(peerId: string): Promise<PeerConnection> {
    const existing = this.peers.get(peerId);
    if (existing) return existing;

    const conn = this.createPeerConnection(peerId, this.signaling, this.localNodeId);
    this.registerPeer(peerId, conn);
    return conn;
  }

  registerPeer(peerId: string, conn: PeerConnection): void {
    conn.onMessage = (envelope) => {
      this.events.onMessage(envelope);
    };
    conn.onClose = () => {
      this.peers.delete(peerId);
      this.events.onPeerDisconnected(peerId);
    };
    this.peers.set(peerId, conn);
    this.events.onPeerConnected(peerId);
  }

  sendTo(peerId: string, envelope: MessageEnvelope): void {
    const conn = this.peers.get(peerId);
    if (!conn) {
      this.events.onError(peerId, new Error(`No connection to peer ${peerId}`));
      return;
    }
    conn.send(envelope);
  }

  getPeer(peerId: string): PeerConnection | undefined {
    return this.peers.get(peerId);
  }

  getConnectedPeers(): string[] {
    return Array.from(this.peers.keys());
  }

  disconnectPeer(peerId: string): void {
    const conn = this.peers.get(peerId);
    if (conn) {
      conn.close();
      this.peers.delete(peerId);
      this.events.onPeerDisconnected(peerId);
    }
  }

  close(): void {
    for (const [peerId, conn] of this.peers) {
      conn.close();
      this.events.onPeerDisconnected(peerId);
    }
    this.peers.clear();
    this.signaling.close();
  }
}
