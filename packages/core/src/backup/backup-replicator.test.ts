import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { MessageEnvelope } from '../types/envelope.js';
import {
  BACKUP_REPLICATION_ACK_TYPE,
  BACKUP_REPLICATION_TYPE,
  BackupReplicator,
  type ReplicationAckPayload,
  type ReplicationPayload,
} from './backup-replicator.js';
import { BackupStore } from './backup-store.js';

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

describe('BackupReplicator', () => {
  let backupStore: BackupStore;
  let replicator: BackupReplicator;
  let onMessageReplicated: ReturnType<typeof vi.fn>;
  let onReplicationFailed: ReturnType<typeof vi.fn>;
  let sendEnvelope: ReturnType<typeof vi.fn>;

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

    onMessageReplicated = vi.fn();
    onReplicationFailed = vi.fn();
    sendEnvelope = vi.fn();

    replicator = new BackupReplicator(
      {
        onMessageReplicated,
        onReplicationFailed,
        sendEnvelope,
      },
      backupStore,
      'self-node-id',
    );
  });

  afterEach(() => {
    replicator.stop();
    backupStore.stop();
    vi.useRealTimers();
  });

  describe('replicateTo', () => {
    it('should send replication request to target node', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      replicator.replicateTo('msg-1', 'backup-node-1');

      expect(sendEnvelope).toHaveBeenCalledWith(
        'backup-node-1',
        expect.objectContaining({
          type: BACKUP_REPLICATION_TYPE,
          to: 'backup-node-1',
        }),
      );
    });

    it('should include message data in replication payload', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      replicator.replicateTo('msg-1', 'backup-node-1');

      const sentEnvelope = sendEnvelope.mock.calls[0][1] as MessageEnvelope;
      const payload = sentEnvelope.payload as ReplicationPayload;

      expect(payload.envelope.id).toBe('msg-1');
      expect(payload.recipientId).toBe('recipient');
      // Uses absolute expiresAt instead of relative ttlMs
      expect(payload.expiresAt).toBeGreaterThan(Date.now());
    });

    it('should not replicate to self', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      replicator.replicateTo('msg-1', 'self-node-id');

      expect(sendEnvelope).not.toHaveBeenCalled();
    });

    it('should not replicate if message not in store', () => {
      replicator.replicateTo('unknown-msg', 'backup-node-1');

      expect(sendEnvelope).not.toHaveBeenCalled();
    });

    it('should not replicate to node that already has copy', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);
      backupStore.recordReplication('msg-1', 'backup-node-1');

      replicator.replicateTo('msg-1', 'backup-node-1');

      expect(sendEnvelope).not.toHaveBeenCalled();
    });

    it('should not send duplicate replication requests', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      replicator.replicateTo('msg-1', 'backup-node-1');
      replicator.replicateTo('msg-1', 'backup-node-1');

      expect(sendEnvelope).toHaveBeenCalledTimes(1);
    });
  });

  describe('handleReplicationRequest', () => {
    it('should accept and store replicated message', () => {
      const originalEnvelope = makeEnvelope('msg-1', 'sender', 'recipient');

      const replicationEnvelope: MessageEnvelope = {
        id: 'repl-1',
        from: 'other-backup',
        to: 'self-node-id',
        via: [],
        type: BACKUP_REPLICATION_TYPE,
        payload: {
          envelope: originalEnvelope,
          recipientId: 'recipient',
          expiresAt: Date.now() + 60000,
          viabilityScore: 75,
          replicatedTo: [],
        } as ReplicationPayload,
        timestamp: Date.now(),
        signature: '',
      };

      const result = replicator.handleReplicationRequest(replicationEnvelope);

      expect(result).toBe(true);
      expect(backupStore.hasMessage('msg-1')).toBe(true);
    });

    it('should send ACK on successful replication', () => {
      const originalEnvelope = makeEnvelope('msg-1', 'sender', 'recipient');

      const replicationEnvelope: MessageEnvelope = {
        id: 'repl-1',
        from: 'other-backup',
        to: 'self-node-id',
        via: [],
        type: BACKUP_REPLICATION_TYPE,
        payload: {
          envelope: originalEnvelope,
          recipientId: 'recipient',
          expiresAt: Date.now() + 60000,
          viabilityScore: 75,
          replicatedTo: [],
        } as ReplicationPayload,
        timestamp: Date.now(),
        signature: '',
      };

      replicator.handleReplicationRequest(replicationEnvelope);

      expect(sendEnvelope).toHaveBeenCalledWith(
        'other-backup',
        expect.objectContaining({
          type: BACKUP_REPLICATION_ACK_TYPE,
        }),
      );

      const ackEnvelope = sendEnvelope.mock.calls[0][1] as MessageEnvelope;
      const ackPayload = ackEnvelope.payload as ReplicationAckPayload;
      expect(ackPayload.accepted).toBe(true);
      expect(ackPayload.messageId).toBe('msg-1');
    });

    it('should reject if message already expired', () => {
      const originalEnvelope = makeEnvelope('msg-1', 'sender', 'recipient');

      const replicationEnvelope: MessageEnvelope = {
        id: 'repl-1',
        from: 'other-backup',
        to: 'self-node-id',
        via: [],
        type: BACKUP_REPLICATION_TYPE,
        payload: {
          envelope: originalEnvelope,
          recipientId: 'recipient',
          expiresAt: Date.now() - 1000, // Already expired
          viabilityScore: 75,
          replicatedTo: [],
        } as ReplicationPayload,
        timestamp: Date.now(),
        signature: '',
      };

      const result = replicator.handleReplicationRequest(replicationEnvelope);

      expect(result).toBe(false);

      const ackEnvelope = sendEnvelope.mock.calls[0][1] as MessageEnvelope;
      const ackPayload = ackEnvelope.payload as ReplicationAckPayload;
      expect(ackPayload.accepted).toBe(false);
      expect(ackPayload.reason).toBe('expired');
    });

    it('should reject if message already stored', () => {
      const originalEnvelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', originalEnvelope);

      const replicationEnvelope: MessageEnvelope = {
        id: 'repl-1',
        from: 'other-backup',
        to: 'self-node-id',
        via: [],
        type: BACKUP_REPLICATION_TYPE,
        payload: {
          envelope: originalEnvelope,
          recipientId: 'recipient',
          expiresAt: Date.now() + 60000,
          viabilityScore: 75,
          replicatedTo: [],
        } as ReplicationPayload,
        timestamp: Date.now(),
        signature: '',
      };

      const result = replicator.handleReplicationRequest(replicationEnvelope);

      expect(result).toBe(false);

      const ackEnvelope = sendEnvelope.mock.calls[0][1] as MessageEnvelope;
      const ackPayload = ackEnvelope.payload as ReplicationAckPayload;
      expect(ackPayload.accepted).toBe(false);
      expect(ackPayload.reason).toBe('already-stored');
    });

    it('should preserve existing replication info', () => {
      const originalEnvelope = makeEnvelope('msg-1', 'sender', 'recipient');

      const replicationEnvelope: MessageEnvelope = {
        id: 'repl-1',
        from: 'other-backup',
        to: 'self-node-id',
        via: [],
        type: BACKUP_REPLICATION_TYPE,
        payload: {
          envelope: originalEnvelope,
          recipientId: 'recipient',
          expiresAt: Date.now() + 60000,
          viabilityScore: 75,
          replicatedTo: ['backup-a', 'backup-b'],
        } as ReplicationPayload,
        timestamp: Date.now(),
        signature: '',
      };

      replicator.handleReplicationRequest(replicationEnvelope);

      const replicated = backupStore.getReplicatedNodes('msg-1');
      expect(replicated).toContain('backup-a');
      expect(replicated).toContain('backup-b');
      expect(replicated).toContain('other-backup'); // Sender also recorded
    });

    it('should ignore non-replication envelopes', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');

      const result = replicator.handleReplicationRequest(envelope);

      expect(result).toBe(false);
      expect(backupStore.hasMessage('msg-1')).toBe(false);
    });
  });

  describe('handleReplicationAck', () => {
    it('should record successful replication on positive ACK', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);
      replicator.replicateTo('msg-1', 'backup-node-1');
      sendEnvelope.mockClear();

      const ackEnvelope: MessageEnvelope = {
        id: 'ack-1',
        from: 'backup-node-1',
        to: 'self-node-id',
        via: [],
        type: BACKUP_REPLICATION_ACK_TYPE,
        payload: {
          messageId: 'msg-1',
          accepted: true,
        } as ReplicationAckPayload,
        timestamp: Date.now(),
        signature: '',
      };

      replicator.handleReplicationAck(ackEnvelope);

      expect(onMessageReplicated).toHaveBeenCalledWith('msg-1', 'backup-node-1');
      expect(backupStore.getReplicatedNodes('msg-1')).toContain('backup-node-1');
    });

    it('should emit failure event on negative ACK', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);
      replicator.replicateTo('msg-1', 'backup-node-1');

      const ackEnvelope: MessageEnvelope = {
        id: 'ack-1',
        from: 'backup-node-1',
        to: 'self-node-id',
        via: [],
        type: BACKUP_REPLICATION_ACK_TYPE,
        payload: {
          messageId: 'msg-1',
          accepted: false,
          reason: 'no-capacity',
        } as ReplicationAckPayload,
        timestamp: Date.now(),
        signature: '',
      };

      replicator.handleReplicationAck(ackEnvelope);

      expect(onReplicationFailed).toHaveBeenCalledWith('msg-1', 'backup-node-1', 'no-capacity');
    });
  });

  describe('replicateToMultiple', () => {
    it('should replicate to multiple nodes in parallel', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      replicator.replicateToMultiple('msg-1', ['backup-1', 'backup-2', 'backup-3']);

      expect(sendEnvelope).toHaveBeenCalledTimes(3);
    });
  });

  describe('getReplicatedNodes', () => {
    it('should return nodes that have copies', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);
      backupStore.recordReplication('msg-1', 'backup-1');
      backupStore.recordReplication('msg-1', 'backup-2');

      const nodes = replicator.getReplicatedNodes('msg-1');

      expect(nodes).toContain('backup-1');
      expect(nodes).toContain('backup-2');
    });
  });

  describe('isReplicatedTo', () => {
    it('should check if message is replicated to specific node', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);
      backupStore.recordReplication('msg-1', 'backup-1');

      expect(replicator.isReplicatedTo('msg-1', 'backup-1')).toBe(true);
      expect(replicator.isReplicatedTo('msg-1', 'backup-2')).toBe(false);
    });
  });

  describe('pending replication timeout', () => {
    it('should timeout pending replications after 30 seconds', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      replicator.start();
      replicator.replicateTo('msg-1', 'backup-node-1');

      // Advance time past timeout (30s) + purge interval (10s)
      vi.advanceTimersByTime(40000);

      expect(onReplicationFailed).toHaveBeenCalledWith('msg-1', 'backup-node-1', 'timeout');
    });

    it('should allow retry after timeout clears pending', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      replicator.start();
      replicator.replicateTo('msg-1', 'backup-node-1');

      // First call
      expect(sendEnvelope).toHaveBeenCalledTimes(1);

      // Advance past timeout
      vi.advanceTimersByTime(40000);

      // Should now be able to retry
      sendEnvelope.mockClear();
      replicator.replicateTo('msg-1', 'backup-node-1');

      expect(sendEnvelope).toHaveBeenCalledTimes(1);
    });

    it('should cancel pending replications on explicit cancel', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);

      replicator.replicateTo('msg-1', 'backup-node-1');
      replicator.cancelPendingReplications('msg-1');

      // Should now be able to replicate again (pending was cleared)
      sendEnvelope.mockClear();
      replicator.replicateTo('msg-1', 'backup-node-1');

      expect(sendEnvelope).toHaveBeenCalledTimes(1);
    });
  });

  describe('race condition handling', () => {
    it('should handle ACK for message no longer in store', () => {
      const envelope = makeEnvelope('msg-1', 'sender', 'recipient');
      backupStore.storeForRecipient('recipient', envelope);
      replicator.replicateTo('msg-1', 'backup-node-1');

      // Message delivered before ACK arrives
      backupStore.markDelivered('msg-1');

      const ackEnvelope: MessageEnvelope = {
        id: 'ack-1',
        from: 'backup-node-1',
        to: 'self-node-id',
        via: [],
        type: BACKUP_REPLICATION_ACK_TYPE,
        payload: {
          messageId: 'msg-1',
          accepted: true,
        } as ReplicationAckPayload,
        timestamp: Date.now(),
        signature: '',
      };

      replicator.handleReplicationAck(ackEnvelope);

      // Should not crash and should not emit replicated event
      expect(onMessageReplicated).not.toHaveBeenCalled();
    });
  });
});
