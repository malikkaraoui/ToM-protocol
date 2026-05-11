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

**Example request:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "method": "tools/call",
  "params": {
    "name": "tom_connect",
    "arguments": {}
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"status\": \"connected\",\n  \"nodeId\": \"abc123...\",\n  \"username\": \"mcp-agent\",\n  \"signalingUrl\": \"ws://localhost:3001\"\n}"
      }
    ]
  }
}
```

---

### 2. `tom_disconnect`

Disconnect from the ToM network.

**Parameters:** None

**Example request:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "method": "tools/call",
  "params": {
    "name": "tom_disconnect",
    "arguments": {}
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 2,
  "result": {
    "content": [{ "type": "text", "text": "Disconnected from ToM network" }]
  }
}
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

**Example request:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "method": "tools/call",
  "params": {
    "name": "tom_send_message",
    "arguments": {
      "to": "alice",
      "message": "Hello from the MCP server!"
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"status\": \"sent\",\n  \"messageId\": \"msg-abc123\",\n  \"to\": \"node-id-xyz\",\n  \"message\": \"Hello from the MCP server!\"\n}"
      }
    ]
  }
}
```

**Error (recipient not found):**
```json
{
  "jsonrpc": "2.0",
  "id": 3,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "Error: Participant not found: bob. Available: alice, charlie"
      }
    ],
    "isError": true
  }
}
```

---

### 4. `tom_list_participants`

List all currently connected participants.

**Parameters:** None

**Example request:**
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "method": "tools/call",
  "params": {
    "name": "tom_list_participants",
    "arguments": {}
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 4,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"count\": 2,\n  \"participants\": [\n    {\n      \"nodeId\": \"abc123...\",\n      \"username\": \"alice\",\n      \"roles\": [\"client\", \"relay\"],\n      \"status\": \"online\"\n    },\n    {\n      \"nodeId\": \"def456...\",\n      \"username\": \"bob\",\n      \"roles\": [\"client\"],\n      \"status\": \"online\"\n    }\n  ]\n}"
      }
    ]
  }
}
```

---

### 5. `tom_get_network_status`

Get current network status including role, peers, and statistics.

**Parameters:** None

**Example request:**
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "method": "tools/call",
  "params": {
    "name": "tom_get_network_status",
    "arguments": {}
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 5,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"nodeId\": \"abc123...\",\n  \"username\": \"mcp-agent\",\n  \"roles\": [\"client\", \"relay\"],\n  \"connectedPeers\": 5,\n  \"gossip\": {\n    \"totalPeers\": 8,\n    \"bootstrapPeers\": 3,\n    \"gossipPeers\": 5,\n    \"connectedPeers\": 5\n  },\n  \"subnets\": {\n    \"totalSubnets\": 2,\n    \"totalNodesInSubnets\": 4\n  }\n}"
      }
    ]
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

**Example request:**
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "method": "tools/call",
  "params": {
    "name": "tom_get_message_history",
    "arguments": {
      "limit": 2
    }
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 6,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"count\": 2,\n  \"messages\": [\n    {\n      \"id\": \"msg-123\",\n      \"from\": \"abc123...\",\n      \"to\": \"def456...\",\n      \"text\": \"Hello!\",\n      \"timestamp\": \"2026-02-06T10:30:00.000Z\",\n      \"status\": \"sent\"\n    },\n    {\n      \"id\": \"msg-456\",\n      \"from\": \"def456...\",\n      \"to\": \"abc123...\",\n      \"text\": \"Hi there!\",\n      \"timestamp\": \"2026-02-06T10:30:05.000Z\",\n      \"status\": \"received\"\n    }\n  ]\n}"
      }
    ]
  }
}
```

---

### 7. `tom_get_gossip_stats`

Get peer discovery statistics from the gossip protocol.

**Parameters:** None

**Example request:**
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "method": "tools/call",
  "params": {
    "name": "tom_get_gossip_stats",
    "arguments": {}
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 7,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"totalPeers\": 12,\n  \"bootstrapPeers\": 4,\n  \"gossipPeers\": 8,\n  \"connectedPeers\": 10,\n  \"bootstrapDependency\": \"33.3%\"\n}"
      }
    ]
  }
}
```

**Note:** `bootstrapDependency` shows how much the network still relies on the signaling server. Lower is better - means gossip discovery is working.

---

### 8. `tom_get_subnet_stats`

Get ephemeral subnet statistics.

**Parameters:** None

**Example request:**
```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "method": "tools/call",
  "params": {
    "name": "tom_get_subnet_stats",
    "arguments": {}
  }
}
```

**Response:**
```json
{
  "jsonrpc": "2.0",
  "id": 8,
  "result": {
    "content": [
      {
        "type": "text",
        "text": "{\n  \"totalSubnets\": 2,\n  \"totalNodesInSubnets\": 6,\n  \"averageSubnetSize\": \"3.0\",\n  \"communicationEdges\": 8,\n  \"subnets\": [\n    {\n      \"id\": \"subnet-abc\",\n      \"members\": 3,\n      \"formedAt\": \"2026-02-06T10:00:00.000Z\",\n      \"lastActivity\": \"2026-02-06T10:30:00.000Z\",\n      \"densityScore\": \"0.85\"\n    }\n  ]\n}"
      }
    ]
  }
}
```

---

## Error Handling

Tool errors return structured MCP tool results:

```json
{
  "content": [{ "type": "text", "text": "Error: Not connected. Call tom_connect first." }],
  "isError": true
}
```

JSON-RPC protocol errors return an `error` object:

```json
{
  "jsonrpc": "2.0",
  "id": 9,
  "error": {
    "code": -32601,
    "message": "Method not found: unknown/method"
  }
}
```

Common error cases:

| Case | Error shape |
|------|-------------|
| Invalid JSON input | JSON-RPC error `-32700`, `Parse error` |
| Missing `method` field | JSON-RPC error `-32600`, `Invalid Request: must be an object with method` |
| Unsupported JSON-RPC method | JSON-RPC error `-32601`, `Method not found: <method>` |
| `tools/call` without `params.name` | JSON-RPC error `-32602`, `Invalid params: missing or invalid "name"` |
| Unknown ToM tool name | Tool result with `isError: true` and `Error: Unknown tool: <name>` |
| Tool called before `tom_connect` | Tool result with `isError: true` and `Error: Not connected. Call tom_connect first.` |
| `tom_send_message` recipient missing | Tool result with `isError: true` and available participant names |
| Tool execution exceeds 30 seconds | JSON-RPC error `-32603`, `Tool execution timed out after 30s` |

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
