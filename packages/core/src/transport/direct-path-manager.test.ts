import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { MessageEnvelope } from '../types/envelope.js';
import { DirectPathManager } from './direct-path-manager.js';
import type { PeerConnection, TransportLayer } from './transport-layer.js';

function makeMockTransport(): TransportLayer {
  return {
    connectToPeer: vi.fn(),
    getPeer: vi.fn(),
    getConnectedPeers: vi.fn(() => []),
    registerPeer: vi.fn(),
    sendTo: vi.fn(),
    disconnectPeer: vi.fn(),
    close: vi.fn(),
  } as unknown as TransportLayer;
}

function makeMockPeer(peerId: string): PeerConnection {
  return {
    peerId,
    send: vi.fn(),
    close: vi.fn(),
    onMessage: null,
    onClose: null,
  };
}

function makeEnvelope(from: string, to: string): MessageEnvelope {
  return {
    id: `msg-${Date.now()}`,
    from,
    to,
    via: ['relay-1'],
    type: 'chat',
    payload: { text: 'hello' },
    timestamp: Date.now(),
    signature: '',
  };
}

describe('DirectPathManager', () => {
  let manager: DirectPathManager;
  let transport: TransportLayer;
  let events: {
    onDirectPathEstablished: ReturnType<typeof vi.fn>;
    onDirectPathLost: ReturnType<typeof vi.fn>;
    onDirectPathRestored: ReturnType<typeof vi.fn>;
  };

  beforeEach(() => {
    transport = makeMockTransport();
    events = {
      onDirectPathEstablished: vi.fn(),
      onDirectPathLost: vi.fn(),
      onDirectPathRestored: vi.fn(),
    };
    manager = new DirectPathManager('local-node', transport, events);
  });

  describe('conversation tracking', () => {
    it('should track conversation after message exchange via relay', () => {
      const envelope = makeEnvelope('local-node', 'peer-a');
      manager.trackConversation(envelope);

      expect(manager.hasConversation('peer-a')).toBe(true);
    });

    it('should not track conversation with self', () => {
      const envelope = makeEnvelope('local-node', 'local-node');
      manager.trackConversation(envelope);

      expect(manager.hasConversation('local-node')).toBe(false);
    });

    it('should track received messages', () => {
      const envelope = makeEnvelope('peer-b', 'local-node');
      manager.trackConversation(envelope);

      expect(manager.hasConversation('peer-b')).toBe(true);
    });
  });

  describe('direct path initiation', () => {
    it('should attempt direct connection after first relay exchange', async () => {
      const mockPeer = makeMockPeer('peer-a');
      vi.mocked(transport.connectToPeer).mockResolvedValue(mockPeer);

      const envelope = makeEnvelope('local-node', 'peer-a');
      manager.trackConversation(envelope);

      // Simulate connection success
      await manager.attemptDirectPath('peer-a');

      expect(transport.connectToPeer).toHaveBeenCalledWith('peer-a');
    });

    it('should emit direct-path-established event on success', async () => {
      const mockPeer = makeMockPeer('peer-a');
      vi.mocked(transport.connectToPeer).mockResolvedValue(mockPeer);

      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));
      await manager.attemptDirectPath('peer-a');

      expect(events.onDirectPathEstablished).toHaveBeenCalledWith('peer-a');
    });

    it('should not attempt direct path if no conversation exists', async () => {
      await manager.attemptDirectPath('unknown-peer');

      expect(transport.connectToPeer).not.toHaveBeenCalled();
    });

    it('should not attempt direct path if already connected', async () => {
      const mockPeer = makeMockPeer('peer-a');
      vi.mocked(transport.getPeer).mockReturnValue(mockPeer);

      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));
      manager.markDirectPathActive('peer-a');

      await manager.attemptDirectPath('peer-a');

      expect(transport.connectToPeer).not.toHaveBeenCalled();
    });
  });

  describe('connection state', () => {
    it('should return direct when direct path is active', () => {
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));
      manager.markDirectPathActive('peer-a');

      expect(manager.getConnectionType('peer-a')).toBe('direct');
    });

    it('should return relay when conversation exists but no direct path', () => {
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));

      expect(manager.getConnectionType('peer-a')).toBe('relay');
    });

    it('should return disconnected when no conversation exists', () => {
      expect(manager.getConnectionType('unknown')).toBe('disconnected');
    });
  });

  describe('fallback to relay', () => {
    it('should mark peer as relay-only when direct path is lost', () => {
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));
      manager.markDirectPathActive('peer-a');

      expect(manager.getConnectionType('peer-a')).toBe('direct');

      manager.handleDirectPathLost('peer-a');

      expect(manager.getConnectionType('peer-a')).toBe('relay');
      expect(events.onDirectPathLost).toHaveBeenCalledWith('peer-a');
    });
  });

  describe('reconnection', () => {
    it('should emit direct-path-restored on successful reconnection', async () => {
      const mockPeer = makeMockPeer('peer-a');
      vi.mocked(transport.connectToPeer).mockResolvedValue(mockPeer);

      // Setup: had conversation, then lost direct path
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));
      manager.markDirectPathActive('peer-a');
      manager.handleDirectPathLost('peer-a');

      // Reconnect
      await manager.attemptDirectPath('peer-a');

      expect(events.onDirectPathRestored).toHaveBeenCalledWith('peer-a');
    });
  });

  describe('getDirectPeers', () => {
    it('should return list of peers with active direct paths', () => {
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));
      manager.trackConversation(makeEnvelope('local-node', 'peer-b'));
      manager.markDirectPathActive('peer-a');

      const directPeers = manager.getDirectPeers();

      expect(directPeers).toContain('peer-a');
      expect(directPeers).not.toContain('peer-b');
    });
  });

  describe('automatic reconnection', () => {
    it('should attempt reconnect when peer comes online after having direct path', async () => {
      vi.useFakeTimers();

      const mockPeer = makeMockPeer('peer-a');
      vi.mocked(transport.connectToPeer).mockResolvedValue(mockPeer);

      // Setup: had conversation with direct path, then lost it
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));
      manager.markDirectPathActive('peer-a');
      manager.handleDirectPathLost('peer-a');

      // Peer comes back online (starts async reconnection with backoff)
      const reconnectPromise = manager.onPeerOnline('peer-a');

      // Advance timers past the backoff delay (1s for first attempt)
      await vi.advanceTimersByTimeAsync(1000);
      await reconnectPromise;

      expect(transport.connectToPeer).toHaveBeenCalledWith('peer-a');
      expect(events.onDirectPathRestored).toHaveBeenCalledWith('peer-a');

      vi.useRealTimers();
    });

    it('should not attempt reconnect if never had direct path', async () => {
      // Just track conversation, never had direct path
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));

      await manager.onPeerOnline('peer-a');

      expect(transport.connectToPeer).not.toHaveBeenCalled();
    });

    it('should not attempt reconnect if already have direct path', async () => {
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));
      manager.markDirectPathActive('peer-a');

      await manager.onPeerOnline('peer-a');

      expect(transport.connectToPeer).not.toHaveBeenCalled();
    });

    it('should not attempt reconnect for unknown peer', async () => {
      await manager.onPeerOnline('unknown-peer');

      expect(transport.connectToPeer).not.toHaveBeenCalled();
    });
  });

  describe('security fixes', () => {
    it('should timeout connection attempts after 10 seconds', async () => {
      vi.useFakeTimers();

      // Make connectToPeer hang indefinitely
      vi.mocked(transport.connectToPeer).mockImplementation(
        () => new Promise(() => {}), // Never resolves
      );

      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));

      // Start connection attempt
      const attemptPromise = manager.attemptDirectPath('peer-a');

      // Advance past timeout
      await vi.advanceTimersByTimeAsync(11000);

      const result = await attemptPromise;

      // Should have failed due to timeout
      expect(result).toBe(false);
      expect(manager.getConnectionType('peer-a')).toBe('relay');

      vi.useRealTimers();
    });

    it('should track reconnection attempts separately from messages (lastAttemptAt)', async () => {
      vi.useFakeTimers();

      const mockPeer = makeMockPeer('peer-a');
      vi.mocked(transport.connectToPeer)
        .mockRejectedValueOnce(new Error('fail 1'))
        .mockRejectedValueOnce(new Error('fail 2'))
        .mockRejectedValueOnce(new Error('fail 3'))
        .mockResolvedValue(mockPeer);

      // Setup: had conversation with direct path, then lost it
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));
      manager.markDirectPathActive('peer-a');
      manager.handleDirectPathLost('peer-a');

      // Attempt reconnects that fail
      for (let i = 0; i < 3; i++) {
        const reconnectPromise = manager.onPeerOnline('peer-a');
        await vi.advanceTimersByTimeAsync(5000);
        await reconnectPromise;
      }

      // Now send a message (updates lastMessageAt but should NOT reset cooldown)
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));

      // Should still be in cooldown (30s from lastAttemptAt, not lastMessageAt)
      const reconnectPromise = manager.onPeerOnline('peer-a');
      await vi.advanceTimersByTimeAsync(1000);
      await reconnectPromise;

      // Should NOT have attempted (cooldown based on lastAttemptAt)
      expect(transport.connectToPeer).toHaveBeenCalledTimes(3); // Only the 3 failed attempts

      vi.useRealTimers();
    });

    it('should purge stale conversations after TTL', async () => {
      vi.useFakeTimers();

      manager.start();
      manager.trackConversation(makeEnvelope('local-node', 'peer-a'));
      manager.trackConversation(makeEnvelope('local-node', 'peer-b'));

      expect(manager.getConversationCount()).toBe(2);

      // Advance past TTL (1 hour) + purge interval (5 minutes)
      vi.advanceTimersByTime(66 * 60 * 1000);

      // Conversations should be purged
      expect(manager.getConversationCount()).toBe(0);

      manager.stop();
      vi.useRealTimers();
    });

    it('should process reconnection batches in chunks of 50', async () => {
      vi.useFakeTimers();

      const mockPeer = makeMockPeer('peer');
      vi.mocked(transport.connectToPeer).mockResolvedValue(mockPeer);

      // Setup 100 peers with previous direct paths
      const peerIds: string[] = [];
      for (let i = 0; i < 100; i++) {
        const peerId = `peer-${i}`;
        peerIds.push(peerId);
        manager.trackConversation(makeEnvelope('local-node', peerId));
        manager.markDirectPathActive(peerId);
        manager.handleDirectPathLost(peerId);
      }

      // Start batch reconnect
      const batchPromise = manager.onMultiplePeersOnline(peerIds);

      // First batch should start
      await vi.advanceTimersByTimeAsync(50 * 100 + 2000); // Staggered + backoff
      // Second batch should start after first completes
      await vi.advanceTimersByTimeAsync(50 * 100 + 2000);

      await batchPromise;

      // All 100 should have been attempted
      expect(transport.connectToPeer).toHaveBeenCalledTimes(100);

      vi.useRealTimers();
    });
  });
});
