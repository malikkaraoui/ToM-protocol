# Code Review Request: Group Chat via Relay Hub (Story 4.6)

## Context

We've implemented a **group chat feature** for a P2P messaging protocol (ToM Protocol). The key innovation is using **relay nodes as temporary hubs** for message fanout, maintaining decentralization while enabling group communication.

## Architecture Overview

```
           ┌─────────────────┐
           │   Relay Hub     │  ← Any relay can host a group
           │  (GroupHub)     │
           └────────┬────────┘
                    │ fanout (1 msg → N members)
        ┌───────────┼───────────┐
        ▼           ▼           ▼
     ┌─────┐     ┌─────┐     ┌─────┐
     │Alice│     │ Bob │     │Carol│
     │(GM) │     │(GM) │     │(GM) │
     └─────┘     └─────┘     └─────┘

GM = GroupManager (tracks membership locally)
```

### Key Design Decisions

1. **Relay as Hub**: Each group has a designated relay that handles fanout
2. **No Central Server**: Different groups can use different relays
3. **Local State**: Each member maintains their own GroupManager
4. **Rate Limiting**: 5 messages/second per user per group
5. **Message History**: Last 200 messages stored on hub for sync

## Files to Review

### 1. Core Types (`packages/core/src/groups/group-types.ts`)

```typescript
// ~250 lines
// Key types: GroupId, GroupInfo, GroupMember, GroupPayload variants
// Type guards: isGroupPayload, isGroupMessage, isGroupInvite, etc.
// Constants: MAX_GROUP_MEMBERS=50, RATE_LIMIT=5/sec, HUB_HEARTBEAT=30s
```

### 2. GroupManager (`packages/core/src/groups/group-manager.ts`)

```typescript
// ~400 lines - Runs on ALL nodes
export class GroupManager {
  // Tracks groups this node is a member of
  private groups = new Map<GroupId, GroupInfo>();
  private messageHistory = new Map<GroupId, GroupMessagePayload[]>();
  private pendingInvites = new Map<GroupId, InviteInfo>();

  // Key methods:
  createGroup(name, hubRelayId, initialMembers): GroupInfo | null
  handleInvite(groupId, groupName, inviterId, inviterUsername): void
  acceptInvite(groupId): boolean
  handleGroupSync(group, recentMessages?): void
  handleMessage(message): void
  handleMemberJoined(groupId, member): void
  handleMemberLeft(groupId, nodeId, username, reason): void
  handleHubMigration(groupId, newHubId, oldHubId): void
}
```

### 3. GroupHub (`packages/core/src/groups/group-hub.ts`)

```typescript
// ~350 lines - Runs on RELAY nodes only
export class GroupHub {
  // Manages groups this relay hosts
  private groups = new Map<GroupId, GroupInfo>();
  private messageHistory = new Map<GroupId, GroupMessagePayload[]>();
  private pendingDeliveries = new Map<string, Set<NodeId>>();
  private rateLimits = new Map<string, RateLimitEntry>();

  // Key methods:
  handlePayload(payload, fromNodeId): void  // Routes to handlers
  handleCreate(payload, fromNodeId): void   // Creates group, sends invites
  handleJoin(payload, fromNodeId): void     // Adds member, sends sync
  handleLeave(payload, fromNodeId): void    // Removes member, broadcasts
  handleMessage(payload, fromNodeId): void  // Fanout to all members

  // Security:
  - Rate limiting (5 msg/sec/user)
  - Member verification before message fanout
  - Max groups/members limits to prevent DoS
}
```

### 4. SDK Integration (`packages/sdk/src/tom-client.ts`)

```typescript
// Added ~200 lines to existing TomClient

// New properties:
private groupManager: GroupManager | null = null;
private groupHub: GroupHub | null = null;

// New public methods:
createGroup(name, initialMembers): Promise<GroupInfo | null>
acceptGroupInvite(groupId): Promise<boolean>
declineGroupInvite(groupId): boolean
sendGroupMessage(groupId, text): Promise<boolean>
leaveGroup(groupId): Promise<boolean>
getGroups(): GroupInfo[]
getGroup(groupId): GroupInfo | null
getPendingGroupInvites(): InviteInfo[]
getGroupMessages(groupId): GroupMessagePayload[]

// Event handlers:
onGroupCreated(handler): void
onGroupInvite(handler): void
onGroupMemberJoined(handler): void
onGroupMemberLeft(handler): void
onGroupMessage(handler): void

// Internal:
private initGroupHub(): void  // Called when node becomes relay
private handleGroupPayload(payload, fromNodeId): void  // Routes group messages
```

## Protocol Messages

| Type | Direction | Purpose |
|------|-----------|---------|
| `group-create` | Member → Hub | Create new group |
| `group-created` | Hub → Creator | Confirmation with GroupInfo |
| `group-invite` | Hub → Invitee | Invite to join |
| `group-join` | Member → Hub | Accept invitation |
| `group-sync` | Hub → New Member | Full group state + history |
| `group-member-joined` | Hub → All | Broadcast new member |
| `group-leave` | Member → Hub | Leave group |
| `group-member-left` | Hub → All | Broadcast departure |
| `group-message` | Member → Hub → All | Chat message with fanout |
| `group-delivery-ack` | Member → Hub | Delivery confirmation |
| `group-hub-migration` | Old Hub → All | Hub failover |

## Questions for Review

### 1. Security Concerns

- Are there any attack vectors we've missed?
- Is the rate limiting sufficient (5 msg/sec)?
- Can a malicious hub manipulate the group?
- Is member verification robust enough?

### 2. Architecture Alternatives

- **Would you have done this differently?**
- Is using relay as hub the right approach?
- Should we use a different fanout strategy (e.g., tree-based)?
- Should group state be replicated across multiple relays?

### 3. Edge Cases

- What happens if the hub crashes mid-fanout?
- How should we handle network partitions?
- Is the hub migration mechanism sufficient?

### 4. Performance

- Is storing 200 messages per group on the hub reasonable?
- Any concerns with the Map-based storage?
- Should we add pagination for message history?

### 5. Missing Features

- What essential features are we missing for MVP?
- Should we add:
  - Admin/moderation features?
  - Message editing/deletion?
  - Read receipts per member?
  - Typing indicators?

## Test Coverage

- **47 tests** for group functionality
- GroupManager: 19 tests
- GroupHub: 16 tests
- Type guards: 12 tests
- All 406 project tests passing

## Code Locations

```
packages/core/src/groups/
├── group-types.ts      # Types, type guards, constants
├── group-types.test.ts
├── group-manager.ts    # Member-side logic
├── group-manager.test.ts
├── group-hub.ts        # Relay-side fanout
├── group-hub.test.ts
└── index.ts            # Exports

packages/sdk/src/tom-client.ts  # SDK integration (~200 new lines)
```

## Your Task

Please provide:

1. **Security Review**: Identify vulnerabilities and attack vectors
2. **Architecture Critique**: Would you structure this differently? Why?
3. **Code Quality**: Any anti-patterns or improvements?
4. **Missing Pieces**: What's needed before this is production-ready?
5. **Alternative Approaches**: How would you have implemented group chat in a P2P system?

Be adversarial. Find the holes. Challenge the design decisions.

---

## Appendix: Key Code Snippets

### A. Rate Limiting Implementation (group-hub.ts)

```typescript
private checkRateLimit(groupId: GroupId, nodeId: NodeId): boolean {
  const key = `${groupId}:${nodeId}`;
  const now = Date.now();
  const windowMs = 1000; // 1 second window

  let entry = this.rateLimits.get(key);
  if (!entry || now - entry.windowStart > windowMs) {
    entry = { count: 0, windowStart: now };
    this.rateLimits.set(key, entry);
  }

  entry.count++;
  if (entry.count > GROUP_RATE_LIMIT_PER_SECOND) {
    return false;
  }

  return true;
}
```

### B. Message Fanout (group-hub.ts)

```typescript
private handleMessage(payload: GroupMessagePayload, fromNodeId: NodeId): void {
  const group = this.groups.get(payload.groupId);
  if (!group) return;

  // Verify sender is a member
  if (!group.members.some((m) => m.nodeId === fromNodeId)) {
    this.events.onHubActivity?.(payload.groupId, 'message-rejected', { reason: 'not-member' });
    return;
  }

  // Rate limiting
  if (!this.checkRateLimit(payload.groupId, fromNodeId)) {
    this.events.onHubActivity?.(payload.groupId, 'message-rejected', { reason: 'rate-limited' });
    return;
  }

  // Store message
  const messages = this.messageHistory.get(payload.groupId) ?? [];
  messages.push(payload);
  if (messages.length > this.maxMessagesPerGroup) {
    messages.shift();
  }
  this.messageHistory.set(payload.groupId, messages);

  // Track pending deliveries
  const pendingNodes = new Set(group.members.map((m) => m.nodeId).filter((id) => id !== fromNodeId));
  if (pendingNodes.size > 0) {
    this.pendingDeliveries.set(payload.messageId, pendingNodes);
  }

  // Fanout to all members except sender
  this.events.broadcastToGroup(payload.groupId, payload, fromNodeId);
}
```

### C. Member Verification (group-hub.ts)

```typescript
private handleJoin(payload: GroupJoinPayload, fromNodeId: NodeId): void {
  const group = this.groups.get(payload.groupId);
  if (!group) {
    this.events.onHubActivity?.(payload.groupId, 'join-rejected', { reason: 'group-not-found' });
    return;
  }

  // Verify the joining node matches the payload
  if (payload.nodeId !== fromNodeId) {
    this.events.onHubActivity?.(payload.groupId, 'join-rejected', { reason: 'node-mismatch' });
    return;
  }

  // Check if already a member
  if (group.members.some((m) => m.nodeId === fromNodeId)) {
    this.sendSync(payload.groupId, fromNodeId); // Just resync
    return;
  }

  // Check max members
  if (group.members.length >= group.maxMembers) {
    this.events.onHubActivity?.(payload.groupId, 'join-rejected', { reason: 'max-members' });
    return;
  }

  // Add new member...
}
```

### D. SDK Integration (tom-client.ts)

```typescript
private handleGroupPayload(payload: GroupPayload, fromNodeId: string): void {
  // If we're a relay hub, handle as hub
  if (this.groupHub) {
    this.groupHub.handlePayload(payload, fromNodeId);
  }

  // Also handle as member (for messages/events directed to us)
  if (this.groupManager) {
    switch (payload.type) {
      case 'group-created':
        if ('groupInfo' in payload) {
          this.groupManager.handleGroupCreated(payload.groupInfo);
        }
        break;
      case 'group-invite':
        if ('inviteeId' in payload && payload.inviteeId === this.nodeId) {
          this.groupManager.handleInvite(
            payload.groupId,
            payload.groupName,
            payload.inviterId,
            payload.inviterUsername,
          );
        }
        break;
      case 'group-message':
        if ('messageId' in payload) {
          this.groupManager.handleMessage(payload as GroupMessagePayload);
        }
        break;
      // ... other cases
    }
  }
}
```

### E. Hub Initialization on Role Change

```typescript
this.roleManager = new RoleManager({
  onRoleChanged: (nodeId, _oldRoles, newRoles) => {
    // ...
    if (nodeId === this.nodeId) {
      // Initialize GroupHub when becoming a relay
      if (newRoles.includes('relay') && !this.groupHub) {
        this.initGroupHub();
      }
    }
  },
});

private initGroupHub(): void {
  if (this.groupHub) return;

  const hubEvents: GroupHubEvents = {
    sendToNode: async (nodeId, payload, _groupId) => {
      await this.sendPayload(nodeId, payload);
    },
    broadcastToGroup: async (groupId, payload, excludeNodeId) => {
      const group = this.groupHub?.getGroup(groupId);
      if (!group) return;

      for (const member of group.members) {
        if (member.nodeId !== excludeNodeId && member.nodeId !== this.nodeId) {
          await this.sendPayload(member.nodeId, payload);
        }
      }
    },
  };

  this.groupHub = new GroupHub(this.nodeId, hubEvents);
}
```
