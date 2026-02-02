# Story 1.2: Node Identity Generation & Persistence

Status: review

## Story

As a network participant,
I want my node to generate a unique Ed25519 keypair on first launch and persist it,
so that I have a stable cryptographic identity across sessions.

## Acceptance Criteria

1. **Given** a node starts for the first time with no existing identity **When** the identity module initializes **Then** an Ed25519 keypair is generated using TweetNaCl.js **And** the keypair is persisted to the configured storage (localStorage in browser, file in Node.js) **And** the node's public key serves as its unique network identifier
2. **Given** a node starts with an existing persisted identity **When** the identity module initializes **Then** the existing keypair is loaded from storage without generating a new one **And** the node's network identifier remains the same as previous sessions
3. **Given** the identity module is asked to sign data **When** a valid payload is provided **Then** it returns a valid Ed25519 signature **And** the signature can be verified using the node's public key

## Tasks / Subtasks

- [x] Task 1: Install tweetnacl dependency in packages/core (AC: #1)
  - [x]Add `tweetnacl` as production dependency in packages/core/package.json
  - [x]Run `pnpm install` to update lockfile
  - [x]Verify tweetnacl types are available (tweetnacl ships its own .d.ts)

- [x]Task 2: Create identity keypair module (AC: #1, #2, #3)
  - [x]Create `packages/core/src/identity/keypair.ts`
  - [x]Implement `generateKeypair(): NodeIdentity` — generates Ed25519 keypair via `tweetnacl.sign.keyPair()`
  - [x]Implement `signData(secretKey: Uint8Array, data: Uint8Array): Uint8Array` — signs using `tweetnacl.sign.detached()`
  - [x]Implement `verifySignature(publicKey: Uint8Array, data: Uint8Array, signature: Uint8Array): boolean` — verifies using `tweetnacl.sign.detached.verify()`
  - [x]Define `NodeIdentity` interface: `{ publicKey: Uint8Array; secretKey: Uint8Array }`
  - [x]Define `NodeId` type alias: hex-encoded public key string

- [x]Task 3: Create storage abstraction (AC: #1, #2)
  - [x]Create `packages/core/src/identity/storage.ts`
  - [x]Define `IdentityStorage` interface: `{ save(identity: NodeIdentity): Promise<void>; load(): Promise<NodeIdentity | null> }`
  - [x]Implement `MemoryStorage` class (for testing and fallback)
  - [x]Implement `LocalStorageAdapter` class (browser — uses `localStorage` with key `tom-identity`)
  - [x]Implement `FileStorageAdapter` class (Node.js — reads/writes `~/.tom/identity.json`)
  - [x]Store keys as hex-encoded strings in JSON format

- [x]Task 4: Create identity manager (AC: #1, #2, #3)
  - [x]Create `packages/core/src/identity/identity-manager.ts`
  - [x]Implement `IdentityManager` class with constructor taking `IdentityStorage`
  - [x]Implement `init(): Promise<NodeIdentity>` — loads from storage or generates new + saves
  - [x]Implement `getNodeId(): NodeId` — returns hex-encoded public key
  - [x]Implement `sign(data: Uint8Array): Uint8Array` — delegates to keypair module
  - [x]Implement `verify(publicKey: Uint8Array, data: Uint8Array, signature: Uint8Array): boolean`

- [x]Task 5: Create public API barrel export (AC: #1, #2, #3)
  - [x]Create `packages/core/src/identity/index.ts` — re-exports public API
  - [x]Update `packages/core/src/index.ts` — re-export from identity module
  - [x]Export: `IdentityManager`, `NodeIdentity`, `NodeId`, `IdentityStorage`, `MemoryStorage`, `LocalStorageAdapter`, `FileStorageAdapter`, `generateKeypair`, `signData`, `verifySignature`

- [x]Task 6: Write comprehensive tests (AC: #1, #2, #3)
  - [x]Create `packages/core/src/identity/keypair.test.ts`
    - Test: generates valid Ed25519 keypair (32-byte public key, 64-byte secret key)
    - Test: signs data and verifies signature successfully
    - Test: verification fails with wrong public key
    - Test: verification fails with tampered data
  - [x]Create `packages/core/src/identity/identity-manager.test.ts`
    - Test: generates new identity when no stored identity exists (using MemoryStorage)
    - Test: loads existing identity from storage without generating new one
    - Test: getNodeId returns consistent hex string
    - Test: sign and verify round-trip works through manager

- [x]Task 7: Build and validate (AC: #1, #2, #3)
  - [x]Run `pnpm build` — zero errors
  - [x]Run `pnpm test` — all tests pass
  - [x]Run `pnpm lint` — zero warnings

## Dev Notes

### Architecture Compliance

- **ADR-005**: Ed25519 keypair generated on first join. Public key = node identity. Browser: localStorage. Node.js: ~/.tom/identity.json [Source: architecture.md#ADR-005]
- **ADR-004**: TweetNaCl.js is the chosen crypto library — minimal, audited, browser + Node.js compatible [Source: architecture.md#ADR-004]
- **ADR-006**: Unified node model — identity module is shared by all node types [Source: architecture.md#ADR-006]
- No central registry, no accounts, no identity server. Your key is your address.

### Technical Requirements

- **Library**: `tweetnacl` (NOT `tweetnacl-ts` or `@noble/ed25519` — architecture specifies TweetNaCl.js)
- **Key format**: Ed25519 — 32-byte public key, 64-byte secret key
- **Signing**: `tweetnacl.sign.detached()` for detached signatures (not nacl.sign which prepends signature)
- **Storage format**: JSON with hex-encoded keys (hex, not base64 — hex is simpler to debug and display)
- **Node ID**: hex-encoded public key string (64 chars)

### Naming Conventions (MUST follow)

- Files: `kebab-case.ts` (e.g., `identity-manager.ts`, `keypair.ts`)
- Test files: `kebab-case.test.ts` co-located next to source
- Classes: `PascalCase` (e.g., `IdentityManager`, `MemoryStorage`)
- Interfaces: `PascalCase` (e.g., `NodeIdentity`, `IdentityStorage`)
- Functions: `camelCase` (e.g., `generateKeypair`, `signData`)
- Constants: `UPPER_SNAKE_CASE`
- Type aliases: `PascalCase` (e.g., `NodeId`)

[Source: architecture.md#Naming Patterns]

### File Structure Requirements

```
packages/core/src/
├── identity/
│   ├── index.ts              # Public API re-exports
│   ├── keypair.ts            # Ed25519 generation, signing, verification
│   ├── keypair.test.ts       # Keypair unit tests
│   ├── storage.ts            # IdentityStorage interface + adapters
│   ├── identity-manager.ts   # High-level identity lifecycle
│   └── identity-manager.test.ts  # Manager tests
└── index.ts                  # Updated: re-exports identity module
```

### Testing Requirements

- Use vitest (already configured)
- Tests co-located with source
- Use `MemoryStorage` in tests (no file system or localStorage mocking needed)
- Test crypto round-trips: generate → sign → verify
- Test persistence round-trips: save → load → compare keys

### Library/Framework Requirements

| Library | Version | Purpose |
|---------|---------|---------|
| tweetnacl | ^1.0.3 | Ed25519 keypair generation, signing, verification |

**Do NOT use**: `@noble/ed25519`, `tweetnacl-ts`, `libsodium-wrappers`, `node:crypto`. Architecture mandates TweetNaCl.js.

### Previous Story Intelligence

From Story 1.1:
- tsup builds ESM + CJS with dts: true
- Package exports: `types` must come BEFORE `import`/`require` in exports map
- biome ignores `.claude`, `_bmad-output`, `_bmad`
- vitest.config.ts uses `test.projects` (not deprecated vitest.workspace.ts)
- Tests run in Node.js environment (no DOM — `document` is not defined)

### Important Implementation Notes

- `FileStorageAdapter` will use dynamic `import('node:fs/promises')` to avoid bundling Node.js modules in browser builds
- `LocalStorageAdapter` should check for `typeof localStorage !== 'undefined'` before using
- Both adapters are optional — `MemoryStorage` is the default/fallback
- The storage abstraction allows future adapters (e.g., IndexedDB, encrypted storage)
- Hex encoding for keys: use `Buffer.from(key).toString('hex')` in Node.js or manual conversion for browser compatibility — or use tweetnacl-util if already available

### References

- [Source: architecture.md#ADR-005] — Node Identity decision
- [Source: architecture.md#ADR-004] — Encryption Stack (TweetNaCl.js)
- [Source: architecture.md#Implementation Patterns & Consistency Rules] — naming, structure
- [Source: epics.md#Story 1.2] — acceptance criteria
- [Source: 1-1-monorepo-scaffold.md] — previous story learnings

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Added `@types/node` as devDep for `process` and `node:*` module type resolution
- Added `declare const localStorage` in storage.ts to avoid needing `dom` lib in tsconfig
- biome auto-fixed import ordering and formatting in identity-manager.ts and storage.ts

### Completion Notes List

- All 7 tasks completed successfully
- 11 new tests added (6 keypair + 5 identity-manager), all passing
- 15/15 total tests pass across all packages (zero regressions)
- Build passes with zero errors across all 4 packages
- Lint passes with zero errors on 30 files
- All 3 acceptance criteria satisfied

### Change Log

- 2026-02-02: Implemented Ed25519 identity module with keypair generation, storage abstraction (Memory/LocalStorage/File), and IdentityManager

### File List

- packages/core/package.json (modified — added tweetnacl dep, @types/node devDep)
- packages/core/src/index.ts (modified — added identity re-exports)
- packages/core/src/identity/index.ts (new)
- packages/core/src/identity/keypair.ts (new)
- packages/core/src/identity/keypair.test.ts (new)
- packages/core/src/identity/storage.ts (new)
- packages/core/src/identity/identity-manager.ts (new)
- packages/core/src/identity/identity-manager.test.ts (new)
- pnpm-lock.yaml (modified)
