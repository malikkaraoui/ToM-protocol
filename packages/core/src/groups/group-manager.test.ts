/**
 * GroupManager Tests (Story 4.6 - Group Messaging)
 */

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GroupManager } from './group-manager';
import type { GroupInfo, GroupMember, GroupMessagePayload } from './group-types';

describe('GroupManager', () => {
  let manager: GroupManager;
  const localNodeId = 'node-alice';
  const localUsername = 'Alice';

  beforeEach(() => {
    manager = new GroupManager(localNodeId, localUsername);
  });

  describe('createGroup', () => {
    it('should create a group with the local node as admin', () => {
      const group = manager.createGroup('Test Group', 'relay-1');

      expect(group).not.toBeNull();
      expect(group?.name).toBe('Test Group');
      expect(group?.hubRelayId).toBe('relay-1');
      expect(group?.createdBy).toBe(localNodeId);
      expect(group?.members).toHaveLength(1);
      expect(group?.members[0].nodeId).toBe(localNodeId);
      expect(group?.members[0].role).toBe('admin');
    });

    it('should generate a unique groupId', () => {
      const group1 = manager.createGroup('Group 1', 'relay-1');
      const group2 = manager.createGroup('Group 2', 'relay-1');

      expect(group1?.groupId).not.toBe(group2?.groupId);
      expect(group1?.groupId).toMatch(/^grp-/);
    });

    it('should respect maxGroups limit', () => {
      const limitedManager = new GroupManager(localNodeId, localUsername, {}, { maxGroups: 2 });

      const group1 = limitedManager.createGroup('Group 1', 'relay-1');
      const group2 = limitedManager.createGroup('Group 2', 'relay-1');
      const group3 = limitedManager.createGroup('Group 3', 'relay-1');

      expect(group1).not.toBeNull();
      expect(group2).not.toBeNull();
      expect(group3).toBeNull();
    });
  });

  describe('handleGroupCreated', () => {
    it('should store group from hub confirmation', () => {
      const group: GroupInfo = {
        groupId: 'grp-123',
        name: 'Test Group',
        hubRelayId: 'relay-1',
        members: [{ nodeId: localNodeId, username: localUsername, joinedAt: Date.now(), role: 'admin' }],
        createdBy: localNodeId,
        createdAt: Date.now(),
        lastActivityAt: Date.now(),
        maxMembers: 50,
      };

      const onGroupCreated = vi.fn();
      const eventManager = new GroupManager(localNodeId, localUsername, { onGroupCreated });

      eventManager.handleGroupCreated(group);

      expect(eventManager.isInGroup('grp-123')).toBe(true);
      expect(onGroupCreated).toHaveBeenCalledWith(group);
    });
  });

  describe('invitations', () => {
    it('should handle incoming invitations', () => {
      const onGroupInvite = vi.fn();
      const eventManager = new GroupManager(localNodeId, localUsername, { onGroupInvite });

      eventManager.handleInvite('grp-123', 'Fun Group', 'node-bob', 'Bob', 'relay-1');

      expect(onGroupInvite).toHaveBeenCalledWith('grp-123', 'Fun Group', 'node-bob', 'Bob');
      expect(eventManager.getPendingInvites()).toHaveLength(1);
    });

    it('should accept invitation', () => {
      manager.handleInvite('grp-123', 'Fun Group', 'node-bob', 'Bob', 'relay-1');

      const accepted = manager.acceptInvite('grp-123');

      expect(accepted).toBe(true);
      // Invitation stays in pending until group-sync confirms the join
      expect(manager.getPendingInvites()).toHaveLength(1);

      // Simulate receiving group-sync (join confirmed)
      manager.handleGroupSync({
        groupId: 'grp-123',
        name: 'Fun Group',
        hubRelayId: 'relay-1',
        members: [{ nodeId: localNodeId, username: localUsername, joinedAt: Date.now(), role: 'member' }],
        createdBy: 'node-bob',
        createdAt: Date.now(),
        lastActivityAt: Date.now(),
        maxMembers: 50,
      });

      // Now invitation should be removed
      expect(manager.getPendingInvites()).toHaveLength(0);
      expect(manager.getGroup('grp-123')).not.toBeNull();
    });

    it('should decline invitation', () => {
      manager.handleInvite('grp-123', 'Fun Group', 'node-bob', 'Bob');

      const declined = manager.declineInvite('grp-123');

      expect(declined).toBe(true);
      expect(manager.getPendingInvites()).toHaveLength(0);
    });

    it('should reject accept if max groups reached', () => {
      const limitedManager = new GroupManager(localNodeId, localUsername, {}, { maxGroups: 1 });

      // Create one group to reach limit
      limitedManager.createGroup('Group 1', 'relay-1');

      // Try to accept invite
      limitedManager.handleInvite('grp-123', 'Fun Group', 'node-bob', 'Bob');
      const accepted = limitedManager.acceptInvite('grp-123');

      expect(accepted).toBe(false);
    });
  });

  describe('handleGroupSync', () => {
    it('should store group and messages from sync', () => {
      const group: GroupInfo = {
        groupId: 'grp-123',
        name: 'Test Group',
        hubRelayId: 'relay-1',
        members: [
          { nodeId: 'node-bob', username: 'Bob', joinedAt: Date.now(), role: 'admin' },
          { nodeId: localNodeId, username: localUsername, joinedAt: Date.now(), role: 'member' },
        ],
        createdBy: 'node-bob',
        createdAt: Date.now(),
        lastActivityAt: Date.now(),
        maxMembers: 50,
      };

      const messages: GroupMessagePayload[] = [
        {
          type: 'group-message',
          groupId: 'grp-123',
          messageId: 'msg-1',
          senderId: 'node-bob',
          senderUsername: 'Bob',
          text: 'Hello!',
          sentAt: Date.now(),
        },
      ];

      manager.handleGroupSync(group, messages);

      expect(manager.isInGroup('grp-123')).toBe(true);
      expect(manager.getMessageHistory('grp-123')).toHaveLength(1);
      expect(manager.getGroup('grp-123')?.members).toHaveLength(2);
    });

    it('should dedupe messages during sync', () => {
      const group: GroupInfo = {
        groupId: 'grp-123',
        name: 'Test Group',
        hubRelayId: 'relay-1',
        members: [],
        createdBy: 'node-bob',
        createdAt: Date.now(),
        lastActivityAt: Date.now(),
        maxMembers: 50,
      };

      const message: GroupMessagePayload = {
        type: 'group-message',
        groupId: 'grp-123',
        messageId: 'msg-1',
        senderId: 'node-bob',
        senderUsername: 'Bob',
        text: 'Hello!',
        sentAt: Date.now(),
      };

      manager.handleGroupSync(group, [message]);
      manager.handleGroupSync(group, [message, message]); // Duplicate

      expect(manager.getMessageHistory('grp-123')).toHaveLength(1);
    });
  });

  describe('membership', () => {
    it('should handle member joined', () => {
      const group = manager.createGroup('Test Group', 'relay-1')!;
      manager.handleGroupCreated(group);

      const newMember: GroupMember = {
        nodeId: 'node-bob',
        username: 'Bob',
        joinedAt: Date.now(),
        role: 'member',
      };

      manager.handleMemberJoined(group.groupId, newMember);

      expect(manager.getGroupMembers(group.groupId)).toHaveLength(2);
    });

    it('should handle member left', () => {
      const group = manager.createGroup('Test Group', 'relay-1')!;
      manager.handleGroupCreated(group);

      const bob: GroupMember = {
        nodeId: 'node-bob',
        username: 'Bob',
        joinedAt: Date.now(),
        role: 'member',
      };
      manager.handleMemberJoined(group.groupId, bob);

      const onMemberLeft = vi.fn();
      const eventManager = new GroupManager(localNodeId, localUsername, { onMemberLeft });
      eventManager.handleGroupCreated(group);
      eventManager.handleMemberJoined(group.groupId, bob);

      eventManager.handleMemberLeft(group.groupId, 'node-bob', 'Bob', 'voluntary');

      expect(onMemberLeft).toHaveBeenCalledWith(group.groupId, 'node-bob', 'Bob', 'voluntary');
      expect(eventManager.getGroupMembers(group.groupId)).toHaveLength(1);
    });

    it('should remove group if local node leaves', () => {
      const group = manager.createGroup('Test Group', 'relay-1')!;
      manager.handleGroupCreated(group);

      const onGroupRemoved = vi.fn();
      const eventManager = new GroupManager(localNodeId, localUsername, { onGroupRemoved });
      eventManager.handleGroupCreated(group);

      eventManager.handleMemberLeft(group.groupId, localNodeId, localUsername, 'voluntary');

      expect(eventManager.isInGroup(group.groupId)).toBe(false);
      expect(onGroupRemoved).toHaveBeenCalledWith(group.groupId, 'voluntary');
    });
  });

  describe('messages', () => {
    it('should store and emit incoming messages', () => {
      const group = manager.createGroup('Test Group', 'relay-1')!;
      manager.handleGroupCreated(group);

      const onGroupMessage = vi.fn();
      const eventManager = new GroupManager(localNodeId, localUsername, { onGroupMessage });
      eventManager.handleGroupCreated(group);

      const message: GroupMessagePayload = {
        type: 'group-message',
        groupId: group.groupId,
        messageId: 'msg-1',
        senderId: 'node-bob',
        senderUsername: 'Bob',
        text: 'Hello everyone!',
        sentAt: Date.now(),
      };

      eventManager.handleMessage(message);

      expect(onGroupMessage).toHaveBeenCalledWith(group.groupId, message);
      expect(eventManager.getMessageHistory(group.groupId)).toHaveLength(1);
    });

    it('should ignore messages for unknown groups', () => {
      const message: GroupMessagePayload = {
        type: 'group-message',
        groupId: 'unknown-group',
        messageId: 'msg-1',
        senderId: 'node-bob',
        senderUsername: 'Bob',
        text: 'Hello!',
        sentAt: Date.now(),
      };

      // Should not throw
      manager.handleMessage(message);
      expect(manager.getMessageHistory('unknown-group')).toHaveLength(0);
    });

    it('should dedupe messages by messageId', () => {
      const group = manager.createGroup('Test Group', 'relay-1')!;
      manager.handleGroupCreated(group);

      const message: GroupMessagePayload = {
        type: 'group-message',
        groupId: group.groupId,
        messageId: 'msg-1',
        senderId: 'node-bob',
        senderUsername: 'Bob',
        text: 'Hello!',
        sentAt: Date.now(),
      };

      manager.handleMessage(message);
      manager.handleMessage(message); // Duplicate

      expect(manager.getMessageHistory(group.groupId)).toHaveLength(1);
    });

    it('should trim message history to max', () => {
      const limitedManager = new GroupManager(localNodeId, localUsername, {}, { maxMessagesPerGroup: 5 });
      const group = limitedManager.createGroup('Test Group', 'relay-1')!;
      limitedManager.handleGroupCreated(group);

      for (let i = 0; i < 10; i++) {
        limitedManager.handleMessage({
          type: 'group-message',
          groupId: group.groupId,
          messageId: `msg-${i}`,
          senderId: 'node-bob',
          senderUsername: 'Bob',
          text: `Message ${i}`,
          sentAt: Date.now() + i,
        });
      }

      const history = limitedManager.getMessageHistory(group.groupId);
      expect(history).toHaveLength(5);
      expect(history[0].messageId).toBe('msg-5'); // Oldest kept
    });
  });

  describe('hub migration', () => {
    it('should update hub on migration', () => {
      const group = manager.createGroup('Test Group', 'relay-1')!;
      manager.handleGroupCreated(group);

      const onHubMigration = vi.fn();
      const eventManager = new GroupManager(localNodeId, localUsername, { onHubMigration });
      eventManager.handleGroupCreated(group);

      eventManager.handleHubMigration(group.groupId, 'relay-2', 'relay-1');

      expect(eventManager.getGroup(group.groupId)?.hubRelayId).toBe('relay-2');
      expect(eventManager.getGroup(group.groupId)?.backupHubId).toBe('relay-1');
      expect(onHubMigration).toHaveBeenCalledWith(group.groupId, 'relay-2', 'relay-1');
    });
  });

  describe('admin checks', () => {
    it('should correctly identify admin', () => {
      const group = manager.createGroup('Test Group', 'relay-1')!;
      manager.handleGroupCreated(group);

      // Add non-admin member
      manager.handleMemberJoined(group.groupId, {
        nodeId: 'node-bob',
        username: 'Bob',
        joinedAt: Date.now(),
        role: 'member',
      });

      expect(manager.isAdmin(group.groupId)).toBe(true); // Local node is admin
      expect(manager.isAdmin(group.groupId, 'node-bob')).toBe(false);
    });
  });

  describe('hub health monitoring', () => {
    it('should track hub heartbeats', () => {
      const group = manager.createGroup('Test Group', 'relay-1')!;
      manager.handleGroupCreated(group);

      // Simulate heartbeat
      manager.handleHubHeartbeat(group.groupId, 2, Date.now());

      // Should not trigger failure immediately
      expect(manager.isInGroup(group.groupId)).toBe(true);
    });

    it('should start and stop health monitoring', () => {
      vi.useFakeTimers();

      manager.startHubHealthMonitoring();
      vi.advanceTimersByTime(60_000);

      // Should not throw
      manager.stopHubHealthMonitoring();

      vi.useRealTimers();
    });

    it('should trigger hub failure after missed heartbeats', () => {
      vi.useFakeTimers();

      const onHubFailure = vi.fn();
      const eventManager = new GroupManager(localNodeId, localUsername, { onHubFailure });

      const group = eventManager.createGroup('Test Group', 'relay-1')!;
      eventManager.handleGroupCreated(group);

      // Start monitoring
      eventManager.startHubHealthMonitoring();

      // Initial heartbeat
      eventManager.handleHubHeartbeat(group.groupId, 1, Date.now());

      // Advance time past failure threshold (3 missed heartbeats at 30s each = 90s + buffer)
      vi.advanceTimersByTime(150_000);

      expect(onHubFailure).toHaveBeenCalledWith(group.groupId, 'relay-1');

      eventManager.stopHubHealthMonitoring();
      vi.useRealTimers();
    });

    it('should reset missed count on heartbeat', () => {
      vi.useFakeTimers();

      const onHubFailure = vi.fn();
      const eventManager = new GroupManager(localNodeId, localUsername, { onHubFailure });

      const group = eventManager.createGroup('Test Group', 'relay-1')!;
      eventManager.handleGroupCreated(group);

      eventManager.startHubHealthMonitoring();

      // Initial heartbeat
      eventManager.handleHubHeartbeat(group.groupId, 1, Date.now());

      // Advance time but receive heartbeat before failure
      vi.advanceTimersByTime(50_000);
      eventManager.handleHubHeartbeat(group.groupId, 1, Date.now());

      vi.advanceTimersByTime(50_000);
      eventManager.handleHubHeartbeat(group.groupId, 1, Date.now());

      // Should not have triggered failure
      expect(onHubFailure).not.toHaveBeenCalled();

      eventManager.stopHubHealthMonitoring();
      vi.useRealTimers();
    });
  });
});
