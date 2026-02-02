import { describe, expect, it, vi } from 'vitest';
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

    expect(events.onMessageForwarded).toHaveBeenCalledWith(envelope, PEER_B);
    const peer = transport.getPeer(PEER_B);
    expect(peer?.send).toHaveBeenCalledWith(envelope);
  });

  it('rejects message for unreachable peer', () => {
    const transport = createTransport();
    const events = createRouterEvents();
    const router = new Router(LOCAL_ID, transport, events);

    const envelope = makeEnvelope({ to: PEER_B });
    router.handleIncoming(envelope);

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

  it('auto-sends ACK when message is delivered locally', async () => {
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
        payload: { originalMessageId: envelope.id },
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
});
