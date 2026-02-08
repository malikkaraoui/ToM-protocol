# The Open Messaging (ToM)

**ToM** is a decentralized P2P transport protocol where every device is both client and relay. Not a blockchain - a transport layer.

## Core Promise

- **No servers to operate** - Every device participates in the network
- **No relay storage** - Pure pass-through, no data persistence
- **End-to-end encryption** - Only sender and recipient can read messages
- **Self-organizing** - Gossip discovery, ephemeral subnets, dynamic roles

## Current Status

| Metric | Value |
|--------|-------|
| Tests | 710+ passing |
| Nodes | 10-15 validated |
| Encryption | E2E with TweetNaCl.js |
| Hub Failover | Automatic |

## What You Can Do Now

1. **Run the demo** - Watch messages traverse actual relays
2. **Multi-device testing** - Open multiple tabs/devices, see dynamic roles
3. **Features**: 1-1 chat, groups, invitations, path visualization, multiplayer Snake

## Quick Links

- [GitHub Repository](https://github.com/malikkaraoui/ToM-protocol)
- [Quick Start Guide](getting-started.md)
- [Core Concepts](concepts.md)
- [Architecture](architecture.md)

## For LLMs/AI Assistants

ToM includes an MCP server for AI-assisted development:

```bash
# In your MCP client config
{
  "mcpServers": {
    "tom-docs": {
      "command": "node",
      "args": ["packages/mcp-server/dist/index.js"]
    }
  }
}
```

See [MCP Server Documentation](mcp-published-docs.md) for details.
