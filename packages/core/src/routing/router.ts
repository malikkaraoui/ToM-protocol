import type { DirectPathManager } from '../transport/direct-path-manager.js';
import type { PeerConnection, TransportLayer } from '../transport/transport-layer.js';
import type { MessageEnvelope } from '../types/envelope.js';
import { MAX_RELAY_DEPTH } from './relay-selector.js';

export const ACK_TYPE = 'ack';
export const READ_RECEIPT_TYPE = 'read-receipt';

/** ACK types for distinguishing relay vs recipient acknowledgments */
export type AckType = 'relay-forwarded' | 'recipient-received' | 'recipient-read';

/** Valid ACK types for runtime validation */
const VALID_ACK_TYPES: AckType[] = ['relay-forwarded', 'recipient-received', 'recipient-read'];

/** Max age for ACK anti-replay cache (5 minutes) */
const ACK_REPLAY_CACHE_TTL_MS = 5 * 60 * 1000;

/** Max entries in ACK anti-replay cache */
const ACK_REPLAY_CACHE_MAX_SIZE = 5000;

export interface AckPayload {
  originalMessageId: string;
  ackType: AckType;
}

export interface RouterEvents {
  onMessageDelivered: (envelope: MessageEnvelope) => void;
  onMessageForwarded: (envelope: MessageEnvelope, nextHop: string) => void;
  onMessageRejected: (envelope: MessageEnvelope, reason: string) => void;
  /** Called when any ACK is received (legacy compatibility) */
  onAckReceived: (originalMessageId: string, from: string) => void;
  onAckFailed: (originalMessageId: string, reason: string) => void;
  /** Called when message is sent via direct path (bypassing relay) */
  onMessageSentDirect?: (envelope: MessageEnvelope, to: string) => void;
  /** Called when relay confirms forwarding (ackType: 'relay-forwarded') */
  onRelayAckReceived?: (originalMessageId: string, relayId: string) => void;
  /** Called when recipient confirms delivery (ackType: 'recipient-received') */
  onDeliveryAckReceived?: (originalMessageId: string, recipientId: string) => void;
  /** Called when read receipt is received */
  onReadReceiptReceived?: (originalMessageId: string, readAt: number, from: string) => void;
}

export interface SignatureVerifier {
  verify(publicKey: Uint8Array, data: Uint8Array, signature: Uint8Array): boolean;
}

export class Router {
  private localNodeId: string;
  private transport: TransportLayer;
  private events: RouterEvents;
  private verifier: SignatureVerifier | null;
  /** Pending connection promises to prevent race conditions */
  private pendingConnections = new Map<string, Promise<PeerConnection>>();
  /** Optional DirectPathManager for direct path routing */
  private directPathManager: DirectPathManager | null = null;
  /** Anti-replay cache for ACKs: messageId -> timestamp when first seen */
  private seenAcks = new Map<string, number>();
  /** Anti-replay cache for read receipts: messageId -> timestamp when first seen */
  private seenReadReceipts = new Map<string, number>();

  constructor(localNodeId: string, transport: TransportLayer, events: RouterEvents, verifier?: SignatureVerifier) {
    this.localNodeId = localNodeId;
    this.transport = transport;
    this.events = events;
    this.verifier = verifier ?? null;
  }

  /**
   * Check if an ACK/receipt has been seen before (anti-replay).
   * Also cleans up old entries to prevent memory leaks.
   */
  private checkAndRecordSeen(cache: Map<string, number>, key: string): boolean {
    const now = Date.now();

    // Clean up old entries periodically
    if (cache.size > ACK_REPLAY_CACHE_MAX_SIZE / 2) {
      for (const [k, timestamp] of cache) {
        if (now - timestamp > ACK_REPLAY_CACHE_TTL_MS) {
          cache.delete(k);
        }
      }
    }

    // Evict oldest if still over limit
    if (cache.size >= ACK_REPLAY_CACHE_MAX_SIZE) {
      const firstKey = cache.keys().next().value;
      if (firstKey) cache.delete(firstKey);
    }

    // Check if already seen
    if (cache.has(key)) {
      return true; // Replay detected
    }

    // Record as seen
    cache.set(key, now);
    return false;
  }

  /**
   * Set the DirectPathManager for direct path routing support.
   * When set, the router will prefer direct paths over relay when available.
   */
  setDirectPathManager(manager: DirectPathManager): void {
    this.directPathManager = manager;
  }

  handleIncoming(envelope: MessageEnvelope): void {
    // If this message is addressed to us, deliver it
    if (envelope.to === this.localNodeId) {
      // Handle ACK messages
      if (envelope.type === ACK_TYPE) {
        const payload = envelope.payload as AckPayload | { originalMessageId?: string };
        const originalId = payload?.originalMessageId;
        if (originalId) {
          // Determine ACK type (default to 'recipient-received' for backward compatibility)
          const rawAckType = 'ackType' in payload ? payload.ackType : 'recipient-received';

          // Validate ACK type (security: prevents bypass of state machine)
          if (!VALID_ACK_TYPES.includes(rawAckType as AckType)) {
            return; // Invalid ACK type, ignore
          }
          const ackType = rawAckType as AckType;

          // Anti-replay check: composite key of messageId + sender + ackType
          const replayKey = `${originalId}:${envelope.from}:${ackType}`;
          if (this.checkAndRecordSeen(this.seenAcks, replayKey)) {
            return; // Replay detected, ignore
          }

          // Always emit the generic ACK event for backward compatibility
          this.events.onAckReceived(originalId, envelope.from);

          // Emit specific ACK type events
          if (ackType === 'relay-forwarded') {
            this.events.onRelayAckReceived?.(originalId, envelope.from);
          } else if (ackType === 'recipient-received') {
            this.events.onDeliveryAckReceived?.(originalId, envelope.from);
          }
        }
        return;
      }

      // Handle read receipt messages
      if (envelope.type === READ_RECEIPT_TYPE) {
        const payload = envelope.payload as { originalMessageId?: string; readAt?: number };
        if (payload?.originalMessageId) {
          // Anti-replay check for read receipts
          const replayKey = `${payload.originalMessageId}:${envelope.from}`;
          if (this.checkAndRecordSeen(this.seenReadReceipts, replayKey)) {
            return; // Replay detected, ignore
          }

          // Validate readAt timestamp (security: prevent temporal manipulation)
          const now = Date.now();
          let safeReadAt = payload.readAt ?? now;
          // Clamp to reasonable range: not in the future, not more than 7 days in the past
          const maxPastMs = 7 * 24 * 60 * 60 * 1000;
          safeReadAt = Math.min(safeReadAt, now); // Not in the future
          safeReadAt = Math.max(safeReadAt, now - maxPastMs); // Not too far in the past

          this.events.onReadReceiptReceived?.(payload.originalMessageId, safeReadAt, envelope.from);
        }
        return;
      }

      this.events.onMessageDelivered(envelope);

      // Auto-send ACK back to sender via the same relay path
      this.sendAck(envelope, 'recipient-received');
      return;
    }

    // Check if we're an intermediate relay in a multi-relay chain
    const myPositionInChain = this.findPositionInRelayChain(envelope);
    if (myPositionInChain !== -1) {
      // We're an intermediate relay, forward to next hop
      this.forwardInChain(envelope, myPositionInChain);
      return;
    }

    // Otherwise, forward directly to the intended recipient
    this.forwardToRecipient(envelope);
  }

  /**
   * Find our position in a multi-relay chain.
   * Returns -1 if we're not in the chain, otherwise returns our index.
   */
  private findPositionInRelayChain(envelope: MessageEnvelope): number {
    if (!envelope.via || envelope.via.length === 0) {
      return -1;
    }
    return envelope.via.indexOf(this.localNodeId);
  }

  /**
   * Forward a message to the next hop in a multi-relay chain.
   */
  private async forwardInChain(envelope: MessageEnvelope, currentPosition: number): Promise<void> {
    const viaPath = envelope.via;

    // Security: reject if chain is too deep
    if (viaPath.length > MAX_RELAY_DEPTH) {
      this.events.onMessageRejected(envelope, 'RELAY_CHAIN_TOO_DEEP');
      return;
    }

    // Determine next hop: if we're the last relay, forward to recipient
    // Otherwise, forward to the next relay in the chain
    const isLastRelay = currentPosition === viaPath.length - 1;
    const nextHop = isLastRelay ? envelope.to : viaPath[currentPosition + 1];

    if (!nextHop) {
      this.events.onMessageRejected(envelope, 'INVALID_RELAY_CHAIN');
      return;
    }

    // Add hop timestamp for latency tracking
    if (!envelope.hopTimestamps) {
      envelope.hopTimestamps = [];
    }
    envelope.hopTimestamps.push(Date.now());

    // Try to get existing peer connection
    let peer = this.transport.getPeer(nextHop);

    // If no connection exists, try to establish one
    if (!peer) {
      try {
        let connectionPromise = this.pendingConnections.get(nextHop);
        if (!connectionPromise) {
          connectionPromise = this.transport.connectToPeer(nextHop);
          this.pendingConnections.set(nextHop, connectionPromise);
          try {
            peer = await connectionPromise;
          } finally {
            this.pendingConnections.delete(nextHop);
          }
        } else {
          await connectionPromise;
          peer = this.transport.getPeer(nextHop);
        }
      } catch {
        this.events.onMessageRejected(envelope, 'NEXT_HOP_UNREACHABLE');
        return;
      }
    }

    if (!peer) {
      this.events.onMessageRejected(envelope, 'NEXT_HOP_UNREACHABLE');
      return;
    }

    peer.send(envelope);
    this.events.onMessageForwarded(envelope, nextHop);

    // Send relay ACK back to sender
    this.sendRelayAck(envelope);
  }

  private async forwardToRecipient(envelope: MessageEnvelope): Promise<void> {
    const nextHop = envelope.to;

    // Try to get existing peer connection
    let peer = this.transport.getPeer(nextHop);

    // If no direct connection exists, establish one via signaling
    if (!peer) {
      try {
        // Use pending connection promise to prevent race conditions
        let connectionPromise = this.pendingConnections.get(nextHop);
        if (!connectionPromise) {
          connectionPromise = this.transport.connectToPeer(nextHop);
          this.pendingConnections.set(nextHop, connectionPromise);
          try {
            peer = await connectionPromise;
          } finally {
            this.pendingConnections.delete(nextHop);
          }
        } else {
          // Another connection is in progress - wait for it
          await connectionPromise;
          peer = this.transport.getPeer(nextHop);
        }
      } catch {
        this.events.onMessageRejected(envelope, 'PEER_UNREACHABLE');
        return;
      }
    }

    if (!peer) {
      this.events.onMessageRejected(envelope, 'PEER_UNREACHABLE');
      return;
    }

    peer.send(envelope);
    this.events.onMessageForwarded(envelope, nextHop);

    // Send relay ACK back to sender to confirm forwarding
    this.sendRelayAck(envelope);
  }

  /**
   * Send a relay ACK back to the sender confirming the message was forwarded.
   */
  private async sendRelayAck(original: MessageEnvelope): Promise<void> {
    const payload: AckPayload = {
      originalMessageId: original.id,
      ackType: 'relay-forwarded',
    };
    const ack = this.createEnvelope(original.from, ACK_TYPE, payload, []);

    // Ensure we have a connection to the sender before sending ACK
    let directPeer = this.transport.getPeer(original.from);
    if (!directPeer) {
      try {
        directPeer = await this.transport.connectToPeer(original.from);
      } catch {
        // Connection failed, ACK is lost (best effort)
        return;
      }
    }

    directPeer.send(ack);
  }

  createEnvelope(
    to: string,
    type: string,
    payload: unknown,
    via: string[],
    sign?: (data: Uint8Array) => Uint8Array,
  ): MessageEnvelope {
    const envelope: MessageEnvelope = {
      id:
        typeof crypto.randomUUID === 'function'
          ? crypto.randomUUID()
          : Array.from(crypto.getRandomValues(new Uint8Array(16)))
              .map((b) => b.toString(16).padStart(2, '0'))
              .join(''),
      from: this.localNodeId,
      to,
      via,
      type,
      payload,
      timestamp: Date.now(),
      signature: '',
    };

    if (sign) {
      const data = new TextEncoder().encode(JSON.stringify({ ...envelope, signature: '' }));
      const sig = sign(data);
      envelope.signature = Array.from(sig)
        .map((b) => b.toString(16).padStart(2, '0'))
        .join('');
    }

    return envelope;
  }

  sendViaRelay(envelope: MessageEnvelope, relayId: string): void {
    const relayPeer = this.transport.getPeer(relayId);
    if (!relayPeer) {
      this.events.onMessageRejected(envelope, 'RELAY_UNREACHABLE');
      return;
    }

    // Add relay to via path if not already present
    if (!envelope.via.includes(relayId)) {
      envelope.via.push(relayId);
    }

    // Mark as relay route
    envelope.routeType = 'relay';
    relayPeer.send(envelope);
  }

  /**
   * Send a message with direct path preference.
   * Tries direct WebRTC connection first, falls back to relay if unavailable.
   *
   * @param envelope - The message to send
   * @param relayId - Fallback relay ID if direct path unavailable
   * @returns true if sent via direct path, false if sent via relay
   */
  sendWithDirectPreference(envelope: MessageEnvelope, relayId: string): boolean {
    const to = envelope.to;

    // Check if we have a direct path to the recipient
    if (this.directPathManager) {
      const connectionType = this.directPathManager.getConnectionType(to);

      if (connectionType === 'direct') {
        // Try to send directly - get peer atomically with the check
        const directPeer = this.transport.getPeer(to);
        if (directPeer) {
          // Mark as direct route and clear via (no relay needed)
          envelope.routeType = 'direct';
          envelope.via = [];
          directPeer.send(envelope);
          this.events.onMessageSentDirect?.(envelope, to);
          return true;
        }
        // Peer connection was lost between state check and send attempt
        // Sync DirectPathManager state to prevent future false positives
        this.directPathManager.handleDirectPathLost(to);
      }
    }

    // Fallback to relay
    this.sendViaRelay(envelope, relayId);
    return false;
  }

  /**
   * Check if a direct path is available to a peer.
   */
  hasDirectPath(peerId: string): boolean {
    if (!this.directPathManager) {
      return false;
    }
    return this.directPathManager.getConnectionType(peerId) === 'direct';
  }

  private sendAck(original: MessageEnvelope, ackType: AckType = 'recipient-received'): void {
    // Reverse the via path for the return journey
    // Original: sender -> relay1 -> relay2 -> recipient
    // ACK:      recipient -> relay2 -> relay1 -> sender
    const reversedVia = [...original.via].reverse();

    const payload: AckPayload = {
      originalMessageId: original.id,
      ackType,
    };
    const ack = this.createEnvelope(original.from, ACK_TYPE, payload, reversedVia);

    // Try to send via the reversed relay path
    // The first relay in reversedVia is the last relay that forwarded to us
    const firstRelayBack = reversedVia[0];
    if (firstRelayBack) {
      const relayPeer = this.transport.getPeer(firstRelayBack);
      if (relayPeer) {
        relayPeer.send(ack);
        return;
      }
    }

    // Try direct send to the original sender
    const directPeer = this.transport.getPeer(original.from);
    if (directPeer) {
      directPeer.send(ack);
      return;
    }

    // ACK failed - non-blocking, just emit warning
    this.events.onAckFailed(original.id, 'no route for ack');
  }
}
