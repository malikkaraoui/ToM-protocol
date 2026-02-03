/**
 * DIRECT PATH MANAGER
 *
 * Manages direct WebRTC connections between peers after initial relay introduction.
 * This is an optimization layer on top of relay routing (ADR-001 compliance).
 *
 * Flow:
 * 1. Messages initially route through relay (required by ADR-001)
 * 2. After successful relay exchange, attempt direct WebRTC connection
 * 3. If direct connection succeeds, prefer it for subsequent messages
 * 4. If direct connection fails/drops, fall back to relay transparently
 *
 * @module transport
 */

import type { NodeId } from '../identity/index.js';
import type { MessageEnvelope } from '../types/envelope.js';
import type { TransportLayer } from './transport-layer.js';

export type ConnectionType = 'direct' | 'relay' | 'disconnected';

export interface DirectPathEvents {
  /** Emitted when a direct path is established to a peer */
  onDirectPathEstablished: (peerId: NodeId) => void;
  /** Emitted when a direct path is lost (peer disconnected) */
  onDirectPathLost: (peerId: NodeId) => void;
  /** Emitted when a direct path is restored after being lost */
  onDirectPathRestored: (peerId: NodeId) => void;
}

interface ConversationState {
  /** When the conversation started */
  startedAt: number;
  /** Last message timestamp */
  lastMessageAt: number;
  /** Whether direct path is currently active */
  directPathActive: boolean;
  /** Whether we previously had a direct path (for restore detection) */
  hadDirectPath: boolean;
  /** Number of reconnection attempts */
  reconnectAttempts: number;
}

export class DirectPathManager {
  private localNodeId: NodeId;
  private transport: TransportLayer;
  private events: DirectPathEvents;

  /** Track conversations with peers (messages exchanged via relay) */
  private conversations = new Map<NodeId, ConversationState>();

  /** Pending connection attempts to prevent race conditions */
  private pendingConnections = new Map<NodeId, Promise<void>>();

  constructor(localNodeId: NodeId, transport: TransportLayer, events: DirectPathEvents) {
    this.localNodeId = localNodeId;
    this.transport = transport;
    this.events = events;
  }

  /**
   * Track a conversation after message exchange.
   * Called when a message is sent or received via relay.
   */
  trackConversation(envelope: MessageEnvelope): void {
    // Determine the peer (other party in conversation)
    const peerId = envelope.from === this.localNodeId ? envelope.to : envelope.from;

    // Don't track conversation with self
    if (peerId === this.localNodeId) {
      return;
    }

    const existing = this.conversations.get(peerId);
    if (existing) {
      existing.lastMessageAt = Date.now();
    } else {
      this.conversations.set(peerId, {
        startedAt: Date.now(),
        lastMessageAt: Date.now(),
        directPathActive: false,
        hadDirectPath: false,
        reconnectAttempts: 0,
      });
    }
  }

  /**
   * Check if we have a conversation history with a peer.
   */
  hasConversation(peerId: NodeId): boolean {
    return this.conversations.has(peerId);
  }

  /**
   * Attempt to establish a direct WebRTC connection to a peer.
   * Only attempts if:
   * - We have a conversation history with the peer
   * - We don't already have an active direct path
   */
  async attemptDirectPath(peerId: NodeId): Promise<boolean> {
    const conversation = this.conversations.get(peerId);
    if (!conversation) {
      return false;
    }

    // Already have direct path
    if (conversation.directPathActive) {
      return true;
    }

    // Check if already attempting connection
    const pending = this.pendingConnections.get(peerId);
    if (pending) {
      await pending;
      return this.conversations.get(peerId)?.directPathActive ?? false;
    }

    // Attempt connection
    const connectionPromise = this.doAttemptDirectPath(peerId, conversation);
    this.pendingConnections.set(peerId, connectionPromise);

    try {
      await connectionPromise;
      return this.conversations.get(peerId)?.directPathActive ?? false;
    } finally {
      this.pendingConnections.delete(peerId);
    }
  }

  private async doAttemptDirectPath(peerId: NodeId, conversation: ConversationState): Promise<void> {
    try {
      // Check if already connected via transport
      const existingPeer = this.transport.getPeer(peerId);
      if (existingPeer && conversation.directPathActive) {
        return;
      }

      // Attempt WebRTC connection
      await this.transport.connectToPeer(peerId);

      // Mark direct path as active
      const wasRestore = conversation.hadDirectPath;
      conversation.directPathActive = true;
      conversation.hadDirectPath = true;
      conversation.reconnectAttempts = 0;

      // Emit appropriate event
      if (wasRestore) {
        this.events.onDirectPathRestored(peerId);
      } else {
        this.events.onDirectPathEstablished(peerId);
      }
    } catch {
      conversation.reconnectAttempts++;
      // Connection failed - will continue using relay
    }
  }

  /**
   * Mark a peer's direct path as active.
   * Called when direct connection is confirmed.
   */
  markDirectPathActive(peerId: NodeId): void {
    const conversation = this.conversations.get(peerId);
    if (conversation) {
      conversation.directPathActive = true;
      conversation.hadDirectPath = true;
    }
  }

  /**
   * Handle direct path loss (peer disconnected).
   * Falls back to relay routing.
   */
  handleDirectPathLost(peerId: NodeId): void {
    const conversation = this.conversations.get(peerId);
    if (conversation?.directPathActive) {
      conversation.directPathActive = false;
      this.events.onDirectPathLost(peerId);
    }
  }

  /**
   * Get the current connection type for a peer.
   */
  getConnectionType(peerId: NodeId): ConnectionType {
    const conversation = this.conversations.get(peerId);
    if (!conversation) {
      return 'disconnected';
    }
    return conversation.directPathActive ? 'direct' : 'relay';
  }

  /**
   * Get list of peers with active direct paths.
   */
  getDirectPeers(): NodeId[] {
    const result: NodeId[] = [];
    for (const [peerId, state] of this.conversations) {
      if (state.directPathActive) {
        result.push(peerId);
      }
    }
    return result;
  }

  /**
   * Get list of all peers we have conversations with.
   */
  getConversationPeers(): NodeId[] {
    return Array.from(this.conversations.keys());
  }

  /**
   * Check if a peer has had a direct path before (for reconnection logic).
   */
  hadPreviousDirectPath(peerId: NodeId): boolean {
    return this.conversations.get(peerId)?.hadDirectPath ?? false;
  }

  /**
   * Get reconnection attempt count for exponential backoff.
   */
  getReconnectAttempts(peerId: NodeId): number {
    return this.conversations.get(peerId)?.reconnectAttempts ?? 0;
  }

  /**
   * Clear conversation state for a peer.
   */
  clearConversation(peerId: NodeId): void {
    this.conversations.delete(peerId);
  }

  /**
   * Called when a peer is detected online (via heartbeat/discovery).
   * Attempts to re-establish direct path if we previously had one.
   * Uses exponential backoff to avoid hammering failed connections.
   */
  async onPeerOnline(peerId: NodeId): Promise<void> {
    const conversation = this.conversations.get(peerId);
    if (!conversation) {
      return;
    }

    // Only attempt reconnect if we previously had a direct path
    if (!conversation.hadDirectPath) {
      return;
    }

    // Skip if already have active direct path
    if (conversation.directPathActive) {
      return;
    }

    // If we've had too many recent attempts, check cooldown FIRST
    const attempts = conversation.reconnectAttempts;
    if (attempts >= 3) {
      // Reset attempts after a cooldown period (30 seconds)
      const lastAttemptAge = Date.now() - conversation.lastMessageAt;
      if (lastAttemptAge < 30000) {
        return; // Still in cooldown, skip entirely
      }
      conversation.reconnectAttempts = 0;
    }

    // Calculate backoff delay based on attempts (after potential reset)
    const currentAttempts = conversation.reconnectAttempts;
    const baseDelay = 1000; // 1 second
    const maxDelay = 4000; // 4 seconds max
    const delay = Math.min(baseDelay * 2 ** currentAttempts, maxDelay);

    // Wait for backoff delay
    await new Promise((resolve) => setTimeout(resolve, delay));

    // Attempt reconnection
    await this.attemptDirectPath(peerId);
  }

  /**
   * Batch reconnect for multiple peers coming online.
   * Useful when network recovers after outage.
   */
  async onMultiplePeersOnline(peerIds: NodeId[]): Promise<void> {
    // Reconnect in parallel with staggered starts to avoid overwhelming signaling
    const promises = peerIds.map((peerId, index) => {
      return new Promise<void>((resolve) => {
        setTimeout(async () => {
          await this.onPeerOnline(peerId);
          resolve();
        }, index * 100); // Stagger by 100ms
      });
    });
    await Promise.all(promises);
  }
}
