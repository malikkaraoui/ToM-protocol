import { beforeEach, describe, expect, it } from 'vitest';
import { NetworkTopology, type PeerInfo } from './network-topology.js';

function makePeer(overrides: Partial<PeerInfo> = {}): PeerInfo {
  return {
    nodeId: 'node-1',
    username: 'alice',
    publicKey: 'pk-1',
    reachableVia: [],
    lastSeen: Date.now(),
    role: 'client',
    ...overrides,
  };
}

describe('NetworkTopology', () => {
  let topology: NetworkTopology;

  beforeEach(() => {
    topology = new NetworkTopology();
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
});
