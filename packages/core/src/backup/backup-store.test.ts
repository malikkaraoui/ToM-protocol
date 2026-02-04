import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { MessageEnvelope } from '../types/envelope.js';
import { BackupStore, DEFAULT_TTL_MS, MAX_TTL_MS } from './backup-store.js';

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

describe('BackupStore', () => {
  let store: BackupStore;
  let onMessageStored: ReturnType<typeof vi.fn>;
  let onMessageExpired: ReturnType<typeof vi.fn>;
  let onMessageDelivered: ReturnType<typeof vi.fn>;
  let onViabilityChanged: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    vi.useFakeTimers();
    onMessageStored = vi.fn();
    onMessageExpired = vi.fn();
    onMessageDelivered = vi.fn();
    onViabilityChanged = vi.fn();
    store = new BackupStore(
      {
        onMessageStored,
        onMessageExpired,
        onMessageDelivered,
        onViabilityChanged,
      },
      { autoStart: false },
    );
  });

  afterEach(() => {
    store.stop();
    vi.useRealTimers();
  });

  describe('storing messages', () => {
    it('should store message for recipient', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);

      expect(store.hasMessage('msg-1')).toBe(true);
      expect(store.size).toBe(1);
      expect(onMessageStored).toHaveBeenCalledWith('msg-1', 'recipient-1');
    });

    it('should not store duplicate messages', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);
      store.storeForRecipient('recipient-1', envelope);

      expect(store.size).toBe(1);
      expect(onMessageStored).toHaveBeenCalledTimes(1);
    });

    it('should enforce max TTL', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      const excessiveTtl = MAX_TTL_MS * 2;
      store.storeForRecipient('recipient-1', envelope, excessiveTtl);

      const msg = store.getMessage('msg-1');
      expect(msg?.ttlMs).toBe(MAX_TTL_MS);
    });

    it('should use default TTL if not specified', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);

      const msg = store.getMessage('msg-1');
      expect(msg?.ttlMs).toBe(DEFAULT_TTL_MS);
    });

    it('should store multiple messages for same recipient', () => {
      const envelope1 = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      const envelope2 = makeEnvelope('msg-2', 'sender-2', 'recipient-1');

      store.storeForRecipient('recipient-1', envelope1);
      store.storeForRecipient('recipient-1', envelope2);

      expect(store.getRecipientMessageCount('recipient-1')).toBe(2);
    });

    it('should initialize viability score to 100', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);

      const msg = store.getMessage('msg-1');
      expect(msg?.viabilityScore).toBe(100);
    });
  });

  describe('retrieving messages', () => {
    it('should get messages for recipient', () => {
      const envelope1 = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      const envelope2 = makeEnvelope('msg-2', 'sender-2', 'recipient-1');
      const envelope3 = makeEnvelope('msg-3', 'sender-3', 'recipient-2');

      store.storeForRecipient('recipient-1', envelope1);
      store.storeForRecipient('recipient-1', envelope2);
      store.storeForRecipient('recipient-2', envelope3);

      const recipient1Messages = store.getMessagesForRecipient('recipient-1');
      expect(recipient1Messages).toHaveLength(2);
      expect(recipient1Messages.map((m) => m.envelope.id)).toContain('msg-1');
      expect(recipient1Messages.map((m) => m.envelope.id)).toContain('msg-2');
    });

    it('should return empty array for unknown recipient', () => {
      const messages = store.getMessagesForRecipient('unknown');
      expect(messages).toEqual([]);
    });

    it('should get specific message by ID', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);

      const msg = store.getMessage('msg-1');
      expect(msg?.envelope.id).toBe('msg-1');
      expect(msg?.recipientId).toBe('recipient-1');
    });

    it('should return undefined for unknown message', () => {
      const msg = store.getMessage('unknown');
      expect(msg).toBeUndefined();
    });

    it('should get all messages', () => {
      store.storeForRecipient('recipient-1', makeEnvelope('msg-1', 's1', 'recipient-1'));
      store.storeForRecipient('recipient-2', makeEnvelope('msg-2', 's2', 'recipient-2'));

      const all = store.getAllMessages();
      expect(all).toHaveLength(2);
    });
  });

  describe('delivery', () => {
    it('should mark message as delivered and remove it', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);

      const result = store.markDelivered('msg-1');

      expect(result).toBe(true);
      expect(store.hasMessage('msg-1')).toBe(false);
      expect(onMessageDelivered).toHaveBeenCalledWith('msg-1', 'recipient-1');
    });

    it('should return false for unknown message', () => {
      const result = store.markDelivered('unknown');
      expect(result).toBe(false);
    });

    it('should update recipient index on delivery', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);
      store.markDelivered('msg-1');

      expect(store.getRecipientMessageCount('recipient-1')).toBe(0);
    });
  });

  describe('viability scoring', () => {
    it('should update viability score', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);

      store.updateViabilityScore('msg-1', 75);

      const msg = store.getMessage('msg-1');
      expect(msg?.viabilityScore).toBe(75);
      expect(onViabilityChanged).toHaveBeenCalledWith('msg-1', 100, 75);
    });

    it('should clamp viability score to 0-100', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);

      store.updateViabilityScore('msg-1', 150);
      expect(store.getMessage('msg-1')?.viabilityScore).toBe(100);

      store.updateViabilityScore('msg-1', -10);
      expect(store.getMessage('msg-1')?.viabilityScore).toBe(0);
    });

    it('should not emit event if score unchanged', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);

      store.updateViabilityScore('msg-1', 100);

      expect(onViabilityChanged).not.toHaveBeenCalled();
    });
  });

  describe('replication tracking', () => {
    it('should record replication to other nodes', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);

      store.recordReplication('msg-1', 'backup-node-1');
      store.recordReplication('msg-1', 'backup-node-2');

      const replicated = store.getReplicatedNodes('msg-1');
      expect(replicated).toContain('backup-node-1');
      expect(replicated).toContain('backup-node-2');
    });

    it('should return empty array for unknown message', () => {
      const replicated = store.getReplicatedNodes('unknown');
      expect(replicated).toEqual([]);
    });
  });

  describe('TTL and expiration', () => {
    it('should detect expired message', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope, 1000); // 1 second TTL

      expect(store.isExpired('msg-1')).toBe(false);

      vi.advanceTimersByTime(1000);

      expect(store.isExpired('msg-1')).toBe(true);
    });

    it('should calculate remaining TTL', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope, 10000); // 10 seconds

      vi.advanceTimersByTime(3000);

      const remaining = store.getRemainingTtl('msg-1');
      expect(remaining).toBe(7000);
    });

    it('should return 0 for expired message TTL', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope, 1000);

      vi.advanceTimersByTime(2000);

      expect(store.getRemainingTtl('msg-1')).toBe(0);
    });

    it('should cleanup expired messages periodically', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope, 30000); // 30 seconds

      store.start();

      // Advance past TTL and cleanup interval
      vi.advanceTimersByTime(90000); // 90 seconds

      expect(store.hasMessage('msg-1')).toBe(false);
      expect(onMessageExpired).toHaveBeenCalledWith('msg-1', 'recipient-1');
    });

    it('should not cleanup non-expired messages', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope, 120000); // 2 minutes

      store.start();

      // Advance 1 minute (cleanup runs but message not expired)
      vi.advanceTimersByTime(60000);

      expect(store.hasMessage('msg-1')).toBe(true);
    });
  });

  describe('self-deletion', () => {
    it('should allow explicit message deletion', () => {
      const envelope = makeEnvelope('msg-1', 'sender-1', 'recipient-1');
      store.storeForRecipient('recipient-1', envelope);

      const result = store.deleteMessage('msg-1');

      expect(result).toBe(true);
      expect(store.hasMessage('msg-1')).toBe(false);
    });

    it('should return false when deleting unknown message', () => {
      const result = store.deleteMessage('unknown');
      expect(result).toBe(false);
    });
  });

  describe('clear', () => {
    it('should clear all messages', () => {
      store.storeForRecipient('r1', makeEnvelope('msg-1', 's1', 'r1'));
      store.storeForRecipient('r2', makeEnvelope('msg-2', 's2', 'r2'));

      store.clear();

      expect(store.size).toBe(0);
      expect(store.getRecipientMessageCount('r1')).toBe(0);
    });
  });
});
