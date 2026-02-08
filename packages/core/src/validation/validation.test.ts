/**
 * Input Validation & Boundary Tests
 *
 * Comprehensive tests for validating inputs, edge cases, and security boundaries
 * across critical ToM protocol modules. These tests complement chaos/stress tests
 * by focusing on deterministic validation of edge cases.
 *
 * @module validation
 */

import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  decryptPayload,
  encryptPayload,
  encryptionKeyToHex,
  generateEncryptionKeypair,
  hexToEncryptionKey,
  isEncryptedPayload,
} from '../crypto/encryption.js';
import { NetworkTopology } from '../discovery/network-topology.js';
import { GroupManager } from '../groups/group-manager.js';
import { hexToUint8Array, uint8ArrayToHex } from '../groups/group-security.js';
import { RoleManager } from '../roles/role-manager.js';
import { MessageTracker } from '../routing/message-tracker.js';
import { RelaySelector } from '../routing/relay-selector.js';
import { RelayStats } from '../routing/relay-stats.js';

// ============================================
// Encryption Module Validation Tests
// ============================================

describe('Encryption Validation', () => {
  describe('key validation', () => {
    it('should reject encryption with empty public key', () => {
      const emptyKey = new Uint8Array(0);
      expect(() => encryptPayload({ test: 'data' }, emptyKey)).toThrow();
    });

    it('should reject encryption with wrong-length public key', () => {
      const shortKey = new Uint8Array(16); // Should be 32 bytes
      expect(() => encryptPayload({ test: 'data' }, shortKey)).toThrow();
    });

    it('should reject decryption with wrong secret key', () => {
      const recipientKeypair = generateEncryptionKeypair();
      const wrongKeypair = generateEncryptionKeypair();

      const encrypted = encryptPayload({ secret: 'message' }, recipientKeypair.publicKey);
      const result = decryptPayload(encrypted, wrongKeypair.secretKey);

      expect(result).toBeNull();
    });

    it('should reject decryption with tampered ciphertext', () => {
      const keypair = generateEncryptionKeypair();
      const encrypted = encryptPayload({ secret: 'message' }, keypair.publicKey);

      // Tamper with ciphertext
      const tamperedCiphertext = `${encrypted.ciphertext.slice(0, -2)}ff`;
      const tampered = { ...encrypted, ciphertext: tamperedCiphertext };

      const result = decryptPayload(tampered, keypair.secretKey);
      expect(result).toBeNull();
    });

    it('should reject decryption with tampered nonce', () => {
      const keypair = generateEncryptionKeypair();
      const encrypted = encryptPayload({ secret: 'message' }, keypair.publicKey);

      // Tamper with nonce
      const tamperedNonce = `${encrypted.nonce.slice(0, -2)}ff`;
      const tampered = { ...encrypted, nonce: tamperedNonce };

      const result = decryptPayload(tampered, keypair.secretKey);
      expect(result).toBeNull();
    });

    it('should reject decryption with invalid hex in ciphertext', () => {
      const keypair = generateEncryptionKeypair();

      const invalid = {
        ciphertext: 'not-valid-hex!@#$',
        nonce: 'also-not-valid',
        ephemeralPublicKey: 'garbage',
      };

      const result = decryptPayload(invalid, keypair.secretKey);
      expect(result).toBeNull();
    });
  });

  describe('hex conversion validation', () => {
    it('should round-trip hex conversion correctly', () => {
      const keypair = generateEncryptionKeypair();
      const hex = encryptionKeyToHex(keypair.publicKey);
      const recovered = hexToEncryptionKey(hex);

      expect(recovered).toEqual(keypair.publicKey);
    });

    it('should handle single-byte arrays', () => {
      const single = new Uint8Array([0xab]);
      const hex = uint8ArrayToHex(single);
      expect(hex).toBe('ab');

      const recovered = hexToUint8Array(hex);
      expect(recovered).toEqual(single);
    });

    it('should handle all byte values (0x00 to 0xFF)', () => {
      const allBytes = new Uint8Array(256);
      for (let i = 0; i < 256; i++) {
        allBytes[i] = i;
      }

      const hex = uint8ArrayToHex(allBytes);
      const recovered = hexToUint8Array(hex);

      expect(recovered).toEqual(allBytes);
    });
  });

  describe('payload type guards', () => {
    it('should identify encrypted payloads correctly', () => {
      const keypair = generateEncryptionKeypair();
      const encrypted = encryptPayload({ test: 'data' }, keypair.publicKey);

      expect(isEncryptedPayload(encrypted)).toBe(true);
    });

    it('should reject non-object payloads', () => {
      expect(isEncryptedPayload(null)).toBe(false);
      expect(isEncryptedPayload(undefined)).toBe(false);
      expect(isEncryptedPayload('string')).toBe(false);
      expect(isEncryptedPayload(123)).toBe(false);
      expect(isEncryptedPayload([])).toBe(false);
    });

    it('should reject payloads missing required fields', () => {
      expect(isEncryptedPayload({})).toBe(false);
      expect(isEncryptedPayload({ ciphertext: 'abc' })).toBe(false);
      expect(isEncryptedPayload({ ciphertext: 'abc', nonce: 'def' })).toBe(false);
      expect(isEncryptedPayload({ nonce: 'def', ephemeralPublicKey: 'ghi' })).toBe(false);
    });

    it('should reject payloads with wrong field types', () => {
      expect(isEncryptedPayload({ ciphertext: 123, nonce: 'def', ephemeralPublicKey: 'ghi' })).toBe(false);
      expect(isEncryptedPayload({ ciphertext: 'abc', nonce: null, ephemeralPublicKey: 'ghi' })).toBe(false);
    });
  });

  describe('payload serialization', () => {
    it('should handle complex nested objects', () => {
      const keypair = generateEncryptionKeypair();
      const complex = {
        nested: { deep: { value: 42 } },
        array: [1, 2, { three: 3 }],
        unicode: '🔐🔑',
      };

      const encrypted = encryptPayload(complex, keypair.publicKey);
      const decrypted = decryptPayload(encrypted, keypair.secretKey);

      expect(decrypted).toEqual(complex);
    });

    it('should handle null and undefined in payloads', () => {
      const keypair = generateEncryptionKeypair();

      // null is serializable
      const encrypted1 = encryptPayload(null, keypair.publicKey);
      expect(decryptPayload(encrypted1, keypair.secretKey)).toBeNull();

      // undefined serializes to undefined in JSON
      const encrypted2 = encryptPayload({ value: undefined }, keypair.publicKey);
      const result = decryptPayload(encrypted2, keypair.secretKey) as { value?: unknown };
      expect(result.value).toBeUndefined();
    });

    it('should handle empty payloads', () => {
      const keypair = generateEncryptionKeypair();
      const encrypted = encryptPayload({}, keypair.publicKey);
      expect(decryptPayload(encrypted, keypair.secretKey)).toEqual({});
    });

    it('should handle very large payloads', () => {
      const keypair = generateEncryptionKeypair();
      const largePayload = { data: 'x'.repeat(100000) }; // 100KB of data

      const encrypted = encryptPayload(largePayload, keypair.publicKey);
      const decrypted = decryptPayload(encrypted, keypair.secretKey);

      expect(decrypted).toEqual(largePayload);
    });
  });
});

// ============================================
// Network Topology Validation Tests
// ============================================

describe('NetworkTopology Validation', () => {
  let topology: NetworkTopology;

  beforeEach(() => {
    vi.useFakeTimers();
    topology = new NetworkTopology(3000);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function makePeer(nodeId: string, username = 'user') {
    return {
      nodeId,
      username,
      publicKey: nodeId,
      reachableVia: [] as string[],
      lastSeen: Date.now(),
      roles: ['client'] as ('client' | 'relay')[],
    };
  }

  describe('peer ID validation', () => {
    it('should handle empty nodeId', () => {
      const peer = makePeer('');
      topology.addPeer(peer);
      expect(topology.getPeer('')).toBeDefined();
    });

    it('should handle very long nodeId', () => {
      const longId = 'x'.repeat(1000);
      const peer = makePeer(longId);
      topology.addPeer(peer);
      expect(topology.getPeer(longId)).toBeDefined();
    });

    it('should handle special characters in nodeId', () => {
      const specialId = 'node/with:special@chars#!';
      const peer = makePeer(specialId);
      topology.addPeer(peer);
      expect(topology.getPeer(specialId)).toBeDefined();
    });

    it('should handle unicode in nodeId', () => {
      const unicodeId = 'nœud-éèê-🌐';
      const peer = makePeer(unicodeId);
      topology.addPeer(peer);
      expect(topology.getPeer(unicodeId)).toBeDefined();
    });
  });

  describe('peer lifecycle', () => {
    it('should handle adding same peer twice', () => {
      const peer = makePeer('node-1');
      topology.addPeer(peer);
      topology.addPeer(peer);

      expect(topology.getReachablePeers().length).toBe(1);
    });

    it('should handle removing non-existent peer', () => {
      topology.removePeer('non-existent');
      // Should not throw
      expect(topology.getPeer('non-existent')).toBeUndefined();
    });

    it('should handle many peers', () => {
      for (let i = 0; i < 100; i++) {
        topology.addPeer(makePeer(`node-${i}`));
      }

      expect(topology.getReachablePeers().length).toBe(100);
    });
  });

  describe('lastSeen validation', () => {
    it('should handle very old timestamps', () => {
      const peer = makePeer('node-1');
      peer.lastSeen = 0; // Epoch
      topology.addPeer(peer);

      const stored = topology.getPeer('node-1');
      expect(stored).toBeDefined();
    });

    it('should handle negative timestamps', () => {
      const peer = makePeer('node-1');
      peer.lastSeen = -1000;
      topology.addPeer(peer);

      const stored = topology.getPeer('node-1');
      expect(stored).toBeDefined();
    });
  });

  describe('stale threshold edge cases', () => {
    it('should handle very small stale threshold', () => {
      const smallThreshold = new NetworkTopology(100);
      const peer = makePeer('node-1');
      smallThreshold.addPeer(peer);

      // Initially online
      expect(smallThreshold.getPeerStatus('node-1')).toBe('online');

      // After threshold but before 2x threshold, should be stale
      vi.advanceTimersByTime(150);
      expect(smallThreshold.getPeerStatus('node-1')).toBe('stale');

      // After 2x threshold, should be offline
      vi.advanceTimersByTime(100);
      expect(smallThreshold.getPeerStatus('node-1')).toBe('offline');
    });

    it('should handle very large stale threshold', () => {
      const largeThreshold = new NetworkTopology(Number.MAX_SAFE_INTEGER);
      const peer = makePeer('node-1');
      largeThreshold.addPeer(peer);

      // Should never go stale
      vi.advanceTimersByTime(1000000);
      expect(largeThreshold.getPeerStatus('node-1')).toBe('online');
    });
  });
});

// ============================================
// RelaySelector Validation Tests
// ============================================

describe('RelaySelector Validation', () => {
  let selector: RelaySelector;
  let topology: NetworkTopology;

  beforeEach(() => {
    vi.useFakeTimers();
    selector = new RelaySelector({ selfNodeId: 'self' });
    topology = new NetworkTopology(3000);
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  function makePeer(nodeId: string, roles: ('client' | 'relay')[] = ['client', 'relay']) {
    return {
      nodeId,
      username: 'user',
      publicKey: nodeId,
      reachableVia: [] as string[],
      lastSeen: Date.now(),
      roles,
    };
  }

  describe('target ID validation', () => {
    it('should handle self as target', () => {
      topology.addPeer(makePeer('relay-1'));
      const result = selector.selectBestRelay('self', topology);
      // Should still return a relay even for self-targeting
      expect(result.relayId).toBeDefined();
    });

    it('should handle empty target ID', () => {
      topology.addPeer(makePeer('relay-1'));
      const result = selector.selectBestRelay('', topology);
      expect(result.relayId).toBeDefined();
    });
  });

  describe('empty/minimal topology', () => {
    it('should handle empty topology', () => {
      const result = selector.selectBestRelay('target', topology);
      expect(result.relayId).toBeNull();
      expect(result.reason).toContain('no');
    });

    it('should handle topology with only non-relay clients', () => {
      topology.addPeer(makePeer('client-1', ['client']));
      topology.addPeer(makePeer('client-2', ['client']));

      const result = selector.selectBestRelay('target', topology);
      expect(result.relayId).toBeNull();
    });
  });

  describe('failed relays handling', () => {
    it('should handle all relays in failed set', () => {
      topology.addPeer(makePeer('relay-1'));
      topology.addPeer(makePeer('relay-2'));

      const result = selector.selectAlternateRelay('target', topology, new Set(['relay-1', 'relay-2']));
      expect(result.relayId).toBeNull();
    });

    it('should handle empty failed set', () => {
      topology.addPeer(makePeer('relay-1'));

      const result = selector.selectAlternateRelay('target', topology, new Set());
      expect(result.relayId).toBe('relay-1');
    });

    it('should handle non-existent relays in failed set', () => {
      topology.addPeer(makePeer('relay-1'));

      const result = selector.selectAlternateRelay('target', topology, new Set(['non-existent']));
      expect(result.relayId).toBe('relay-1');
    });
  });
});

// ============================================
// MessageTracker Validation Tests
// ============================================

describe('MessageTracker Validation', () => {
  let tracker: MessageTracker;
  let onStatusChanged: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.useFakeTimers();
    onStatusChanged = vi.fn();
    tracker = new MessageTracker({ onStatusChanged });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('message ID validation', () => {
    it('should handle empty message ID', () => {
      tracker.track('', 'recipient');
      expect(tracker.getStatus('')?.status).toBe('pending');
    });

    it('should handle very long message ID', () => {
      const longId = 'x'.repeat(10000);
      tracker.track(longId, 'recipient');
      expect(tracker.getStatus(longId)?.status).toBe('pending');
    });

    it('should handle special characters in message ID', () => {
      const specialId = 'msg/with:special@chars#!&=';
      tracker.track(specialId, 'recipient');
      expect(tracker.getStatus(specialId)?.status).toBe('pending');
    });
  });

  describe('status transitions', () => {
    it('should handle duplicate tracking attempts', () => {
      const result1 = tracker.track('msg-1', 'recipient');
      const result2 = tracker.track('msg-1', 'recipient'); // Duplicate

      expect(result1).toBe(true);
      expect(result2).toBe(false); // Already tracked
    });

    it('should prevent status regression', () => {
      tracker.track('msg-1', 'recipient');
      tracker.markDelivered('msg-1'); // Skip relayed

      // Status should be delivered
      expect(tracker.getStatus('msg-1')?.status).toBe('delivered');

      // Try to go back to relayed (should be ignored)
      tracker.markRelayed('msg-1');
      expect(tracker.getStatus('msg-1')?.status).toBe('delivered');
    });

    it('should handle status update for unknown message', () => {
      tracker.markRelayed('unknown');
      tracker.markDelivered('unknown');

      // Should not throw, just return undefined
      expect(tracker.getStatus('unknown')).toBeUndefined();
    });
  });

  describe('cleanup behavior', () => {
    it('should cleanup old read messages', () => {
      tracker.track('msg-1', 'recipient');
      tracker.markRead('msg-1');

      // Advance past cleanup threshold
      vi.advanceTimersByTime(60 * 60 * 1000); // 1 hour

      const removed = tracker.cleanupOldMessages(30 * 60 * 1000); // 30 min threshold
      expect(removed).toBe(1);
      expect(tracker.getStatus('msg-1')).toBeUndefined();
    });
  });
});

// ============================================
// RelayStats Validation Tests
// ============================================

describe('RelayStats Validation', () => {
  let stats: RelayStats;
  let onCapacityWarning: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.useFakeTimers();
    onCapacityWarning = vi.fn();
    stats = new RelayStats({
      capacityThreshold: 10,
      events: { onCapacityWarning },
    });
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe('counter bounds', () => {
    it('should handle recording with zero bytes', () => {
      stats.recordRelay(0);
      stats.recordOwnMessage(0);

      const data = stats.getStats();
      expect(data.messagesRelayed).toBe(1);
      expect(data.ownMessagesSent).toBe(1);
      expect(data.bytesRelayed).toBe(0);
      expect(data.bytesSent).toBe(0);
    });

    it('should handle recording with undefined bytes', () => {
      stats.recordRelay(undefined);
      stats.recordOwnMessage(undefined);

      const data = stats.getStats();
      expect(data.messagesRelayed).toBe(1);
      expect(data.bytesRelayed).toBe(0);
    });

    it('should handle very large byte values', () => {
      stats.recordRelay(Number.MAX_SAFE_INTEGER);

      const data = stats.getStats();
      expect(data.bytesRelayed).toBe(Number.MAX_SAFE_INTEGER);
    });
  });

  describe('warning cooldown', () => {
    it('should respect warning cooldown', () => {
      // Trigger first warning
      for (let i = 0; i < 25; i++) {
        stats.recordRelay(100);
      }
      vi.advanceTimersByTime(100);

      expect(onCapacityWarning).toHaveBeenCalledTimes(1);

      // Try to trigger another warning immediately
      for (let i = 0; i < 25; i++) {
        stats.recordRelay(100);
      }

      expect(onCapacityWarning).toHaveBeenCalledTimes(1); // Still 1

      // Wait for cooldown (10 seconds)
      vi.advanceTimersByTime(10001);
      stats.recordRelay(100);

      expect(onCapacityWarning).toHaveBeenCalledTimes(2);
    });
  });

  describe('ratio calculation', () => {
    it('should handle zero own messages (infinite ratio)', () => {
      stats.recordRelay(100);
      stats.recordRelay(100);

      const data = stats.getStats();
      // When ownMessagesSent is 0, ratio is messagesRelayed (not Infinity)
      expect(data.relayToOwnRatio).toBe(2);
    });

    it('should handle equal relay and own messages', () => {
      stats.recordRelay(100);
      stats.recordOwnMessage(100);

      const data = stats.getStats();
      expect(data.relayToOwnRatio).toBe(1);
    });
  });

  describe('reset behavior', () => {
    it('should reset all counters', () => {
      stats.recordRelay(100);
      stats.recordOwnMessage(50);
      stats.recordRelayAck();

      stats.reset();

      const data = stats.getStats();
      expect(data.messagesRelayed).toBe(0);
      expect(data.ownMessagesSent).toBe(0);
      expect(data.relayAcksReceived).toBe(0);
      expect(data.bytesRelayed).toBe(0);
      expect(data.bytesSent).toBe(0);
    });
  });
});

// ============================================
// RoleManager Validation Tests
// ============================================

describe('RoleManager Validation', () => {
  let manager: RoleManager;
  let topology: NetworkTopology;
  let onRoleChanged: ReturnType<typeof vi.fn>;

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

  function makePeer(nodeId: string) {
    return {
      nodeId,
      username: 'user',
      publicKey: nodeId,
      reachableVia: [] as string[],
      lastSeen: Date.now(),
      roles: ['client'] as ('client' | 'relay')[],
    };
  }

  describe('metrics validation', () => {
    it('should handle bandwidth score over 100', () => {
      manager.updateNodeMetrics('node-1', { bandwidthScore: 150 });
      const metrics = manager.getNodeMetrics('node-1');
      // Implementation accepts values (doesn't clamp)
      expect(metrics.bandwidthScore).toBe(150);
    });

    it('should handle contribution increment on unknown node', () => {
      // Should not throw, should create metrics
      manager.incrementContributionScore('unknown-node', 10);
      const metrics = manager.getNodeMetrics('unknown-node');
      expect(metrics.contributionScore).toBe(10);
    });

    it('should cap contribution score at 100', () => {
      manager.updateNodeMetrics('node-1', { contributionScore: 95 });
      manager.incrementContributionScore('node-1', 20);
      const metrics = manager.getNodeMetrics('node-1');
      expect(metrics.contributionScore).toBe(100);
    });
  });

  describe('topology binding', () => {
    it('should handle rebinding topology', () => {
      const newTopology = new NetworkTopology(5000);
      manager.bindTopology(newTopology);
      manager.bindTopology(topology); // Rebind to original

      topology.addPeer(makePeer('node-1'));
      const roles = manager.evaluateNode('node-1', topology);
      expect(roles).toContain('client');
    });

    it('should handle start/stop without topology', () => {
      const unboundManager = new RoleManager({ onRoleChanged: vi.fn() });
      unboundManager.start();
      vi.advanceTimersByTime(60000);
      unboundManager.stop();
      // Should not throw
    });
  });
});

// ============================================
// GroupManager Validation Tests
// ============================================

describe('GroupManager Validation', () => {
  let manager: GroupManager;

  beforeEach(() => {
    vi.useFakeTimers();
    manager = new GroupManager('self-node', 'self-user', {
      onGroupCreated: vi.fn(),
      onGroupMessage: vi.fn(),
    });
  });

  afterEach(() => {
    manager.stopHubHealthMonitoring();
    manager.stopInviteExpiryMonitoring();
    vi.useRealTimers();
  });

  describe('group name validation', () => {
    it('should handle empty group name', () => {
      const group = manager.createGroup('', 'hub-relay');
      expect(group?.name).toBe('');
    });

    it('should handle very long group name', () => {
      const longName = 'x'.repeat(10000);
      const group = manager.createGroup(longName, 'hub-relay');
      expect(group?.name).toBe(longName);
    });

    it('should handle unicode in group name', () => {
      const unicodeName = '团队聊天 🎉 équipe';
      const group = manager.createGroup(unicodeName, 'hub-relay');
      expect(group?.name).toBe(unicodeName);
    });
  });

  describe('member structure', () => {
    it('should have creator as admin member', () => {
      const group = manager.createGroup('Test', 'hub-relay');
      const creatorMember = group?.members.find((m) => m.nodeId === 'self-node');
      expect(creatorMember).toBeDefined();
      expect(creatorMember?.role).toBe('admin');
    });
  });

  describe('group limits', () => {
    it('should enforce max groups limit', () => {
      // Create max groups (default is 20)
      for (let i = 0; i < 20; i++) {
        manager.createGroup(`Group ${i}`, 'hub-relay');
      }

      // Next one should return null
      const extra = manager.createGroup('Extra', 'hub-relay');
      expect(extra).toBeNull();
    });
  });

  describe('invite handling', () => {
    it('should handle invite for already joined group', () => {
      const group = manager.createGroup('Test', 'hub-relay');
      const groupId = group?.groupId ?? '';

      // Try to invite to existing group - should be ignored
      manager.handleInvite(groupId, 'Test', 'inviter', 'inviter-user', 'hub-relay');

      // No pending invite should exist
      const pendingInvites = manager.getPendingInvites();
      const hasPending = pendingInvites.some((inv) => inv.groupId === groupId);
      expect(hasPending).toBe(false);
    });

    it('should handle duplicate invites', () => {
      manager.handleInvite('grp-1', 'Group', 'inviter', 'inviter-user', 'hub-relay');
      manager.handleInvite('grp-1', 'Group', 'inviter', 'inviter-user', 'hub-relay');

      // Should only have one pending invite
      const pendingInvites = manager.getPendingInvites();
      const count = pendingInvites.filter((inv) => inv.groupId === 'grp-1').length;
      expect(count).toBe(1);
    });
  });
});
