/**
 * BOOTSTRAP LAYER (ADR-002) — TEMPORARY
 *
 * This module contains bootstrap code that uses WebSocket signaling for network discovery.
 * Per ADR-002, this is a temporary mechanism that will be replaced by distributed DHT
 * in Epic 7 (autonomous peer discovery). The bootstrap layer is intentionally isolated
 * to enable future replacement without affecting core protocol logic.
 *
 * Transition path: WebSocket signaling → Multiple seed servers → DHT → Zero fixed infrastructure
 *
 * @see architecture.md#ADR-002 for bootstrap elimination roadmap
 */

import {
  type ConnectionType,
  DirectPathManager,
  HeartbeatManager,
  IdentityManager,
  type IdentityStorage,
  MemoryStorage,
  type MessageEnvelope,
  type MessageStatus,
  type MessageStatusEntry,
  MessageTracker,
  NetworkTopology,
  type NodeId,
  type NodeRole,
  type PeerInfo,
  READ_RECEIPT_TYPE,
  RelaySelector,
  RelayStats,
  type RelayStatsData,
  RoleManager,
  Router,
  TomError,
  type TransportEvents,
  TransportLayer,
} from 'tom-protocol';
import type { PeerConnection, SignalingClient } from 'tom-protocol';

// Re-export MessageStatus for SDK consumers
export type { MessageStatus, MessageStatusEntry } from 'tom-protocol';

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
export type CapacityWarningHandler = (stats: RelayStatsData, reason: string) => void;
export type ConnectionTypeChangedHandler = (peerId: string, connectionType: ConnectionType) => void;
export type MessageStatusChangedHandler = (
  messageId: string,
  previousStatus: MessageStatus,
  newStatus: MessageStatus,
) => void;
export type MessageReadHandler = (messageId: string, readAt: number, from: string) => void;

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
  private relayStats: RelayStats;
  private directPathManager: DirectPathManager | null = null;
  private messageTracker: MessageTracker;
  /** Map of message IDs to sender node IDs for read receipts */
  private messageOrigins = new Map<string, NodeId>();
  /** Set of message IDs for which read receipts have been sent */
  private readReceiptsSent = new Set<string>();
  /** Cleanup interval handle */
  private cleanupInterval: ReturnType<typeof setInterval> | null = null;

  private messageHandlers: MessageHandler[] = [];
  private participantHandlers: ParticipantHandler[] = [];
  private statusHandlers: StatusHandler[] = [];
  private ackHandlers: Array<(messageId: string) => void> = [];
  private peerDiscoveredHandlers: PeerDiscoveredHandler[] = [];
  private peerDepartedHandlers: PeerDepartedHandler[] = [];
  private peerStaleHandlers: PeerStaleHandler[] = [];
  private roleChangedHandlers: RoleChangedHandler[] = [];
  private capacityWarningHandlers: CapacityWarningHandler[] = [];
  private connectionTypeChangedHandlers: ConnectionTypeChangedHandler[] = [];
  private messageStatusChangedHandlers: MessageStatusChangedHandler[] = [];
  private messageReadHandlers: MessageReadHandler[] = [];

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
    this.relayStats = new RelayStats({
      events: {
        onCapacityWarning: (stats, reason) => {
          for (const handler of this.capacityWarningHandlers) handler(stats, reason);
          this.emitStatus('capacity:warning', reason);
        },
      },
    });
    this.messageTracker = new MessageTracker({
      onStatusChanged: (messageId, previousStatus, newStatus) => {
        for (const handler of this.messageStatusChangedHandlers) handler(messageId, previousStatus, newStatus);
        this.emitStatus('message:status', `${messageId}: ${previousStatus} → ${newStatus}`);
      },
    });
  }

  /**
   * Connects this node to the ToM network.
   *
   * **Bootstrap Abstraction (ADR-002):**
   * This method handles all network bootstrap complexity internally. The developer
   * only provides a signaling server URL — all WebSocket management, peer discovery,
   * role assignment, and transport setup are abstracted away.
   *
   * The bootstrap mechanism is temporary and will be replaced by distributed DHT
   * in Epic 7. This abstraction ensures that future bootstrap changes won't affect
   * application code.
   *
   * @example
   * ```typescript
   * const client = new TomClient({ signalingUrl: 'ws://localhost:3001', username: 'alice' });
   * await client.connect(); // Bootstrap happens automatically
   * ```
   *
   * @throws Error if connection to signaling server fails
   */
  async connect(): Promise<void> {
    const identityResult = await this.identity.init();
    this.nodeId = this.identity.getNodeId();

    // Re-bind topology with selfNodeId for periodic role re-evaluation
    this.roleManager.bindTopology(this.topology, this.nodeId);

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
        // Store message origin for read receipt routing
        this.messageOrigins.set(envelope.id, envelope.from);
        for (const handler of this.messageHandlers) handler(envelope);
      },
      onMessageForwarded: (envelope, nextHop) => {
        this.relayStats.recordRelay();
        this.emitStatus('message:forwarded', nextHop);
      },
      onMessageRejected: (envelope, reason) => this.emitStatus('message:rejected', reason),
      onAckReceived: (messageId) => {
        for (const handler of this.ackHandlers) handler(messageId);
      },
      onAckFailed: (messageId, reason) => this.emitStatus('ack:failed', `${messageId}: ${reason}`),
      // Enhanced ACK handling for status tracking
      onRelayAckReceived: (messageId) => {
        this.messageTracker.markRelayed(messageId);
        this.relayStats.recordRelayAck();
      },
      onDeliveryAckReceived: (messageId) => {
        this.messageTracker.markDelivered(messageId);
      },
      onReadReceiptReceived: (messageId, readAt, from) => {
        this.messageTracker.markRead(messageId);
        for (const handler of this.messageReadHandlers) handler(messageId, readAt, from);
      },
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

    // Initialize direct path manager for direct connections after relay introduction
    this.directPathManager = new DirectPathManager(this.nodeId, this.transport, {
      onDirectPathEstablished: (peerId) => {
        for (const handler of this.connectionTypeChangedHandlers) handler(peerId, 'direct');
        this.emitStatus('direct-path:established', peerId);
      },
      onDirectPathLost: (peerId) => {
        for (const handler of this.connectionTypeChangedHandlers) handler(peerId, 'relay');
        this.emitStatus('direct-path:lost', peerId);
      },
      onDirectPathRestored: (peerId) => {
        for (const handler of this.connectionTypeChangedHandlers) handler(peerId, 'direct');
        this.emitStatus('direct-path:restored', peerId);
      },
    });

    // Connect DirectPathManager to Router for direct path preference
    this.router.setDirectPathManager(this.directPathManager);

    // Initial self-evaluation — assign own role
    this.roleManager.evaluateNode(this.nodeId, this.topology);

    // Start periodic cleanup of old message tracking data (every 5 minutes)
    this.cleanupInterval = setInterval(() => this.cleanupMessageTracking(), 5 * 60 * 1000);

    this.emitStatus('connected');
  }

  /**
   * Clean up old message tracking data to prevent memory leaks.
   * Removes messages that have been read for more than 10 minutes.
   */
  private cleanupMessageTracking(): void {
    const maxAgeMs = 10 * 60 * 1000; // 10 minutes
    const removed = this.messageTracker.cleanupOldMessages(maxAgeMs);

    if (removed > 0) {
      // Also clean up associated data
      for (const messageId of this.readReceiptsSent) {
        if (!this.messageTracker.getStatus(messageId)) {
          this.readReceiptsSent.delete(messageId);
          this.messageOrigins.delete(messageId);
        }
      }
      this.emitStatus('cleanup:completed', `${removed} messages`);
    }
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
      this.roleManager.reassignRoles(this.topology, this.nodeId);
    }
    if (msg.action === 'leave') {
      this.topology.removePeer(msg.nodeId);
      this.heartbeat?.untrackPeer(msg.nodeId);
      this.roleManager.removeAssignment(msg.nodeId);
      for (const handler of this.peerDepartedHandlers) handler(msg.nodeId);
      // Re-evaluate roles when network changes
      this.roleManager.reassignRoles(this.topology, this.nodeId);
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

    // Track message status (starts at 'pending')
    this.messageTracker.track(envelope.id, to);

    // Track conversation for direct path optimization
    this.directPathManager?.trackConversation(envelope);

    if (selectedRelay) {
      // Ensure relay peer is connected
      await this.transport.connectToPeer(selectedRelay);

      // Use sendWithDirectPreference if we have a direct path, otherwise use relay
      const sentDirect = this.router.sendWithDirectPreference(envelope, selectedRelay);
      if (sentDirect) {
        this.emitStatus('message:sent:direct', envelope.id);
      }
    } else {
      // Ensure direct peer is connected (fallback when no relay available)
      await this.transport.connectToPeer(to);
      this.transport.sendTo(to, envelope);
    }

    this.relayStats.recordOwnMessage();
    this.messageTracker.markSent(envelope.id);
    this.emitStatus('message:sent', envelope.id);

    // Attempt to establish direct path after first relay exchange (async, non-blocking)
    if (selectedRelay && this.directPathManager) {
      this.directPathManager.attemptDirectPath(to).catch(() => {
        // Direct path attempt failed, continue using relay (silent failure)
      });
    }

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

  onCapacityWarning(handler: CapacityWarningHandler): void {
    this.capacityWarningHandlers.push(handler);
  }

  onConnectionTypeChanged(handler: ConnectionTypeChangedHandler): void {
    this.connectionTypeChangedHandlers.push(handler);
  }

  /**
   * Register a handler for message status changes.
   * Fires when a message transitions through: pending → sent → relayed → delivered → read
   */
  onMessageStatusChanged(handler: MessageStatusChangedHandler): void {
    this.messageStatusChangedHandlers.push(handler);
  }

  /**
   * Register a handler for when a sent message is read by the recipient.
   */
  onMessageRead(handler: MessageReadHandler): void {
    this.messageReadHandlers.push(handler);
  }

  /**
   * Get the current status of a tracked message.
   * @returns The message status entry, or undefined if not tracked
   */
  getMessageStatus(messageId: string): MessageStatusEntry | undefined {
    return this.messageTracker.getStatus(messageId);
  }

  /**
   * Mark a received message as read and send a read receipt to the sender.
   * Call this when your UI displays the message to the user.
   *
   * Idempotent: calling multiple times for the same message only sends one receipt.
   *
   * @param messageId - The ID of the message that was read
   * @returns true if read receipt was sent, false if already sent or message origin unknown
   */
  markAsRead(messageId: string): boolean {
    // Prevent duplicate read receipts
    if (this.readReceiptsSent.has(messageId)) {
      return false;
    }

    const senderId = this.messageOrigins.get(messageId);
    if (!senderId || !this.router || !this.relaySelector) {
      return false;
    }

    // Mark as sent BEFORE sending to prevent race conditions
    this.readReceiptsSent.add(messageId);

    // Create read receipt envelope
    const readReceipt = this.router.createEnvelope(
      senderId,
      READ_RECEIPT_TYPE,
      { originalMessageId: messageId, readAt: Date.now() },
      [],
    );

    // Send read receipt (best-effort, fire-and-forget)
    try {
      const selection = this.relaySelector.selectBestRelay(senderId, this.topology);
      if (selection.relayId) {
        this.router.sendWithDirectPreference(readReceipt, selection.relayId);
      } else {
        // Try direct send if no relay available
        this.transport?.sendTo(senderId, readReceipt);
      }
      this.emitStatus('read-receipt:sent', messageId);
      return true;
    } catch {
      // Read receipt failed - best effort, don't throw
      this.emitStatus('read-receipt:failed', messageId);
      return false;
    }
  }

  /**
   * Get the connection type for a peer.
   * @returns 'direct' if direct path is active, 'relay' if using relay, 'disconnected' if no conversation
   */
  getConnectionType(peerId: NodeId): ConnectionType {
    if (!this.directPathManager) {
      return 'disconnected';
    }
    return this.directPathManager.getConnectionType(peerId);
  }

  /**
   * Get list of peers with active direct connections.
   */
  getDirectPeers(): NodeId[] {
    return this.directPathManager?.getDirectPeers() ?? [];
  }

  getRelayStats(): RelayStatsData {
    return this.relayStats.getStats();
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
    if (this.cleanupInterval) {
      clearInterval(this.cleanupInterval);
      this.cleanupInterval = null;
    }
    this.transport?.close();
    this.ws?.close();
    this.transport = null;
    this.router = null;
    this.ws = null;
    this.heartbeat = null;
  }

  private handleIncomingMessage(envelope: MessageEnvelope): void {
    // Track conversation for direct path optimization
    this.directPathManager?.trackConversation(envelope);
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
