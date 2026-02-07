/**
 * Hub Election Tests (Action 1: Hub Failover & Resilience Groups)
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { GroupInfo } from './group-types.js';
import { type HubCandidate, HubElection, type HubElectionEvents } from './hub-election.js';

describe('HubElection', () => {
  const localNodeId = 'node-local';
  let election: HubElection;
  let events: Partial<HubElectionEvents>;

  beforeEach(() => {
    events = {
      onElectedAsHub: vi.fn(),
      onHubElected: vi.fn(),
      onElectionFailed: vi.fn(),
    };
    election = new HubElection(localNodeId, events);
  });

  const createGroupInfo = (hubRelayId: string, backupHubId?: string): GroupInfo => ({
    groupId: 'grp-test',
    name: 'Test Group',
    hubRelayId,
    backupHubId,
    members: [
      { nodeId: 'node-alice', username: 'Alice', joinedAt: Date.now(), role: 'admin' },
      { nodeId: 'node-bob', username: 'Bob', joinedAt: Date.now(), role: 'member' },
    ],
    createdBy: 'node-alice',
    createdAt: Date.now(),
    lastActivityAt: Date.now(),
    maxMembers: 50,
  });

  const createCandidates = (nodeIds: string[], isRelay = true): HubCandidate[] =>
    nodeIds.map((nodeId) => ({
      nodeId,
      isRelay,
      lastSeen: Date.now(),
    }));

  describe('initiateElection', () => {
    it('should select the lexicographically first relay as new hub', () => {
      const candidates = createCandidates(['node-charlie', 'node-alice', 'node-bob']);
      const groupInfo = createGroupInfo('node-failed');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      expect(result.newHubId).toBe('node-alice'); // Alphabetically first
      expect(result.reason).toBe('deterministic');
      expect(result.candidates).toHaveLength(3);
    });

    it('should exclude the failed hub from candidates', () => {
      const candidates = createCandidates(['node-failed', 'node-bob', 'node-charlie']);
      const groupInfo = createGroupInfo('node-failed');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      expect(result.newHubId).toBe('node-bob');
      expect(result.candidates.some((c) => c.nodeId === 'node-failed')).toBe(false);
    });

    it('should prefer backup hub if available', () => {
      const candidates = createCandidates(['node-alice', 'node-backup', 'node-charlie']);
      const groupInfo = createGroupInfo('node-failed', 'node-backup');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      expect(result.newHubId).toBe('node-backup');
      expect(result.reason).toBe('backup');
    });

    it('should return null if no candidates available', () => {
      const candidates: HubCandidate[] = [];
      const groupInfo = createGroupInfo('node-failed');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      expect(result.newHubId).toBeNull();
      expect(result.reason).toBe('none');
      expect(events.onElectionFailed).toHaveBeenCalledWith('grp-test', 'insufficient-candidates');
    });

    it('should exclude non-relay candidates', () => {
      const candidates: HubCandidate[] = [
        { nodeId: 'node-alice', isRelay: false, lastSeen: Date.now() },
        { nodeId: 'node-bob', isRelay: true, lastSeen: Date.now() },
        { nodeId: 'node-charlie', isRelay: false, lastSeen: Date.now() },
      ];
      const groupInfo = createGroupInfo('node-failed');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      expect(result.newHubId).toBe('node-bob'); // Only relay
      expect(result.candidates).toHaveLength(1);
    });

    it('should exclude stale candidates', () => {
      const oldTime = Date.now() - 120_000; // 2 minutes ago (beyond 60s default)
      const candidates: HubCandidate[] = [
        { nodeId: 'node-alice', isRelay: true, lastSeen: oldTime },
        { nodeId: 'node-bob', isRelay: true, lastSeen: Date.now() },
      ];
      const groupInfo = createGroupInfo('node-failed');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      expect(result.newHubId).toBe('node-bob');
      expect(result.candidates).toHaveLength(1);
    });

    it('should call onElectedAsHub when local node is elected', () => {
      const candidates = createCandidates(['node-zzz', localNodeId]); // localNodeId comes before zzz alphabetically
      const groupInfo = createGroupInfo('node-failed');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      expect(result.newHubId).toBe(localNodeId);
      expect(events.onElectedAsHub).toHaveBeenCalledWith('grp-test', groupInfo);
      expect(events.onHubElected).not.toHaveBeenCalled();
    });

    it('should call onHubElected when another node is elected', () => {
      const candidates = createCandidates(['node-alice', localNodeId]);
      const groupInfo = createGroupInfo('node-failed');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      expect(result.newHubId).toBe('node-alice');
      expect(events.onHubElected).toHaveBeenCalledWith('grp-test', 'node-alice');
      expect(events.onElectedAsHub).not.toHaveBeenCalled();
    });
  });

  describe('shouldBecomeHub', () => {
    it('should return true when local node is first alphabetically', () => {
      const candidates = createCandidates(['node-zzz', localNodeId, 'node-xyz']);

      const result = election.shouldBecomeHub(candidates);

      expect(result).toBe(true);
    });

    it('should return false when another node is first', () => {
      const candidates = createCandidates(['node-alice', localNodeId, 'node-bob']);

      const result = election.shouldBecomeHub(candidates);

      expect(result).toBe(false);
    });

    it('should exclude specified node from consideration', () => {
      const candidates = createCandidates(['node-alice', localNodeId, 'node-bob']);

      const result = election.shouldBecomeHub(candidates, 'node-alice');

      // localNodeId vs node-bob, localNodeId is "node-local" which comes before "node-bob"
      expect(result).toBe(false); // 'node-bob' < 'node-local'
    });

    it('should return false if no candidates', () => {
      const result = election.shouldBecomeHub([]);

      expect(result).toBe(false);
    });
  });

  describe('selectHub', () => {
    it('should return first eligible relay', () => {
      const candidates = createCandidates(['node-charlie', 'node-alice', 'node-bob']);

      const result = election.selectHub(candidates);

      expect(result).toBe('node-alice');
    });

    it('should exclude specified node', () => {
      const candidates = createCandidates(['node-alice', 'node-bob', 'node-charlie']);

      const result = election.selectHub(candidates, 'node-alice');

      expect(result).toBe('node-bob');
    });

    it('should return null if no eligible candidates', () => {
      const candidates = createCandidates(['node-alice'], false); // not a relay

      const result = election.selectHub(candidates);

      expect(result).toBeNull();
    });
  });

  describe('election state management', () => {
    it('should not have active election initially', () => {
      expect(election.isElectionInProgress('grp-test')).toBe(false);
    });

    it('should track active election', () => {
      const candidates = createCandidates(['node-alice', 'node-bob']);
      const groupInfo = createGroupInfo('node-failed');

      election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      // Election completes immediately in this implementation
      // (real implementation would have async state transfer)
      expect(election.isElectionInProgress('grp-test')).toBe(false);
    });

    it('should cancel election', () => {
      // Set up internal state manually for testing
      // @ts-expect-error - accessing private for test
      election.activeElections.set('grp-test', {
        startedAt: Date.now(),
        candidates: [],
        failedHubId: 'node-failed',
      });

      expect(election.isElectionInProgress('grp-test')).toBe(true);

      const cancelled = election.cancelElection('grp-test');

      expect(cancelled).toBe(true);
      expect(election.isElectionInProgress('grp-test')).toBe(false);
    });

    it('should get election info', () => {
      const candidates = createCandidates(['node-alice']);

      // @ts-expect-error - accessing private for test
      election.activeElections.set('grp-test', {
        startedAt: 12345,
        candidates,
        failedHubId: 'node-failed',
      });

      const info = election.getElectionInfo('grp-test');

      expect(info).not.toBeNull();
      expect(info?.startedAt).toBe(12345);
      expect(info?.failedHubId).toBe('node-failed');
    });

    it('should clear all elections', () => {
      // @ts-expect-error - accessing private for test
      election.activeElections.set('grp-1', { startedAt: 1, candidates: [], failedHubId: 'a' });
      // @ts-expect-error - accessing private for test
      election.activeElections.set('grp-2', { startedAt: 2, candidates: [], failedHubId: 'b' });

      election.clear();

      expect(election.isElectionInProgress('grp-1')).toBe(false);
      expect(election.isElectionInProgress('grp-2')).toBe(false);
    });
  });

  describe('deterministic selection', () => {
    it('should always select same hub regardless of candidate order', () => {
      const candidates1 = createCandidates(['node-charlie', 'node-alice', 'node-bob']);
      const candidates2 = createCandidates(['node-bob', 'node-charlie', 'node-alice']);
      const candidates3 = createCandidates(['node-alice', 'node-bob', 'node-charlie']);
      const groupInfo = createGroupInfo('node-failed');

      const result1 = election.initiateElection('grp-1', 'node-failed', candidates1, groupInfo);
      const result2 = election.initiateElection('grp-2', 'node-failed', candidates2, groupInfo);
      const result3 = election.initiateElection('grp-3', 'node-failed', candidates3, groupInfo);

      expect(result1.newHubId).toBe('node-alice');
      expect(result2.newHubId).toBe('node-alice');
      expect(result3.newHubId).toBe('node-alice');
    });
  });

  describe('edge cases', () => {
    it('should handle single candidate', () => {
      const candidates = createCandidates(['node-solo']);
      const groupInfo = createGroupInfo('node-failed');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      expect(result.newHubId).toBe('node-solo');
    });

    it('should handle all candidates being the failed hub', () => {
      const candidates = createCandidates(['node-failed']);
      const groupInfo = createGroupInfo('node-failed');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      expect(result.newHubId).toBeNull();
      expect(result.reason).toBe('none');
    });

    it('should handle backup hub not in candidates', () => {
      const candidates = createCandidates(['node-alice', 'node-bob']);
      const groupInfo = createGroupInfo('node-failed', 'node-backup-missing');

      const result = election.initiateElection('grp-test', 'node-failed', candidates, groupInfo);

      // Should fall back to deterministic selection
      expect(result.newHubId).toBe('node-alice');
      expect(result.reason).toBe('deterministic');
    });
  });
});
