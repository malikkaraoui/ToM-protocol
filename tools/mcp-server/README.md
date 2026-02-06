# ToM Protocol MCP Server

Model Context Protocol server enabling LLMs to interact with the ToM network.

## Quick Start

```bash
# Build and run
cd tools/mcp-server
pnpm build
pnpm start
```

## Configuration

Environment variables:
- `TOM_SIGNALING_URL`: WebSocket signaling server URL (default: `ws://localhost:3001`)
- `TOM_USERNAME`: Username for the MCP agent (default: `mcp-agent`)

## Available Tools

### 1. `tom_connect`

Connect to the ToM network. **Must be called before other operations.**

**Parameters:** None

**Response:**
```json
{
  "status": "connected",
  "nodeId": "abc123...",
  "username": "mcp-agent",
  "signalingUrl": "ws://localhost:3001"
}
```

---

### 2. `tom_disconnect`

Disconnect from the ToM network.

**Parameters:** None

**Response:**
```
Disconnected from ToM network
```

---

### 3. `tom_send_message`

Send a message to another participant.

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `to` | string | Yes | Node ID or username of recipient |
| `message` | string | Yes | Message text to send |

**Example:**
```json
{
  "to": "alice",
  "message": "Hello from the MCP server!"
}
```

**Response:**
```json
{
  "status": "sent",
  "messageId": "msg-abc123",
  "to": "node-id-xyz",
  "message": "Hello from the MCP server!"
}
```

**Error (recipient not found):**
```
Error: Participant not found: bob. Available: alice, charlie
```

---

### 4. `tom_list_participants`

List all currently connected participants.

**Parameters:** None

**Response:**
```json
{
  "count": 3,
  "participants": [
    {
      "nodeId": "abc123...",
      "username": "alice",
      "roles": ["client", "relay"],
      "status": "online"
    },
    {
      "nodeId": "def456...",
      "username": "bob",
      "roles": ["client"],
      "status": "online"
    }
  ]
}
```

---

### 5. `tom_get_network_status`

Get current network status including role, peers, and statistics.

**Parameters:** None

**Response:**
```json
{
  "nodeId": "abc123...",
  "username": "mcp-agent",
  "roles": ["client", "relay"],
  "connectedPeers": 5,
  "gossip": {
    "totalPeers": 8,
    "bootstrapPeers": 3,
    "gossipPeers": 5,
    "connectedPeers": 5
  },
  "subnets": {
    "totalSubnets": 2,
    "totalNodesInSubnets": 4
  }
}
```

---

### 6. `tom_get_message_history`

Get recent message history (sent and received).

**Parameters:**
| Name | Type | Required | Description |
|------|------|----------|-------------|
| `limit` | number | No | Max messages to return (default: 20, max: 1000) |

**Response:**
```json
{
  "count": 2,
  "messages": [
    {
      "id": "msg-123",
      "from": "abc123...",
      "to": "def456...",
      "text": "Hello!",
      "timestamp": "2026-02-06T10:30:00.000Z",
      "status": "sent"
    },
    {
      "id": "msg-456",
      "from": "def456...",
      "to": "abc123...",
      "text": "Hi there!",
      "timestamp": "2026-02-06T10:30:05.000Z",
      "status": "received"
    }
  ]
}
```

---

### 7. `tom_get_gossip_stats`

Get peer discovery statistics from the gossip protocol.

**Parameters:** None

**Response:**
```json
{
  "totalPeers": 12,
  "bootstrapPeers": 4,
  "gossipPeers": 8,
  "connectedPeers": 10,
  "bootstrapDependency": "33.3%"
}
```

**Note:** `bootstrapDependency` shows how much the network still relies on the signaling server. Lower is better - means gossip discovery is working.

---

### 8. `tom_get_subnet_stats`

Get ephemeral subnet statistics.

**Parameters:** None

**Response:**
```json
{
  "totalSubnets": 2,
  "totalNodesInSubnets": 6,
  "averageSubnetSize": "3.0",
  "communicationEdges": 8,
  "subnets": [
    {
      "id": "subnet-abc",
      "members": 3,
      "formedAt": "2026-02-06T10:00:00.000Z",
      "lastActivity": "2026-02-06T10:30:00.000Z",
      "densityScore": "0.85"
    }
  ]
}
```

---

## Error Handling

All errors return structured responses:

```json
{
  "content": [{ "type": "text", "text": "Error: Not connected. Call tom_connect first." }],
  "isError": true
}
```

## Usage with Claude Desktop

Add to your Claude Desktop config (`~/Library/Application Support/Claude/claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "tom-protocol": {
      "command": "node",
      "args": ["/path/to/tom-protocol/tools/mcp-server/dist/cli.js"],
      "env": {
        "TOM_SIGNALING_URL": "ws://localhost:3001",
        "TOM_USERNAME": "claude-agent"
      }
    }
  }
}
```

## Development

```bash
# Run tests
pnpm test

# Build
pnpm build

# Run with debug output
DEBUG=tom:* pnpm start
```
