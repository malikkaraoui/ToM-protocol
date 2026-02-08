/**
 * Chaos/Stress Test Suite
 *
 * Tests the system under chaotic conditions with random operations:
 * - Random group creation/deletion
 * - Random peer connection/disconnection
 * - Random message routing
 * - Random timing variations
 * - Concurrent operations at scale
 *
 * Target: 100% pass rate under chaos
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { secureRandomBytes, secureRandomHex, secureRandomUUID } from '../crypto/secure-random.js';
import { EphemeralSubnetManager } from '../discovery/ephemeral-subnet.js';
import { NetworkTopology, type PeerInfo } from '../discovery/network-topology.js';
import { GroupManager } from '../groups/group-manager.js';
import { type HubCandidate, HubElection } from '../groups/hub-election.js';
import type { NodeId } from '../identity/index.js';
import { RoleManager, type RoleManagerEvents } from '../roles/role-manager.js';
import { MessageTracker, type MessageTrackerEvents } from '../routing/message-tracker.js';
import { OfflineDetector } from '../routing/offline-detector.js';
import { RelaySelector } from '../routing/relay-selector.js';
import { RelayStats } from '../routing/relay-stats.js';
import { Router, type RouterEvents } from '../routing/router.js';
import type { TransportLayer } from '../transport/transport-layer.js';
import type { MessageEnvelope } from '../types/envelope.js';

// Seeded random for reproducibility
function seededRandom(initialSeed: number): () => number {
  let state = initialSeed;
  return () => {
    state = (state * 1103515245 + 12345) & 0x7fffffff;
    return state / 0x7fffffff;
  };
}

// Random helpers
function randomElement<T>(arr: T[], rand: () => number): T {
  return arr[Math.floor(rand() * arr.length)];
}

function randomInt(min: number, max: number, rand: () => number): number {
  return Math.floor(rand() * (max - min + 1)) + min;
}

function randomNodeId(rand: () => number): NodeId {
  const chars = 'abcdefghijklmnopqrstuvwxyz0123456789';
  let id = 'node-';
  for (let i = 0; i < 8; i++) {
    id += chars[Math.floor(rand() * chars.length)];
  }
  return id;
}

function randomUsername(rand: () => number): string {
  const adjectives = ['happy', 'quick', 'lazy', 'brave', 'silent', 'wild', 'calm', 'fierce'];
  const nouns = ['tiger', 'eagle', 'wolf', 'bear', 'fox', 'hawk', 'lion', 'shark'];
  return `${randomElement(adjectives, rand)}_${randomElement(nouns, rand)}_${randomInt(1, 999, rand)}`;
}

function makePeer(nodeId: NodeId, roles: string[] = ['client'], rand?: () => number): PeerInfo {
  const r = rand ?? Math.random;
  return {
    nodeId,
    username: randomUsername(r),
    publicKey: `pk-${nodeId}`,
    reachableVia: [],
    lastSeen: Date.now() - randomInt(0, 5000, r),
    roles: roles as PeerInfo['roles'],
  };
}

describe('Chaos/Stress Tests', () => {
  describe('GroupManager chaos', () => {
    it('should handle rapid group creation and deletion cycles', () => {
      const rand = seededRandom(12345);
      const manager = new GroupManager('node-local', 'local_user', {}, { maxGroups: 50 });

      const groupIds: string[] = [];

      // Rapid creation
      for (let i = 0; i < 30; i++) {
        const group = manager.createGroup(`chaos-group-${i}`, randomNodeId(rand));
        if (group) {
          groupIds.push(group.groupId);
        }
      }

      expect(groupIds.length).toBe(30);

      // Random deletions
      for (let i = 0; i < 15; i++) {
        const idx = randomInt(0, groupIds.length - 1, rand);
        const groupId = groupIds[idx];
        manager.leaveGroup(groupId);
        manager.handleMemberLeft(groupId, 'node-local', 'local_user', 'chaos-leave');
        groupIds.splice(idx, 1);
      }

      expect(manager.getAllGroups().length).toBe(15);

      // Create more groups
      for (let i = 0; i < 10; i++) {
        const group = manager.createGroup(`chaos-group-new-${i}`, randomNodeId(rand));
        if (group) {
          groupIds.push(group.groupId);
        }
      }

      expect(manager.getAllGroups().length).toBe(25);
    });

    it('should handle concurrent invitations from multiple sources', () => {
      const rand = seededRandom(67890);
      const events = {
        onGroupInvite: vi.fn(),
      };
      const manager = new GroupManager('node-local', 'local_user', events);

      // Simulate invites from many sources
      const inviters: NodeId[] = [];
      for (let i = 0; i < 20; i++) {
        inviters.push(randomNodeId(rand));
      }

      // Send invites randomly
      for (let i = 0; i < 50; i++) {
        const groupId = `grp-${secureRandomUUID()}`;
        const inviter = randomElement(inviters, rand);
        manager.handleInvite(groupId, `Random Group ${i}`, inviter, randomUsername(rand), randomNodeId(rand));
      }

      // All unique invites should be tracked
      expect(manager.getPendingInvites().length).toBe(50);
      expect(events.onGroupInvite).toHaveBeenCalledTimes(50);

      // Accept random invites
      const invites = manager.getPendingInvites();
      for (let i = 0; i < 20; i++) {
        const invite = randomElement(invites, rand);
        manager.acceptInvite(invite.groupId);
      }

      // Decline random invites
      for (let i = 0; i < 10; i++) {
        const invite = randomElement(invites, rand);
        manager.declineInvite(invite.groupId);
      }

      // Some invites should remain
      expect(manager.getPendingInvites().length).toBeGreaterThan(0);
    });

    it('should handle message flood without memory explosion', () => {
      const rand = seededRandom(11111);
      const manager = new GroupManager(
        'node-local',
        'local_user',
        {},
        {
          maxMessagesPerGroup: 100,
        },
      );

      const group = manager.createGroup('flood-test', 'hub-1');
      expect(group).not.toBeNull();

      const groupId = group!.groupId;

      // Flood with messages
      for (let i = 0; i < 1000; i++) {
        manager.handleMessage({
          groupId,
          messageId: `msg-${i}`,
          senderId: randomNodeId(rand),
          senderUsername: randomUsername(rand),
          content: `Message content ${i} - ${'x'.repeat(randomInt(10, 500, rand))}`,
          sentAt: Date.now() + i,
        });
      }

      // Should be capped at maxMessagesPerGroup
      const history = manager.getMessageHistory(groupId);
      expect(history.length).toBe(100);

      // Should have the most recent messages
      expect(history[99].messageId).toBe('msg-999');
    });

    it('should handle hub migration during active operations', () => {
      const rand = seededRandom(22222);
      const events = {
        onHubMigration: vi.fn(),
        onGroupMessage: vi.fn(),
      };
      const manager = new GroupManager('node-local', 'local_user', events);

      const group = manager.createGroup('migration-test', 'hub-original');
      const groupId = group!.groupId;

      // Simulate concurrent messages and migrations
      for (let i = 0; i < 100; i++) {
        // Random operation
        const op = randomInt(0, 2, rand);

        if (op === 0) {
          // Send message
          manager.handleMessage({
            groupId,
            messageId: `msg-${i}`,
            senderId: randomNodeId(rand),
            senderUsername: randomUsername(rand),
            content: `Message ${i}`,
            sentAt: Date.now() + i,
          });
        } else if (op === 1) {
          // Migrate hub
          const newHub = randomNodeId(rand);
          const currentGroup = manager.getGroup(groupId);
          manager.handleHubMigration(groupId, newHub, currentGroup!.hubRelayId);
        } else {
          // Member join/leave
          if (rand() > 0.5) {
            manager.handleMemberJoined(groupId, {
              nodeId: randomNodeId(rand),
              username: randomUsername(rand),
              joinedAt: Date.now(),
              role: 'member',
            });
          }
        }
      }

      // Group should still be functional
      const finalGroup = manager.getGroup(groupId);
      expect(finalGroup).not.toBeNull();
      expect(events.onGroupMessage.mock.calls.length).toBeGreaterThan(0);
    });
  });

  describe('NetworkTopology chaos', () => {
    it('should handle rapid peer churn (100 adds/removes)', () => {
      const rand = seededRandom(33333);
      const topology = new NetworkTopology(5000);

      const activePeers: NodeId[] = [];

      for (let i = 0; i < 100; i++) {
        const action = rand() > 0.3 ? 'add' : 'remove';

        if (action === 'add' || activePeers.length === 0) {
          const nodeId = randomNodeId(rand);
          topology.addPeer(makePeer(nodeId, ['client'], rand));
          activePeers.push(nodeId);
        } else {
          const idx = randomInt(0, activePeers.length - 1, rand);
          topology.removePeer(activePeers[idx]);
          activePeers.splice(idx, 1);
        }
      }

      // Topology should be consistent
      expect(topology.size()).toBe(activePeers.length);
      for (const nodeId of activePeers) {
        expect(topology.getPeer(nodeId)).toBeDefined();
      }
    });

    it('should maintain correct role counts under random role changes', () => {
      const rand = seededRandom(44444);
      const topology = new NetworkTopology();

      // Add initial peers
      for (let i = 0; i < 50; i++) {
        const roles = rand() > 0.7 ? ['relay'] : ['client'];
        topology.addPeer(makePeer(`node-${i}`, roles, rand));
      }

      // Random role updates
      for (let i = 0; i < 100; i++) {
        const nodeId = `node-${randomInt(0, 49, rand)}`;
        const peer = topology.getPeer(nodeId);
        if (peer) {
          const newRoles = rand() > 0.5 ? ['relay'] : ['client'];
          topology.addPeer({ ...peer, roles: newRoles as PeerInfo['roles'] });
        }
      }

      const relays = topology.getRelayNodes();
      const clients = topology.getReachablePeers().filter((p) => p.roles.includes('client'));

      // Every peer should have exactly one primary role
      for (const peer of topology.getReachablePeers()) {
        const hasRelay = peer.roles.includes('relay');
        const hasClient = peer.roles.includes('client');
        expect(hasRelay || hasClient).toBe(true);
      }
    });

    it('should handle timestamp manipulation attempts gracefully', () => {
      const topology = new NetworkTopology(5000);
      const now = Date.now();

      // Try extreme future timestamp
      topology.addPeer({
        nodeId: 'future-node',
        username: 'future',
        publicKey: 'pk-future',
        reachableVia: [],
        lastSeen: now + 1000 * 60 * 60 * 24 * 365, // 1 year in future
        roles: ['client'],
      });

      const futurePeer = topology.getPeer('future-node');
      expect(futurePeer!.lastSeen).toBeLessThan(now + 1000 * 60 * 10); // Clamped to ~5min

      // Try extreme past timestamp
      topology.addPeer({
        nodeId: 'past-node',
        username: 'past',
        publicKey: 'pk-past',
        reachableVia: [],
        lastSeen: 0, // Unix epoch
        roles: ['client'],
      });

      const pastPeer = topology.getPeer('past-node');
      expect(pastPeer!.lastSeen).toBeGreaterThan(now - 1000 * 60 * 60 * 2); // Clamped to ~1hr
    });
  });

  describe('OfflineDetector chaos', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('should handle 50 peers with random online/offline cycles', () => {
      const rand = seededRandom(55555);
      const events = {
        onPeerOffline: vi.fn(),
        onPeerOnline: vi.fn(),
      };
      const detector = new OfflineDetector(events, 100);

      const peers: NodeId[] = [];
      for (let i = 0; i < 50; i++) {
        peers.push(`peer-${i}`);
      }

      // Random operations
      for (let i = 0; i < 200; i++) {
        const peer = randomElement(peers, rand);
        const action = rand();

        if (action < 0.3) {
          detector.recordPeerActivity(peer);
        } else if (action < 0.6) {
          detector.handlePeerDeparted(peer);
        } else if (action < 0.8) {
          detector.markPeerOnline(peer);
        }

        // Random time advance
        vi.advanceTimersByTime(randomInt(0, 200, rand));
      }

      // Should not throw and state should be consistent
      const offlinePeers = detector.getOfflinePeers();
      for (const info of offlinePeers) {
        expect(detector.isOffline(info.nodeId)).toBe(true);
      }

      detector.destroy();
    });

    it('should handle destroy during active transitions without memory leaks', () => {
      const events = {
        onPeerOffline: vi.fn(),
        onPeerOnline: vi.fn(),
      };

      // Create and destroy many detectors with pending timers
      for (let cycle = 0; cycle < 10; cycle++) {
        const detector = new OfflineDetector(events, 100);

        // Create many pending transitions
        for (let i = 0; i < 50; i++) {
          detector.handlePeerDeparted(`peer-${i}`);
          detector.recordPeerActivity(`peer-${i}`);
        }

        // Destroy without waiting for timers
        detector.destroy();
      }

      // Advance time to ensure no callbacks fire
      vi.advanceTimersByTime(10000);

      // No events should have fired after destroy
      expect(events.onPeerOnline).not.toHaveBeenCalled();
    });
  });

  describe('RelaySelector chaos', () => {
    it('should always find a relay under random topology changes', () => {
      const rand = seededRandom(66666);
      const selector = new RelaySelector({ selfNodeId: 'self-node' });
      const topology = new NetworkTopology();

      // Add some relays
      for (let i = 0; i < 10; i++) {
        topology.addPeer(makePeer(`relay-${i}`, ['relay'], rand));
      }

      // Random selection with topology changes
      for (let i = 0; i < 100; i++) {
        // Modify topology randomly
        if (rand() > 0.7) {
          const nodeId = `relay-${randomInt(0, 9, rand)}`;
          if (topology.getPeer(nodeId)) {
            topology.removePeer(nodeId);
          } else {
            topology.addPeer(makePeer(nodeId, ['relay'], rand));
          }
        }

        const target = randomNodeId(rand);
        const result = selector.selectBestRelay(target, topology);

        // Should find relay if any exist
        if (topology.getRelayNodes().length > 0) {
          expect(result.relayId).not.toBeNull();
        }
      }
    });

    it('should handle failed relay tracking correctly', () => {
      const rand = seededRandom(77777);
      const selector = new RelaySelector({ selfNodeId: 'self-node' });
      const topology = new NetworkTopology();

      // Add relays
      for (let i = 0; i < 5; i++) {
        topology.addPeer(makePeer(`relay-${i}`, ['relay'], rand));
      }

      const target = 'target-node';
      const failedRelays = new Set<NodeId>();

      // Keep selecting and failing relays
      for (let i = 0; i < 10; i++) {
        const result = selector.selectAlternateRelay(target, topology, failedRelays);

        if (result.relayId) {
          failedRelays.add(result.relayId);
        }
      }

      // Eventually should run out of relays
      const final = selector.selectAlternateRelay(target, topology, failedRelays);
      expect(final.relayId).toBeNull();
    });
  });

  describe('RoleManager chaos', () => {
    it('should maintain relay quota under random network changes', () => {
      const rand = seededRandom(88888);
      const events: RoleManagerEvents = {
        onRoleChanged: vi.fn(),
      };
      const manager = new RoleManager(events);
      const topology = new NetworkTopology();

      // Add initial peers
      for (let i = 0; i < 30; i++) {
        topology.addPeer(makePeer(`node-${i}`, ['client'], rand));
      }

      // Random topology changes and role reassignments
      for (let i = 0; i < 50; i++) {
        // Random change
        if (rand() > 0.5 && topology.size() < 50) {
          topology.addPeer(makePeer(randomNodeId(rand), ['client'], rand));
        } else if (topology.size() > 5) {
          const peers = topology.getReachablePeers();
          const victim = randomElement(peers, rand);
          topology.removePeer(victim.nodeId);
        }

        const roles = manager.reassignRoles(topology);

        // Relay count should be roughly 1/3 of network
        const relayCount = Array.from(roles.values()).filter((r) => r.includes('relay')).length;
        const expectedRelays = Math.ceil(topology.size() / 3);
        expect(relayCount).toBeGreaterThanOrEqual(Math.min(expectedRelays - 2, topology.size()));
      }
    });
  });

  describe('HubElection chaos', () => {
    it('should elect deterministic hub under random candidate orders', () => {
      const rand = seededRandom(99999);
      const now = Date.now();

      // Create candidates
      const candidates = ['alpha', 'beta', 'gamma', 'delta', 'epsilon'];

      // Run many elections with shuffled inputs
      const results: (NodeId | null)[] = [];
      for (let i = 0; i < 20; i++) {
        const shuffled = [...candidates].sort(() => rand() - 0.5);
        const election = new HubElection(shuffled[0], {});

        // Create candidate list in shuffled order
        const hubCandidates: HubCandidate[] = shuffled.map((id) => ({
          nodeId: id,
          isRelay: true,
          lastSeen: now,
        }));

        const result = election.selectHub(hubCandidates, 'failed-hub');
        results.push(result);
      }

      // All results should be the same (deterministic)
      const uniqueResults = new Set(results);
      expect(uniqueResults.size).toBe(1);

      // Should be lexicographically first
      expect(results[0]).toBe('alpha');
    });
  });

  describe('EphemeralSubnet chaos', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('should handle random communication patterns without crashing', () => {
      const rand = seededRandom(10101);
      const events = {
        onSubnetFormed: vi.fn(),
        onSubnetDissolved: vi.fn(),
      };
      const manager = new EphemeralSubnetManager('local-node', events);

      const peers = Array.from({ length: 20 }, (_, i) => `peer-${i}`);

      // Random communications - focused to trigger subnet formation
      for (let i = 0; i < 500; i++) {
        const from = randomElement(peers, rand);
        const to = randomElement(peers, rand);
        if (from !== to) {
          manager.recordCommunication(from, to);
        }

        // Advance time for evaluation (subnet manager evaluates periodically)
        if (i % 50 === 0) {
          vi.advanceTimersByTime(5000);
        }
      }

      // Whether or not subnets formed, the manager should be stable
      expect(() => manager.stop()).not.toThrow();
    });
  });

  describe('Router stress', () => {
    it('should handle 100 message envelope creations', () => {
      const rand = seededRandom(20202);
      const events: RouterEvents = {
        onMessageDelivered: vi.fn(),
        onMessageForwarded: vi.fn(),
        onMessageRejected: vi.fn(),
        onAckReceived: vi.fn(),
        onAckFailed: vi.fn(),
      };
      const transport = {
        send: vi.fn().mockResolvedValue(undefined),
        getConnection: vi.fn().mockReturnValue(null),
        connect: vi.fn().mockResolvedValue({ send: vi.fn() }),
      };

      const router = new Router('local-node', transport as unknown as TransportLayer, events);

      // Create many envelopes
      const envelopes: MessageEnvelope[] = [];
      for (let i = 0; i < 100; i++) {
        const envelope = router.createEnvelope(randomNodeId(rand), 'chat', { text: `Message ${i}` }, [
          randomNodeId(rand),
        ]);
        envelopes.push(envelope);
      }

      // All envelopes should be unique
      const ids = new Set(envelopes.map((e) => e.id));
      expect(ids.size).toBe(100);
    });

    it('should deduplicate messages under replay attacks', () => {
      const events: RouterEvents = {
        onMessageDelivered: vi.fn(),
        onMessageForwarded: vi.fn(),
        onMessageRejected: vi.fn(),
        onAckReceived: vi.fn(),
        onAckFailed: vi.fn(),
      };
      const mockPeer = { send: vi.fn() };
      const transport = {
        send: vi.fn().mockResolvedValue(undefined),
        getConnection: vi.fn().mockReturnValue(mockPeer),
        connect: vi.fn().mockResolvedValue(mockPeer),
        getPeer: vi.fn().mockReturnValue(mockPeer),
      };

      const router = new Router('target-node', transport as unknown as TransportLayer, events);

      // Create a single message
      const envelope: MessageEnvelope = {
        id: 'dup-message-id',
        from: 'sender-node',
        to: 'target-node',
        via: [],
        type: 'chat',
        payload: { text: 'Hello' },
        timestamp: Date.now(),
        signature: 'sig',
      };

      // Try to deliver it 100 times (replay attack)
      for (let i = 0; i < 100; i++) {
        router.handleIncoming(envelope);
      }

      // Should only deliver once (deduplication)
      expect(events.onMessageDelivered).toHaveBeenCalledTimes(1);
    });
  });

  describe('MessageTracker stress', () => {
    it('should track 1000 messages without memory explosion', () => {
      const trackerEvents: MessageTrackerEvents = {
        onStatusChanged: vi.fn(),
      };
      const tracker = new MessageTracker(trackerEvents);

      // Track many messages (some will be evicted due to MAX_TRACKED_MESSAGES = 10000)
      for (let i = 0; i < 1000; i++) {
        tracker.track(`msg-${i}`, `recipient-${i % 10}`);
      }

      // All should be tracked (under the 10000 limit)
      expect(tracker.size).toBe(1000);

      // Mark some as delivered
      for (let i = 0; i < 500; i++) {
        tracker.markDelivered(`msg-${i}`);
      }

      // Mark some as read
      for (let i = 500; i < 700; i++) {
        tracker.markRead(`msg-${i}`);
      }

      // Status transitions should have been tracked
      expect(trackerEvents.onStatusChanged).toHaveBeenCalled();

      // Check individual message status
      const status = tracker.getStatus('msg-0');
      expect(status).toBeDefined();
      expect(status?.status).toBe('delivered');
    });
  });

  describe('RelayStats stress', () => {
    it('should maintain accurate stats under high-frequency updates', () => {
      const rand = seededRandom(30303);
      const warningHandler = vi.fn();
      const stats = new RelayStats({
        events: { onCapacityWarning: warningHandler },
      });

      // High-frequency relay and own message recording
      for (let i = 0; i < 1000; i++) {
        const byteSize = randomInt(100, 5000, rand);

        if (rand() > 0.3) {
          stats.recordRelay(byteSize);
        } else {
          stats.recordOwnMessage(byteSize);
        }
      }

      // Stats should be accurate
      const finalStats = stats.getStats();
      expect(finalStats.messagesRelayed + finalStats.ownMessagesSent).toBe(1000);
      expect(finalStats.bytesRelayed).toBeGreaterThan(0);
      expect(finalStats.bytesSent).toBeGreaterThan(0);

      // Relay ACK recording
      for (let i = 0; i < 100; i++) {
        stats.recordRelayAck();
      }

      expect(stats.getStats().relayAcksReceived).toBe(100);
    });
  });

  describe('Crypto stress', () => {
    it('should generate 1000 unique random values', () => {
      const values = new Set<string>();

      for (let i = 0; i < 1000; i++) {
        values.add(secureRandomUUID());
      }

      expect(values.size).toBe(1000);
    });

    it('should handle rapid random byte generation', () => {
      const totalBytes = [];

      for (let i = 0; i < 100; i++) {
        const bytes = secureRandomBytes(32);
        expect(bytes.length).toBe(32);
        totalBytes.push(bytes);
      }

      // All should be different
      const hexValues = new Set(
        totalBytes.map((b) =>
          Array.from(b)
            .map((x) => x.toString(16))
            .join(''),
        ),
      );
      expect(hexValues.size).toBe(100);
    });

    it('should generate valid hex strings of various lengths', () => {
      for (let len = 1; len <= 64; len++) {
        const hex = secureRandomHex(len);
        expect(hex.length).toBe(len);
        expect(/^[0-9a-f]+$/.test(hex)).toBe(true);
      }
    });
  });

  describe('Combined system chaos', () => {
    beforeEach(() => {
      vi.useFakeTimers();
    });

    afterEach(() => {
      vi.useRealTimers();
    });

    it('should survive full system chaos simulation', () => {
      const rand = seededRandom(40404);

      // Initialize all components
      const topology = new NetworkTopology();
      const roleEvents: RoleManagerEvents = { onRoleChanged: vi.fn() };
      const roleManager = new RoleManager(roleEvents);
      const groupManager = new GroupManager('local-node', 'local_user', {});
      const offlineDetector = new OfflineDetector({ onPeerOffline: vi.fn(), onPeerOnline: vi.fn() }, 100);
      const relaySelector = new RelaySelector({ selfNodeId: 'local-node' });
      const subnetManager = new EphemeralSubnetManager('local-node', {});

      // Add initial peers
      for (let i = 0; i < 20; i++) {
        topology.addPeer(makePeer(`node-${i}`, rand() > 0.5 ? ['relay'] : ['client'], rand));
      }

      // Run chaos operations
      for (let tick = 0; tick < 500; tick++) {
        const op = randomInt(0, 9, rand);

        switch (op) {
          case 0: // Peer join
            topology.addPeer(makePeer(randomNodeId(rand), ['client'], rand));
            break;

          case 1: // Peer leave
            if (topology.size() > 5) {
              const peers = topology.getReachablePeers();
              topology.removePeer(randomElement(peers, rand).nodeId);
            }
            break;

          case 2: // Role reassignment
            roleManager.reassignRoles(topology);
            break;

          case 3: // Create group
            groupManager.createGroup(`group-${tick}`, randomNodeId(rand));
            break;

          case 4: // Peer offline
            offlineDetector.handlePeerDeparted(`node-${randomInt(0, 19, rand)}`);
            break;

          case 5: // Peer activity
            offlineDetector.recordPeerActivity(`node-${randomInt(0, 19, rand)}`);
            break;

          case 6: // Select relay
            relaySelector.selectBestRelay(randomNodeId(rand), topology);
            break;

          case 7: // Record communication
            subnetManager.recordCommunication(`node-${randomInt(0, 19, rand)}`, `node-${randomInt(0, 19, rand)}`);
            break;

          case 8: // Time advance
            vi.advanceTimersByTime(randomInt(100, 2000, rand));
            break;

          case 9: {
            // Group message
            const groups = groupManager.getAllGroups();
            if (groups.length > 0) {
              const group = randomElement(groups, rand);
              groupManager.handleMessage({
                groupId: group.groupId,
                messageId: `msg-${tick}`,
                senderId: randomNodeId(rand),
                senderUsername: randomUsername(rand),
                content: `Chaos message ${tick}`,
                sentAt: Date.now(),
              });
            }
            break;
          }
        }
      }

      // System should be in a consistent state
      expect(topology.size()).toBeGreaterThan(0);
      expect(() => roleManager.reassignRoles(topology)).not.toThrow();
      expect(() => offlineDetector.getOfflinePeers()).not.toThrow();
      expect(() => subnetManager.stop()).not.toThrow();
    });
  });
});
