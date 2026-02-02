# Story 1.1: Monorepo Scaffold

Status: done

<!-- Note: Validation is optional. Run validate-create-story for quality check before dev-story. -->

## Story

As a developer,
I want to clone the repo and have a working monorepo with all packages building and testing,
so that I have a solid foundation to start implementing protocol features.

## Acceptance Criteria

1. **Given** a fresh clone of the repository **When** I run `pnpm install && pnpm build` **Then** all packages (core, sdk, demo, signaling-server) build successfully with zero errors
2. **Given** the monorepo is built **When** I run `pnpm test` **Then** vitest runs across all packages with a passing placeholder test in each
3. **Given** the monorepo is built **When** I run `pnpm lint` **Then** biome runs with zero warnings
4. **Given** the dependency configuration **When** inspecting package.json files **Then** the dependency direction is enforced: demo → sdk → core, never reversed
5. **Given** all tsconfig.json files **When** TypeScript compiles **Then** strict mode is enabled in all packages
6. **Given** the signaling-server package **When** inspecting the source **Then** it contains a comment marking it as temporary (ADR-002)

## Tasks / Subtasks

- [x] Task 1: Initialize root workspace (AC: #1, #3, #5)
  - [x] Create root `package.json` with pnpm workspace scripts (`build`, `test`, `lint`, `clean`)
  - [x] Create `pnpm-workspace.yaml` defining `packages/*`, `apps/*`, `tools/*`
  - [x] Create `tsconfig.base.json` with strict mode, ESM target, path aliases
  - [x] Create `biome.json` with formatting + linting rules (kebab-case enforcement where possible)
  - [x] Create `vitest.workspace.ts` for cross-package test execution
  - [x] Create `.gitignore` (node_modules, dist, .turbo, *.tsbuildinfo)
  - [x] Create `LICENSE` (MIT)
  - [x] Create minimal `README.md`

- [x] Task 2: Scaffold packages/core (AC: #1, #2, #4, #5)
  - [x] Create `packages/core/package.json` (name: `tom-protocol`, private for now)
  - [x] Create `packages/core/tsconfig.json` extending base, strict: true
  - [x] Create `packages/core/tsup.config.ts` (ESM + CJS dual output, dts: true)
  - [x] Create `packages/core/src/index.ts` (placeholder export)
  - [x] Create `packages/core/src/index.test.ts` (passing placeholder test using vitest)

- [x] Task 3: Scaffold packages/sdk (AC: #1, #2, #4, #5)
  - [x] Create `packages/sdk/package.json` (name: `tom-sdk`, depends on `tom-protocol` workspace:*)
  - [x] Create `packages/sdk/tsconfig.json` extending base, strict: true
  - [x] Create `packages/sdk/tsup.config.ts` (ESM + CJS dual output, dts: true)
  - [x] Create `packages/sdk/src/index.ts` (placeholder export)
  - [x] Create `packages/sdk/src/index.test.ts` (passing placeholder test)

- [x] Task 4: Scaffold apps/demo (AC: #1, #2, #4, #5)
  - [x] Create `apps/demo/package.json` (depends on `tom-sdk` workspace:*)
  - [x] Create `apps/demo/vite.config.ts` (vanilla mode, no framework)
  - [x] Create `apps/demo/tsconfig.json` extending base
  - [x] Create `apps/demo/index.html` (minimal HTML)
  - [x] Create `apps/demo/src/main.ts` (placeholder)
  - [x] Create `apps/demo/src/main.test.ts` (passing placeholder test)

- [x] Task 5: Scaffold tools/signaling-server (AC: #1, #2, #5, #6)
  - [x] Create `tools/signaling-server/package.json` (independent, no workspace deps)
  - [x] Create `tools/signaling-server/tsconfig.json` extending base, strict: true
  - [x] Create `tools/signaling-server/src/index.ts` with `// TEMPORARY: Bootstrap signaling server (ADR-002) — marked for elimination`
  - [x] Create `tools/signaling-server/src/index.test.ts` (passing placeholder test)

- [x] Task 6: Validate dependency direction (AC: #4)
  - [x] Verify demo depends on sdk only (never core directly)
  - [x] Verify sdk depends on core only
  - [x] Verify core has zero internal workspace dependencies
  - [x] Verify signaling-server has zero workspace dependencies

- [x] Task 7: Run full validation (AC: #1, #2, #3)
  - [x] Run `pnpm install` — zero errors
  - [x] Run `pnpm build` — all packages build with zero errors
  - [x] Run `pnpm test` — all placeholder tests pass
  - [x] Run `pnpm lint` — biome reports zero warnings

## Dev Notes

### Architecture Compliance

- **ADR-002**: Signaling server is temporary — mark with comment in source [Source: architecture.md#ADR-002]
- **ADR-006**: Unified node model — all packages follow same patterns [Source: architecture.md#ADR-006]
- **Dependency direction**: `demo → sdk → core`, never reversed. signaling-server is independent [Source: architecture.md#Architectural Boundaries]
- **No starter template**: Manual scaffold — this IS the first story [Source: architecture.md#Starter Template Evaluation]

### Technical Requirements

- **TypeScript**: 5.x, strict mode in ALL packages
- **Build**: tsup — ESM (`dist/index.mjs`) + CJS (`dist/index.cjs`) + types (`dist/index.d.ts`)
- **Test**: vitest with workspace config
- **Lint/Format**: biome (single tool, replaces ESLint + Prettier)
- **Package manager**: pnpm (strict dependency resolution)
- **Demo**: Vite dev server in vanilla mode (no framework)

### Naming Conventions (MUST follow)

- Files: `kebab-case.ts` (e.g., `transport-layer.ts`)
- Test files: `kebab-case.test.ts` co-located next to source
- Index files: `index.ts` for public API re-exports only
- Classes/Interfaces: `PascalCase`
- Functions/Variables: `camelCase`
- Constants: `UPPER_SNAKE_CASE`
- No `utils.ts` or `helpers.ts` — name by purpose

[Source: architecture.md#Naming Patterns]

### File Structure Requirements

Complete directory structure from architecture:

```
tom-protocol/
├── package.json
├── pnpm-workspace.yaml
├── tsconfig.base.json
├── biome.json
├── vitest.workspace.ts
├── .gitignore
├── LICENSE (MIT)
├── README.md
├── packages/
│   ├── core/
│   │   ├── package.json
│   │   ├── tsconfig.json
│   │   ├── tsup.config.ts
│   │   └── src/
│   │       └── index.ts
│   └── sdk/
│       ├── package.json
│       ├── tsconfig.json
│       ├── tsup.config.ts
│       └── src/
│           └── index.ts
├── apps/
│   └── demo/
│       ├── package.json
│       ├── vite.config.ts
│       ├── tsconfig.json
│       ├── index.html
│       └── src/
│           └── main.ts
└── tools/
    └── signaling-server/
        ├── package.json
        ├── tsconfig.json
        └── src/
            └── index.ts
```

[Source: architecture.md#Complete Project Directory Structure]

### Testing Requirements

- Each package MUST have at least one passing placeholder test
- Tests use vitest (not jest, not mocha)
- Test files co-located with source: `*.test.ts`
- vitest.workspace.ts at root for cross-package execution
- Placeholder tests should be meaningful: verify that the module exports what it should

### Library/Framework Requirements

| Tool | Purpose | Version |
|------|---------|---------|
| pnpm | Package manager | Latest stable |
| TypeScript | Language | 5.x |
| tsup | Build/bundle | Latest stable |
| vitest | Testing | Latest stable |
| biome | Lint + format | Latest stable |
| vite | Demo dev server | Latest stable |

**Zero framework dependency for demo** — vanilla HTML/JS + Vite for dev server only.

### Project Structure Notes

- This is the FIRST file in the project — everything is created from scratch
- No existing code to integrate with
- All paths must match architecture.md exactly
- The scaffold must be complete enough that Story 1.2 (Node Identity) can start immediately after

### References

- [Source: architecture.md#Starter Template Evaluation] — pnpm monorepo, manual scaffold
- [Source: architecture.md#Complete Project Directory Structure] — full directory tree
- [Source: architecture.md#Architectural Boundaries] — dependency direction
- [Source: architecture.md#Implementation Patterns & Consistency Rules] — naming, structure, format
- [Source: architecture.md#Core Architectural Decisions] — ADR-002 signaling temporary
- [Source: epics.md#Story 1.1] — acceptance criteria

## Dev Agent Record

### Agent Model Used

Claude Opus 4.5 (claude-opus-4-5-20251101)

### Debug Log References

- Fixed esbuild warning: `types` condition must come before `import`/`require` in package.json exports
- Added `.claude`, `_bmad-output`, `_bmad` to biome ignore list to avoid linting non-project files
- vitest.workspace.ts shows deprecation warning in vitest 3.x (non-blocking, tests pass)

### Completion Notes List

- All 7 tasks completed successfully
- 4/4 packages build with zero errors and zero warnings
- 4/4 test suites pass (4 tests total)
- biome lint passes with zero errors on 24 project files
- Dependency direction enforced: demo → sdk → core, signaling-server independent
- TypeScript strict mode enabled in all packages via tsconfig.base.json
- Signaling server marked temporary with ADR-002 comment
- All 6 acceptance criteria satisfied

### Change Log

- 2026-02-02: Initial monorepo scaffold created — all packages, configs, and placeholder tests

### File List

- package.json (new)
- pnpm-workspace.yaml (new)
- tsconfig.base.json (new)
- biome.json (new)
- vitest.workspace.ts (new)
- .gitignore (new)
- LICENSE (new)
- README.md (new)
- packages/core/package.json (new)
- packages/core/tsconfig.json (new)
- packages/core/tsup.config.ts (new)
- packages/core/src/index.ts (new)
- packages/core/src/index.test.ts (new)
- packages/sdk/package.json (new)
- packages/sdk/tsconfig.json (new)
- packages/sdk/tsup.config.ts (new)
- packages/sdk/src/index.ts (new)
- packages/sdk/src/index.test.ts (new)
- apps/demo/package.json (new)
- apps/demo/vite.config.ts (new)
- apps/demo/tsconfig.json (new)
- apps/demo/index.html (new)
- apps/demo/src/main.ts (new)
- apps/demo/src/main.test.ts (new)
- tools/signaling-server/package.json (new)
- tools/signaling-server/tsconfig.json (new)
- tools/signaling-server/tsup.config.ts (new)
- tools/signaling-server/src/index.ts (new)
- tools/signaling-server/src/index.test.ts (new)
