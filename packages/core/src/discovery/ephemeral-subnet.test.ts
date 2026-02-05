import { beforeEach, describe, expect, it, vi } from 'vitest';
import { EphemeralSubnetManager } from './ephemeral-subnet.js';
import type { SubnetEvents, SubnetInfo } from './ephemeral-subnet.js';

describe('EphemeralSubnetManager', () => {
  let manager: EphemeralSubnetManager;
  let events: SubnetEvents;
  let formedSubnets: SubnetInfo[];
  let dissolvedSubnets: Array<{ subnetId: string; reason: string }>;
  let joinedEvents: Array<{ subnetId: string; nodeId: string }>;
  let leftEvents: Array<{ subnetId: string; nodeId: string }>;

  beforeEach(() => {
    formedSubnets = [];
    dissolvedSubnets = [];
    joinedEvents = [];
    leftEvents = [];

    events = {
      onSubnetFormed: (subnet) => formedSubnets.push(subnet),
      onSubnetDissolved: (subnetId, reason) => dissolvedSubnets.push({ subnetId, reason }),
      onNodeJoinedSubnet: (subnetId, nodeId) => joinedEvents.push({ subnetId, nodeId }),
      onNodeLeftSubnet: (subnetId, nodeId) => leftEvents.push({ subnetId, nodeId }),
    };

    manager = new EphemeralSubnetManager('self-node', events, {
      minEdgeWeight: 2,
      minSubnetSize: 3,
      maxSubnetSize: 5,
      inactivityTimeoutMs: 1000,
      evaluationIntervalMs: 100,
      edgeDecayTimeMs: 5000,
    });
  });

  describe('recordCommunication', () => {
    it('should track communication between nodes', () => {
      manager.recordCommunication('node-a', 'node-b');
      manager.recordCommunication('node-a', 'node-b');

      const stats = manager.getStats();
      expect(stats.communicationEdges).toBe(1);
    });

    it('should treat edges as bidirectional', () => {
      manager.recordCommunication('node-a', 'node-b');
      manager.recordCommunication('node-b', 'node-a');

      const stats = manager.getStats();
      expect(stats.communicationEdges).toBe(1);
    });
  });

  describe('subnet formation', () => {
    it('should form subnet when cluster meets criteria', () => {
      // Create a cluster of 3 nodes with sufficient communication
      for (let i = 0; i < 3; i++) {
        manager.recordCommunication('node-a', 'node-b');
        manager.recordCommunication('node-b', 'node-c');
        manager.recordCommunication('node-a', 'node-c');
      }

      // Trigger evaluation manually
      manager.forceEvaluate();

      expect(formedSubnets.length).toBe(1);
      expect(formedSubnets[0].members.size).toBe(3);
      expect(joinedEvents.length).toBe(3);
    });

    it('should not form subnet below minimum size', () => {
      // Only 2 nodes communicating
      for (let i = 0; i < 3; i++) {
        manager.recordCommunication('node-a', 'node-b');
      }

      manager.forceEvaluate();

      expect(formedSubnets.length).toBe(0);
    });

    it('should not form subnet below minimum edge weight', () => {
      // 3 nodes but only 1 message each - below threshold
      manager.recordCommunication('node-a', 'node-b');
      manager.recordCommunication('node-b', 'node-c');
      manager.recordCommunication('node-a', 'node-c');

      manager.forceEvaluate();

      expect(formedSubnets.length).toBe(0);
    });

    it('should respect maximum subnet size', () => {
      // Create 7 nodes all communicating
      const nodes = ['a', 'b', 'c', 'd', 'e', 'f', 'g'].map((n) => `node-${n}`);
      for (let round = 0; round < 3; round++) {
        for (let i = 0; i < nodes.length; i++) {
          for (let j = i + 1; j < nodes.length; j++) {
            manager.recordCommunication(nodes[i], nodes[j]);
          }
        }
      }

      manager.forceEvaluate();

      // Should form but be limited to maxSubnetSize (5)
      expect(formedSubnets.length).toBeGreaterThanOrEqual(1);
      expect(formedSubnets[0].members.size).toBeLessThanOrEqual(5);
    });
  });

  describe('subnet queries', () => {
    beforeEach(() => {
      // Form a subnet first
      for (let i = 0; i < 3; i++) {
        manager.recordCommunication('node-a', 'node-b');
        manager.recordCommunication('node-b', 'node-c');
        manager.recordCommunication('node-a', 'node-c');
      }
      manager.forceEvaluate();
    });

    it('should return node subnet', () => {
      const subnet = manager.getNodeSubnet('node-a');
      expect(subnet).not.toBeNull();
      expect(subnet?.members.has('node-a')).toBe(true);
    });

    it('should return null for node not in subnet', () => {
      const subnet = manager.getNodeSubnet('node-x');
      expect(subnet).toBeNull();
    });

    it('should detect if two nodes are in same subnet', () => {
      expect(manager.areInSameSubnet('node-a', 'node-b')).toBe(true);
      expect(manager.areInSameSubnet('node-a', 'node-x')).toBe(false);
    });

    it('should return all subnets', () => {
      const subnets = manager.getAllSubnets();
      expect(subnets.length).toBe(1);
    });
  });

  describe('subnet dissolution', () => {
    beforeEach(() => {
      // Form a subnet first
      for (let i = 0; i < 3; i++) {
        manager.recordCommunication('node-a', 'node-b');
        manager.recordCommunication('node-b', 'node-c');
        manager.recordCommunication('node-a', 'node-c');
      }
      manager.forceEvaluate();
    });

    it('should dissolve subnet when node removed makes it too small', () => {
      manager.removeNode('node-a');

      expect(dissolvedSubnets.length).toBe(1);
      expect(dissolvedSubnets[0].reason).toBe('insufficient-members');
      expect(leftEvents.some((e) => e.nodeId === 'node-a')).toBe(true);
    });

    it('should dissolve subnet when inactive', async () => {
      // Wait for inactivity timeout (1000ms in config)
      await new Promise((resolve) => setTimeout(resolve, 1100));

      manager.forceEvaluate();

      expect(dissolvedSubnets.length).toBe(1);
      expect(dissolvedSubnets[0].reason).toBe('inactive');
    });

    it('should update activity when members communicate', async () => {
      // Wait a bit then communicate
      await new Promise((resolve) => setTimeout(resolve, 500));

      manager.recordCommunication('node-a', 'node-b');

      // Wait a bit more
      await new Promise((resolve) => setTimeout(resolve, 600));

      manager.forceEvaluate();

      // Subnet should still exist because of recent activity
      expect(dissolvedSubnets.length).toBe(0);
    });
  });

  describe('getStats', () => {
    it('should return accurate statistics', () => {
      // Form a subnet
      for (let i = 0; i < 3; i++) {
        manager.recordCommunication('node-a', 'node-b');
        manager.recordCommunication('node-b', 'node-c');
        manager.recordCommunication('node-a', 'node-c');
      }
      manager.forceEvaluate();

      const stats = manager.getStats();

      expect(stats.totalSubnets).toBe(1);
      expect(stats.totalNodesInSubnets).toBe(3);
      expect(stats.averageSubnetSize).toBe(3);
      expect(stats.communicationEdges).toBe(3);
    });

    it('should return zero stats when no subnets', () => {
      const stats = manager.getStats();

      expect(stats.totalSubnets).toBe(0);
      expect(stats.totalNodesInSubnets).toBe(0);
      expect(stats.averageSubnetSize).toBe(0);
    });
  });

  describe('start/stop', () => {
    it('should start and stop evaluation interval', () => {
      vi.useFakeTimers();

      manager.start();

      // Fast-forward time
      vi.advanceTimersByTime(150);

      manager.stop();

      vi.useRealTimers();
    });
  });
});
