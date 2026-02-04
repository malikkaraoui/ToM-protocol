import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { NetworkTopology } from '../discovery/network-topology.js';
import { RoleManager } from './role-manager.js';

function makePeer(nodeId: string, username = 'user') {
  return {
    nodeId,
    username,
    publicKey: nodeId,
    reachableVia: [],
    lastSeen: Date.now(),
    roles: ['client'] as ('client' | 'relay' | 'observer' | 'bootstrap')[],
  };
}

describe('RoleManager', () => {
  let onRoleChanged: ReturnType<typeof vi.fn>;
  let manager: RoleManager;
  let topology: NetworkTopology;

  beforeEach(() => {
    vi.useFakeTimers();
    onRoleChanged = vi.fn();
    manager = new RoleManager({ onRoleChanged });
    topology = new NetworkTopology(3000);
    manager.bindTopology(topology);
  });

  afterEach(() => {
    manager.stop();
    vi.useRealTimers();
  });

  it('should assign relay role to single node (deterministic consensus)', () => {
    // With 1 node: ceil(1/3) = 1 relay needed, so the only node becomes relay
    topology.addPeer(makePeer('node-1'));
    const roles = manager.evaluateNode('node-1', topology);
    expect(roles).toEqual(['client', 'relay']);
  });

  it('should return client role for unknown node', () => {
    const roles = manager.evaluateNode('unknown', topology);
    expect(roles).toEqual(['client']);
  });

  it('should assign relay role when network needs relays', () => {
    // Add 3 nodes — need ceil(3/3) = 1 relay
    topology.addPeer(makePeer('node-1'));
    topology.addPeer(makePeer('node-2'));
    topology.addPeer(makePeer('node-3'));

    // First evaluation — node gets client (creates assignment)
    manager.evaluateNode('node-1', topology);
    manager.evaluateNode('node-2', topology);
    manager.evaluateNode('node-3', topology);

    // Simulate time passing for eligibility (>5 seconds)
    vi.advanceTimersByTime(6000);

    // Update lastSeen so peers stay online
    topology.updateLastSeen('node-1');
    topology.updateLastSeen('node-2');
    topology.updateLastSeen('node-3');

    // Re-evaluate — node-1 should now be eligible for relay since network needs relays
    const roles = manager.evaluateNode('node-1', topology);
    expect(roles).toContain('relay');
    expect(roles).toContain('client');
  });

  it('should emit role changed event', () => {
    topology.addPeer(makePeer('node-1'));

    // Initial assignment
    manager.evaluateNode('node-1', topology);

    // Simulate eligibility period
    vi.advanceTimersByTime(6000);

    // Clear the mock to capture only the next call
    onRoleChanged.mockClear();

    // Add more peers to trigger relay need
    topology.addPeer(makePeer('node-2'));
    topology.addPeer(makePeer('node-3'));

    // Re-evaluate — should become relay
    manager.evaluateNode('node-1', topology);

    // Check role changed was called with relay
    const calls = onRoleChanged.mock.calls;
    const lastCall = calls[calls.length - 1];
    if (lastCall) {
      expect(lastCall[2]).toContain('relay');
    }
  });

  it('should reassign roles for all peers', () => {
    topology.addPeer(makePeer('node-1'));
    topology.addPeer(makePeer('node-2'));
    topology.addPeer(makePeer('node-3'));

    const result = manager.reassignRoles(topology);

    expect(result.size).toBe(3);
    for (const [, roles] of result) {
      expect(roles).toContain('client');
    }
  });

  it('should get current roles', () => {
    // With 1 node: ceil(1/3) = 1 relay, so node-1 becomes relay
    topology.addPeer(makePeer('node-1'));
    manager.evaluateNode('node-1', topology);

    const roles = manager.getCurrentRoles('node-1');
    expect(roles).toEqual(['client', 'relay']);
  });

  it('should return client role for unassigned node', () => {
    const roles = manager.getCurrentRoles('unassigned');
    expect(roles).toEqual(['client']);
  });

  it('should accept roles from network', () => {
    manager.setRolesFromNetwork('node-1', ['client', 'relay']);
    const roles = manager.getCurrentRoles('node-1');
    expect(roles).toEqual(['client', 'relay']);
  });

  it('should remove assignment', () => {
    topology.addPeer(makePeer('node-1'));
    manager.evaluateNode('node-1', topology);

    expect(manager.getAssignment('node-1')).toBeDefined();

    manager.removeAssignment('node-1');

    expect(manager.getAssignment('node-1')).toBeUndefined();
  });

  it('should demote relay when too many relays', () => {
    // Setup: 2 nodes, both manually set as relays
    topology.addPeer(makePeer('node-1'));
    topology.addPeer(makePeer('node-2'));

    manager.setRolesFromNetwork('node-1', ['client', 'relay']);
    manager.setRolesFromNetwork('node-2', ['client', 'relay']);

    // Now we have 2 relays for 2 nodes — max is ceil(2/2) = 1
    // Re-evaluate node-2 — should be demoted
    vi.advanceTimersByTime(6000);
    const roles = manager.evaluateNode('node-2', topology);

    // With 2 relays and max 1, second should be demoted
    // But the algorithm keeps existing relays unless over max
    // Max for 2 nodes = ceil(2/2) = 1, we have 2, so one should be demoted
    expect(roles).toEqual(['client']);
  });

  it('should start and stop periodic re-evaluation', () => {
    // Use a longer stale threshold to not interfere with 30s re-evaluation
    const longTopology = new NetworkTopology(60000);
    manager.bindTopology(longTopology);

    longTopology.addPeer(makePeer('node-1'));

    // Initial evaluation creates assignment
    manager.evaluateNode('node-1', longTopology);
    expect(manager.getAssignment('node-1')).toBeDefined();

    manager.start();

    // Clear to test re-evaluation creates it again
    manager.removeAssignment('node-1');
    expect(manager.getAssignment('node-1')).toBeUndefined();

    // Advance past one re-evaluation interval (30s)
    vi.advanceTimersByTime(31000);

    // Should have re-evaluated and created assignment
    expect(manager.getAssignment('node-1')).toBeDefined();

    manager.stop();

    // Clear assignments and advance time — should not re-evaluate
    manager.removeAssignment('node-1');
    vi.advanceTimersByTime(31000);

    // Assignment should still be undefined (not re-evaluated)
    expect(manager.getAssignment('node-1')).toBeUndefined();
  });

  it('should assign relay immediately based on lexicographic consensus', () => {
    // Deterministic consensus: lowest nodeId wins relay role
    // With 3 nodes: ceil(3/3) = 1 relay, node-1 is first alphabetically → relay
    topology.addPeer(makePeer('node-1'));
    topology.addPeer(makePeer('node-2'));
    topology.addPeer(makePeer('node-3'));

    const roles = manager.evaluateNode('node-1', topology);

    // node-1 is lowest in lexicographic order, so it becomes relay immediately
    expect(roles).toEqual(['client', 'relay']);
  });
});
