import { beforeEach, describe, expect, it } from 'vitest';
import { NetworkTopology, type PeerInfo } from './network-topology.js';

function makePeer(overrides: Partial<PeerInfo> = {}): PeerInfo {
  return {
    nodeId: 'node-1',
    username: 'alice',
    publicKey: 'pk-1',
    reachableVia: [],
    lastSeen: Date.now(),
    roles: ['client'],
    ...overrides,
  };
}

describe('NetworkTopology', () => {
  let topology: NetworkTopology;

  beforeEach(() => {
    topology = new NetworkTopology(3000);
  });

  it('should add and retrieve a peer', () => {
    const peer = makePeer();
    topology.addPeer(peer);
    expect(topology.getPeer('node-1')).toMatchObject({ nodeId: 'node-1', username: 'alice' });
  });

  it('should remove a peer', () => {
    topology.addPeer(makePeer());
    expect(topology.removePeer('node-1')).toBe(true);
    expect(topology.getPeer('node-1')).toBeUndefined();
  });

  it('should return false when removing non-existent peer', () => {
    expect(topology.removePeer('non-existent')).toBe(false);
  });

  it('should list all reachable peers', () => {
    topology.addPeer(makePeer({ nodeId: 'a' }));
    topology.addPeer(makePeer({ nodeId: 'b' }));
    expect(topology.getReachablePeers()).toHaveLength(2);
  });

  it('should distinguish direct vs indirect peers', () => {
    topology.addPeer(makePeer({ nodeId: 'direct', reachableVia: [] }));
    topology.addPeer(makePeer({ nodeId: 'indirect', reachableVia: ['relay-1'] }));

    expect(topology.getDirectPeers()).toHaveLength(1);
    expect(topology.getDirectPeers()[0].nodeId).toBe('direct');
    expect(topology.getIndirectPeers()).toHaveLength(1);
    expect(topology.getIndirectPeers()[0].nodeId).toBe('indirect');
  });

  it('should report peer status based on lastSeen', () => {
    const now = Date.now();
    topology.addPeer(makePeer({ nodeId: 'online', lastSeen: now }));
    topology.addPeer(makePeer({ nodeId: 'stale', lastSeen: now - 4000 }));
    topology.addPeer(makePeer({ nodeId: 'offline', lastSeen: now - 7000 }));

    expect(topology.getPeerStatus('online')).toBe('online');
    expect(topology.getPeerStatus('stale')).toBe('stale');
    expect(topology.getPeerStatus('offline')).toBe('offline');
    expect(topology.getPeerStatus('unknown')).toBe('offline');
  });

  it('should update lastSeen', () => {
    topology.addPeer(makePeer({ nodeId: 'a', lastSeen: 0 }));
    topology.updateLastSeen('a');
    const peer = topology.getPeer('a');
    expect(peer?.lastSeen).toBeGreaterThan(0);
  });

  it('should report correct size and clear', () => {
    topology.addPeer(makePeer({ nodeId: 'a' }));
    topology.addPeer(makePeer({ nodeId: 'b' }));
    expect(topology.size()).toBe(2);
    topology.clear();
    expect(topology.size()).toBe(0);
  });

  it('should filter online peers', () => {
    const now = Date.now();
    topology.addPeer(makePeer({ nodeId: 'alive', lastSeen: now }));
    topology.addPeer(makePeer({ nodeId: 'dead', lastSeen: now - 10000 }));
    expect(topology.getOnlinePeers()).toHaveLength(1);
    expect(topology.getOnlinePeers()[0].nodeId).toBe('alive');
  });

  it('should get relay nodes', () => {
    topology.addPeer(makePeer({ nodeId: 'client-only', roles: ['client'] }));
    topology.addPeer(makePeer({ nodeId: 'relay-node', roles: ['client', 'relay'] }));
    topology.addPeer(makePeer({ nodeId: 'another-relay', roles: ['relay'] }));

    const relays = topology.getRelayNodes();
    expect(relays).toHaveLength(2);
    expect(relays.map((r) => r.nodeId).sort()).toEqual(['another-relay', 'relay-node']);
  });

  it('should get nodes by role', () => {
    topology.addPeer(makePeer({ nodeId: 'client-1', roles: ['client'] }));
    topology.addPeer(makePeer({ nodeId: 'client-2', roles: ['client'] }));
    topology.addPeer(makePeer({ nodeId: 'relay-1', roles: ['client', 'relay'] }));
    topology.addPeer(makePeer({ nodeId: 'observer-1', roles: ['observer'] }));

    expect(topology.getNodesByRole('client')).toHaveLength(3);
    expect(topology.getNodesByRole('relay')).toHaveLength(1);
    expect(topology.getNodesByRole('observer')).toHaveLength(1);
    expect(topology.getNodesByRole('bootstrap')).toHaveLength(0);
  });

  describe('lastSeen clamping (security)', () => {
    it('should clamp lastSeen when adding peer with future timestamp', () => {
      const futureTime = Date.now() + 60 * 60 * 1000; // 1 hour in future
      topology.addPeer(makePeer({ nodeId: 'future-peer', lastSeen: futureTime }));

      const peer = topology.getPeer('future-peer');
      // Should be clamped to at most 5 minutes in the future
      const maxAllowed = Date.now() + 5 * 60 * 1000;
      expect(peer?.lastSeen).toBeLessThanOrEqual(maxAllowed);
    });

    it('should clamp lastSeen when adding peer with very old timestamp', () => {
      const veryOldTime = Date.now() - 24 * 60 * 60 * 1000; // 24 hours ago
      topology.addPeer(makePeer({ nodeId: 'old-peer', lastSeen: veryOldTime }));

      const peer = topology.getPeer('old-peer');
      // Should be clamped to at most 1 hour in the past
      const minAllowed = Date.now() - 60 * 60 * 1000;
      expect(peer?.lastSeen).toBeGreaterThanOrEqual(minAllowed - 1000); // 1s tolerance
    });

    it('should clamp updateLastSeen with future timestamp', () => {
      topology.addPeer(makePeer({ nodeId: 'a' }));

      const futureTime = Date.now() + 60 * 60 * 1000; // 1 hour in future
      topology.updateLastSeen('a', futureTime);

      const peer = topology.getPeer('a');
      // Should be clamped to at most 5 minutes in the future
      const maxAllowed = Date.now() + 5 * 60 * 1000;
      expect(peer?.lastSeen).toBeLessThanOrEqual(maxAllowed);
    });

    it('should clamp updateLastSeen with very old timestamp', () => {
      topology.addPeer(makePeer({ nodeId: 'a' }));

      const veryOldTime = Date.now() - 24 * 60 * 60 * 1000; // 24 hours ago
      topology.updateLastSeen('a', veryOldTime);

      const peer = topology.getPeer('a');
      // Should be clamped to at most 1 hour in the past
      const minAllowed = Date.now() - 60 * 60 * 1000;
      expect(peer?.lastSeen).toBeGreaterThanOrEqual(minAllowed - 1000); // 1s tolerance
    });

    it('should allow reasonable timestamps without clamping', () => {
      const recentTime = Date.now() - 30 * 1000; // 30 seconds ago
      topology.addPeer(makePeer({ nodeId: 'normal-peer', lastSeen: recentTime }));

      const peer = topology.getPeer('normal-peer');
      // Should be exactly what we set (within tolerance for test execution)
      expect(peer?.lastSeen).toBeGreaterThanOrEqual(recentTime - 1000);
      expect(peer?.lastSeen).toBeLessThanOrEqual(recentTime + 1000);
    });
  });
});
