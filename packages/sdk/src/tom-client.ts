import {
  IdentityManager,
  type IdentityStorage,
  MemoryStorage,
  type MessageEnvelope,
  type NodeId,
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

export class TomClient {
  private identity: IdentityManager;
  private transport: TransportLayer | null = null;
  private router: Router | null = null;
  private ws: WebSocket | null = null;
  private nodeId: NodeId = '';
  private username: string;
  private signalingUrl: string;

  private messageHandlers: MessageHandler[] = [];
  private participantHandlers: ParticipantHandler[] = [];
  private statusHandlers: StatusHandler[] = [];
  private ackHandlers: Array<(messageId: string) => void> = [];

  constructor(options: TomClientOptions) {
    this.username = options.username;
    this.signalingUrl = options.signalingUrl;
    this.identity = new IdentityManager(options.storage ?? new MemoryStorage());
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

    // Register with signaling server
    this.ws.send(JSON.stringify({ type: 'register', nodeId: this.nodeId, username: this.username }));

    // Handle signaling messages
    this.ws.onmessage = (event) => {
      const msg = JSON.parse(event.data as string);
      if (msg.type === 'participants') {
        for (const handler of this.participantHandlers) handler(msg.participants);
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

    this.ws.onclose = () => this.emitStatus('signaling:disconnected');

    this.emitStatus('connected');
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

  getNodeId(): NodeId {
    return this.nodeId;
  }

  disconnect(): void {
    this.transport?.close();
    this.ws?.close();
    this.transport = null;
    this.router = null;
    this.ws = null;
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
