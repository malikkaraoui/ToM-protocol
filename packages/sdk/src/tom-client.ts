import {
  HeartbeatManager,
  IdentityManager,
  type IdentityStorage,
  MemoryStorage,
  type MessageEnvelope,
  NetworkTopology,
  type NodeId,
  type NodeRole,
  type PeerInfo,
  RelaySelector,
  RoleManager,
  Router,
  TomError,
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
export type RoleChangedHandler = (nodeId: string, roles: NodeRole[]) => void;

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
  private roleManager: RoleManager;
  private relaySelector: RelaySelector | null = null;

  private messageHandlers: MessageHandler[] = [];
  private participantHandlers: ParticipantHandler[] = [];
  private statusHandlers: StatusHandler[] = [];
  private ackHandlers: Array<(messageId: string) => void> = [];
  private peerDiscoveredHandlers: PeerDiscoveredHandler[] = [];
  private peerDepartedHandlers: PeerDepartedHandler[] = [];
  private peerStaleHandlers: PeerStaleHandler[] = [];
  private roleChangedHandlers: RoleChangedHandler[] = [];

  constructor(options: TomClientOptions) {
    this.username = options.username;
    this.signalingUrl = options.signalingUrl;
    this.identity = new IdentityManager(options.storage ?? new MemoryStorage());
    this.topology = new NetworkTopology();
    this.roleManager = new RoleManager({
      onRoleChanged: (nodeId, _oldRoles, newRoles) => {
        for (const handler of this.roleChangedHandlers) handler(nodeId, newRoles);
        // Broadcast own role changes to network
        if (nodeId === this.nodeId) {
          this.ws?.send(JSON.stringify({ type: 'role-assign', nodeId, roles: newRoles }));
          this.emitStatus('role:changed', newRoles.join(', '));
        }
        // Update topology with new roles
        const peer = this.topology.getPeer(nodeId);
        if (peer) {
          peer.roles = [...newRoles];
        }
      },
    });
    this.roleManager.bindTopology(this.topology);
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
          // Don't remove from topology — status is determined by lastSeen age
          // Removal only happens on presence:leave (actual disconnect)
          for (const handler of this.peerDepartedHandlers) handler(nodeId);
        },
      },
      5000,
      10000,
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
        // Sync topology with participant list
        for (const p of msg.participants as Array<{ nodeId: string; username: string }>) {
          if (p.nodeId === this.nodeId) continue;
          const existing = this.topology.getPeer(p.nodeId);
          if (existing) {
            this.topology.updateLastSeen(p.nodeId);
            this.heartbeat?.recordHeartbeat(p.nodeId);
          } else {
            this.topology.addPeer({
              nodeId: p.nodeId,
              username: p.username,
              publicKey: p.nodeId,
              reachableVia: [],
              lastSeen: Date.now(),
              roles: ['client'],
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
        // Re-track peer if it was removed by departed timeout
        this.heartbeat?.trackPeer(msg.from);
        this.heartbeat?.recordHeartbeat(msg.from);
      }

      if (msg.type === 'role-assign' && msg.nodeId && msg.roles) {
        if (msg.nodeId !== this.nodeId) {
          this.roleManager.setRolesFromNetwork(msg.nodeId, msg.roles);
          const peer = this.topology.getPeer(msg.nodeId);
          if (peer) {
            peer.roles = [...msg.roles];
          }
        }
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
    this.roleManager.start();

    // Initialize relay selector
    this.relaySelector = new RelaySelector({ selfNodeId: this.nodeId });

    // Initial self-evaluation — assign own role
    this.roleManager.evaluateNode(this.nodeId, this.topology);

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
        roles: ['client'],
      };
      this.topology.addPeer(peerInfo);
      this.heartbeat?.trackPeer(msg.nodeId);
      for (const handler of this.peerDiscoveredHandlers) handler(peerInfo);
      // Re-evaluate roles when network changes
      this.roleManager.reassignRoles(this.topology);
    }
    if (msg.action === 'leave') {
      this.topology.removePeer(msg.nodeId);
      this.heartbeat?.untrackPeer(msg.nodeId);
      this.roleManager.removeAssignment(msg.nodeId);
      for (const handler of this.peerDepartedHandlers) handler(msg.nodeId);
      // Re-evaluate roles when network changes
      this.roleManager.reassignRoles(this.topology);
    }
  }

  async sendMessage(to: NodeId, text: string, relayId?: NodeId): Promise<MessageEnvelope | null> {
    if (!this.router || !this.transport) return null;

    // Auto-select relay if not provided
    let selectedRelay = relayId;
    if (!selectedRelay && this.relaySelector) {
      const selection = this.relaySelector.selectBestRelay(to, this.topology);

      if (selection.relayId) {
        selectedRelay = selection.relayId;
        this.emitStatus('relay:selected', selectedRelay);
      } else if (selection.reason === 'recipient-is-self') {
        throw new TomError('PEER_UNREACHABLE', 'Cannot send message to self', { to, reason: selection.reason });
      } else if (selection.reason === 'no-relays-available' || selection.reason === 'no-peers') {
        // No relay available - attempt direct connection as fallback
        this.emitStatus('relay:none', selection.reason);
      }
    }

    const envelope = this.router.createEnvelope(to, 'chat', { text }, selectedRelay ? [selectedRelay] : []);

    if (selectedRelay) {
      // Ensure relay peer is connected
      await this.transport.connectToPeer(selectedRelay);
      this.router.sendViaRelay(envelope, selectedRelay);
    } else {
      // Ensure direct peer is connected (fallback when no relay available)
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

  onRoleChanged(handler: RoleChangedHandler): void {
    this.roleChangedHandlers.push(handler);
  }

  getCurrentRoles(): NodeRole[] {
    return this.roleManager.getCurrentRoles(this.nodeId);
  }

  getPeerRoles(nodeId: NodeId): NodeRole[] {
    return this.roleManager.getCurrentRoles(nodeId);
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
    this.roleManager.stop();
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
