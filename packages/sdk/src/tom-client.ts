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
  GroupHub,
  type GroupHubEvents,
  type GroupId,
  type GroupInfo,
  GroupManager,
  type GroupManagerEvents,
  type GroupMember,
  type GroupMessagePayload,
  type GroupMigrationData,
  type GroupPayload,
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
  type PathInfo,
  type PeerInfo,
  type PublicGroupInfo,
  READ_RECEIPT_TYPE,
  RelaySelector,
  RelayStats,
  type RelayStatsData,
  RoleManager,
  Router,
  TomError,
  type TransportEvents,
  TransportLayer,
  extractPathInfo,
  isGroupHubHeartbeat,
  isGroupPayload,
} from 'tom-protocol';
import type { PeerConnection, SignalingClient } from 'tom-protocol';

// Re-export types for SDK consumers
export type { MessageStatus, MessageStatusEntry, PathInfo } from 'tom-protocol';
export { formatLatency } from 'tom-protocol';

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

// Group event handlers (Story 4.6)
export type GroupCreatedHandler = (group: GroupInfo) => void;
export type GroupInviteHandler = (
  groupId: string,
  groupName: string,
  inviterId: string,
  inviterUsername: string,
) => void;
export type GroupMemberJoinedHandler = (groupId: string, member: GroupMember) => void;
export type GroupMemberLeftHandler = (groupId: string, nodeId: string, username: string, reason: string) => void;
export type GroupMessageHandler = (groupId: string, message: GroupMessagePayload) => void;
export type PublicGroupAnnouncedHandler = (group: PublicGroupInfo) => void;
export type GroupJoinProgressHandler = (
  groupId: string,
  status: 'connecting' | 'requesting' | 'waiting' | 'retrying' | 'success' | 'failed',
  attempt: number,
  maxAttempts: number,
) => void;

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
  /** Map of message IDs to received envelope data for path visualization (Story 4.3) */
  private receivedEnvelopes = new Map<string, { envelope: MessageEnvelope; receivedAt: number }>();
  /** Cleanup interval handle */
  private cleanupInterval: ReturnType<typeof setInterval> | null = null;
  /** Group manager for this node (Story 4.6) */
  private groupManager: GroupManager | null = null;
  /** Group hub for relay nodes (Story 4.6) */
  private groupHub: GroupHub | null = null;
  /** Track pending group join requests to prevent duplicate clicks */
  private pendingGroupJoins = new Set<string>();
  /** Track failed relays per message for rerouting (Story 5.2) */
  private failedRelaysPerMessage = new Map<string, Set<string>>();
  /** Track messages currently being rerouted (mutex to prevent parallel reroutes) */
  private reroutingInProgress = new Set<string>();
  /** Maximum reroute attempts per message */
  private static readonly MAX_REROUTE_ATTEMPTS = 3;

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
  // Group handlers (Story 4.6)
  private groupCreatedHandlers: GroupCreatedHandler[] = [];
  private groupInviteHandlers: GroupInviteHandler[] = [];
  private groupMemberJoinedHandlers: GroupMemberJoinedHandler[] = [];
  private groupMemberLeftHandlers: GroupMemberLeftHandler[] = [];
  private groupMessageHandlers: GroupMessageHandler[] = [];
  private publicGroupAnnouncedHandlers: PublicGroupAnnouncedHandler[] = [];
  private groupJoinProgressHandlers: GroupJoinProgressHandler[] = [];
  /** Promise resolvers for pending group joins - resolved when group-sync is received */
  private pendingJoinResolvers = new Map<
    string,
    { resolve: (group: GroupInfo) => void; reject: (error: Error) => void }
  >();

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

          // Initialize GroupHub when becoming a relay (Story 4.6)
          if (newRoles.includes('relay') && !this.groupHub) {
            this.initGroupHub();
          }
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
        // Store envelope data for path visualization (Story 4.3)
        this.receivedEnvelopes.set(envelope.id, { envelope, receivedAt: Date.now() });
        for (const handler of this.messageHandlers) handler(envelope);
      },
      onMessageForwarded: (envelope, nextHop) => {
        const byteSize = new TextEncoder().encode(JSON.stringify(envelope)).length;
        this.relayStats.recordRelay(byteSize);
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
      // Rerouting handlers (Story 5.2)
      onRerouteNeeded: (envelope, failedRelayId) => {
        this.handleRerouteNeeded(envelope, failedRelayId);
      },
      onMessageQueued: (envelope, reason) => {
        this.emitStatus('message:queued', `${envelope.id}: ${reason}`);
      },
      onDuplicateMessage: (messageId, from) => {
        this.emitStatus('message:duplicate', `${messageId} from ${from}`);
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

    // Initialize GroupManager for group chat (Story 4.6)
    this.groupManager = new GroupManager(this.nodeId, this.username, {
      onGroupCreated: (group) => {
        for (const handler of this.groupCreatedHandlers) handler(group);
        this.emitStatus('group:created', group.groupId);
      },
      onGroupInvite: (groupId, groupName, inviterId, inviterUsername) => {
        for (const handler of this.groupInviteHandlers) handler(groupId, groupName, inviterId, inviterUsername);
        this.emitStatus('group:invite', `${groupName} from ${inviterUsername}`);
      },
      onMemberJoined: (groupId, member) => {
        for (const handler of this.groupMemberJoinedHandlers) handler(groupId, member);
        this.emitStatus('group:member-joined', `${member.username} joined ${groupId}`);
      },
      onMemberLeft: (groupId, nodeId, username, reason) => {
        for (const handler of this.groupMemberLeftHandlers) handler(groupId, nodeId, username, reason);
        this.emitStatus('group:member-left', `${username} left ${groupId}`);
      },
      onGroupMessage: (groupId, message) => {
        for (const handler of this.groupMessageHandlers) handler(groupId, message);
      },
      onPublicGroupAnnounced: (group) => {
        for (const handler of this.publicGroupAnnouncedHandlers) handler(group);
        this.emitStatus('group:announced', `${group.groupName} by ${group.creatorUsername}`);
      },
    });

    // Initial self-evaluation — assign own role
    this.roleManager.evaluateNode(this.nodeId, this.topology);

    // Start periodic cleanup of old message tracking data (every 5 minutes)
    this.cleanupInterval = setInterval(() => this.cleanupMessageTracking(), 5 * 60 * 1000);

    this.emitStatus('connected');
  }

  /**
   * Clean up old message tracking data to prevent memory leaks.
   * Removes messages that have been read for more than 10 minutes.
   * Also cleans up router caches (deduplication, ACK replay).
   */
  private cleanupMessageTracking(): void {
    const maxAgeMs = 10 * 60 * 1000; // 10 minutes
    const removed = this.messageTracker.cleanupOldMessages(maxAgeMs);

    // Clean up router caches (deduplication, ACK replay, read receipt replay)
    const routerCacheRemoved = this.router?.cleanupCaches() ?? 0;

    // Clean up rerouting tracking for stale messages
    for (const messageId of this.failedRelaysPerMessage.keys()) {
      if (!this.messageTracker.getStatus(messageId)) {
        this.failedRelaysPerMessage.delete(messageId);
      }
    }

    if (removed > 0 || routerCacheRemoved > 0) {
      // Also clean up associated data
      for (const messageId of this.readReceiptsSent) {
        if (!this.messageTracker.getStatus(messageId)) {
          this.readReceiptsSent.delete(messageId);
          this.messageOrigins.delete(messageId);
          this.receivedEnvelopes.delete(messageId);
        }
      }
      this.emitStatus('cleanup:completed', `${removed} messages, ${routerCacheRemoved} cache entries`);
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

    const byteSize = new TextEncoder().encode(JSON.stringify(envelope)).length;
    this.relayStats.recordOwnMessage(byteSize);
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

  /**
   * Send an arbitrary payload to a peer (for game messages, etc.)
   * Unlike sendMessage which wraps text in { text }, this sends the payload directly.
   */
  async sendPayload(to: NodeId, payload: object, relayId?: NodeId): Promise<MessageEnvelope | null> {
    if (!this.router || !this.transport) return null;

    // Auto-select relay if not provided
    let selectedRelay = relayId;
    if (!selectedRelay && this.relaySelector) {
      const selection = this.relaySelector.selectBestRelay(to, this.topology);

      if (selection.relayId) {
        selectedRelay = selection.relayId;
        this.emitStatus('relay:selected', selectedRelay);
      } else if (selection.reason === 'recipient-is-self') {
        throw new TomError('PEER_UNREACHABLE', 'Cannot send payload to self', { to, reason: selection.reason });
      } else if (selection.reason === 'no-relays-available' || selection.reason === 'no-peers') {
        this.emitStatus('relay:none', selection.reason);
      }
    }

    // Send payload directly (not wrapped in { text })
    const envelope = this.router.createEnvelope(to, 'app', payload, selectedRelay ? [selectedRelay] : []);

    // Track message status
    this.messageTracker.track(envelope.id, to);

    // Track conversation for direct path optimization
    this.directPathManager?.trackConversation(envelope);

    if (selectedRelay) {
      await this.transport.connectToPeer(selectedRelay);
      const sentDirect = this.router.sendWithDirectPreference(envelope, selectedRelay);
      if (sentDirect) {
        this.emitStatus('message:sent:direct', envelope.id);
      }
    } else {
      await this.transport.connectToPeer(to);
      this.transport.sendTo(to, envelope);
    }

    const byteSize = new TextEncoder().encode(JSON.stringify(envelope)).length;
    this.relayStats.recordOwnMessage(byteSize);
    this.messageTracker.markSent(envelope.id);
    this.emitStatus('message:sent', envelope.id);

    if (selectedRelay && this.directPathManager) {
      this.directPathManager.attemptDirectPath(to).catch(() => {});
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
   * Get path information for a received message (Story 4.3 - FR14).
   * Shows route type (direct/relay), relay hops, and latency.
   * Derived from envelope metadata — no extra network requests.
   *
   * @param messageId - The ID of the received message
   * @returns PathInfo with routing details, or undefined if message not found
   */
  getPathInfo(messageId: string): PathInfo | undefined {
    const stored = this.receivedEnvelopes.get(messageId);
    if (!stored) {
      return undefined;
    }
    return extractPathInfo(stored.envelope, stored.receivedAt);
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

  // ============================================
  // Group Methods (Story 4.6)
  // ============================================

  /**
   * Create a new group chat.
   * Automatically selects a relay to act as hub.
   */
  async createGroup(
    name: string,
    initialMembers: { nodeId: string; username: string }[] = [],
  ): Promise<GroupInfo | null> {
    if (!this.groupManager || !this.relaySelector) return null;

    // Find a relay to act as hub
    const relays = this.topology.getReachablePeers().filter((p) => p.roles?.includes('relay'));

    // Check if we are a relay ourselves - we can be our own hub
    const myRoles = this.getCurrentRoles();
    const selfIsRelay = myRoles.includes('relay');

    let hubRelayId: string;
    if (selfIsRelay) {
      // Use ourselves as the hub
      hubRelayId = this.nodeId;
    } else if (relays.length > 0) {
      // Select relay with best availability (most recent)
      const hubRelay = relays.sort((a, b) => b.lastSeen - a.lastSeen)[0];
      hubRelayId = hubRelay.nodeId;
    } else {
      this.emitStatus('group:error', 'No relays available to host group');
      return null;
    }

    const group = this.groupManager.createGroup(name, hubRelayId, initialMembers);

    if (group) {
      // Send group creation request to hub
      const createPayload: GroupPayload = {
        type: 'group-create',
        groupId: group.groupId,
        name,
        creatorUsername: this.username,
        initialMembers,
      };

      if (hubRelayId === this.nodeId) {
        // We are the hub - initialize GroupHub if needed and process locally
        this.initGroupHub();
        this.groupHub!.handlePayload(createPayload, this.nodeId);
      } else {
        // Connect directly to hub before sending to ensure delivery
        console.log(`[TomClient] Creating group on remote hub ${hubRelayId}, establishing connection first`);
        try {
          await this.transport?.connectToPeer(hubRelayId);
          console.log(`[TomClient] Connected to hub ${hubRelayId}, sending group-create`);

          // Send directly to hub (not through another relay)
          const envelope = this.router!.createEnvelope(hubRelayId, 'app', createPayload, []);
          const hubPeer = this.transport?.getPeer(hubRelayId);
          if (hubPeer) {
            hubPeer.send(envelope);
            console.log('[TomClient] group-create sent directly to hub');
          } else {
            // Fallback to relay routing
            console.log('[TomClient] Direct connection failed, falling back to relay');
            await this.sendPayload(hubRelayId, createPayload);
          }
        } catch (error) {
          console.log('[TomClient] Failed to connect to hub, using relay:', error);
          await this.sendPayload(hubRelayId, createPayload);
        }
      }
    }

    return group;
  }

  /**
   * Accept a group invitation.
   * Handles hub recovery if the original hub is no longer available.
   * When the hub is offline, creates a new group on an available relay.
   */
  /**
   * Accept a group invitation with active retry loop.
   * Returns a Promise that resolves with the GroupInfo when join is confirmed,
   * or rejects after max attempts.
   *
   * Emits progress events so the UI can show feedback.
   */
  async acceptGroupInvite(groupId: string): Promise<GroupInfo> {
    if (!this.groupManager) throw new Error('GroupManager not initialized');

    // Prevent duplicate join requests
    if (this.pendingGroupJoins.has(groupId)) {
      console.log(`[TomClient] Join already pending for group ${groupId}, ignoring`);
      throw new Error('Join already in progress');
    }

    const invites = this.groupManager.getPendingInvites();
    const invite = invites.find((i) => i.groupId === groupId);
    if (!invite) throw new Error('Invitation not found');

    if (!this.groupManager.acceptInvite(groupId)) throw new Error('Cannot accept invite');

    // Mark as pending
    this.pendingGroupJoins.add(groupId);

    const MAX_ATTEMPTS = 5;
    const RETRY_DELAY_MS = 2000;

    // Create a promise that will be resolved when group-sync is received
    const joinPromise = new Promise<GroupInfo>((resolve, reject) => {
      this.pendingJoinResolvers.set(groupId, { resolve, reject });
    });

    // Find the hub
    let targetHub = invite.hubRelayId;
    const hubPeer = this.topology.getPeer(targetHub);
    const hubStatus = this.topology.getPeerStatus(targetHub);
    const hubIsOnline = hubPeer && hubStatus === 'online';

    // If hub is offline, find an alternative
    if (!hubIsOnline && targetHub !== this.nodeId) {
      this.emitJoinProgress(groupId, 'connecting', 0, MAX_ATTEMPTS);
      console.log(`[TomClient] Hub ${targetHub} is offline, finding alternative`);

      const myRoles = this.getCurrentRoles();
      if (myRoles.includes('relay')) {
        targetHub = this.nodeId;
        this.groupManager.updateInviteHub(groupId, targetHub);
      } else {
        const onlineRelays = this.topology
          .getReachablePeers()
          .filter(
            (p) =>
              p.nodeId !== this.nodeId &&
              p.roles?.includes('relay') &&
              this.topology.getPeerStatus(p.nodeId) === 'online',
          )
          .sort((a, b) => b.lastSeen - a.lastSeen);

        if (onlineRelays.length > 0) {
          targetHub = onlineRelays[0].nodeId;
          this.groupManager.updateInviteHub(groupId, targetHub);
        } else {
          this.cleanupJoin(groupId, 'failed', 0, MAX_ATTEMPTS);
          throw new Error('No online relays available');
        }
      }
    }

    // Active retry loop
    const attemptJoin = async (attempt: number): Promise<void> => {
      if (attempt > MAX_ATTEMPTS) {
        this.cleanupJoin(groupId, 'failed', attempt, MAX_ATTEMPTS);
        const resolver = this.pendingJoinResolvers.get(groupId);
        resolver?.reject(new Error(`Failed to join group after ${MAX_ATTEMPTS} attempts`));
        this.pendingJoinResolvers.delete(groupId);
        return;
      }

      // Check if already joined (group-sync received)
      if (this.groupManager?.isInGroup(groupId)) {
        console.log(`[TomClient] Already in group ${groupId}, join complete`);
        return;
      }

      console.log(`[TomClient] Join attempt ${attempt}/${MAX_ATTEMPTS} for group ${groupId}`);
      this.emitJoinProgress(groupId, attempt === 1 ? 'requesting' : 'retrying', attempt, MAX_ATTEMPTS);

      const joinPayload: GroupPayload = {
        type: 'group-join',
        groupId,
        nodeId: this.nodeId,
        username: this.username,
      };

      try {
        if (targetHub === this.nodeId && this.groupHub) {
          this.groupHub.handlePayload(joinPayload, this.nodeId);
        } else {
          // Connect and send directly to hub
          await this.transport?.connectToPeer(targetHub);
          const peer = this.transport?.getPeer(targetHub);
          if (peer) {
            const envelope = this.router!.createEnvelope(targetHub, 'app', joinPayload, []);
            peer.send(envelope);
          } else {
            await this.sendPayload(targetHub, joinPayload);
          }
        }

        this.emitJoinProgress(groupId, 'waiting', attempt, MAX_ATTEMPTS);

        // Wait for response or timeout
        await new Promise<void>((resolve) => {
          const checkInterval = setInterval(() => {
            if (this.groupManager?.isInGroup(groupId)) {
              clearInterval(checkInterval);
              resolve();
            }
          }, 200);

          // Timeout after RETRY_DELAY_MS
          setTimeout(() => {
            clearInterval(checkInterval);
            resolve();
          }, RETRY_DELAY_MS);
        });

        // If still not in group, retry
        if (!this.groupManager?.isInGroup(groupId)) {
          await attemptJoin(attempt + 1);
        }
      } catch (error) {
        console.warn(`[TomClient] Join attempt ${attempt} failed:`, error);
        await new Promise((r) => setTimeout(r, RETRY_DELAY_MS));
        await attemptJoin(attempt + 1);
      }
    };

    // Start the retry loop (non-blocking)
    attemptJoin(1).catch((error) => {
      console.error('[TomClient] Join loop failed:', error);
    });

    return joinPromise;
  }

  /** Emit join progress event to handlers */
  private emitJoinProgress(
    groupId: string,
    status: 'connecting' | 'requesting' | 'waiting' | 'retrying' | 'success' | 'failed',
    attempt: number,
    maxAttempts: number,
  ): void {
    for (const handler of this.groupJoinProgressHandlers) {
      handler(groupId, status, attempt, maxAttempts);
    }
    this.emitStatus('group:join-progress', `${groupId}: ${status} (${attempt}/${maxAttempts})`);
  }

  /** Cleanup after join completes or fails */
  private cleanupJoin(groupId: string, status: 'success' | 'failed', attempt: number, maxAttempts: number): void {
    this.pendingGroupJoins.delete(groupId);
    this.emitJoinProgress(groupId, status, attempt, maxAttempts);
  }

  /**
   * Decline a group invitation.
   */
  declineGroupInvite(groupId: string): boolean {
    return this.groupManager?.declineInvite(groupId) ?? false;
  }

  /**
   * Invite a user to a group. Only admins can invite.
   */
  async inviteToGroup(groupId: string, inviteeNodeId: string, inviteeUsername: string): Promise<boolean> {
    if (!this.groupManager) return false;

    const group = this.groupManager.getGroup(groupId);
    if (!group) return false;

    // Check if current user is an admin
    if (!this.groupManager.isAdmin(groupId)) {
      console.log(`[TomClient] Cannot invite: not an admin of group ${groupId}`);
      return false;
    }

    // Check if invitee is already a member
    if (group.members.some((m: { nodeId: string }) => m.nodeId === inviteeNodeId)) {
      console.log(`[TomClient] User ${inviteeNodeId} is already a member of group ${groupId}`);
      return false;
    }

    const invitePayload: GroupPayload = {
      type: 'group-invite',
      groupId,
      groupName: group.name,
      inviteeId: inviteeNodeId,
      inviteeUsername,
      inviterId: this.nodeId,
      inviterUsername: this.username,
      hubRelayId: group.hubRelayId,
      memberCount: group.members.length,
    };

    console.log(`[TomClient] Sending group invite to ${inviteeNodeId} for group ${groupId}`);
    // Send invite directly to the user
    await this.sendPayload(inviteeNodeId, invitePayload);
    console.log('[TomClient] Group invite sent successfully');
    return true;
  }

  /**
   * Send a message to a group.
   */
  async sendGroupMessage(groupId: string, text: string): Promise<boolean> {
    if (!this.groupManager) return false;

    const group = this.groupManager.getGroup(groupId);
    if (!group) return false;

    const messageId =
      typeof crypto.randomUUID === 'function'
        ? crypto.randomUUID()
        : Array.from({ length: 32 }, () => Math.floor(Math.random() * 16).toString(16)).join('');

    const messagePayload: GroupMessagePayload = {
      type: 'group-message',
      groupId,
      messageId,
      senderId: this.nodeId,
      senderUsername: this.username,
      text,
      sentAt: Date.now(),
    };

    // Add message to our own history first (so we see it immediately)
    this.groupManager.handleMessage(messagePayload);

    // Send to hub for fanout to other members
    if (group.hubRelayId === this.nodeId && this.groupHub) {
      // We are the hub - process locally
      this.groupHub.handlePayload(messagePayload, this.nodeId);
    } else {
      await this.sendPayload(group.hubRelayId, messagePayload);
    }
    return true;
  }

  /**
   * Leave a group.
   */
  async leaveGroup(groupId: string): Promise<boolean> {
    if (!this.groupManager) return false;

    const group = this.groupManager.getGroup(groupId);
    if (!group) return false;

    const leavePayload: GroupPayload = {
      type: 'group-leave',
      groupId,
      nodeId: this.nodeId,
    };

    if (group.hubRelayId === this.nodeId && this.groupHub) {
      // We are the hub - process locally
      this.groupHub.handlePayload(leavePayload, this.nodeId);
    } else {
      await this.sendPayload(group.hubRelayId, leavePayload);
    }
    return this.groupManager.leaveGroup(groupId);
  }

  /**
   * Get all groups this node is a member of.
   */
  getGroups(): GroupInfo[] {
    return this.groupManager?.getAllGroups() ?? [];
  }

  /**
   * Get a specific group.
   */
  getGroup(groupId: string): GroupInfo | null {
    return this.groupManager?.getGroup(groupId) ?? null;
  }

  /**
   * Get pending group invitations.
   */
  getPendingGroupInvites(): Array<{ groupId: string; groupName: string; inviterId: string; inviterUsername: string }> {
    return this.groupManager?.getPendingInvites() ?? [];
  }

  /**
   * Get message history for a group.
   */
  getGroupMessages(groupId: string): GroupMessagePayload[] {
    return this.groupManager?.getMessageHistory(groupId) ?? [];
  }

  /**
   * Get available public groups (not yet joined).
   */
  getAvailableGroups(): PublicGroupInfo[] {
    return this.groupManager?.getAvailableGroups() ?? [];
  }

  /**
   * Join a public group without an invitation.
   * Uses active retry loop with progress events.
   * Returns a Promise that resolves with GroupInfo when join is confirmed.
   */
  async joinPublicGroup(groupId: string): Promise<GroupInfo> {
    if (!this.groupManager) throw new Error('GroupManager not initialized');

    // Prevent duplicate join requests
    if (this.pendingGroupJoins.has(groupId)) {
      console.log(`[TomClient] Join already pending for group ${groupId}, ignoring`);
      throw new Error('Join already in progress');
    }

    const availableGroups = this.groupManager.getAvailableGroups();
    const publicGroup = availableGroups.find((g) => g.groupId === groupId);
    if (!publicGroup) throw new Error('Public group not found');

    const targetHub = publicGroup.hubRelayId;
    const MAX_ATTEMPTS = 5;
    const RETRY_DELAY_MS = 2000;

    // Mark as pending
    this.pendingGroupJoins.add(groupId);

    // Create a promise that will be resolved when group-sync is received
    const joinPromise = new Promise<GroupInfo>((resolve, reject) => {
      this.pendingJoinResolvers.set(groupId, { resolve, reject });
    });

    // Active retry loop
    const attemptJoin = async (attempt: number): Promise<void> => {
      if (attempt > MAX_ATTEMPTS) {
        this.cleanupJoin(groupId, 'failed', attempt, MAX_ATTEMPTS);
        const resolver = this.pendingJoinResolvers.get(groupId);
        resolver?.reject(new Error(`Failed to join group after ${MAX_ATTEMPTS} attempts`));
        this.pendingJoinResolvers.delete(groupId);
        return;
      }

      // Check if already joined
      if (this.groupManager?.isInGroup(groupId)) {
        console.log(`[TomClient] Already in group ${groupId}, join complete`);
        return;
      }

      console.log(`[TomClient] Join attempt ${attempt}/${MAX_ATTEMPTS} for public group ${groupId}`);
      this.emitJoinProgress(groupId, attempt === 1 ? 'requesting' : 'retrying', attempt, MAX_ATTEMPTS);

      const joinPayload: GroupPayload = {
        type: 'group-join',
        groupId,
        nodeId: this.nodeId,
        username: this.username,
      };

      try {
        // Connect and send directly to hub
        await this.transport?.connectToPeer(targetHub);
        const peer = this.transport?.getPeer(targetHub);
        if (peer) {
          const envelope = this.router!.createEnvelope(targetHub, 'app', joinPayload, []);
          peer.send(envelope);
        } else {
          await this.sendPayload(targetHub, joinPayload);
        }

        this.emitJoinProgress(groupId, 'waiting', attempt, MAX_ATTEMPTS);

        // Wait for response or timeout
        await new Promise<void>((resolve) => {
          const checkInterval = setInterval(() => {
            if (this.groupManager?.isInGroup(groupId)) {
              clearInterval(checkInterval);
              resolve();
            }
          }, 200);

          setTimeout(() => {
            clearInterval(checkInterval);
            resolve();
          }, RETRY_DELAY_MS);
        });

        // If still not in group, retry
        if (!this.groupManager?.isInGroup(groupId)) {
          await attemptJoin(attempt + 1);
        }
      } catch (error) {
        console.warn(`[TomClient] Join attempt ${attempt} failed:`, error);
        await new Promise((r) => setTimeout(r, RETRY_DELAY_MS));
        await attemptJoin(attempt + 1);
      }
    };

    // Start the retry loop (non-blocking)
    attemptJoin(1).catch((error) => {
      console.error('[TomClient] Join loop failed:', error);
    });

    return joinPromise;
  }

  // Group event handlers
  onGroupCreated(handler: GroupCreatedHandler): void {
    this.groupCreatedHandlers.push(handler);
  }

  onGroupInvite(handler: GroupInviteHandler): void {
    this.groupInviteHandlers.push(handler);
  }

  onGroupMemberJoined(handler: GroupMemberJoinedHandler): void {
    this.groupMemberJoinedHandlers.push(handler);
  }

  onGroupMemberLeft(handler: GroupMemberLeftHandler): void {
    this.groupMemberLeftHandlers.push(handler);
  }

  onGroupMessage(handler: GroupMessageHandler): void {
    this.groupMessageHandlers.push(handler);
  }

  onPublicGroupAnnounced(handler: PublicGroupAnnouncedHandler): void {
    this.publicGroupAnnouncedHandlers.push(handler);
  }

  /**
   * Register a handler for group join progress updates.
   * Allows UI to show feedback during the active retry loop.
   */
  onGroupJoinProgress(handler: GroupJoinProgressHandler): void {
    this.groupJoinProgressHandlers.push(handler);
  }

  /**
   * Initialize GroupHub when this node becomes a relay.
   * @internal
   */
  private initGroupHub(): void {
    if (this.groupHub) return;

    const hubEvents: GroupHubEvents = {
      sendToNode: async (nodeId, payload, _groupId) => {
        // Handle self-send case: process locally instead of network send
        if (nodeId === this.nodeId) {
          console.log(`[TomClient] Hub sendToNode to self, processing locally: ${payload.type}`);
          this.handleGroupPayload(payload, this.nodeId);
          return;
        }
        // Hub sends DIRECTLY to the node without relay selection
        // This is critical - the hub IS the relay, so it shouldn't route through another relay
        console.log(`[TomClient] Hub sendToNode ${payload.type} directly to ${nodeId}`);
        try {
          await this.transport?.connectToPeer(nodeId);
          const envelope = this.router!.createEnvelope(nodeId, 'app', payload, []);
          const peer = this.transport?.getPeer(nodeId);
          if (peer) {
            peer.send(envelope);
          } else {
            console.warn(`[TomClient] Hub sendToNode: no peer connection to ${nodeId}`);
          }
        } catch (error) {
          console.warn(`[TomClient] Hub sendToNode failed to ${nodeId}:`, error);
        }
      },
      broadcastToGroup: async (groupId, payload, excludeNodeId) => {
        const group = this.groupHub?.getGroup(groupId);
        if (!group) return;

        // Get list of members to send to (excluding self and excludeNodeId)
        const targetMembers = group.members.filter((m) => m.nodeId !== this.nodeId && m.nodeId !== excludeNodeId);

        // WARM-UP: Connect to ALL members in parallel first
        // This eliminates the "cold start" delay on first messages
        await Promise.all(
          targetMembers.map((member) =>
            this.transport?.connectToPeer(member.nodeId).catch(() => {
              // Connection failures are handled below when sending
            }),
          ),
        );

        // Process locally for self if we're a member
        if (group.members.some((m) => m.nodeId === this.nodeId)) {
          console.log(`[TomClient] Hub broadcastToGroup to self, processing locally: ${payload.type}`);
          this.handleGroupPayload(payload, this.nodeId);
        }

        // Now send to all target members (connections are already warm)
        for (const member of targetMembers) {
          console.log(`[TomClient] Hub broadcasting ${payload.type} directly to ${member.nodeId}`);
          try {
            const envelope = this.router!.createEnvelope(member.nodeId, 'app', payload, []);
            const peer = this.transport?.getPeer(member.nodeId);
            if (peer) {
              peer.send(envelope);
            } else {
              console.warn(`[TomClient] Hub broadcast: no peer connection to ${member.nodeId}`);
            }
          } catch (error) {
            console.warn(`[TomClient] Hub broadcast failed to ${member.nodeId}:`, error);
          }
        }
      },
      broadcastAnnouncement: async (payload) => {
        // Broadcast group announcement to ALL known peers on the network
        const peers = this.topology.getReachablePeers();
        for (const peer of peers) {
          if (peer.nodeId !== this.nodeId) {
            await this.sendPayload(peer.nodeId, payload);
          }
        }
        this.emitStatus('group:announcement-broadcast', `to ${peers.length} peers`);
      },
      onHubActivity: (groupId, activity, details) => {
        this.emitStatus(`hub:${activity}`, `${groupId}: ${JSON.stringify(details)}`);
      },
    };

    this.groupHub = new GroupHub(this.nodeId, hubEvents);
    this.emitStatus('group-hub:initialized');
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

    // Handle group payloads (Story 4.6)
    // IMPORTANT: Only process if this message is for us, not if we're just relaying
    if (envelope.type === 'app' && isGroupPayload(envelope.payload)) {
      if (envelope.to === this.nodeId) {
        console.log(
          `[TomClient] Received group payload: ${(envelope.payload as { type: string }).type}`,
          envelope.payload,
        );
        this.handleGroupPayload(envelope.payload, envelope.from);
        return;
      }
      // Message is not for us - let router handle relay
    }

    this.router?.handleIncoming(envelope);
  }

  /**
   * Handle rerouting when a relay fails (Story 5.2).
   * Attempts to find an alternate path avoiding the failed relay.
   * If no alternate path exists, queues the message for backup delivery.
   *
   * Uses a mutex (reroutingInProgress) to prevent parallel reroutes for the same message,
   * which could cause race conditions and double sends.
   */
  private handleRerouteNeeded(envelope: MessageEnvelope, failedRelayId: string): void {
    if (!this.router || !this.relaySelector || !this.transport) return;

    const messageId = envelope.id;

    // Mutex: prevent parallel reroutes for the same message
    if (this.reroutingInProgress.has(messageId)) {
      this.emitStatus('reroute:skipped', `${messageId}: reroute already in progress`);
      return;
    }

    // Track failed relays for this message
    let failedRelays = this.failedRelaysPerMessage.get(messageId);
    if (!failedRelays) {
      failedRelays = new Set<string>();
      this.failedRelaysPerMessage.set(messageId, failedRelays);
    }
    failedRelays.add(failedRelayId);

    // Check if we've exceeded max reroute attempts
    if (failedRelays.size >= TomClient.MAX_REROUTE_ATTEMPTS) {
      this.emitStatus('reroute:max-attempts', `${messageId}: all relays failed`);
      this.router.emitMessageQueued(envelope, 'max reroute attempts exceeded');
      this.failedRelaysPerMessage.delete(messageId);
      return;
    }

    this.emitStatus('reroute:attempting', `${messageId}: relay ${failedRelayId} failed`);

    // Try to find an alternate relay
    const selection = this.relaySelector.selectAlternateRelay(envelope.to, this.topology, failedRelays);

    if (selection.relayId) {
      this.emitStatus('reroute:alternate-found', `${messageId}: using ${selection.relayId}`);

      // Clone envelope to avoid mutation issues with retries
      const reroutableEnvelope: MessageEnvelope = {
        ...envelope,
        via: [selection.relayId], // New relay path
      };

      // Lock: mark as rerouting to prevent parallel attempts
      this.reroutingInProgress.add(messageId);

      // Attempt to send via the alternate relay
      this.transport
        .connectToPeer(selection.relayId)
        .then(() => {
          const success = this.router!.sendViaRelay(reroutableEnvelope, selection.relayId!);
          if (success) {
            this.emitStatus('reroute:success', `${messageId}: sent via ${selection.relayId}`);
            // Clean up tracking on success
            this.failedRelaysPerMessage.delete(messageId);
          }
          // If sendViaRelay fails, it will call handleRerouteNeeded again
        })
        .catch(() => {
          // Connection failed, will trigger another reroute attempt
          // Release lock before recursive call
          this.reroutingInProgress.delete(messageId);
          this.handleRerouteNeeded(reroutableEnvelope, selection.relayId!);
        })
        .finally(() => {
          // Release lock after operation completes
          this.reroutingInProgress.delete(messageId);
        });
    } else {
      // No alternate path available - queue for backup delivery
      this.emitStatus('reroute:no-alternate', `${messageId}: no alternate relays available`);
      this.router.emitMessageQueued(envelope, 'no alternate relays available');
      this.failedRelaysPerMessage.delete(messageId);
    }
  }

  /**
   * Handle incoming group payloads.
   * Routes to GroupManager (member) or GroupHub (relay).
   */
  private handleGroupPayload(payload: GroupPayload, fromNodeId: string): void {
    console.log(
      `[TomClient] handleGroupPayload: type=${payload.type}, from=${fromNodeId}, hasGroupHub=${!!this.groupHub}`,
    );

    // Check if we should be the hub for this group (lazy init)
    // This handles the case where we created a group but groupHub wasn't initialized
    if (!this.groupHub && this.groupManager) {
      const group = this.groupManager.getGroup(payload.groupId);
      if (group && group.hubRelayId === this.nodeId) {
        console.log(`[TomClient] Lazy-initializing GroupHub for group ${payload.groupId}`);
        this.initGroupHub();
      }
    }

    // If we're a relay hub, handle as hub
    if (this.groupHub) {
      console.log('[TomClient] Forwarding to GroupHub');
      this.groupHub.handlePayload(payload, fromNodeId);
    } else {
      console.log('[TomClient] No GroupHub, processing as member only');
    }

    // Also handle as member (for messages/events directed to us)
    if (this.groupManager) {
      switch (payload.type) {
        case 'group-created':
          if ('groupInfo' in payload) {
            this.groupManager.handleGroupCreated(payload.groupInfo);
            // Note: No broadcast - groups are only visible via direct invitations
          }
          break;
        case 'group-invite':
          console.log(
            `[TomClient] group-invite received: inviteeId=${(payload as { inviteeId?: string }).inviteeId}, myNodeId=${this.nodeId}`,
          );
          if ('inviteeId' in payload && payload.inviteeId === this.nodeId && 'hubRelayId' in payload) {
            console.log(`[TomClient] Calling handleInvite for group ${payload.groupId}`);
            this.groupManager.handleInvite(
              payload.groupId,
              payload.groupName,
              payload.inviterId,
              payload.inviterUsername,
              payload.hubRelayId,
            );
          } else {
            console.log('[TomClient] Invite not for me or missing hubRelayId, ignoring');
          }
          break;
        case 'group-sync':
          if ('groupInfo' in payload) {
            this.groupManager.handleGroupSync(payload.groupInfo, payload.recentMessages);
            // Remove from available groups now that we've joined
            this.groupManager.removeFromAvailable(payload.groupId);
            // Clear pending join state - join is complete
            this.pendingGroupJoins.delete(payload.groupId);
            // Resolve the pending join promise - this completes the active retry loop
            const resolver = this.pendingJoinResolvers.get(payload.groupId);
            if (resolver) {
              this.emitJoinProgress(payload.groupId, 'success', 1, 5);
              resolver.resolve(payload.groupInfo);
              this.pendingJoinResolvers.delete(payload.groupId);
            }
          }
          break;
        case 'group-member-joined':
          if ('member' in payload) {
            this.groupManager.handleMemberJoined(payload.groupId, payload.member);
          }
          break;
        case 'group-member-left':
          if ('nodeId' in payload && 'username' in payload) {
            this.groupManager.handleMemberLeft(
              payload.groupId,
              payload.nodeId,
              payload.username,
              payload.reason ?? 'voluntary',
            );
          }
          break;
        case 'group-message':
          if ('messageId' in payload) {
            this.groupManager.handleMessage(payload as GroupMessagePayload);
          }
          break;
        case 'group-hub-migration':
          if ('newHubId' in payload && 'oldHubId' in payload) {
            this.groupManager.handleHubMigration(payload.groupId, payload.newHubId, payload.oldHubId);
          }
          break;
        case 'group-hub-heartbeat':
          if (isGroupHubHeartbeat(payload)) {
            this.groupManager.handleHubHeartbeat(payload.groupId, payload.memberCount, payload.timestamp);
          }
          break;
        // Note: group-announcement disabled - groups only via direct invitations
      }
    }
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
