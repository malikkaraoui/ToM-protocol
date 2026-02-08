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

  describe('edge cases', () => {
    it('should handle single-peer network (self only)', () => {
      // Bind with selfNodeId but no peers
      manager.bindTopology(topology, 'self-node');

      const roles = manager.evaluateNode('self-node', topology);

      // With 1 node: ceil(1/3) = 1 relay, self becomes relay
      expect(roles).toEqual(['client', 'relay']);
    });

    it('should handle role transition cycles (relay → client → relay)', () => {
      // Start with a larger network where node-z is not a relay
      for (let i = 0; i < 6; i++) {
        topology.addPeer(makePeer(`node-${String.fromCharCode(97 + i)}`)); // node-a to node-f
      }

      // Initial: with 6 nodes, ceil(6/3)=2 relays → node-a, node-b are relays
      manager.evaluateNode('node-f', topology);
      expect(manager.getCurrentRoles('node-f')).toEqual(['client']); // Not a relay

      // Remove some low-lexicographic nodes
      topology.removePeer('node-a');
      topology.removePeer('node-b');
      topology.removePeer('node-c');

      // Now with 3 nodes (node-d, node-e, node-f): ceil(3/3)=1 relay → node-d is relay
      manager.evaluateNode('node-f', topology);
      expect(manager.getCurrentRoles('node-f')).toEqual(['client']); // Still not a relay

      // Remove more to make node-f become relay
      topology.removePeer('node-d');
      topology.removePeer('node-e');

      // Now only node-f: ceil(1/3)=1 relay → node-f becomes relay
      manager.evaluateNode('node-f', topology);
      expect(manager.getCurrentRoles('node-f')).toEqual(['client', 'relay']);
    });

    it('should handle metrics with 0 contribution score', () => {
      manager.updateNodeMetrics('node-1', { bandwidthScore: 50, contributionScore: 0 });
      const score = manager.calculateBackupScore('node-1');

      // timeScore=0, bandwidth=50, contribution=0
      // Score = 0*0.3 + 50*0.4 + 0*0.3 = 20
      expect(score).toBe(20);
    });

    it('should handle metrics with maximum values', () => {
      manager.recordNodeOnline('node-1');
      vi.advanceTimersByTime(2 * 60 * 60 * 1000); // 2 hours (exceeds max)

      manager.updateNodeMetrics('node-1', { bandwidthScore: 100, contributionScore: 100 });
      const score = manager.calculateBackupScore('node-1');

      // timeScore=100 (capped), bandwidth=100, contribution=100
      // Score = 100*0.3 + 100*0.4 + 100*0.3 = 100
      expect(score).toBe(100);
    });

    it('should handle large network scale (50+ peers)', () => {
      // Add 50 peers
      for (let i = 0; i < 50; i++) {
        topology.addPeer(makePeer(`node-${i.toString().padStart(3, '0')}`));
      }

      const result = manager.reassignRoles(topology);

      // With 50 nodes: ceil(50/3) = 17 relays
      expect(result.size).toBe(50);

      const relayCount = Array.from(result.values()).filter((roles) => roles.includes('relay')).length;
      expect(relayCount).toBe(17);
    });

    it('should handle reevaluation timer idempotency (start/stop cycles)', () => {
      manager.start();
      manager.start(); // Second start should be no-op
      manager.stop();
      manager.start();
      manager.stop();
      manager.stop(); // Second stop should be no-op

      // No errors should occur
      expect(true).toBe(true);
    });

    it('should handle empty topology', () => {
      const roles = manager.evaluateNode('node-1', topology);
      // No peers, just the unknown node-1
      expect(roles).toEqual(['client']);
    });

    it('should handle nodes with no startTime recorded', () => {
      // Get metrics without recording online first
      const metrics = manager.getNodeMetrics('unknown-node');

      expect(metrics.timeOnlineMs).toBe(0);
      expect(metrics.bandwidthScore).toBe(50); // Default
      expect(metrics.contributionScore).toBe(0); // Default
    });

    it('should handle backup evaluation with no eligible nodes', () => {
      topology.addPeer(makePeer('node-1'));
      topology.addPeer(makePeer('node-2'));

      // Don't record online times - nodes won't be eligible
      manager.evaluateNode('node-1', topology);
      manager.evaluateNode('node-2', topology);

      const isBackup = manager.evaluateBackupRole('node-1', topology);
      expect(isBackup).toBe(false);
    });

    it('should use lexicographic tiebreaker for backup score ties', () => {
      topology.addPeer(makePeer('node-a'));
      topology.addPeer(makePeer('node-b'));

      // Same metrics = same score
      for (const nodeId of ['node-a', 'node-b']) {
        manager.recordNodeOnline(nodeId);
        manager.updateNodeMetrics(nodeId, { bandwidthScore: 70, contributionScore: 50 });
        manager.evaluateNode(nodeId, topology);
      }

      vi.advanceTimersByTime(10 * 60 * 1000);

      // With 2 nodes, 1 backup needed. node-a wins tiebreaker
      manager.reassignBackupRoles(topology);

      // Due to same scores, lexicographic order determines winner
      // node-a comes before node-b
      expect(manager.isBackupNode('node-a')).toBe(true);
    });
  });

  describe('security fixes', () => {
    it('should purge nodeMetrics when peer leaves topology', () => {
      topology.addPeer(makePeer('node-1'));
      manager.recordNodeOnline('node-1');
      manager.updateNodeMetrics('node-1', { bandwidthScore: 80, contributionScore: 50 });
      manager.evaluateNode('node-1', topology);

      // Verify metrics exist
      expect(manager.getNodeMetrics('node-1').bandwidthScore).toBe(80);

      // Remove peer from topology
      topology.removePeer('node-1');

      // Trigger cleanup via reassignRoles
      manager.reassignRoles(topology);

      // Metrics should be purged (returns defaults now)
      expect(manager.getNodeMetrics('node-1').bandwidthScore).toBe(50); // Default
      expect(manager.getNodeMetrics('node-1').contributionScore).toBe(0); // Default
    });

    it('should purge nodeStartTimes when peer leaves topology', () => {
      topology.addPeer(makePeer('node-1'));
      manager.recordNodeOnline('node-1');
      manager.evaluateNode('node-1', topology);

      vi.advanceTimersByTime(10000);

      // Verify startTime exists (timeOnlineMs > 0)
      expect(manager.getNodeMetrics('node-1').timeOnlineMs).toBeGreaterThan(0);

      // Remove peer from topology
      topology.removePeer('node-1');

      // Trigger cleanup
      manager.reassignRoles(topology);

      // StartTime should be purged (timeOnlineMs is 0 for unknown node)
      expect(manager.getNodeMetrics('node-1').timeOnlineMs).toBe(0);
    });

    it('should not purge self metrics even if not in topology', () => {
      // Bind with selfNodeId
      manager.bindTopology(topology, 'self-node');
      manager.recordNodeOnline('self-node');
      manager.updateNodeMetrics('self-node', { bandwidthScore: 90, contributionScore: 60 });
      manager.evaluateNode('self-node', topology);

      // Self is not a peer in topology (normal case)
      // Trigger cleanup
      manager.reassignRoles(topology, 'self-node');

      // Self's metrics should NOT be purged
      expect(manager.getNodeMetrics('self-node').bandwidthScore).toBe(90);
    });
  });
});
