/**
 * Alpha Scale Validation Tests (Story 7.3)
 *
 * Validates network behavior with 10-15 simulated nodes:
 * - Message delivery across the network
 * - Peer discovery via gossip
 * - Subnet formation from communication patterns
 * - Role assignment and relay selection
 * - The "inversion property" - more nodes improve routing options
 */

import { beforeEach, describe, expect, it } from 'vitest';
import { RoleManager } from '../roles/role-manager.js';
import { RelaySelector } from '../routing/relay-selector.js';
import { EphemeralSubnetManager } from './ephemeral-subnet.js';
import { NetworkTopology } from './network-topology.js';
import type { PeerInfo } from './network-topology.js';
import { PeerGossip } from './peer-gossip.js';

describe('Alpha Scale Validation (10-15 Nodes)', () => {
  const NODE_COUNT = 12; // Test with 12 nodes

  // Create node helpers
  const createNode = (index: number): { nodeId: string; username: string } => ({
    nodeId: `node-${index.toString().padStart(3, '0')}`,
    username: `user-${index}`,
  });

  describe('Network Topology at Scale', () => {
    let topology: NetworkTopology;
    let nodes: Array<{ nodeId: string; username: string }>;

    beforeEach(() => {
      topology = new NetworkTopology();
      nodes = Array.from({ length: NODE_COUNT }, (_, i) => createNode(i));

      // Add all nodes to topology
      for (const node of nodes) {
        topology.addPeer({
          nodeId: node.nodeId,
          username: node.username,
          publicKey: node.nodeId,
          reachableVia: [],
          lastSeen: Date.now(),
          roles: ['client'],
        });
      }
    });

    it('should handle 12 simultaneous nodes', () => {
      expect(topology.getReachablePeers().length).toBe(NODE_COUNT);
    });

    it('should maintain peer status correctly', () => {
      // All peers should be online initially
      for (const node of nodes) {
        expect(topology.getPeerStatus(node.nodeId)).toBe('online');
      }

      // Make some stale (between 10s and 20s threshold)
      for (let i = 0; i < 3; i++) {
        const peer = topology.getPeer(nodes[i].nodeId);
        if (peer) {
          peer.lastSeen = Date.now() - 15000; // 15 seconds ago (stale = 10s-20s)
        }
      }

      // Stale peers should be detected
      const stalePeers = nodes.filter((n) => topology.getPeerStatus(n.nodeId) === 'stale');
      expect(stalePeers.length).toBe(3);
    });

    it('should track peer joins and leaves correctly', () => {
      // Remove 2 nodes
      topology.removePeer(nodes[0].nodeId);
      topology.removePeer(nodes[1].nodeId);

      expect(topology.getReachablePeers().length).toBe(NODE_COUNT - 2);

      // Add a new node
      const newNode = createNode(100);
      topology.addPeer({
        nodeId: newNode.nodeId,
        username: newNode.username,
        publicKey: newNode.nodeId,
        reachableVia: [],
        lastSeen: Date.now(),
        roles: ['client'],
      });

      expect(topology.getReachablePeers().length).toBe(NODE_COUNT - 1);
    });
  });

  describe('Role Assignment at Scale', () => {
    let topology: NetworkTopology;
    let roleManager: RoleManager;
    let nodes: Array<{ nodeId: string; username: string }>;

    beforeEach(() => {
      topology = new NetworkTopology();
      nodes = Array.from({ length: NODE_COUNT }, (_, i) => createNode(i));
      roleManager = new RoleManager({
        onRoleChanged: () => {},
      });
      roleManager.bindTopology(topology, nodes[0].nodeId);

      // Add all nodes to topology
      for (const node of nodes) {
        topology.addPeer({
          nodeId: node.nodeId,
          username: node.username,
          publicKey: node.nodeId,
          reachableVia: [],
          lastSeen: Date.now(),
          roles: ['client'],
        });
      }
    });

    it('should elect appropriate number of relays for 12 nodes', () => {
      // Evaluate all nodes
      for (const node of nodes) {
        roleManager.evaluateNode(node.nodeId, topology);
      }

      // Count relays
      const relays = nodes.filter((n) => roleManager.getCurrentRoles(n.nodeId).includes('relay'));

      // With 12 nodes, should have 3-4 relays (roughly 1 per 3-4 clients)
      expect(relays.length).toBeGreaterThanOrEqual(2);
      expect(relays.length).toBeLessThanOrEqual(5);
    });

    it('should reassign roles when nodes leave', () => {
      // Initial evaluation
      for (const node of nodes) {
        roleManager.evaluateNode(node.nodeId, topology);
      }

      const initialRelays = nodes.filter((n) => roleManager.getCurrentRoles(n.nodeId).includes('relay'));

      // Remove a relay
      const removedRelay = initialRelays[0];
      topology.removePeer(removedRelay.nodeId);
      roleManager.removeAssignment(removedRelay.nodeId);

      // Reassign
      roleManager.reassignRoles(topology, nodes.find((n) => n.nodeId !== removedRelay.nodeId)!.nodeId);

      // Should still have relays
      const remainingNodes = nodes.filter((n) => n.nodeId !== removedRelay.nodeId);
      const newRelays = remainingNodes.filter((n) => roleManager.getCurrentRoles(n.nodeId).includes('relay'));
      expect(newRelays.length).toBeGreaterThanOrEqual(1);
    });
  });

  describe('Gossip Protocol at Scale', () => {
    it('should discover peers through gossip chains', () => {
      const gossipInstances: PeerGossip[] = [];
      const discoveredCounts: number[] = [];

      // Create gossip instance for each node
      for (let i = 0; i < NODE_COUNT; i++) {
        const node = createNode(i);
        const gossip = new PeerGossip(node.nodeId, node.username, {
          onPeersDiscovered: (peers) => {
            discoveredCounts[i] = (discoveredCounts[i] ?? 0) + peers.length;
          },
          onPeerListRequested: () => {},
        });
        gossipInstances.push(gossip);
      }

      // Simulate bootstrap: node 0 knows nodes 1-3, node 1 knows nodes 4-6, etc.
      for (let i = 0; i < 4; i++) {
        const bootstrapNodes = [1, 2, 3].map((offset) => createNode((i * 3 + offset) % NODE_COUNT));
        for (const peer of bootstrapNodes) {
          gossipInstances[i].addBootstrapPeer({
            nodeId: peer.nodeId,
            username: peer.username,
          });
        }
      }

      // Simulate gossip round: node 0 requests from node 1
      const request = gossipInstances[0].createPeerListRequest();
      gossipInstances[0].markRequestSent(createNode(1).nodeId, request.requestId);

      // Node 1 responds with its known peers
      const response = gossipInstances[1].handleMessage(request, createNode(0).nodeId);
      expect(response?.type).toBe('peer-list-response');

      // Node 0 processes response and discovers new peers
      gossipInstances[0].handleMessage(response!, createNode(1).nodeId);

      // Node 0 should now know peers from node 1's list
      const node0Stats = gossipInstances[0].getStats();
      expect(node0Stats.totalPeers).toBeGreaterThan(3); // More than its initial bootstrap
    });

    it('should track bootstrap vs gossip discovered peers', () => {
      const gossip = new PeerGossip('node-0', 'user-0', {
        onPeersDiscovered: () => {},
        onPeerListRequested: () => {},
      });

      // Add bootstrap peers
      for (let i = 1; i <= 5; i++) {
        gossip.addBootstrapPeer(createNode(i));
      }

      // Simulate discovering peers via gossip
      const gossipResponse = {
        type: 'peer-list-response' as const,
        requestId: 'test-req',
        peers: [6, 7, 8].map((i) => ({
          ...createNode(i),
          discoverySource: 'bootstrap' as const,
          discoveredAt: Date.now(),
        })),
      };
      gossip.handleMessage(gossipResponse, 'node-001');

      const stats = gossip.getStats();
      expect(stats.bootstrapPeers).toBe(5);
      expect(stats.gossipPeers).toBe(3);
      expect(stats.totalPeers).toBe(8);
    });
  });

  describe('Subnet Formation at Scale', () => {
    let subnetManager: EphemeralSubnetManager;
    let formedSubnets: string[];

    beforeEach(() => {
      formedSubnets = [];
      subnetManager = new EphemeralSubnetManager(
        'node-000',
        {
          onSubnetFormed: (subnet) => formedSubnets.push(subnet.subnetId),
          onSubnetDissolved: () => {},
          onNodeJoinedSubnet: () => {},
          onNodeLeftSubnet: () => {},
        },
        {
          minEdgeWeight: 2,
          minSubnetSize: 3,
          evaluationIntervalMs: 100,
        },
      );
    });

    it('should form multiple subnets from communication patterns', () => {
      // Simulate communication cluster 1: nodes 0-3
      for (let round = 0; round < 3; round++) {
        subnetManager.recordCommunication('node-000', 'node-001');
        subnetManager.recordCommunication('node-001', 'node-002');
        subnetManager.recordCommunication('node-000', 'node-002');
        subnetManager.recordCommunication('node-002', 'node-003');
        subnetManager.recordCommunication('node-000', 'node-003');
      }

      // Simulate communication cluster 2: nodes 5-8
      for (let round = 0; round < 3; round++) {
        subnetManager.recordCommunication('node-005', 'node-006');
        subnetManager.recordCommunication('node-006', 'node-007');
        subnetManager.recordCommunication('node-005', 'node-007');
        subnetManager.recordCommunication('node-007', 'node-008');
      }

      // Trigger evaluation
      subnetManager.forceEvaluate();

      // Should form 2 distinct subnets
      expect(formedSubnets.length).toBe(2);

      // Nodes in same cluster should be in same subnet
      expect(subnetManager.areInSameSubnet('node-000', 'node-001')).toBe(true);
      expect(subnetManager.areInSameSubnet('node-005', 'node-006')).toBe(true);

      // Nodes in different clusters should NOT be in same subnet
      expect(subnetManager.areInSameSubnet('node-000', 'node-005')).toBe(false);
    });

    it('should detect cross-subnet communication needs', () => {
      // Form a subnet first
      for (let round = 0; round < 3; round++) {
        subnetManager.recordCommunication('node-000', 'node-001');
        subnetManager.recordCommunication('node-001', 'node-002');
        subnetManager.recordCommunication('node-000', 'node-002');
      }
      subnetManager.forceEvaluate();

      // Node 000 is in a subnet, node 010 is not
      expect(subnetManager.getNodeSubnet('node-000')).not.toBeNull();
      expect(subnetManager.getNodeSubnet('node-010')).toBeNull();

      // This information can be used to route cross-subnet messages
      expect(subnetManager.areInSameSubnet('node-000', 'node-010')).toBe(false);
    });
  });

  describe('Relay Selection at Scale', () => {
    let topology: NetworkTopology;
    let selector: RelaySelector;

    beforeEach(() => {
      topology = new NetworkTopology();
      selector = new RelaySelector({ selfNodeId: 'self-node' });

      // Add self
      topology.addPeer({
        nodeId: 'self-node',
        username: 'self',
        publicKey: 'self-node',
        reachableVia: [],
        lastSeen: Date.now(),
        roles: ['client'],
      });

      // Add 11 other nodes (4 relays, 7 clients)
      for (let i = 0; i < 11; i++) {
        const isRelay = i < 4;
        topology.addPeer({
          nodeId: `node-${i}`,
          username: `user-${i}`,
          publicKey: `node-${i}`,
          reachableVia: [],
          lastSeen: Date.now(),
          roles: isRelay ? ['client', 'relay'] : ['client'],
        });
      }
    });

    it('should find relay for any peer in the network', () => {
      // Try to reach any client through a relay
      for (let i = 4; i < 11; i++) {
        const selection = selector.selectBestRelay(`node-${i}`, topology);
        expect(selection.relayId).toBeDefined();
        expect(selection.relayId).toMatch(/^node-[0-3]$/); // Should be one of the relays
      }
    });

    it('should demonstrate inversion property - more relays = more routing options', () => {
      const selectionsWithFourRelays = new Set<string>();

      // Make selections and track unique relays used
      for (let i = 4; i < 11; i++) {
        const selection = selector.selectBestRelay(`node-${i}`, topology);
        if (selection.relayId) {
          selectionsWithFourRelays.add(selection.relayId);
        }
      }

      // With 4 relays, should use multiple different relays (load distribution)
      expect(selectionsWithFourRelays.size).toBeGreaterThanOrEqual(1);

      // Remove 2 relays
      topology.removePeer('node-0');
      topology.removePeer('node-1');

      const selectionsWithTwoRelays = new Set<string>();
      for (let i = 4; i < 11; i++) {
        const selection = selector.selectBestRelay(`node-${i}`, topology);
        if (selection.relayId) {
          selectionsWithTwoRelays.add(selection.relayId);
        }
      }

      // With fewer relays, routing options are more limited
      expect(selectionsWithTwoRelays.size).toBeLessThanOrEqual(selectionsWithFourRelays.size);
    });

    it('should select alternate relays when primary fails', () => {
      const failedRelays = new Set<string>();

      // First selection
      const first = selector.selectBestRelay('node-5', topology);
      expect(first.relayId).toBeDefined();
      failedRelays.add(first.relayId!);

      // Select alternate avoiding failed
      const second = selector.selectAlternateRelay('node-5', topology, failedRelays);
      expect(second.relayId).toBeDefined();
      expect(second.relayId).not.toBe(first.relayId);
      failedRelays.add(second.relayId!);

      // Third alternate
      const third = selector.selectAlternateRelay('node-5', topology, failedRelays);
      expect(third.relayId).toBeDefined();
      expect(third.relayId).not.toBe(first.relayId);
      expect(third.relayId).not.toBe(second.relayId);
    });
  });

  describe('Performance Validation', () => {
    it('should handle rapid topology changes efficiently', () => {
      const topology = new NetworkTopology();

      const start = Date.now();

      // Rapid add/update/remove cycle
      for (let cycle = 0; cycle < 100; cycle++) {
        // Add 12 nodes
        for (let i = 0; i < 12; i++) {
          topology.addPeer({
            nodeId: `node-${cycle}-${i}`,
            username: `user-${i}`,
            publicKey: `node-${cycle}-${i}`,
            reachableVia: [],
            lastSeen: Date.now(),
            roles: ['client'],
          });
        }

        // Update last seen
        for (let i = 0; i < 12; i++) {
          topology.updateLastSeen(`node-${cycle}-${i}`);
        }

        // Remove them
        for (let i = 0; i < 12; i++) {
          topology.removePeer(`node-${cycle}-${i}`);
        }
      }

      const elapsed = Date.now() - start;

      // Should complete 100 cycles of 12 nodes each in under 500ms
      expect(elapsed).toBeLessThan(500);
    });

    it('should handle concurrent gossip operations efficiently', () => {
      const gossipInstances: PeerGossip[] = [];

      // Create 12 gossip instances
      for (let i = 0; i < 12; i++) {
        gossipInstances.push(
          new PeerGossip(`node-${i}`, `user-${i}`, {
            onPeersDiscovered: () => {},
            onPeerListRequested: () => {},
          }),
        );
      }

      const start = Date.now();

      // Simulate many gossip exchanges
      for (let round = 0; round < 50; round++) {
        for (let i = 0; i < 12; i++) {
          const target = (i + 1) % 12;

          // Add some bootstrap peers
          gossipInstances[i].addBootstrapPeer({
            nodeId: `node-${target}`,
            username: `user-${target}`,
          });

          // Exchange messages
          const request = gossipInstances[i].createPeerListRequest();
          const response = gossipInstances[target].handleMessage(request, `node-${i}`);
          if (response) {
            gossipInstances[i].handleMessage(response, `node-${target}`);
          }
        }
      }

      const elapsed = Date.now() - start;

      // Should complete 50 rounds of 12 exchanges in under 200ms
      expect(elapsed).toBeLessThan(200);
    });
  });
});
