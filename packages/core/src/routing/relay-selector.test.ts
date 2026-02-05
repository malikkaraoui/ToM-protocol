import { describe, expect, it } from 'vitest';
import { NetworkTopology } from '../discovery/network-topology.js';
import { MAX_RELAY_DEPTH, RelaySelector } from './relay-selector.js';

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
  it('should return direct-fallback when no relays but recipient is online', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    // Add only client nodes (no relay role)
    topology.addPeer(makePeer('node-1', { roles: ['client'] }));
    topology.addPeer(makePeer('node-2', { roles: ['client'] }));

    const result = selector.selectBestRelay('node-1', topology);
    expect(result.relayId).toBeNull();
    expect(result.reason).toBe('direct-fallback');
  });

  it('should return no-relays-available when no relays and recipient is offline', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Add only client nodes (no relay role)
    topology.addPeer(makePeer('node-1', { roles: ['client'], lastSeen: now }));
    // Recipient is offline (stale)
    topology.addPeer(makePeer('node-2', { roles: ['client'], lastSeen: now - 20000 }));

    const result = selector.selectBestRelay('node-2', topology);
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

  it('should return direct-fallback when all relays are offline but recipient is online', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Add offline relay (lastSeen way too old)
    topology.addPeer(makePeer('relay-offline', { roles: ['client', 'relay'], lastSeen: now - 20000 }));
    // Add online recipient
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now }));

    const result = selector.selectBestRelay('recipient', topology);
    expect(result.relayId).toBeNull();
    expect(result.reason).toBe('direct-fallback');
  });

  it('should return no-relays-available when all relays and recipient are offline', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Add offline relay
    topology.addPeer(makePeer('relay-offline', { roles: ['client', 'relay'], lastSeen: now - 20000 }));
    // Add offline recipient
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now - 20000 }));

    const result = selector.selectBestRelay('recipient', topology);
    expect(result.relayId).toBeNull();
    expect(result.reason).toBe('no-relays-available');
  });
});

describe('RelaySelector - Multi-relay path selection', () => {
  it('should return direct when recipient is online and directly reachable', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now }));

    const result = selector.selectPathToRecipient('recipient', topology);
    expect(result.path).toEqual([]);
    expect(result.reason).toBe('direct');
  });

  it('should return single-relay path when relay is available', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Recipient is offline (needs relay)
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now - 20000 }));
    // Relay is online
    topology.addPeer(makePeer('relay-1', { roles: ['client', 'relay'], lastSeen: now }));

    const result = selector.selectPathToRecipient('recipient', topology);
    expect(result.path).toEqual(['relay-1']);
    expect(result.reason).toBe('single-relay');
  });

  it('should return recipient-is-self when sending to self', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const result = selector.selectPathToRecipient('self', topology);
    expect(result.path).toEqual([]);
    expect(result.reason).toBe('recipient-is-self');
  });

  it('should return no-path when topology is empty', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const result = selector.selectPathToRecipient('recipient', topology);
    expect(result.path).toEqual([]);
    expect(result.reason).toBe('no-path');
  });

  it('should return no-path when no online relays available', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Recipient is offline
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now - 20000 }));
    // Relay is offline too
    topology.addPeer(makePeer('relay-offline', { roles: ['client', 'relay'], lastSeen: now - 20000 }));

    const result = selector.selectPathToRecipient('recipient', topology);
    expect(result.path).toEqual([]);
    expect(result.reason).toBe('no-path');
  });

  it('should not include self in relay path', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now - 20000 }));
    topology.addPeer(makePeer('self', { roles: ['client', 'relay'], lastSeen: now }));
    topology.addPeer(makePeer('other-relay', { roles: ['client', 'relay'], lastSeen: now }));

    const result = selector.selectPathToRecipient('recipient', topology);
    expect(result.path).not.toContain('self');
    expect(result.path).toEqual(['other-relay']);
  });

  it('should not include recipient in relay path', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Recipient is a relay but offline as direct target
    topology.addPeer(makePeer('recipient', { roles: ['client', 'relay'], lastSeen: now - 20000 }));
    topology.addPeer(makePeer('other-relay', { roles: ['client', 'relay'], lastSeen: now }));

    const result = selector.selectPathToRecipient('recipient', topology);
    expect(result.path).not.toContain('recipient');
    expect(result.path).toEqual(['other-relay']);
  });

  it('MAX_RELAY_DEPTH should be 4', () => {
    expect(MAX_RELAY_DEPTH).toBe(4);
  });
});

describe('RelaySelector - Alternate relay selection (Story 5.2)', () => {
  it('should exclude failed relays when selecting alternate', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    topology.addPeer(makePeer('relay-failed', { roles: ['client', 'relay'], lastSeen: now }));
    topology.addPeer(makePeer('relay-working', { roles: ['client', 'relay'], lastSeen: now }));
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now }));

    const failedRelays = new Set(['relay-failed']);
    const result = selector.selectAlternateRelay('recipient', topology, failedRelays);

    expect(result.relayId).toBe('relay-working');
    expect(result.reason).toBe('best-available');
  });

  it('should return no-relays-available when all relays are failed', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    topology.addPeer(makePeer('relay-1', { roles: ['client', 'relay'], lastSeen: now }));
    topology.addPeer(makePeer('relay-2', { roles: ['client', 'relay'], lastSeen: now }));
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now - 20000 }));

    const failedRelays = new Set(['relay-1', 'relay-2']);
    const result = selector.selectAlternateRelay('recipient', topology, failedRelays);

    expect(result.relayId).toBeNull();
    expect(result.reason).toBe('no-relays-available');
  });

  it('should use selectBestRelay with excludeRelays parameter', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    // Best relay (most recent) is excluded
    topology.addPeer(makePeer('relay-best', { roles: ['client', 'relay'], lastSeen: now }));
    // Fallback relay
    topology.addPeer(makePeer('relay-fallback', { roles: ['client', 'relay'], lastSeen: now - 500 }));
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now }));

    const excludeRelays = new Set(['relay-best']);
    const result = selector.selectBestRelay('recipient', topology, excludeRelays);

    expect(result.relayId).toBe('relay-fallback');
    expect(result.reason).toBe('best-available');
  });

  it('should return direct-fallback when all relays excluded but recipient online', () => {
    const selector = new RelaySelector({ selfNodeId: 'self' });
    const topology = new NetworkTopology(3000);

    const now = Date.now();
    topology.addPeer(makePeer('relay-only', { roles: ['client', 'relay'], lastSeen: now }));
    topology.addPeer(makePeer('recipient', { roles: ['client'], lastSeen: now }));

    const excludeRelays = new Set(['relay-only']);
    const result = selector.selectBestRelay('recipient', topology, excludeRelays);

    expect(result.relayId).toBeNull();
    expect(result.reason).toBe('direct-fallback');
  });
});
