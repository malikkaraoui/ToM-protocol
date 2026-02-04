import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { MessageEnvelope } from '../types/envelope.js';
import { BackupStore } from './backup-store.js';
import { DELETION_THRESHOLD, MessageViability, REPLICATION_THRESHOLD } from './message-viability.js';

function makeEnvelope(id: string, from: string, to: string): MessageEnvelope {
  return {
    id,
    from,
    to,
    via: [],
    type: 'message',
    payload: { text: 'test' },
    timestamp: Date.now(),
    signature: 'sig',
  };
}

describe('MessageViability', () => {
  let backupStore: BackupStore;
  let viability: MessageViability;
  let onReplicationNeeded: ReturnType<typeof vi.fn>;
  let onSelfDeleteNeeded: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.useFakeTimers();

    backupStore = new BackupStore(
      {
        onMessageStored: vi.fn(),
        onMessageExpired: vi.fn(),
        onMessageDelivered: vi.fn(),
        onViabilityChanged: vi.fn(),
      },
      { autoStart: false },
    );

    onReplicationNeeded = vi.fn();
    onSelfDeleteNeeded = vi.fn();

    viability = new MessageViability(
      {
        onReplicationNeeded,
        onSelfDeleteNeeded,
      },
      backupStore,
    );
  });

  afterEach(() => {
    viability.stop();
    backupStore.stop();
    vi.useRealTimers();
  });

  describe('score calculation', () => {
    it('should calculate viability score with default factors', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      const score = viability.calculateScore('msg-1');

      // Default factors are all 50
      // Score = 50*0.25 + 50*0.3 + 50*0.25 + 50*0.2 = 50
      expect(score).toBe(50);
    });

    it('should return 0 for unknown message', () => {
      const score = viability.calculateScore('unknown');
      expect(score).toBe(0);
    });

    it('should calculate score with custom host factors', () => {
      viability.updateHostFactors({
        connectionStability: 80,
        bandwidthCapacity: 90,
        contributionScore: 70,
      });

      // Set recipient timezone same as host for predictable timezone score (100)
      const hostTz = viability.getHostTimezoneOffset();
      viability.setRecipientTimezone('recipient', hostTz);

      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      const score = viability.calculateScore('msg-1');

      // Score = 100*0.25 + 80*0.3 + 90*0.25 + 70*0.2 = 25 + 24 + 22.5 + 14 = 85.5 → 86
      expect(score).toBe(86);
    });

    it('should factor in timezone alignment', () => {
      viability.updateHostFactors({
        connectionStability: 50,
        bandwidthCapacity: 50,
        contributionScore: 50,
      });

      // Set recipient timezone same as host
      const hostTz = viability.getHostTimezoneOffset();
      viability.setRecipientTimezone('recipient', hostTz);

      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      const score = viability.calculateScore('msg-1');

      // Same timezone = 100 alignment
      // Score = 100*0.25 + 50*0.3 + 50*0.25 + 50*0.2 = 25 + 15 + 12.5 + 10 = 62.5 → 63
      expect(score).toBe(63);
    });

    it('should penalize distant timezones', () => {
      viability.updateHostFactors({
        connectionStability: 50,
        bandwidthCapacity: 50,
        contributionScore: 50,
      });

      // Set recipient timezone 12 hours away from host
      const hostTz = viability.getHostTimezoneOffset();
      viability.setRecipientTimezone('recipient', hostTz + 12);

      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      const score = viability.calculateScore('msg-1');

      // 12 hours difference = 0 alignment
      // Score = 0*0.25 + 50*0.3 + 50*0.25 + 50*0.2 = 0 + 15 + 12.5 + 10 = 37.5 → 38
      expect(score).toBe(38);
    });

    it('should handle 24h timezone wrap correctly', () => {
      viability.updateHostFactors({
        connectionStability: 50,
        bandwidthCapacity: 50,
        contributionScore: 50,
      });

      // Simulate host at UTC+11, recipient at UTC-11
      // Raw diff = 22h, but actual distance is only 2h (wraps around)
      const hostTz = viability.getHostTimezoneOffset();
      viability.setRecipientTimezone('recipient', hostTz + 22); // +22 wraps to -2 equivalent

      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      const score = viability.calculateScore('msg-1');

      // After wrap fix: actualDiff = min(22, 24-22) = min(22, 2) = 2h
      // Timezone score = 100 - (2/12)*100 = 100 - 16.67 = 83.33 → 83
      // Total = 83*0.25 + 50*0.3 + 50*0.25 + 50*0.2 = 20.75 + 15 + 12.5 + 10 = 58.25 → 58
      expect(score).toBe(58);
    });
  });

  describe('threshold detection', () => {
    it('should emit replication needed when below threshold', () => {
      // Set low factors to get score below 30
      // Timezone unknown = 50, so: 50*0.25 + 20*0.3 + 20*0.25 + 20*0.2 = 12.5 + 6 + 5 + 4 = 27.5 → 28
      viability.updateHostFactors({
        connectionStability: 20,
        bandwidthCapacity: 20,
        contributionScore: 20,
      });

      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      viability.checkMessage('msg-1');

      expect(onReplicationNeeded).toHaveBeenCalledWith('msg-1', expect.any(Number));
    });

    it('should emit self-delete needed when below critical threshold', () => {
      // Set very low factors to get score below 10
      // Set recipient timezone 12h away (0 alignment): 0*0.25 + 5*0.3 + 5*0.25 + 5*0.2 = 0 + 1.5 + 1.25 + 1 = 3.75 → 4
      viability.updateHostFactors({
        connectionStability: 5,
        bandwidthCapacity: 5,
        contributionScore: 5,
      });

      // Set timezone 12 hours away to get 0 alignment score
      const hostTz = viability.getHostTimezoneOffset();
      viability.setRecipientTimezone('recipient', hostTz + 12);

      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      viability.checkMessage('msg-1');

      expect(onSelfDeleteNeeded).toHaveBeenCalledWith('msg-1', expect.any(Number));
    });

    it('should not emit events when score is healthy', () => {
      // Default factors give score of 50
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      viability.checkMessage('msg-1');

      expect(onReplicationNeeded).not.toHaveBeenCalled();
      expect(onSelfDeleteNeeded).not.toHaveBeenCalled();
    });
  });

  describe('host degradation', () => {
    it('should degrade host stability', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      const scoreBefore = viability.calculateScore('msg-1');
      viability.degradeHostStability(40);
      const scoreAfter = viability.calculateScore('msg-1');

      expect(scoreAfter).toBeLessThan(scoreBefore);
    });

    it('should trigger replication after degradation', () => {
      // Set factors low so that degradation pushes below 30% threshold
      // Unknown timezone = 50 alignment
      viability.updateHostFactors({
        connectionStability: 30,
        bandwidthCapacity: 30,
        contributionScore: 30,
      });

      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      // Score before: 50*0.25 + 30*0.3 + 30*0.25 + 30*0.2 = 12.5 + 9 + 7.5 + 6 = 35
      // After degradation: 50*0.25 + 10*0.3 + 30*0.25 + 30*0.2 = 12.5 + 3 + 7.5 + 6 = 29
      viability.degradeHostStability(20);

      expect(onReplicationNeeded).toHaveBeenCalled();
    });

    it('should improve host stability', () => {
      viability.updateHostFactors({ connectionStability: 30 });

      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      const scoreBefore = viability.calculateScore('msg-1');
      viability.improveHostStability(30);
      const scoreAfter = viability.calculateScore('msg-1');

      expect(scoreAfter).toBeGreaterThan(scoreBefore);
    });
  });

  describe('periodic checking', () => {
    it('should check all messages periodically', () => {
      const envelope1 = makeEnvelope('msg-1', 'sender', 'recipient-1');
      const envelope2 = makeEnvelope('msg-2', 'sender', 'recipient-2');
      backupStore.storeForRecipient('recipient-1', envelope1);
      backupStore.storeForRecipient('recipient-2', envelope2);

      viability.start();

      // Advance 30 seconds (check interval)
      vi.advanceTimersByTime(30000);

      // Both messages should have updated scores
      const msg1 = backupStore.getMessage('msg-1');
      const msg2 = backupStore.getMessage('msg-2');

      // Initial score is 100, after check it should be calculated (50 with defaults)
      expect(msg1?.viabilityScore).toBe(50);
      expect(msg2?.viabilityScore).toBe(50);
    });

    it('should stop periodic checking', () => {
      const checkSpy = vi.spyOn(viability, 'checkAllMessages');

      viability.start();
      viability.stop();

      vi.advanceTimersByTime(60000);

      // Should not be called after stop (or only once before stop)
      expect(checkSpy).not.toHaveBeenCalled();
    });
  });

  describe('getHostFactors', () => {
    it('should return copy of host factors', () => {
      viability.updateHostFactors({
        connectionStability: 75,
        bandwidthCapacity: 80,
      });

      const factors = viability.getHostFactors();

      expect(factors.connectionStability).toBe(75);
      expect(factors.bandwidthCapacity).toBe(80);

      // Should be a copy
      factors.connectionStability = 0;
      expect(viability.getHostFactors().connectionStability).toBe(75);
    });
  });

  describe('thresholds', () => {
    it('should have correct replication threshold', () => {
      expect(REPLICATION_THRESHOLD).toBe(30);
    });

    it('should have correct deletion threshold', () => {
      expect(DELETION_THRESHOLD).toBe(10);
    });
  });
});
