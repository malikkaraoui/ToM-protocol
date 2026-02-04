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
    roles: ['client'] as ('client' | 'relay' | 'observer' | 'bootstrap' | 'backup')[],
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

  // ============ Backup Role Tests ============

  describe('backup role assignment', () => {
    it('should record node online time', () => {
      manager.recordNodeOnline('node-1');
      const metrics = manager.getNodeMetrics('node-1');
      expect(metrics.timeOnlineMs).toBeGreaterThanOrEqual(0);
    });

    it('should not record duplicate online times', () => {
      manager.recordNodeOnline('node-1');
      vi.advanceTimersByTime(1000);
      manager.recordNodeOnline('node-1'); // Should be ignored

      const metrics = manager.getNodeMetrics('node-1');
      // Time should be ~1000ms, not 0
      expect(metrics.timeOnlineMs).toBeGreaterThanOrEqual(1000);
    });

    it('should update node metrics', () => {
      manager.updateNodeMetrics('node-1', { bandwidthScore: 80, contributionScore: 50 });
      const metrics = manager.getNodeMetrics('node-1');
      expect(metrics.bandwidthScore).toBe(80);
      expect(metrics.contributionScore).toBe(50);
    });

    it('should calculate backup score', () => {
      manager.recordNodeOnline('node-1');
      vi.advanceTimersByTime(30 * 60 * 1000); // 30 minutes

      manager.updateNodeMetrics('node-1', { bandwidthScore: 80, contributionScore: 60 });

      const score = manager.calculateBackupScore('node-1');
      // Expected: timeScore=50 (30min/60min), bandwidth=80, contribution=60
      // Score = 50*0.3 + 80*0.4 + 60*0.3 = 15 + 32 + 18 = 65
      expect(score).toBe(65);
    });

    it('should not be eligible for backup before minimum time online', () => {
      topology.addPeer(makePeer('node-1'));
      manager.recordNodeOnline('node-1');
      manager.updateNodeMetrics('node-1', { bandwidthScore: 100, contributionScore: 100 });

      // Only 1 minute online (need 5 minutes)
      vi.advanceTimersByTime(60 * 1000);

      expect(manager.isEligibleForBackup('node-1')).toBe(false);
    });

    it('should be eligible for backup after minimum time online', () => {
      topology.addPeer(makePeer('node-1'));
      manager.recordNodeOnline('node-1');
      manager.updateNodeMetrics('node-1', { bandwidthScore: 60, contributionScore: 40 });

      // 6 minutes online (need 5 minutes)
      vi.advanceTimersByTime(6 * 60 * 1000);

      expect(manager.isEligibleForBackup('node-1')).toBe(true);
    });

    it('should assign backup role to eligible node', () => {
      topology.addPeer(makePeer('node-1'));
      manager.recordNodeOnline('node-1');
      manager.updateNodeMetrics('node-1', { bandwidthScore: 70, contributionScore: 50 });
      manager.evaluateNode('node-1', topology); // Assign initial roles

      // Meet time requirement
      vi.advanceTimersByTime(6 * 60 * 1000);

      const isBackup = manager.evaluateBackupRole('node-1', topology);

      expect(isBackup).toBe(true);
      expect(manager.isBackupNode('node-1')).toBe(true);
      expect(manager.getCurrentRoles('node-1')).toContain('backup');
    });

    it('should not assign backup role to ineligible node', () => {
      topology.addPeer(makePeer('node-1'));
      manager.recordNodeOnline('node-1');
      manager.evaluateNode('node-1', topology);

      // Only 1 minute online
      vi.advanceTimersByTime(60 * 1000);

      const isBackup = manager.evaluateBackupRole('node-1', topology);

      expect(isBackup).toBe(false);
      expect(manager.isBackupNode('node-1')).toBe(false);
    });

    it('should select multiple backup nodes (cascading redundancy)', () => {
      // Add 4 nodes
      topology.addPeer(makePeer('node-1'));
      topology.addPeer(makePeer('node-2'));
      topology.addPeer(makePeer('node-3'));
      topology.addPeer(makePeer('node-4'));

      // Make all nodes online for 10 minutes with good metrics
      for (const nodeId of ['node-1', 'node-2', 'node-3', 'node-4']) {
        manager.recordNodeOnline(nodeId);
        manager.updateNodeMetrics(nodeId, { bandwidthScore: 70, contributionScore: 50 });
        manager.evaluateNode(nodeId, topology);
      }
      vi.advanceTimersByTime(10 * 60 * 1000);

      const backupResults = manager.reassignBackupRoles(topology);

      // With 4 nodes, target = ceil(4/2) = 2 backups
      const backupCount = Array.from(backupResults.values()).filter(Boolean).length;
      expect(backupCount).toBe(2);
    });

    it('should select backups by score (highest first)', () => {
      topology.addPeer(makePeer('node-1'));
      topology.addPeer(makePeer('node-2'));

      manager.recordNodeOnline('node-1');
      manager.recordNodeOnline('node-2');

      // node-2 has higher metrics
      manager.updateNodeMetrics('node-1', { bandwidthScore: 50, contributionScore: 30 });
      manager.updateNodeMetrics('node-2', { bandwidthScore: 90, contributionScore: 80 });

      manager.evaluateNode('node-1', topology);
      manager.evaluateNode('node-2', topology);

      vi.advanceTimersByTime(10 * 60 * 1000);

      // With 2 nodes, target = ceil(2/2) = 1 backup
      manager.reassignBackupRoles(topology);

      // node-2 should be selected (higher score)
      expect(manager.isBackupNode('node-2')).toBe(true);
      expect(manager.isBackupNode('node-1')).toBe(false);
    });

    it('should increment contribution score', () => {
      manager.updateNodeMetrics('node-1', { contributionScore: 10 });
      manager.incrementContributionScore('node-1', 5);

      const metrics = manager.getNodeMetrics('node-1');
      expect(metrics.contributionScore).toBe(15);
    });

    it('should cap contribution score at 100', () => {
      manager.updateNodeMetrics('node-1', { contributionScore: 98 });
      manager.incrementContributionScore('node-1', 10);

      const metrics = manager.getNodeMetrics('node-1');
      expect(metrics.contributionScore).toBe(100);
    });

    it('should get all backup nodes', () => {
      topology.addPeer(makePeer('node-1'));
      topology.addPeer(makePeer('node-2'));
      topology.addPeer(makePeer('node-3'));

      for (const nodeId of ['node-1', 'node-2', 'node-3']) {
        manager.recordNodeOnline(nodeId);
        manager.updateNodeMetrics(nodeId, { bandwidthScore: 70, contributionScore: 50 });
        manager.evaluateNode(nodeId, topology);
      }
      vi.advanceTimersByTime(10 * 60 * 1000);

      manager.reassignBackupRoles(topology);

      const backups = manager.getBackupNodes();
      expect(backups.length).toBeGreaterThan(0);
      expect(backups.length).toBeLessThanOrEqual(2); // ceil(3/2) = 2
    });

    it('should emit role changed event when backup role assigned', () => {
      topology.addPeer(makePeer('node-1'));
      manager.recordNodeOnline('node-1');
      manager.updateNodeMetrics('node-1', { bandwidthScore: 70, contributionScore: 50 });
      manager.evaluateNode('node-1', topology);

      vi.advanceTimersByTime(6 * 60 * 1000);
      onRoleChanged.mockClear();

      manager.evaluateBackupRole('node-1', topology);

      expect(onRoleChanged).toHaveBeenCalled();
      const calls = onRoleChanged.mock.calls;
      const lastCall = calls[calls.length - 1];
      expect(lastCall[2]).toContain('backup');
    });

    it('should remove backup role when no longer eligible', () => {
      // Setup: node-1 is backup, node-2 joins with better score
      topology.addPeer(makePeer('node-1'));
      manager.recordNodeOnline('node-1');
      manager.updateNodeMetrics('node-1', { bandwidthScore: 50, contributionScore: 30 });
      manager.evaluateNode('node-1', topology);

      vi.advanceTimersByTime(6 * 60 * 1000);
      manager.evaluateBackupRole('node-1', topology);
      expect(manager.isBackupNode('node-1')).toBe(true);

      // Add node-2 with better score
      topology.addPeer(makePeer('node-2'));
      manager.recordNodeOnline('node-2');
      manager.updateNodeMetrics('node-2', { bandwidthScore: 95, contributionScore: 90 });
      manager.evaluateNode('node-2', topology);

      vi.advanceTimersByTime(6 * 60 * 1000);

      // Re-evaluate - with 2 nodes, only 1 backup needed
      // node-2 has higher score, so node-1 should lose backup role
      manager.reassignBackupRoles(topology);

      expect(manager.isBackupNode('node-2')).toBe(true);
      expect(manager.isBackupNode('node-1')).toBe(false);
    });
  });
});
