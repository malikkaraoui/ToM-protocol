import { describe, expect, it } from 'vitest';
import { NetworkTopology } from '../discovery/network-topology.js';
import { RelaySelector } from './relay-selector.js';

function makePeer(
  nodeId: string,
  options: { roles?: ('client' | 'relay' | 'observer' | 'bootstrap')[]; lastSeen?: number } = {},
) {
  return {
    nodeId,
    username: `user-${nodeId}`,
    publicKey: nodeId,
    reachableVia: [],
    lastSeen: options.lastSeen ?? Date.now(),
    roles: options.roles ?? ['client'],
  };
}

describe('RelaySelector', () => {
  it('should return no-relays-available when no relays in topology', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    // Add only client nodes (no relay role)
    topology.addPeer(makePeer('node-1', { roles: ['client'] }));
    topology.addPeer(makePeer('node-2', { roles: ['client'] }));

    const result = selector.selectBestRelay('node-1', topology);
    expect(result.relayId).toBeNull();
    expect(result.reason).toBe('no-relays-available');
  });

  it('should select only online relays (not stale/offline)', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Add stale relay (lastSeen too old)
    topology.addPeer(makePeer('relay-stale', { roles: ['client', 'relay'], lastSeen: now - 10000 }));
    // Add online relay
    topology.addPeer(makePeer('relay-online', { roles: ['client', 'relay'], lastSeen: now }));
    // Add recipient
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now }));

    const result = selector.selectBestRelay('recipient', topology);
    expect(result.relayId).toBe('relay-online');
    expect(result.reason).toBe('best-available');
  });

  it('should select relay with relay role (not client-only)', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Add client-only node
    topology.addPeer(makePeer('client-only', { roles: ['client'], lastSeen: now }));
    // Add relay node
    topology.addPeer(makePeer('relay-node', { roles: ['client', 'relay'], lastSeen: now }));
    // Add recipient
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now }));

    const result = selector.selectBestRelay('recipient', topology);
    expect(result.relayId).toBe('relay-node');
    expect(result.reason).toBe('best-available');
  });

  it('should return no-peers when topology is empty', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const result = selector.selectBestRelay('recipient', topology);
    expect(result.relayId).toBeNull();
    expect(result.reason).toBe('no-peers');
  });

  it('should return recipient-is-self when sending to self', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);
    topology.addPeer(makePeer('relay', { roles: ['client', 'relay'] }));

    const result = selector.selectBestRelay('self', topology);
    expect(result.relayId).toBeNull();
    expect(result.reason).toBe('recipient-is-self');
  });

  it('should prefer relay with most recent lastSeen when multiple available', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Add older relay
    topology.addPeer(makePeer('relay-old', { roles: ['client', 'relay'], lastSeen: now - 1000 }));
    // Add newer relay
    topology.addPeer(makePeer('relay-new', { roles: ['client', 'relay'], lastSeen: now }));
    // Add recipient
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now }));

    const result = selector.selectBestRelay('recipient', topology);
    expect(result.relayId).toBe('relay-new');
    expect(result.reason).toBe('best-available');
  });

  it('should not select self as relay', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Add self as relay
    topology.addPeer(makePeer('self', { roles: ['client', 'relay'], lastSeen: now }));
    // Add other relay
    topology.addPeer(makePeer('other-relay', { roles: ['client', 'relay'], lastSeen: now }));
    // Add recipient
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now }));

    const result = selector.selectBestRelay('recipient', topology);
    expect(result.relayId).toBe('other-relay');
    expect(result.reason).toBe('best-available');
  });

  it('should not select recipient as relay', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Add recipient who is also a relay
    topology.addPeer(makePeer('recipient', { roles: ['client', 'relay'], lastSeen: now }));
    // Add another relay
    topology.addPeer(makePeer('other-relay', { roles: ['client', 'relay'], lastSeen: now }));

    const result = selector.selectBestRelay('recipient', topology);
    expect(result.relayId).toBe('other-relay');
    expect(result.reason).toBe('best-available');
  });

  it('should return no-relays-available when all relays are offline', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Add offline relay (lastSeen way too old)
    topology.addPeer(makePeer('relay-offline', { roles: ['client', 'relay'], lastSeen: now - 20000 }));
    // Add recipient
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now }));

    const result = selector.selectBestRelay('recipient', topology);
    expect(result.relayId).toBeNull();
    expect(result.reason).toBe('no-relays-available');
  });
});
