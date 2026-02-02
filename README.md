# ToM Protocol

The Open Messaging — a distributed P2P data transport protocol where every device is the network.

## Quick Start

```bash
pnpm install
pnpm build
pnpm test
```

## Packages

- **packages/core** — Raw protocol primitives (connect, send, receive, roles)
- **packages/sdk** — Plug-and-play abstraction (TomClient, auto-relay, auto-encrypt)
- **apps/demo** — Vanilla HTML/JS demo app
- **tools/signaling-server** — Temporary bootstrap WebSocket server (ADR-002)

## License

MIT
