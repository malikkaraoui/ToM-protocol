# Contributing to ToM Protocol

Welcome! This guide covers how to contribute to ToM Protocol, with special consideration for LLM-assisted development.

## Micro-Session Contribution Model

ToM Protocol is designed for **micro-session contributions** - small, focused changes that can be completed in a single session. This works well for:
- LLM coding assistants (Claude, etc.)
- Short coding sessions
- Learning the codebase

### Issue Complexity Levels

| Level | Time | Scope | Example |
|-------|------|-------|---------|
| **micro** | < 30 min | Single file | Fix typo, add JSDoc |
| **small** | 30-60 min | 2-3 files | Add a test, fix a bug |
| **medium** | 1-2 hours | Multiple components | New feature, refactor |

### Issue Categories

| Category | Description |
|----------|-------------|
| **verification** | Code review, testing existing features |
| **building** | New features, bug fixes |
| **analysis** | Investigation, documentation |
| **testing** | Add tests, improve coverage |

### Finding Work

1. Browse [open issues](https://github.com/malikkaraoui/ToM-protocol/issues)
2. Filter by `good first issue` or complexity level (`micro`, `small`)
3. Check for `help wanted` label
4. Claim an issue by commenting before starting

### What Makes a Good Micro-Session?

| Good ✅ | Avoid ❌ |
|---------|----------|
| Fix a single bug | Rewrite entire subsystem |
| Add one test | Add tests for everything |
| Implement one feature | Multi-epic feature work |
| Update documentation | Complete documentation overhaul |
| Refactor one module | Cross-cutting refactors |

### Session Workflow

1. **Claim** - Comment on the issue to claim it
2. **Understand** - Read relevant files (CLAUDE.md, llms.txt)
3. **Plan** - Identify specific, scoped changes
4. **Implement** - Make focused changes
5. **Test** - Run `pnpm test` to verify
6. **Commit** - One commit per logical change
7. **PR** - Reference the issue in your PR

## Quick Start

```bash
# Clone and setup
git clone https://github.com/malikkaraoui/ToM-protocol.git
cd tom-protocol
pnpm install

# Build and test
pnpm build
pnpm test

# Run demo
./scripts/start-demo.sh
```

## Repository Structure

```
tom-protocol/
├── packages/
│   ├── core/           # Core protocol implementation
│   └── sdk/            # High-level SDK (TomClient)
├── tools/
│   ├── signaling-server/  # Bootstrap server
│   ├── mcp-server/        # MCP server for LLMs
│   └── vscode-extension/  # VS Code extension
├── apps/
│   └── demo/           # Browser demo app
├── llms.txt            # LLM quick reference
├── CLAUDE.md           # Detailed LLM guide
└── _bmad-output/
    └── planning-artifacts/
        └── architecture.md  # Architecture decisions (ADRs)
```

## For LLM Assistants

### Reading Order

1. `llms.txt` - Quick protocol overview
2. `CLAUDE.md` - Detailed implementation guide
3. `_bmad-output/planning-artifacts/architecture.md` - Design decisions (ADRs)
4. Relevant source files

### Common Tasks

#### Add a Test
```typescript
// packages/core/src/feature/feature.test.ts
import { describe, it, expect } from 'vitest';
import { MyFeature } from './feature.js';

describe('MyFeature', () => {
  it('should do something', () => {
    const feature = new MyFeature();
    expect(feature.method()).toBe(expected);
  });
});
```

#### Fix a Bug
1. Read the relevant module
2. Write a failing test
3. Fix the bug
4. Verify test passes

#### Add Documentation
- Update `CLAUDE.md` for API changes
- Update `llms.txt` for protocol changes
- Add JSDoc to new exports

### Code Style

- TypeScript with strict mode
- Biome for linting and formatting
- Run `pnpm lint:fix` before committing
- No `any` types (use `unknown` or proper types)
- Import with `.js` extension (ESM)

### Commit Format

```
type(scope): description

- Bullet points for details
- Keep under 72 chars per line

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`

## Testing

```bash
# Run all tests
pnpm test

# Run specific package tests
pnpm --filter tom-protocol test

# Watch mode
pnpm --filter tom-protocol test:watch
```

## Architecture Overview

### Core Concepts

- **Node**: Any participant in the network
- **Relay**: Node that forwards messages for others
- **Direct**: P2P connection via WebRTC
- **Bootstrap**: Initial connection via signaling server

### Key Files

| File | Purpose |
|------|---------|
| `router.ts` | Message routing logic |
| `relay-selector.ts` | Relay selection algorithm |
| `network-topology.ts` | Network state tracking |
| `peer-gossip.ts` | Distributed peer discovery |
| `ephemeral-subnet.ts` | Subnet formation |
| `tom-client.ts` | SDK entry point |

### Data Flow

```
Send Message:
  TomClient.sendMessage()
    → Router.route()
      → RelaySelector.selectRelay() (if needed)
        → TransportLayer.send()
          → WebRTC DataChannel

Receive Message:
  WebRTC DataChannel
    → TransportLayer.onMessage()
      → Router.handleIncoming()
        → TomClient.onMessage() callback
```

## Pull Request Guidelines

1. **One logical change per PR**
2. **Include tests** for new code
3. **Update documentation** if API changes
4. **Run full test suite** before submitting
5. **Keep PRs small** (< 200 lines preferred)

## Getting Help

- Read `CLAUDE.md` for implementation details
- Check `_bmad-output/planning-artifacts/architecture.md` for design rationale
- Open an issue for questions
- Use the MCP server to explore programmatically

## License

MIT License - See LICENSE file.
