# ToM Protocol VS Code Extension

Distributed serverless messaging directly in VS Code with LLM integration.

## Features

- **Chat Panel**: Real-time messaging in a webview panel
- **Network Status**: Monitor your node's role, peers, and relay statistics
- **Participants View**: See who's online in the activity bar
- **Command Palette**: Quick access to connect, disconnect, and status commands

## Installation

### From Source

```bash
cd tools/vscode-extension
pnpm install
pnpm build
code --install-extension tom-vscode-1.0.0.vsix
```

## Configuration

Configure via VS Code settings:

| Setting | Default | Description |
|---------|---------|-------------|
| `tom.signalingUrl` | `ws://localhost:3001` | WebSocket URL for signaling server |
| `tom.username` | `""` | Your username on the ToM network |
| `tom.autoConnect` | `false` | Auto-connect on VS Code startup |

## Commands

- **ToM: Open Chat Panel** - Opens the chat interface
- **ToM: Connect to Network** - Connect to the ToM network
- **ToM: Disconnect from Network** - Disconnect from the network
- **ToM: Show Network Status** - Display current network status

## LLM Integration

This extension is designed to work with LLMs (like Claude) via the MCP server.
See the [CLAUDE.md](../../CLAUDE.md) file for integration guide.

### With Claude Code

The ToM MCP server can be configured in Claude Code to enable:
- Sending messages through Claude
- Querying network status
- Discovering participants
- Monitoring peer gossip statistics

## Development

```bash
# Build
pnpm build

# Watch mode
pnpm dev

# Package for distribution
pnpm package
```

## Architecture

```
┌─────────────────────────────────────────────────┐
│                 VS Code Extension               │
├─────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────┐ │
│  │ Chat Panel  │  │ Participants│  │ Status  │ │
│  │  (Webview)  │  │ (TreeView)  │  │(TreeView│ │
│  └──────┬──────┘  └──────┬──────┘  └────┬────┘ │
│         │                │              │      │
│         └────────────────┼──────────────┘      │
│                          │                     │
│                   ┌──────┴──────┐              │
│                   │  TomClient  │              │
│                   │   (SDK)     │              │
│                   └──────┬──────┘              │
└──────────────────────────┼──────────────────────┘
                           │
                    WebSocket/WebRTC
                           │
                 ┌─────────┴─────────┐
                 │  ToM P2P Network  │
                 └───────────────────┘
```

## License

MIT
