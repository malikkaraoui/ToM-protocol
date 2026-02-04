/**
 * Group Types Tests (Story 4.6 - Group Messaging)
 */

import { describe, expect, it } from 'vitest';
import {
  type GroupCreatePayload,
  type GroupInvitePayload,
  type GroupMessagePayload,
  type GroupSyncPayload,
  isGroupCreate,
  isGroupInvite,
  isGroupMessage,
  isGroupPayload,
  isGroupSync,
} from './group-types';

describe('Group Type Guards', () => {
  describe('isGroupPayload', () => {
    it('should return true for valid group payloads', () => {
      const payloads = [
        { type: 'group-create', groupId: 'grp-123' },
        { type: 'group-message', groupId: 'grp-456' },
        { type: 'group-invite', groupId: 'grp-789' },
        { type: 'group-join', groupId: 'grp-abc' },
        { type: 'group-leave', groupId: 'grp-def' },
        { type: 'group-sync', groupId: 'grp-ghi' },
      ];

      for (const payload of payloads) {
        expect(isGroupPayload(payload)).toBe(true);
      }
    });

    it('should return false for invalid payloads', () => {
      const invalidPayloads = [
        null,
        undefined,
        {},
        { type: 'group-create' }, // Missing groupId
        { groupId: 'grp-123' }, // Missing type
        { type: 'not-a-group-type', groupId: 'grp-123' },
        { type: 'group-message', groupId: '' }, // Empty groupId
        { type: 123, groupId: 'grp-123' }, // Wrong type for type
        'not an object',
        42,
      ];

      for (const payload of invalidPayloads) {
        expect(isGroupPayload(payload)).toBe(false);
      }
    });
  });

  describe('isGroupMessage', () => {
    it('should return true for valid group messages', () => {
      const payload: GroupMessagePayload = {
        type: 'group-message',
        groupId: 'grp-123',
        messageId: 'msg-1',
        senderId: 'node-alice',
        senderUsername: 'Alice',
        text: 'Hello!',
        sentAt: Date.now(),
      };

      expect(isGroupMessage(payload)).toBe(true);
    });

    it('should return false for incomplete messages', () => {
      const invalidPayloads = [
        {
          type: 'group-message',
          groupId: 'grp-123',
          // Missing other fields
        },
        {
          type: 'group-message',
          groupId: 'grp-123',
          messageId: 'msg-1',
          senderId: 'node-alice',
          // Missing senderUsername, text, sentAt
        },
        {
          type: 'group-message',
          groupId: 'grp-123',
          messageId: 123, // Wrong type
          senderId: 'node-alice',
          senderUsername: 'Alice',
          text: 'Hello!',
          sentAt: Date.now(),
        },
      ];

      for (const payload of invalidPayloads) {
        expect(isGroupMessage(payload)).toBe(false);
      }
    });
  });

  describe('isGroupInvite', () => {
    it('should return true for valid invites', () => {
      const payload: GroupInvitePayload = {
        type: 'group-invite',
        groupId: 'grp-123',
        inviteeId: 'node-bob',
        inviteeUsername: 'Bob',
        inviterId: 'node-alice',
        inviterUsername: 'Alice',
        groupName: 'Fun Group',
        memberCount: 3,
      };

      expect(isGroupInvite(payload)).toBe(true);
    });

    it('should return false for incomplete invites', () => {
      const invalidPayloads = [
        {
          type: 'group-invite',
          groupId: 'grp-123',
          // Missing other fields
        },
        {
          type: 'group-invite',
          groupId: 'grp-123',
          inviteeId: 'node-bob',
          // Missing inviterId, groupName
        },
      ];

      for (const payload of invalidPayloads) {
        expect(isGroupInvite(payload)).toBe(false);
      }
    });
  });

  describe('isGroupCreate', () => {
    it('should return true for valid create payloads', () => {
      const payload: GroupCreatePayload = {
        type: 'group-create',
        groupId: 'grp-123',
        name: 'New Group',
        initialMembers: [],
      };

      expect(isGroupCreate(payload)).toBe(true);
    });

    it('should return true with initial members', () => {
      const payload: GroupCreatePayload = {
        type: 'group-create',
        groupId: 'grp-123',
        name: 'New Group',
        initialMembers: [
          { nodeId: 'node-bob', username: 'Bob' },
          { nodeId: 'node-carol', username: 'Carol' },
        ],
      };

      expect(isGroupCreate(payload)).toBe(true);
    });

    it('should return false for invalid create payloads', () => {
      const invalidPayloads = [
        {
          type: 'group-create',
          groupId: 'grp-123',
          // Missing name and initialMembers
        },
        {
          type: 'group-create',
          groupId: 'grp-123',
          name: 'New Group',
          initialMembers: 'not an array',
        },
      ];

      for (const payload of invalidPayloads) {
        expect(isGroupCreate(payload)).toBe(false);
      }
    });
  });

  describe('isGroupSync', () => {
    it('should return true for valid sync payloads', () => {
      const payload: GroupSyncPayload = {
        type: 'group-sync',
        groupId: 'grp-123',
        groupInfo: {
          groupId: 'grp-123',
          name: 'Test Group',
          hubRelayId: 'relay-1',
          members: [],
          createdBy: 'node-alice',
          createdAt: Date.now(),
          lastActivityAt: Date.now(),
          maxMembers: 50,
        },
      };

      expect(isGroupSync(payload)).toBe(true);
    });

    it('should return true with recent messages', () => {
      const payload: GroupSyncPayload = {
        type: 'group-sync',
        groupId: 'grp-123',
        groupInfo: {
          groupId: 'grp-123',
          name: 'Test Group',
          hubRelayId: 'relay-1',
          members: [],
          createdBy: 'node-alice',
          createdAt: Date.now(),
          lastActivityAt: Date.now(),
          maxMembers: 50,
        },
        recentMessages: [
          {
            type: 'group-message',
            groupId: 'grp-123',
            messageId: 'msg-1',
            senderId: 'node-alice',
            senderUsername: 'Alice',
            text: 'Hello!',
            sentAt: Date.now(),
          },
        ],
      };

      expect(isGroupSync(payload)).toBe(true);
    });

    it('should return false for invalid sync payloads', () => {
      const invalidPayloads = [
        {
          type: 'group-sync',
          groupId: 'grp-123',
          // Missing groupInfo
        },
        {
          type: 'group-sync',
          groupId: 'grp-123',
          groupInfo: {
            // Missing groupId
            name: 'Test',
          },
        },
      ];

      for (const payload of invalidPayloads) {
        expect(isGroupSync(payload)).toBe(false);
      }
    });
  });
});
