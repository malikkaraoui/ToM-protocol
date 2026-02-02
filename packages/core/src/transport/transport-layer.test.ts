import { describe, expect, it, vi } from 'vitest';
import type { MessageEnvelope } from '../types/envelope.js';
import type { PeerConnection, SignalingClient, TransportEvents } from './transport-layer.js';
import { TransportLayer } from './transport-layer.js';

function createMockSignaling(): SignalingClient {
  return { send: vi.fn(), onMessage: null, close: vi.fn() };
}

function createMockEvents(): TransportEvents {
  return {
    onPeerConnected: vi.fn(),
    onPeerDisconnected: vi.fn(),
    onMessage: vi.fn(),
    onError: vi.fn(),
  };
}

function createMockPeerConnection(peerId: string): PeerConnection {
  return {
    peerId,
    send: vi.fn(),
    close: vi.fn(),
    onMessage: null,
    onClose: null,
  };
}

function makeEnvelope(overrides?: Partial<MessageEnvelope>): MessageEnvelope {
  return {
    id: 'msg-1',
    from: 'a'.repeat(64),
    to: 'b'.repeat(64),
    via: [],
    type: 'chat',
    payload: { text: 'hello' },
    timestamp: Date.now(),
    signature: 'sig',
    ...overrides,
  };
}

describe('TransportLayer', () => {
  it('connects to a peer and fires onPeerConnected', async () => {
    const signaling = createMockSignaling();
    const events = createMockEvents();
    const mockConn = createMockPeerConnection('peer-1');

    const transport = new TransportLayer('local', signaling, events, () => mockConn);
    const conn = await transport.connectToPeer('peer-1');

    expect(conn).toBe(mockConn);
    expect(events.onPeerConnected).toHaveBeenCalledWith('peer-1');
    expect(transport.getConnectedPeers()).toContain('peer-1');
  });

  it('returns existing connection on duplicate connect', async () => {
    const signaling = createMockSignaling();
    const events = createMockEvents();
    const mockConn = createMockPeerConnection('peer-1');

    const transport = new TransportLayer('local', signaling, events, () => mockConn);
    const conn1 = await transport.connectToPeer('peer-1');
    const conn2 = await transport.connectToPeer('peer-1');

    expect(conn1).toBe(conn2);
    expect(events.onPeerConnected).toHaveBeenCalledTimes(1);
  });

  it('sends envelope to connected peer', async () => {
    const signaling = createMockSignaling();
    const events = createMockEvents();
    const mockConn = createMockPeerConnection('peer-1');

    const transport = new TransportLayer('local', signaling, events, () => mockConn);
    await transport.connectToPeer('peer-1');

    const envelope = makeEnvelope();
    transport.sendTo('peer-1', envelope);

    expect(mockConn.send).toHaveBeenCalledWith(envelope);
  });

  it('fires onError when sending to unknown peer', () => {
    const signaling = createMockSignaling();
    const events = createMockEvents();

    const transport = new TransportLayer('local', signaling, events, () => createMockPeerConnection('x'));

    transport.sendTo('unknown', makeEnvelope());
    expect(events.onError).toHaveBeenCalledWith('unknown', expect.any(Error));
  });

  it('receives messages from peer via onMessage callback', async () => {
    const signaling = createMockSignaling();
    const events = createMockEvents();
    const mockConn = createMockPeerConnection('peer-1');

    const transport = new TransportLayer('local', signaling, events, () => mockConn);
    await transport.connectToPeer('peer-1');

    const envelope = makeEnvelope();
    // Simulate incoming message
    mockConn.onMessage?.(envelope);

    expect(events.onMessage).toHaveBeenCalledWith(envelope);
  });

  it('fires onPeerDisconnected when peer connection closes', async () => {
    const signaling = createMockSignaling();
    const events = createMockEvents();
    const mockConn = createMockPeerConnection('peer-1');

    const transport = new TransportLayer('local', signaling, events, () => mockConn);
    await transport.connectToPeer('peer-1');

    // Simulate disconnection
    mockConn.onClose?.();

    expect(events.onPeerDisconnected).toHaveBeenCalledWith('peer-1');
    expect(transport.getConnectedPeers()).not.toContain('peer-1');
  });

  it('disconnectPeer closes connection and removes peer', async () => {
    const signaling = createMockSignaling();
    const events = createMockEvents();
    const mockConn = createMockPeerConnection('peer-1');

    const transport = new TransportLayer('local', signaling, events, () => mockConn);
    await transport.connectToPeer('peer-1');
    transport.disconnectPeer('peer-1');

    expect(mockConn.close).toHaveBeenCalled();
    expect(events.onPeerDisconnected).toHaveBeenCalledWith('peer-1');
    expect(transport.getConnectedPeers()).toHaveLength(0);
  });

  it('close() disconnects all peers and closes signaling', async () => {
    const signaling = createMockSignaling();
    const events = createMockEvents();
    const conn1 = createMockPeerConnection('peer-1');
    const conn2 = createMockPeerConnection('peer-2');
    let callCount = 0;

    const transport = new TransportLayer('local', signaling, events, () => {
      callCount++;
      return callCount === 1 ? conn1 : conn2;
    });
    await transport.connectToPeer('peer-1');
    await transport.connectToPeer('peer-2');

    transport.close();

    expect(conn1.close).toHaveBeenCalled();
    expect(conn2.close).toHaveBeenCalled();
    expect(signaling.close).toHaveBeenCalled();
    expect(transport.getConnectedPeers()).toHaveLength(0);
  });
});
