import {
  HeartbeatManager,
  IdentityManager,
  type IdentityStorage,
  MemoryStorage,
  type MessageEnvelope,
  NetworkTopology,
  type NodeId,
  type PeerInfo,
  Router,
  type TransportEvents,
  TransportLayer,
} from 'tom-protocol';
import type { PeerConnection, SignalingClient } from 'tom-protocol';

export interface TomClientOptions {
  signalingUrl: string;
  username: string;
  storage?: IdentityStorage;
}

export type MessageHandler = (envelope: MessageEnvelope) => void;
export type ParticipantHandler = (participants: Array<{ nodeId: string; username: string }>) => void;
export type StatusHandler = (status: string, detail?: string) => void;
export type PeerDiscoveredHandler = (peer: PeerInfo) => void;
export type PeerDepartedHandler = (nodeId: string) => void;
export type PeerStaleHandler = (nodeId: string) => void;

export class TomClient {
  private identity: IdentityManager;
  private transport: TransportLayer | null = null;
  private router: Router | null = null;
  private ws: WebSocket | null = null;
  private nodeId: NodeId = '';
  private username: string;
  private signalingUrl: string;
  private topology: NetworkTopology;
  private heartbeat: HeartbeatManager | null = null;

  private messageHandlers: MessageHandler[] = [];
  private participantHandlers: ParticipantHandler[] = [];
  private statusHandlers: StatusHandler[] = [];
  private ackHandlers: Array<(messageId: string) => void> = [];
  private peerDiscoveredHandlers: PeerDiscoveredHandler[] = [];
  private peerDepartedHandlers: PeerDepartedHandler[] = [];
  private peerStaleHandlers: PeerStaleHandler[] = [];

  constructor(options: TomClientOptions) {
    this.username = options.username;
    this.signalingUrl = options.signalingUrl;
    this.identity = new IdentityManager(options.storage ?? new MemoryStorage());
    this.topology = new NetworkTopology();
  }

  async connect(): Promise<void> {
    const identityResult = await this.identity.init();
    this.nodeId = this.identity.getNodeId();

    this.emitStatus('identity:ready', this.nodeId);

    // Connect to signaling server
    this.ws = new WebSocket(this.signalingUrl);

    const ws = this.ws;
    if (!ws) throw new Error('WebSocket not initialized');

    await new Promise<void>((resolve, reject) => {
      ws.onopen = () => resolve();
      ws.onerror = () => reject(new Error('Failed to connect to signaling server'));
    });

    const signaling: SignalingClient = {
      send: (msg) => this.ws?.send(JSON.stringify(msg)),
      onMessage: null,
      close: () => this.ws?.close(),
    };

    const transportEvents: TransportEvents = {
      onPeerConnected: (peerId) => this.emitStatus('peer:connected', peerId),
      onPeerDisconnected: (peerId) => this.emitStatus('peer:disconnected', peerId),
      onMessage: (envelope) => this.handleIncomingMessage(envelope),
      onError: (peerId, error) => this.emitStatus('error', `${peerId}: ${error.message}`),
    };

    this.transport = new TransportLayer(this.nodeId, signaling, transportEvents, (peerId) =>
      this.createSimplePeerConnection(peerId),
    );

    this.router = new Router(this.nodeId, this.transport, {
      onMessageDelivered: (envelope) => {
        for (const handler of this.messageHandlers) handler(envelope);
      },
      onMessageForwarded: (envelope, nextHop) => this.emitStatus('message:forwarded', nextHop),
      onMessageRejected: (envelope, reason) => this.emitStatus('message:rejected', reason),
      onAckReceived: (messageId) => {
        for (const handler of this.ackHandlers) handler(messageId);
      },
      onAckFailed: (messageId, reason) => this.emitStatus('ack:failed', `${messageId}: ${reason}`),
    });

    // Setup heartbeat
    this.heartbeat = new HeartbeatManager(
      {
        sendHeartbeat: (_nodeId) => {
          this.ws?.send(JSON.stringify({ type: 'heartbeat' }));
        },
        broadcastHeartbeat: () => {
          this.ws?.send(JSON.stringify({ type: 'heartbeat' }));
        },
      },
      {
        onPeerStale: (nodeId) => {
          for (const handler of this.peerStaleHandlers) handler(nodeId);
        },
        onPeerDeparted: (nodeId) => {
          this.topology.removePeer(nodeId);
          for (const handler of this.peerDepartedHandlers) handler(nodeId);
        },
      },
    );

    // Register with signaling server (nodeId is derived from publicKey)
    this.ws.send(
      JSON.stringify({
        type: 'register',
        nodeId: this.nodeId,
        username: this.username,
        publicKey: this.nodeId,
      }),
    );

    // Handle signaling messages
    this.ws.onmessage = (event) => {
      const msg = JSON.parse(event.data as string);

      if (msg.type === 'participants') {
        // Sync topology with participant list — ensures peers connected before us are tracked
        for (const p of msg.participants as Array<{ nodeId: string; username: string }>) {
          if (p.nodeId !== this.nodeId && !this.topology.getPeer(p.nodeId)) {
            this.topology.addPeer({
              nodeId: p.nodeId,
              username: p.username,
              publicKey: p.nodeId,
              reachableVia: [],
              lastSeen: Date.now(),
              role: 'client',
            });
            this.heartbeat?.trackPeer(p.nodeId);
          }
        }
        for (const handler of this.participantHandlers) handler(msg.participants);
      }

      if (msg.type === 'presence') {
        this.handlePresence(msg);
      }

      if (msg.type === 'heartbeat' && msg.from) {
        this.topology.updateLastSeen(msg.from);
        this.heartbeat?.recordHeartbeat(msg.from);
      }

      if (msg.type === 'signal') {
        // Check if this is a relayed message envelope
        if (msg.payload?.type === 'message' && msg.payload?.envelope) {
          this.handleIncomingMessage(msg.payload.envelope);
        } else {
          signaling.onMessage?.(msg);
        }
      }
    };

    this.ws.onclose = () => {
      this.heartbeat?.stop();
      this.emitStatus('signaling:disconnected');
    };

    this.heartbeat.start();
    this.emitStatus('connected');
  }

  private handlePresence(msg: {
    action: string;
    nodeId: string;
    username: string;
    publicKey?: string;
  }): void {
    if (msg.action === 'join') {
      const peerInfo: PeerInfo = {
        nodeId: msg.nodeId,
        username: msg.username,
        publicKey: msg.publicKey ?? '',
        reachableVia: [],
        lastSeen: Date.now(),
        role: 'client',
      };
      this.topology.addPeer(peerInfo);
      this.heartbeat?.trackPeer(msg.nodeId);
      for (const handler of this.peerDiscoveredHandlers) handler(peerInfo);
    }
    if (msg.action === 'leave') {
      this.topology.removePeer(msg.nodeId);
      this.heartbeat?.untrackPeer(msg.nodeId);
      for (const handler of this.peerDepartedHandlers) handler(msg.nodeId);
    }
  }

  async sendMessage(to: NodeId, text: string, relayId?: NodeId): Promise<MessageEnvelope | null> {
    if (!this.router || !this.transport) return null;

    const envelope = this.router.createEnvelope(to, 'chat', { text }, relayId ? [relayId] : []);

    if (relayId) {
      // Ensure relay peer is connected
      await this.transport.connectToPeer(relayId);
      this.router.sendViaRelay(envelope, relayId);
    } else {
      // Ensure direct peer is connected
      await this.transport.connectToPeer(to);
      this.transport.sendTo(to, envelope);
    }

    this.emitStatus('message:sent', envelope.id);
    return envelope;
  }

  onMessage(handler: MessageHandler): void {
    this.messageHandlers.push(handler);
  }

  onParticipants(handler: ParticipantHandler): void {
    this.participantHandlers.push(handler);
  }

  onStatus(handler: StatusHandler): void {
    this.statusHandlers.push(handler);
  }

  onAck(handler: (messageId: string) => void): void {
    this.ackHandlers.push(handler);
  }

  onPeerDiscovered(handler: PeerDiscoveredHandler): void {
    this.peerDiscoveredHandlers.push(handler);
  }

  onPeerDeparted(handler: PeerDepartedHandler): void {
    this.peerDepartedHandlers.push(handler);
  }

  onPeerStale(handler: PeerStaleHandler): void {
    this.peerStaleHandlers.push(handler);
  }

  getNodeId(): NodeId {
    return this.nodeId;
  }

  getTopology(): PeerInfo[] {
    return this.topology.getReachablePeers();
  }

  getTopologyInstance(): NetworkTopology {
    return this.topology;
  }

  disconnect(): void {
    this.heartbeat?.stop();
    this.transport?.close();
    this.ws?.close();
    this.transport = null;
    this.router = null;
    this.ws = null;
    this.heartbeat = null;
  }

  private handleIncomingMessage(envelope: MessageEnvelope): void {
    this.router?.handleIncoming(envelope);
  }

  private emitStatus(status: string, detail?: string): void {
    for (const handler of this.statusHandlers) handler(status, detail);
  }

  private createSimplePeerConnection(peerId: string): PeerConnection {
    // Simplified peer connection using signaling relay for PoC
    // In production, this would use WebRTC DataChannels
    return {
      peerId,
      send: (envelope) => {
        // Route through signaling server as relay for PoC
        this.ws?.send(
          JSON.stringify({
            type: 'signal',
            from: this.nodeId,
            to: peerId,
            payload: { type: 'message', envelope },
          }),
        );
      },
      close: () => {},
      onMessage: null,
      onClose: null,
    };
  }
}
