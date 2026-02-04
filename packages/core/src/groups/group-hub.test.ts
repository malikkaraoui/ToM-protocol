/**
 * GroupHub Tests (Story 4.6 - Group Messaging)
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GroupHub, type GroupHubEvents } from './group-hub';
import type { GroupCreatePayload, GroupJoinPayload, GroupLeavePayload, GroupMessagePayload } from './group-types';

describe('GroupHub', () => {
  let hub: GroupHub;
  let events: GroupHubEvents;
  const hubNodeId = 'relay-hub-1';

  beforeEach(() => {
    events = {
      sendToNode: vi.fn(),
      broadcastToGroup: vi.fn(),
      onHubActivity: vi.fn(),
    };
    hub = new GroupHub(hubNodeId, events);
  });

  describe('group creation', () => {
    it('should create group and send confirmation', () => {
      const payload: GroupCreatePayload = {
        type: 'group-create',
        groupId: 'grp-123',
        name: 'Test Group',
        initialMembers: [],
      };

      hub.handlePayload(payload, 'node-alice');

      expect(events.sendToNode).toHaveBeenCalledWith(
        'node-alice',
        expect.objectContaining({
          type: 'group-created',
          groupId: 'grp-123',
        }),
        'grp-123',
      );

      expect(hub.getGroup('grp-123')).not.toBeNull();
      expect(hub.getGroup('grp-123')?.createdBy).toBe('node-alice');
    });

    it('should send invitations to initial members', () => {
      const payload: GroupCreatePayload = {
        type: 'group-create',
        groupId: 'grp-123',
        name: 'Test Group',
        initialMembers: [
          { nodeId: 'node-bob', username: 'Bob' },
          { nodeId: 'node-carol', username: 'Carol' },
        ],
      };

      hub.handlePayload(payload, 'node-alice');

      // Should send invites to both Bob and Carol
      expect(events.sendToNode).toHaveBeenCalledWith(
        'node-bob',
        expect.objectContaining({
          type: 'group-invite',
          groupId: 'grp-123',
        }),
        'grp-123',
      );
      expect(events.sendToNode).toHaveBeenCalledWith(
        'node-carol',
        expect.objectContaining({
          type: 'group-invite',
        }),
        'grp-123',
      );
    });

    it('should reject creation if max groups reached', () => {
      const limitedHub = new GroupHub(hubNodeId, events, { maxGroups: 2 });

      for (let i = 0; i < 3; i++) {
        limitedHub.handlePayload(
          {
            type: 'group-create',
            groupId: `grp-${i}`,
            name: `Group ${i}`,
            initialMembers: [],
          },
          'node-alice',
        );
      }

      expect(limitedHub.getAllGroups()).toHaveLength(2);
      expect(events.onHubActivity).toHaveBeenCalledWith('grp-2', 'create-rejected', { reason: 'max-groups' });
    });
  });

  describe('join/leave', () => {
    beforeEach(() => {
      // Create a group first
      hub.handlePayload(
        {
          type: 'group-create',
          groupId: 'grp-123',
          name: 'Test Group',
          initialMembers: [],
        },
        'node-alice',
      );
    });

    it('should handle member join', () => {
      const joinPayload: GroupJoinPayload = {
        type: 'group-join',
        groupId: 'grp-123',
        nodeId: 'node-bob',
        username: 'Bob',
      };

      hub.handlePayload(joinPayload, 'node-bob');

      // Should send sync to new member
      expect(events.sendToNode).toHaveBeenCalledWith(
        'node-bob',
        expect.objectContaining({
          type: 'group-sync',
          groupId: 'grp-123',
        }),
        'grp-123',
      );

      // Should broadcast join to existing members
      expect(events.broadcastToGroup).toHaveBeenCalledWith(
        'grp-123',
        expect.objectContaining({
          type: 'group-member-joined',
        }),
        'node-bob',
      );

      expect(hub.isMember('grp-123', 'node-bob')).toBe(true);
    });

    it('should reject join for unknown group', () => {
      const joinPayload: GroupJoinPayload = {
        type: 'group-join',
        groupId: 'unknown-group',
        nodeId: 'node-bob',
        username: 'Bob',
      };

      hub.handlePayload(joinPayload, 'node-bob');

      expect(events.onHubActivity).toHaveBeenCalledWith('unknown-group', 'join-rejected', {
        reason: 'group-not-found',
      });
    });

    it('should reject join if max members reached', () => {
      // Create a group with max 2 members
      hub.handlePayload(
        {
          type: 'group-create',
          groupId: 'grp-small',
          name: 'Small Group',
          initialMembers: [],
          maxMembers: 2,
        },
        'node-alice',
      );

      // Bob joins
      hub.handlePayload(
        {
          type: 'group-join',
          groupId: 'grp-small',
          nodeId: 'node-bob',
          username: 'Bob',
        },
        'node-bob',
      );

      // Carol tries to join - should be rejected
      hub.handlePayload(
        {
          type: 'group-join',
          groupId: 'grp-small',
          nodeId: 'node-carol',
          username: 'Carol',
        },
        'node-carol',
      );

      expect(hub.isMember('grp-small', 'node-carol')).toBe(false);
    });

    it('should handle member leave', () => {
      // Bob joins
      hub.handlePayload(
        {
          type: 'group-join',
          groupId: 'grp-123',
          nodeId: 'node-bob',
          username: 'Bob',
        },
        'node-bob',
      );

      // Bob leaves
      const leavePayload: GroupLeavePayload = {
        type: 'group-leave',
        groupId: 'grp-123',
        nodeId: 'node-bob',
      };

      hub.handlePayload(leavePayload, 'node-bob');

      expect(hub.isMember('grp-123', 'node-bob')).toBe(false);
      expect(events.broadcastToGroup).toHaveBeenCalledWith(
        'grp-123',
        expect.objectContaining({
          type: 'group-member-left',
          nodeId: 'node-bob',
        }),
      );
    });

    it('should delete group when last member leaves', () => {
      const leavePayload: GroupLeavePayload = {
        type: 'group-leave',
        groupId: 'grp-123',
        nodeId: 'node-alice',
      };

      hub.handlePayload(leavePayload, 'node-alice');

      expect(hub.getGroup('grp-123')).toBeNull();
      expect(events.onHubActivity).toHaveBeenCalledWith('grp-123', 'group-deleted', { reason: 'no-members' });
    });
  });

  describe('messages', () => {
    beforeEach(() => {
      // Create group with Alice and Bob
      hub.handlePayload(
        {
          type: 'group-create',
          groupId: 'grp-123',
          name: 'Test Group',
          initialMembers: [],
        },
        'node-alice',
      );
      hub.handlePayload(
        {
          type: 'group-join',
          groupId: 'grp-123',
          nodeId: 'node-bob',
          username: 'Bob',
        },
        'node-bob',
      );
    });

    it('should fan out messages to all members', () => {
      const message: GroupMessagePayload = {
        type: 'group-message',
        groupId: 'grp-123',
        messageId: 'msg-1',
        senderId: 'node-alice',
        senderUsername: 'Alice',
        text: 'Hello everyone!',
        sentAt: Date.now(),
      };

      hub.handlePayload(message, 'node-alice');

      expect(events.broadcastToGroup).toHaveBeenCalledWith('grp-123', message, 'node-alice');
    });

    it('should reject messages from non-members', () => {
      const message: GroupMessagePayload = {
        type: 'group-message',
        groupId: 'grp-123',
        messageId: 'msg-1',
        senderId: 'node-eve',
        senderUsername: 'Eve',
        text: 'I am not a member!',
        sentAt: Date.now(),
      };

      hub.handlePayload(message, 'node-eve');

      expect(events.onHubActivity).toHaveBeenCalledWith('grp-123', 'message-rejected', { reason: 'not-member' });
      expect(events.broadcastToGroup).not.toHaveBeenCalledWith('grp-123', message, expect.anything());
    });

    it('should rate limit messages', () => {
      vi.useFakeTimers();

      // Send many messages quickly
      for (let i = 0; i < 10; i++) {
        hub.handlePayload(
          {
            type: 'group-message',
            groupId: 'grp-123',
            messageId: `msg-${i}`,
            senderId: 'node-alice',
            senderUsername: 'Alice',
            text: `Message ${i}`,
            sentAt: Date.now(),
          },
          'node-alice',
        );
      }

      // Should have rate limited some messages
      expect(events.onHubActivity).toHaveBeenCalledWith('grp-123', 'message-rejected', { reason: 'rate-limited' });

      vi.useRealTimers();
    });

    it('should store messages for sync', () => {
      // Send a message
      hub.handlePayload(
        {
          type: 'group-message',
          groupId: 'grp-123',
          messageId: 'msg-1',
          senderId: 'node-alice',
          senderUsername: 'Alice',
          text: 'Hello!',
          sentAt: Date.now(),
        },
        'node-alice',
      );

      // Carol joins
      hub.handlePayload(
        {
          type: 'group-join',
          groupId: 'grp-123',
          nodeId: 'node-carol',
          username: 'Carol',
        },
        'node-carol',
      );

      // Sync should include the message
      expect(events.sendToNode).toHaveBeenCalledWith(
        'node-carol',
        expect.objectContaining({
          type: 'group-sync',
          recentMessages: expect.arrayContaining([
            expect.objectContaining({
              messageId: 'msg-1',
            }),
          ]),
        }),
        'grp-123',
      );
    });
  });

  describe('admin functions', () => {
    beforeEach(() => {
      hub.handlePayload(
        {
          type: 'group-create',
          groupId: 'grp-123',
          name: 'Test Group',
          initialMembers: [],
        },
        'node-alice',
      );
    });

    it('should allow admin to invite members', () => {
      const result = hub.inviteMember('grp-123', 'node-bob', 'Bob', 'node-alice', 'Alice');

      expect(result).toBe(true);
      expect(events.sendToNode).toHaveBeenCalledWith(
        'node-bob',
        expect.objectContaining({
          type: 'group-invite',
          inviteeId: 'node-bob',
          inviterId: 'node-alice',
        }),
        'grp-123',
      );
    });

    it('should reject invite from non-admin', () => {
      // Bob joins as regular member
      hub.handlePayload(
        {
          type: 'group-join',
          groupId: 'grp-123',
          nodeId: 'node-bob',
          username: 'Bob',
        },
        'node-bob',
      );

      // Bob tries to invite Carol
      const result = hub.inviteMember('grp-123', 'node-carol', 'Carol', 'node-bob', 'Bob');

      expect(result).toBe(false);
    });
  });

  describe('statistics', () => {
    it('should track hub statistics', () => {
      hub.handlePayload(
        {
          type: 'group-create',
          groupId: 'grp-1',
          name: 'Group 1',
          initialMembers: [],
        },
        'node-alice',
      );
      hub.handlePayload(
        {
          type: 'group-create',
          groupId: 'grp-2',
          name: 'Group 2',
          initialMembers: [],
        },
        'node-bob',
      );
      hub.handlePayload(
        {
          type: 'group-join',
          groupId: 'grp-1',
          nodeId: 'node-bob',
          username: 'Bob',
        },
        'node-bob',
      );

      const stats = hub.getStats();

      expect(stats.groupCount).toBe(2);
      expect(stats.totalMembers).toBe(3); // Alice in grp-1, Bob in grp-1 and grp-2
    });
  });

  describe('cleanup', () => {
    it('should clean up stale rate limit entries', () => {
      vi.useFakeTimers();

      // Send a message to create rate limit entry
      hub.handlePayload(
        {
          type: 'group-create',
          groupId: 'grp-123',
          name: 'Test Group',
          initialMembers: [],
        },
        'node-alice',
      );
      hub.handlePayload(
        {
          type: 'group-message',
          groupId: 'grp-123',
          messageId: 'msg-1',
          senderId: 'node-alice',
          senderUsername: 'Alice',
          text: 'Hello!',
          sentAt: Date.now(),
        },
        'node-alice',
      );

      // Advance time past stale threshold
      vi.advanceTimersByTime(120_000);

      // Cleanup should not throw
      hub.cleanupRateLimits();

      vi.useRealTimers();
    });
  });

  describe('hub migration', () => {
    beforeEach(() => {
      // Create group with messages
      hub.handlePayload(
        {
          type: 'group-create',
          groupId: 'grp-123',
          name: 'Test Group',
          initialMembers: [],
        },
        'node-alice',
      );
      hub.handlePayload(
        {
          type: 'group-join',
          groupId: 'grp-123',
          nodeId: 'node-bob',
          username: 'Bob',
        },
        'node-bob',
      );
      hub.handlePayload(
        {
          type: 'group-message',
          groupId: 'grp-123',
          messageId: 'msg-1',
          senderId: 'node-alice',
          senderUsername: 'Alice',
          text: 'Hello!',
          sentAt: Date.now(),
        },
        'node-alice',
      );
    });

    it('should export group data for migration', () => {
      const migrationData = hub.exportGroupForMigration('grp-123');

      expect(migrationData).not.toBeNull();
      expect(migrationData?.groupInfo.groupId).toBe('grp-123');
      expect(migrationData?.groupInfo.members).toHaveLength(2);
      expect(migrationData?.messageHistory).toHaveLength(1);
    });

    it('should return null for unknown group', () => {
      const migrationData = hub.exportGroupForMigration('unknown');
      expect(migrationData).toBeNull();
    });

    it('should import group from migration data', () => {
      const migrationData = hub.exportGroupForMigration('grp-123');
      expect(migrationData).not.toBeNull();

      // Create new hub and import
      const newHub = new GroupHub('relay-hub-2', events);
      const result = newHub.importGroupFromMigration(migrationData!);

      expect(result).toBe(true);
      expect(newHub.getGroup('grp-123')).not.toBeNull();
      expect(newHub.getGroup('grp-123')?.hubRelayId).toBe('relay-hub-2');
      expect(newHub.isMember('grp-123', 'node-alice')).toBe(true);
      expect(newHub.isMember('grp-123', 'node-bob')).toBe(true);
    });

    it('should reject import if max groups reached', () => {
      const limitedHub = new GroupHub('relay-hub-2', events, { maxGroups: 0 });
      const migrationData = hub.exportGroupForMigration('grp-123');

      const result = limitedHub.importGroupFromMigration(migrationData!);

      expect(result).toBe(false);
    });

    it('should initiate hub migration and notify members', () => {
      hub.initiateHubMigration('grp-123', 'relay-hub-2', 'failure');

      expect(events.broadcastToGroup).toHaveBeenCalledWith(
        'grp-123',
        expect.objectContaining({
          type: 'group-hub-migration',
          newHubId: 'relay-hub-2',
          oldHubId: hubNodeId,
          reason: 'failure',
        }),
      );

      // Group should be removed from old hub
      expect(hub.getGroup('grp-123')).toBeNull();
    });

    it('should start and stop heartbeats', () => {
      vi.useFakeTimers();

      hub.startHeartbeats();

      // Advance time to trigger heartbeat
      vi.advanceTimersByTime(35_000);

      expect(events.broadcastToGroup).toHaveBeenCalledWith(
        'grp-123',
        expect.objectContaining({
          type: 'group-hub-heartbeat',
        }),
      );

      hub.stopHeartbeats();
      vi.useRealTimers();
    });

    it('should shutdown and migrate all groups', () => {
      hub.shutdown('relay-hub-backup');

      expect(events.broadcastToGroup).toHaveBeenCalledWith(
        'grp-123',
        expect.objectContaining({
          type: 'group-hub-migration',
          newHubId: 'relay-hub-backup',
        }),
      );
    });
  });
});
