import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { MessageEnvelope } from '../types/envelope.js';
import {
  BackupCoordinator,
  PENDING_QUERY_TYPE,
  PENDING_RESPONSE_TYPE,
  type PendingQueryPayload,
  type PendingResponsePayload,
  RECEIVED_CONFIRMATION_TYPE,
  type ReceivedConfirmationPayload,
} from './backup-coordinator.js';
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

describe('BackupCoordinator', () => {
  let backupStore: BackupStore;
  let coordinator: BackupCoordinator;
  let sendEnvelope: ReturnType<typeof vi.fn>;
  let broadcastToBackups: ReturnType<typeof vi.fn>;
  let onPendingMessagesReceived: ReturnType<typeof vi.fn>;
  let onBackupCleared: ReturnType<typeof vi.fn>;

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

    sendEnvelope = vi.fn();
    broadcastToBackups = vi.fn();
    onPendingMessagesReceived = vi.fn();
    onBackupCleared = vi.fn();

    coordinator = new BackupCoordinator(
      {
        sendEnvelope,
        broadcastToBackups,
        onPendingMessagesReceived,
        onBackupCleared,
      },
      backupStore,
      'self-node-id',
    );
  });

  afterEach(() => {
    backupStore.stop();
    vi.useRealTimers();
  });

  describe('queryPendingMessages', () => {
    it('should broadcast query to backup nodes', () => {
      const queryId = coordinator.queryPendingMessages('recipient-1');

      expect(broadcastToBackups).toHaveBeenCalledWith(
        expect.objectContaining({
          type: PENDING_QUERY_TYPE,
        }),
      );

      const sentEnvelope = broadcastToBackups.mock.calls[0][0] as MessageEnvelope;
      const payload = sentEnvelope.payload as PendingQueryPayload;
      expect(payload.recipientId).toBe('recipient-1');
      expect(payload.queryId).toBe(queryId);
    });

    it('should debounce queries for same recipient', () => {
      coordinator.queryPendingMessages('recipient-1');
      const result = coordinator.queryPendingMessages('recipient-1');

      expect(broadcastToBackups).toHaveBeenCalledTimes(1);
      expect(result).toBeNull(); // Debounced query returns null
    });

    it('should allow query after debounce period', () => {
      coordinator.queryPendingMessages('recipient-1');

      vi.advanceTimersByTime(6000);

      coordinator.queryPendingMessages('recipient-1');

      expect(broadcastToBackups).toHaveBeenCalledTimes(2);
    });

    it('should return unique queryId for each query', () => {
      const queryId1 = coordinator.queryPendingMessages('recipient-1');

      vi.advanceTimersByTime(6000);

      const queryId2 = coordinator.queryPendingMessages('recipient-1');

      expect(queryId1).not.toBe(queryId2);
    });
  });

  describe('handlePendingQuery', () => {
    it('should respond with stored messages when requester is recipient', () => {
      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-1');
      const msg2 = makeEnvelope('msg-2', 'sender', 'recipient-1');
      backupStore.storeForRecipient('recipient-1', msg1);
      backupStore.storeForRecipient('recipient-1', msg2);

      const queryEnvelope: MessageEnvelope = {
        id: 'query-1',
        from: 'recipient-1', // Requester IS the recipient (auth check passes)
        to: 'self-node-id',
        via: [],
        type: PENDING_QUERY_TYPE,
        payload: { queryId: 'q-123', recipientId: 'recipient-1' } as PendingQueryPayload,
        timestamp: Date.now(),
        signature: '',
      };

      coordinator.handlePendingQuery(queryEnvelope);

      expect(sendEnvelope).toHaveBeenCalledWith(
        'recipient-1',
        expect.objectContaining({
          type: PENDING_RESPONSE_TYPE,
        }),
      );

      const responseEnvelope = sendEnvelope.mock.calls[0][1] as MessageEnvelope;
      const responsePayload = responseEnvelope.payload as PendingResponsePayload;
      expect(responsePayload.messages).toHaveLength(2);
      expect(responsePayload.messageIds).toContain('msg-1');
      expect(responsePayload.messageIds).toContain('msg-2');
      expect(responsePayload.queryId).toBe('q-123');
      expect(responsePayload.recipientId).toBe('recipient-1');
    });

    it('should reject query when requester is not the recipient (auth check)', () => {
      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-1');
      backupStore.storeForRecipient('recipient-1', msg1);

      const queryEnvelope: MessageEnvelope = {
        id: 'query-1',
        from: 'malicious-node', // Requester is NOT the recipient
        to: 'self-node-id',
        via: [],
        type: PENDING_QUERY_TYPE,
        payload: { queryId: 'q-123', recipientId: 'recipient-1' } as PendingQueryPayload,
        timestamp: Date.now(),
        signature: '',
      };

      coordinator.handlePendingQuery(queryEnvelope);

      // Should not respond - auth check failed
      expect(sendEnvelope).not.toHaveBeenCalled();
    });

    it('should not respond if no messages for recipient', () => {
      const queryEnvelope: MessageEnvelope = {
        id: 'query-1',
        from: 'unknown-recipient',
        to: 'self-node-id',
        via: [],
        type: PENDING_QUERY_TYPE,
        payload: { queryId: 'q-123', recipientId: 'unknown-recipient' } as PendingQueryPayload,
        timestamp: Date.now(),
        signature: '',
      };

      coordinator.handlePendingQuery(queryEnvelope);

      expect(sendEnvelope).not.toHaveBeenCalled();
    });
  });

  describe('handlePendingResponse', () => {
    it('should deliver pending messages when query is active', () => {
      // First, send a query to create an active query
      const queryId = coordinator.queryPendingMessages('recipient-1');

      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-1');
      const msg2 = makeEnvelope('msg-2', 'sender', 'recipient-1');

      const responseEnvelope: MessageEnvelope = {
        id: 'response-1',
        from: 'backup-node',
        to: 'self-node-id',
        via: [],
        type: PENDING_RESPONSE_TYPE,
        payload: {
          queryId,
          recipientId: 'recipient-1',
          messages: [msg1, msg2],
          messageIds: ['msg-1', 'msg-2'],
        } as PendingResponsePayload,
        timestamp: Date.now(),
        signature: '',
      };

      const newMessages = coordinator.handlePendingResponse(responseEnvelope);

      expect(newMessages).toHaveLength(2);
      expect(onPendingMessagesReceived).toHaveBeenCalledWith([msg1, msg2]);
    });

    it('should reject response for unknown queryId', () => {
      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-1');

      const responseEnvelope: MessageEnvelope = {
        id: 'response-1',
        from: 'backup-node',
        to: 'self-node-id',
        via: [],
        type: PENDING_RESPONSE_TYPE,
        payload: {
          queryId: 'unknown-query-id',
          recipientId: 'recipient-1',
          messages: [msg1],
          messageIds: ['msg-1'],
        } as PendingResponsePayload,
        timestamp: Date.now(),
        signature: '',
      };

      const newMessages = coordinator.handlePendingResponse(responseEnvelope);

      expect(newMessages).toHaveLength(0);
      expect(onPendingMessagesReceived).not.toHaveBeenCalled();
    });

    it('should reject response with mismatched recipientId', () => {
      // Query for recipient-1
      const queryId = coordinator.queryPendingMessages('recipient-1');

      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-2'); // Wrong recipient

      const responseEnvelope: MessageEnvelope = {
        id: 'response-1',
        from: 'backup-node',
        to: 'self-node-id',
        via: [],
        type: PENDING_RESPONSE_TYPE,
        payload: {
          queryId,
          recipientId: 'recipient-2', // Mismatched recipient
          messages: [msg1],
          messageIds: ['msg-1'],
        } as PendingResponsePayload,
        timestamp: Date.now(),
        signature: '',
      };

      const newMessages = coordinator.handlePendingResponse(responseEnvelope);

      expect(newMessages).toHaveLength(0);
      expect(onPendingMessagesReceived).not.toHaveBeenCalled();
    });

    it('should filter messages not for queried recipient', () => {
      const queryId = coordinator.queryPendingMessages('recipient-1');

      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-1'); // Correct
      const msg2 = makeEnvelope('msg-2', 'sender', 'recipient-2'); // Wrong recipient

      const responseEnvelope: MessageEnvelope = {
        id: 'response-1',
        from: 'backup-node',
        to: 'self-node-id',
        via: [],
        type: PENDING_RESPONSE_TYPE,
        payload: {
          queryId,
          recipientId: 'recipient-1',
          messages: [msg1, msg2],
          messageIds: ['msg-1', 'msg-2'],
        } as PendingResponsePayload,
        timestamp: Date.now(),
        signature: '',
      };

      const newMessages = coordinator.handlePendingResponse(responseEnvelope);

      expect(newMessages).toHaveLength(1);
      expect(newMessages[0].id).toBe('msg-1');
    });

    it('should deduplicate messages', () => {
      const queryId = coordinator.queryPendingMessages('recipient-1');

      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-1');

      // First response
      const response1: MessageEnvelope = {
        id: 'response-1',
        from: 'backup-node-1',
        to: 'self-node-id',
        via: [],
        type: PENDING_RESPONSE_TYPE,
        payload: {
          queryId,
          recipientId: 'recipient-1',
          messages: [msg1],
          messageIds: ['msg-1'],
        } as PendingResponsePayload,
        timestamp: Date.now(),
        signature: '',
      };

      coordinator.handlePendingResponse(response1);

      // Second response with same message
      const response2: MessageEnvelope = {
        id: 'response-2',
        from: 'backup-node-2',
        to: 'self-node-id',
        via: [],
        type: PENDING_RESPONSE_TYPE,
        payload: {
          queryId,
          recipientId: 'recipient-1',
          messages: [msg1],
          messageIds: ['msg-1'],
        } as PendingResponsePayload,
        timestamp: Date.now(),
        signature: '',
      };

      const newMessages = coordinator.handlePendingResponse(response2);

      expect(newMessages).toHaveLength(0);
      expect(onPendingMessagesReceived).toHaveBeenCalledTimes(1); // Only once for first response
    });

    it('should reject responses after query timeout', () => {
      const queryId = coordinator.queryPendingMessages('recipient-1');

      // Advance time past query timeout (30s)
      vi.advanceTimersByTime(31000);

      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-1');

      const responseEnvelope: MessageEnvelope = {
        id: 'response-1',
        from: 'backup-node',
        to: 'self-node-id',
        via: [],
        type: PENDING_RESPONSE_TYPE,
        payload: {
          queryId,
          recipientId: 'recipient-1',
          messages: [msg1],
          messageIds: ['msg-1'],
        } as PendingResponsePayload,
        timestamp: Date.now(),
        signature: '',
      };

      const newMessages = coordinator.handlePendingResponse(responseEnvelope);

      expect(newMessages).toHaveLength(0);
      expect(onPendingMessagesReceived).not.toHaveBeenCalled();
    });
  });

  describe('confirmMessagesReceived', () => {
    it('should broadcast confirmation', () => {
      coordinator.confirmMessagesReceived(['msg-1', 'msg-2'], 'recipient-1');

      expect(broadcastToBackups).toHaveBeenCalledWith(
        expect.objectContaining({
          type: RECEIVED_CONFIRMATION_TYPE,
        }),
      );

      const confirmEnvelope = broadcastToBackups.mock.calls[0][0] as MessageEnvelope;
      const payload = confirmEnvelope.payload as ReceivedConfirmationPayload;
      expect(payload.messageIds).toEqual(['msg-1', 'msg-2']);
      expect(payload.recipientId).toBe('recipient-1');
    });

    it('should not broadcast if no message IDs', () => {
      coordinator.confirmMessagesReceived([], 'recipient-1');

      expect(broadcastToBackups).not.toHaveBeenCalled();
    });
  });

  describe('handleReceivedConfirmation', () => {
    it('should clear backup copies', () => {
      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-1');
      backupStore.storeForRecipient('recipient-1', msg1);

      const confirmEnvelope: MessageEnvelope = {
        id: 'confirm-1',
        from: 'other-node',
        to: 'self-node-id',
        via: [],
        type: RECEIVED_CONFIRMATION_TYPE,
        payload: {
          messageIds: ['msg-1'],
          recipientId: 'recipient-1',
        } as ReceivedConfirmationPayload,
        timestamp: Date.now(),
        signature: '',
      };

      coordinator.handleReceivedConfirmation(confirmEnvelope);

      expect(backupStore.hasMessage('msg-1')).toBe(false);
      expect(onBackupCleared).toHaveBeenCalledWith('msg-1');
    });

    it('should ignore messages not in store', () => {
      const confirmEnvelope: MessageEnvelope = {
        id: 'confirm-1',
        from: 'other-node',
        to: 'self-node-id',
        via: [],
        type: RECEIVED_CONFIRMATION_TYPE,
        payload: {
          messageIds: ['unknown-msg'],
          recipientId: 'recipient-1',
        } as ReceivedConfirmationPayload,
        timestamp: Date.now(),
        signature: '',
      };

      coordinator.handleReceivedConfirmation(confirmEnvelope);

      expect(onBackupCleared).not.toHaveBeenCalled();
    });
  });

  describe('handleEnvelope', () => {
    it('should handle pending query', () => {
      const queryEnvelope: MessageEnvelope = {
        id: 'query-1',
        from: 'recipient-1', // Must match recipientId for auth
        to: 'self-node-id',
        via: [],
        type: PENDING_QUERY_TYPE,
        payload: { queryId: 'q-123', recipientId: 'recipient-1' },
        timestamp: Date.now(),
        signature: '',
      };

      const handled = coordinator.handleEnvelope(queryEnvelope);
      expect(handled).toBe(true);
    });

    it('should return false for non-backup envelopes', () => {
      const regularEnvelope = makeEnvelope('msg-1', 'sender', 'recipient');
      const handled = coordinator.handleEnvelope(regularEnvelope);
      expect(handled).toBe(false);
    });
  });

  describe('isBackupEnvelope', () => {
    it('should identify backup-related envelopes', () => {
      expect(coordinator.isBackupEnvelope({ type: PENDING_QUERY_TYPE } as MessageEnvelope)).toBe(true);
      expect(coordinator.isBackupEnvelope({ type: PENDING_RESPONSE_TYPE } as MessageEnvelope)).toBe(true);
      expect(coordinator.isBackupEnvelope({ type: RECEIVED_CONFIRMATION_TYPE } as MessageEnvelope)).toBe(true);
      expect(coordinator.isBackupEnvelope({ type: 'message' } as MessageEnvelope)).toBe(false);
    });
  });

  describe('deduplication cache', () => {
    it('should track cache size', () => {
      const queryId = coordinator.queryPendingMessages('recipient-1');
      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-1');

      const response: MessageEnvelope = {
        id: 'response-1',
        from: 'backup-node',
        to: 'self-node-id',
        via: [],
        type: PENDING_RESPONSE_TYPE,
        payload: {
          queryId,
          recipientId: 'recipient-1',
          messages: [msg1],
          messageIds: ['msg-1'],
        } as PendingResponsePayload,
        timestamp: Date.now(),
        signature: '',
      };

      coordinator.handlePendingResponse(response);

      expect(coordinator.getDeduplicationCacheSize()).toBe(1);
    });

    it('should clear cache', () => {
      const queryId = coordinator.queryPendingMessages('recipient-1');
      const msg1 = makeEnvelope('msg-1', 'sender', 'recipient-1');

      const response: MessageEnvelope = {
        id: 'response-1',
        from: 'backup-node',
        to: 'self-node-id',
        via: [],
        type: PENDING_RESPONSE_TYPE,
        payload: {
          queryId,
          recipientId: 'recipient-1',
          messages: [msg1],
          messageIds: ['msg-1'],
        } as PendingResponsePayload,
        timestamp: Date.now(),
        signature: '',
      };

      coordinator.handlePendingResponse(response);
      coordinator.clearDeduplicationCache();

      expect(coordinator.getDeduplicationCacheSize()).toBe(0);
    });
  });
});
