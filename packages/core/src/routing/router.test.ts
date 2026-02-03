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

      const readReceipt = makeEnvelope({
        from: PEER_B,
        to: LOCAL_ID,
        type: 'read-receipt',
        payload: { originalMessageId: 'msg-original', readAt: 1234567890 },
      });
      router.handleIncoming(readReceipt);

      expect(events.onReadReceiptReceived).toHaveBeenCalledWith('msg-original', 1234567890, PEER_B);
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
});
