import { describe, expect, it, vi } from 'vitest';
import { DirectPathManager } from '../transport/direct-path-manager.js';
import type { PeerConnection, SignalingClient, TransportEvents } from '../transport/transport-layer.js';
import { TransportLayer } from '../transport/transport-layer.js';
import type { MessageEnvelope } from '../types/envelope.js';
import type { RouterEvents } from './router.js';
import { Router } from './router.js';

const LOCAL_ID = 'a'.repeat(64);
const PEER_B = 'b'.repeat(64);
const RELAY_R = 'r'.repeat(64);
const RELAY_S = 's'.repeat(64);

function makeEnvelope(overrides?: Partial<MessageEnvelope>): MessageEnvelope {
  return {
    id: 'msg-1',
    from: PEER_B,
    to: LOCAL_ID,
    via: [],
    type: 'chat',
    payload: { text: 'hello' },
    timestamp: Date.now(),
    signature: 'sig',
    ...overrides,
  };
}

function createMockPeer(peerId: string): PeerConnection {
  return { peerId, send: vi.fn(), close: vi.fn(), onMessage: null, onClose: null };
}

function createTransport(): TransportLayer {
  const signaling: SignalingClient = { send: vi.fn(), onMessage: null, close: vi.fn() };
  const events: TransportEvents = {
    onPeerConnected: vi.fn(),
    onPeerDisconnected: vi.fn(),
    onMessage: vi.fn(),
    onError: vi.fn(),
  };
  return new TransportLayer(LOCAL_ID, signaling, events, (peerId) => createMockPeer(peerId));
}

function createRouterEvents(): RouterEvents {
  return {
    onMessageDelivered: vi.fn(),
    onMessageForwarded: vi.fn(),
    onMessageRejected: vi.fn(),
    onAckReceived: vi.fn(),
    onAckFailed: vi.fn(),
  };
}

describe('Router', () => {
  it('delivers message addressed to local node', () => {
    const transport = createTransport();
    const events = createRouterEvents();
    const router = new Router(LOCAL_ID, transport, events);

    const envelope = makeEnvelope({ to: LOCAL_ID });
    router.handleIncoming(envelope);

    expect(events.onMessageDelivered).toHaveBeenCalledWith(envelope);
    expect(events.onMessageForwarded).not.toHaveBeenCalled();
  });

  it('forwards message addressed to another peer', async () => {
    const transport = createTransport();
    const events = createRouterEvents();
    const router = new Router(LOCAL_ID, transport, events);

    await transport.connectToPeer(PEER_B);
    const envelope = makeEnvelope({ from: RELAY_R, to: PEER_B });
    router.handleIncoming(envelope);

    // Wait for async forward to complete
    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events.onMessageForwarded).toHaveBeenCalledWith(envelope, PEER_B);
    const peer = transport.getPeer(PEER_B);
    expect(peer?.send).toHaveBeenCalledWith(envelope);
  });

  it('rejects message for unreachable peer', async () => {
    const transport = createTransport();
    const events = createRouterEvents();
    const router = new Router(LOCAL_ID, transport, events);

    // Mock connectToPeer to fail (simulating unreachable peer)
    vi.spyOn(transport, 'connectToPeer').mockRejectedValue(new Error('Connection failed'));

    const envelope = makeEnvelope({ to: PEER_B });
    router.handleIncoming(envelope);

    // Wait for async forward to complete
    await new Promise((resolve) => setTimeout(resolve, 10));

    expect(events.onMessageRejected).toHaveBeenCalledWith(envelope, 'PEER_UNREACHABLE');
  });

  it('sends message via relay', async () => {
    const transport = createTransport();
    const events = createRouterEvents();
    const router = new Router(LOCAL_ID, transport, events);

    await transport.connectToPeer(RELAY_R);
    const envelope = makeEnvelope({ from: LOCAL_ID, to: PEER_B, via: [] });
    router.sendViaRelay(envelope, RELAY_R);

    const relay = transport.getPeer(RELAY_R);
    expect(relay?.send).toHaveBeenCalledWith(envelope);
    expect(envelope.via).toContain(RELAY_R);
  });

  it('rejects send via unreachable relay', () => {
    const transport = createTransport();
    const events = createRouterEvents();
    const router = new Router(LOCAL_ID, transport, events);

    const envelope = makeEnvelope({ from: LOCAL_ID, to: PEER_B });
    router.sendViaRelay(envelope, RELAY_R);

    expect(events.onMessageRejected).toHaveBeenCalledWith(envelope, 'RELAY_UNREACHABLE');
  });

  it('createEnvelope creates a valid envelope', () => {
    const transport = createTransport();
    const events = createRouterEvents();
    const router = new Router(LOCAL_ID, transport, events);

    const envelope = router.createEnvelope(PEER_B, 'chat', { text: 'hi' }, [RELAY_R]);

    expect(envelope.from).toBe(LOCAL_ID);
    expect(envelope.to).toBe(PEER_B);
    expect(envelope.type).toBe('chat');
    expect(envelope.via).toEqual([RELAY_R]);
    expect(envelope.id).toBeTruthy();
    expect(envelope.timestamp).toBeGreaterThan(0);
  });

  it('auto-sends ACK with recipient-received type when message is delivered locally', async () => {
    const transport = createTransport();
    const events = createRouterEvents();
    const router = new Router(LOCAL_ID, transport, events);

    // Connect relay so ACK can be sent back
    await transport.connectToPeer(RELAY_R);
    const envelope = makeEnvelope({ from: PEER_B, to: LOCAL_ID, via: [RELAY_R] });
    router.handleIncoming(envelope);

    expect(events.onMessageDelivered).toHaveBeenCalledWith(envelope);
    const relayPeer = transport.getPeer(RELAY_R);
    expect(relayPeer?.send).toHaveBeenCalledWith(
      expect.objectContaining({
        type: 'ack',
        from: LOCAL_ID,
        to: PEER_B,
        payload: { originalMessageId: envelope.id, ackType: 'recipient-received' },
      }),
    );
  });

  it('handles incoming ACK and fires onAckReceived', () => {
    const transport = createTransport();
    const events = createRouterEvents();
    const router = new Router(LOCAL_ID, transport, events);

    const ack = makeEnvelope({
      from: PEER_B,
      to: LOCAL_ID,
      type: 'ack',
      payload: { originalMessageId: 'msg-original' },
    });
    router.handleIncoming(ack);

    expect(events.onAckReceived).toHaveBeenCalledWith('msg-original', PEER_B);
    expect(events.onMessageDelivered).not.toHaveBeenCalled();
  });

  describe('ACK types', () => {
    it('fires onRelayAckReceived for relay-forwarded ACK', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onRelayAckReceived = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const ack = makeEnvelope({
        from: RELAY_R,
        to: LOCAL_ID,
        type: 'ack',
        payload: { originalMessageId: 'msg-original', ackType: 'relay-forwarded' },
      });
      router.handleIncoming(ack);

      expect(events.onAckReceived).toHaveBeenCalledWith('msg-original', RELAY_R);
      expect(events.onRelayAckReceived).toHaveBeenCalledWith('msg-original', RELAY_R);
    });

    it('fires onDeliveryAckReceived for recipient-received ACK', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onDeliveryAckReceived = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const ack = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        type: 'ack',
        payload: { originalMessageId: 'msg-original', ackType: 'recipient-received' },
      });
      router.handleIncoming(ack);

      expect(events.onAckReceived).toHaveBeenCalledWith('msg-original', PEER_B);
      expect(events.onDeliveryAckReceived).toHaveBeenCalledWith('msg-original', PEER_B);
    });

    it('sends relay ACK when forwarding message', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      // Connect both sender and recipient
      await transport.connectToPeer(PEER_B);
      await transport.connectToPeer(RELAY_R);

      const envelope = makeEnvelope({ from: RELAY_R, to: PEER_B, via: [LOCAL_ID] });
      router.handleIncoming(envelope);

      // Wait for async forward to complete
      await new Promise((resolve) => setTimeout(resolve, 10));

      expect(events.onMessageForwarded).toHaveBeenCalledWith(envelope, PEER_B);

      // Check that relay ACK was sent back to sender
      const senderPeer = transport.getPeer(RELAY_R);
      expect(senderPeer?.send).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'ack',
          from: LOCAL_ID,
          to: RELAY_R,
          payload: { originalMessageId: envelope.id, ackType: 'relay-forwarded' },
        }),
      );
    });
  });

  describe('Read receipts', () => {
    it('handles incoming read receipt and fires onReadReceiptReceived', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onReadReceiptReceived = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      // Use a recent timestamp (within 7 days) to avoid clamping
      const recentReadAt = Date.now() - 1000; // 1 second ago

      const readReceipt = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        type: 'read-receipt',
        payload: { originalMessageId: 'msg-original', readAt: recentReadAt },
      });
      router.handleIncoming(readReceipt);

      expect(events.onReadReceiptReceived).toHaveBeenCalledWith('msg-original', recentReadAt, PEER_B);
      expect(events.onMessageDelivered).not.toHaveBeenCalled();
    });

    it('uses current time if readAt not provided', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onReadReceiptReceived = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const now = Date.now();
      const readReceipt = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        type: 'read-receipt',
        payload: { originalMessageId: 'msg-original' },
      });
      router.handleIncoming(readReceipt);

      expect(events.onReadReceiptReceived).toHaveBeenCalledWith('msg-original', expect.any(Number), PEER_B);
      const callArgs = (events.onReadReceiptReceived as ReturnType<typeof vi.fn>).mock.calls[0];
      expect(callArgs[1]).toBeGreaterThanOrEqual(now);
    });

    it('clamps readAt to not be in the future', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onReadReceiptReceived = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const futureTime = Date.now() + 1000000; // Way in the future
      const readReceipt = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        type: 'read-receipt',
        payload: { originalMessageId: 'msg-future', readAt: futureTime },
      });
      router.handleIncoming(readReceipt);

      const callArgs = (events.onReadReceiptReceived as ReturnType<typeof vi.fn>).mock.calls[0];
      expect(callArgs[1]).toBeLessThanOrEqual(Date.now());
    });

    it('clamps readAt to not be more than 7 days in the past', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onReadReceiptReceived = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const veryOldTime = Date.now() - 30 * 24 * 60 * 60 * 1000; // 30 days ago
      const readReceipt = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        type: 'read-receipt',
        payload: { originalMessageId: 'msg-old', readAt: veryOldTime },
      });
      router.handleIncoming(readReceipt);

      const callArgs = (events.onReadReceiptReceived as ReturnType<typeof vi.fn>).mock.calls[0];
      const maxPastMs = 7 * 24 * 60 * 60 * 1000;
      expect(callArgs[1]).toBeGreaterThanOrEqual(Date.now() - maxPastMs - 1000); // Allow 1s tolerance
    });

    it('rejects duplicate read receipts (anti-replay)', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onReadReceiptReceived = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const readReceipt = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        type: 'read-receipt',
        payload: { originalMessageId: 'msg-replay', readAt: Date.now() },
      });

      router.handleIncoming(readReceipt);
      router.handleIncoming(readReceipt); // Replay

      // Should only fire once
      expect(events.onReadReceiptReceived).toHaveBeenCalledTimes(1);
    });
  });

  describe('ACK security', () => {
    it('rejects invalid ACK types', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onRelayAckReceived = vi.fn();
      events.onDeliveryAckReceived = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const invalidAck = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        type: 'ack',
        payload: { originalMessageId: 'msg-1', ackType: 'invalid-type' },
      });

      router.handleIncoming(invalidAck);

      expect(events.onAckReceived).not.toHaveBeenCalled();
      expect(events.onRelayAckReceived).not.toHaveBeenCalled();
      expect(events.onDeliveryAckReceived).not.toHaveBeenCalled();
    });

    it('rejects duplicate ACKs (anti-replay)', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      const ack = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        type: 'ack',
        payload: { originalMessageId: 'msg-replay-ack', ackType: 'recipient-received' },
      });

      router.handleIncoming(ack);
      router.handleIncoming(ack); // Replay

      // Should only fire once
      expect(events.onAckReceived).toHaveBeenCalledTimes(1);
    });

    it('allows same messageId with different ackTypes', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onRelayAckReceived = vi.fn();
      events.onDeliveryAckReceived = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const relayAck = makeEnvelope({
        from: RELAY_R,
        to: LOCAL_ID,
        type: 'ack',
        payload: { originalMessageId: 'msg-multi', ackType: 'relay-forwarded' },
      });

      const deliveryAck = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        type: 'ack',
        payload: { originalMessageId: 'msg-multi', ackType: 'recipient-received' },
      });

      router.handleIncoming(relayAck);
      router.handleIncoming(deliveryAck);

      expect(events.onRelayAckReceived).toHaveBeenCalledTimes(1);
      expect(events.onDeliveryAckReceived).toHaveBeenCalledTimes(1);
    });
  });

  it('fires onAckFailed when no route for ACK', () => {
    const transport = createTransport();
    const events = createRouterEvents();
    const router = new Router(LOCAL_ID, transport, events);

    // No relay connected, no direct peer — ACK will fail
    const envelope = makeEnvelope({ from: PEER_B, to: LOCAL_ID, via: [RELAY_R] });
    router.handleIncoming(envelope);

    expect(events.onMessageDelivered).toHaveBeenCalled();
    expect(events.onAckFailed).toHaveBeenCalledWith(envelope.id, 'no route for ack');
  });

  describe('Direct path preference', () => {
    function createDirectPathManager(transport: TransportLayer) {
      return new DirectPathManager(LOCAL_ID, transport, {
        onDirectPathEstablished: vi.fn(),
        onDirectPathLost: vi.fn(),
        onDirectPathRestored: vi.fn(),
      });
    }

    it('sends via direct path when available', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onMessageSentDirect = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);
      const directPathManager = createDirectPathManager(transport);

      router.setDirectPathManager(directPathManager);

      // Setup: connect to peer and mark direct path active
      await transport.connectToPeer(PEER_B);
      directPathManager.trackConversation(makeEnvelope({ from: LOCAL_ID, to: PEER_B }));
      directPathManager.markDirectPathActive(PEER_B);

      const envelope = makeEnvelope({ from: LOCAL_ID, to: PEER_B, via: [] });
      const sentDirect = router.sendWithDirectPreference(envelope, RELAY_R);

      expect(sentDirect).toBe(true);
      expect(envelope.routeType).toBe('direct');
      expect(envelope.via).toEqual([]);
      expect(events.onMessageSentDirect).toHaveBeenCalledWith(envelope, PEER_B);
    });

    it('falls back to relay when no direct path', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);
      const directPathManager = createDirectPathManager(transport);

      router.setDirectPathManager(directPathManager);

      // Setup: connect to relay, but no direct path to peer
      await transport.connectToPeer(RELAY_R);
      directPathManager.trackConversation(makeEnvelope({ from: LOCAL_ID, to: PEER_B }));
      // NOT marking direct path active

      const envelope = makeEnvelope({ from: LOCAL_ID, to: PEER_B, via: [] });
      const sentDirect = router.sendWithDirectPreference(envelope, RELAY_R);

      expect(sentDirect).toBe(false);
      expect(envelope.routeType).toBe('relay');
      expect(envelope.via).toContain(RELAY_R);
    });

    it('hasDirectPath returns correct status', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);
      const directPathManager = createDirectPathManager(transport);

      router.setDirectPathManager(directPathManager);

      expect(router.hasDirectPath(PEER_B)).toBe(false);

      directPathManager.trackConversation(makeEnvelope({ from: LOCAL_ID, to: PEER_B }));
      directPathManager.markDirectPathActive(PEER_B);

      expect(router.hasDirectPath(PEER_B)).toBe(true);
    });

    it('syncs DirectPathManager state when peer connection lost during send', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);
      const directPathManager = createDirectPathManager(transport);

      router.setDirectPathManager(directPathManager);

      // Setup: track conversation and mark direct path active
      directPathManager.trackConversation(makeEnvelope({ from: LOCAL_ID, to: PEER_B }));
      directPathManager.markDirectPathActive(PEER_B);

      // Connect relay for fallback
      await transport.connectToPeer(RELAY_R);

      // Direct path manager thinks we have direct connection, but peer is NOT connected
      // (simulates race condition where connection dropped between state check and send)
      expect(directPathManager.getConnectionType(PEER_B)).toBe('direct');

      const envelope = makeEnvelope({ from: LOCAL_ID, to: PEER_B, via: [] });
      const sentDirect = router.sendWithDirectPreference(envelope, RELAY_R);

      // Should have fallen back to relay
      expect(sentDirect).toBe(false);
      expect(envelope.routeType).toBe('relay');

      // DirectPathManager state should be synced (no longer thinks it's 'direct')
      expect(directPathManager.getConnectionType(PEER_B)).toBe('relay');
    });
  });

  describe('Multi-relay chain forwarding', () => {
    it('forwards to next relay when we are intermediate in the chain', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      // Connect to next relay in chain
      await transport.connectToPeer(RELAY_S);

      // Message with multi-relay path: PEER_B -> RELAY_R -> LOCAL_ID -> RELAY_S -> recipient
      // We are LOCAL_ID, positioned between RELAY_R and RELAY_S
      const envelope = makeEnvelope({
        from: PEER_B,
        to: 'recipient',
        via: [RELAY_R, LOCAL_ID, RELAY_S],
      });

      router.handleIncoming(envelope);

      // Wait for async forward
      await new Promise((resolve) => setTimeout(resolve, 10));

      // Should forward to RELAY_S (next hop after us)
      expect(events.onMessageForwarded).toHaveBeenCalledWith(envelope, RELAY_S);
      const nextRelay = transport.getPeer(RELAY_S);
      expect(nextRelay?.send).toHaveBeenCalledWith(envelope);
    });

    it('forwards to recipient when we are the last relay in the chain', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      // Connect to recipient
      await transport.connectToPeer(PEER_B);

      // Message with multi-relay path: sender -> RELAY_R -> LOCAL_ID -> PEER_B (recipient)
      // We are LOCAL_ID, the last relay before recipient
      const envelope = makeEnvelope({
        from: RELAY_R,
        to: PEER_B,
        via: [RELAY_R, LOCAL_ID],
      });

      router.handleIncoming(envelope);

      // Wait for async forward
      await new Promise((resolve) => setTimeout(resolve, 10));

      // Should forward to PEER_B (recipient)
      expect(events.onMessageForwarded).toHaveBeenCalledWith(envelope, PEER_B);
      const recipient = transport.getPeer(PEER_B);
      expect(recipient?.send).toHaveBeenCalledWith(envelope);
    });

    it('delivers message when addressed to us even if we are in via chain', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      // Message addressed to us
      const envelope = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        via: [RELAY_R],
      });

      router.handleIncoming(envelope);

      // Should deliver to us, not forward
      expect(events.onMessageDelivered).toHaveBeenCalledWith(envelope);
      expect(events.onMessageForwarded).not.toHaveBeenCalled();
    });

    it('forwards directly when we are not in the via chain', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      await transport.connectToPeer(PEER_B);

      // Message where we are not in the via chain - standard single-hop forwarding
      const envelope = makeEnvelope({
        from: RELAY_R,
        to: PEER_B,
        via: [RELAY_R], // Only RELAY_R, not us
      });

      router.handleIncoming(envelope);

      // Wait for async forward
      await new Promise((resolve) => setTimeout(resolve, 10));

      // Should forward to recipient directly
      expect(events.onMessageForwarded).toHaveBeenCalledWith(envelope, PEER_B);
    });

    it('adds hop timestamp when forwarding in chain', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      await transport.connectToPeer(RELAY_S);

      const envelope = makeEnvelope({
        from: PEER_B,
        to: 'recipient',
        via: [RELAY_R, LOCAL_ID, RELAY_S],
        hopTimestamps: [Date.now() - 100], // Previous relay's timestamp
      });

      const originalTimestampCount = envelope.hopTimestamps?.length ?? 0;

      router.handleIncoming(envelope);

      // Wait for async forward
      await new Promise((resolve) => setTimeout(resolve, 10));

      // Should have added our hop timestamp
      expect(envelope.hopTimestamps?.length).toBe(originalTimestampCount + 1);
    });

    it('rejects message with via chain too deep', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      // Create a chain with 6 relays (exceeds MAX_RELAY_DEPTH of 4)
      const envelope = makeEnvelope({
        from: PEER_B,
        to: 'recipient',
        via: ['r1', 'r2', 'r3', LOCAL_ID, 'r5', 'r6'],
      });

      router.handleIncoming(envelope);

      // Wait for async forward
      await new Promise((resolve) => setTimeout(resolve, 10));

      // Should reject due to chain depth
      expect(events.onMessageRejected).toHaveBeenCalledWith(envelope, 'RELAY_CHAIN_TOO_DEEP');
    });

    it('sends relay ACK when forwarding in chain', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      // Connect to both sender and next hop
      await transport.connectToPeer(RELAY_R);
      await transport.connectToPeer(RELAY_S);

      const envelope = makeEnvelope({
        from: RELAY_R,
        to: 'recipient',
        via: [RELAY_R, LOCAL_ID, RELAY_S],
      });

      router.handleIncoming(envelope);

      // Wait for async forward
      await new Promise((resolve) => setTimeout(resolve, 10));

      // Should send relay ACK back to sender
      const senderPeer = transport.getPeer(RELAY_R);
      expect(senderPeer?.send).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'ack',
          from: LOCAL_ID,
          to: RELAY_R,
          payload: { originalMessageId: envelope.id, ackType: 'relay-forwarded' },
        }),
      );
    });

    it('reverses via path for ACK return journey', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      // Connect to the last relay in the original path
      await transport.connectToPeer(RELAY_S);

      // Message came through RELAY_R -> RELAY_S -> us
      const envelope = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        via: [RELAY_R, RELAY_S],
      });

      router.handleIncoming(envelope);

      // Should send ACK via reversed path: RELAY_S first (was last in original)
      const relayPeer = transport.getPeer(RELAY_S);
      expect(relayPeer?.send).toHaveBeenCalledWith(
        expect.objectContaining({
          type: 'ack',
          from: LOCAL_ID,
          to: PEER_B,
          via: [RELAY_S, RELAY_R], // Reversed!
        }),
      );
    });
  });

  describe('Relay failure rerouting (Story 5.2)', () => {
    it('emits onRerouteNeeded when relay is unreachable', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onRerouteNeeded = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const envelope = makeEnvelope({ from: LOCAL_ID, to: PEER_B });

      // Try to send via unreachable relay
      const success = router.sendViaRelay(envelope, RELAY_R);

      expect(success).toBe(false);
      expect(events.onRerouteNeeded).toHaveBeenCalledWith(envelope, RELAY_R);
      expect(events.onMessageRejected).toHaveBeenCalledWith(envelope, 'RELAY_UNREACHABLE');
    });

    it('returns true when sendViaRelay succeeds', async () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onRerouteNeeded = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      await transport.connectToPeer(RELAY_R);
      const envelope = makeEnvelope({ from: LOCAL_ID, to: PEER_B, via: [] });

      const success = router.sendViaRelay(envelope, RELAY_R);

      expect(success).toBe(true);
      expect(events.onRerouteNeeded).not.toHaveBeenCalled();
    });

    it('emits onMessageQueued via emitMessageQueued', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onMessageQueued = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const envelope = makeEnvelope({ from: LOCAL_ID, to: PEER_B });

      router.emitMessageQueued(envelope, 'no alternate relays');

      expect(events.onMessageQueued).toHaveBeenCalledWith(envelope, 'no alternate relays');
    });
  });

  describe('Message deduplication (Story 5.2)', () => {
    it('delivers first message and blocks duplicate', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      events.onDuplicateMessage = vi.fn();
      const router = new Router(LOCAL_ID, transport, events);

      const envelope = makeEnvelope({
        id: 'msg-dedup-test',
        from: PEER_B,
        to: LOCAL_ID,
        via: [RELAY_R],
      });

      // First delivery should succeed
      router.handleIncoming(envelope);
      expect(events.onMessageDelivered).toHaveBeenCalledTimes(1);
      expect(events.onDuplicateMessage).not.toHaveBeenCalled();

      // Second delivery of same message should be blocked
      router.handleIncoming(envelope);
      expect(events.onMessageDelivered).toHaveBeenCalledTimes(1); // Still only 1
      expect(events.onDuplicateMessage).toHaveBeenCalledWith('msg-dedup-test', PEER_B);
    });

    it('allows different messages from same sender', () => {
      const transport = createTransport();
      const events = createRouterEvents();
      const router = new Router(LOCAL_ID, transport, events);

      const envelope1 = makeEnvelope({
        id: 'msg-1',
        from: PEER_B,
        to: LOCAL_ID,
      });

      const envelope2 = makeEnvelope({
        id: 'msg-2',
        from: PEER_B,
        to: LOCAL_ID,
      });

      router.handleIncoming(envelope1);
      router.handleIncoming(envelope2);

      expect(events.onMessageDelivered).toHaveBeenCalledTimes(2);
    });
  });
});
