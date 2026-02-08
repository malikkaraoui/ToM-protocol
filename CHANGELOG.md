# Changelog

## [1.1.0](https://github.com/malikkaraoui/ToM-protocol/compare/v1.0.0...v1.1.0) (2026-02-08)


### Features

* add automatic rerouting on relay failure (Story 5.2) ([be815b8](https://github.com/malikkaraoui/ToM-protocol/commit/be815b8ca8c89f1b43447d19115e448b9c6d7f72))
* add chaos/stress test suite (22 tests) ([a315452](https://github.com/malikkaraoui/ToM-protocol/commit/a315452aac717998d92c4d37f12f4357d35bf2b8))
* add comprehensive input validation & boundary tests (61 tests) ([e7c62a6](https://github.com/malikkaraoui/ToM-protocol/commit/e7c62a61c7d8d1de71db3bce7adb77fa8166912f))
* add deterministic HubElection for hub failover (Action 1) ([60b3808](https://github.com/malikkaraoui/ToM-protocol/commit/60b38088568c8d7b546621c68145ced8349985fd))
* add end-to-end encryption with TweetNaCl.js (Story 6.1) ([211b15d](https://github.com/malikkaraoui/ToM-protocol/commit/211b15df3d943878dd1d08000e3a2d2b9324681d))
* add hub failover and E2E metrics testing framework ([bcd6c6c](https://github.com/malikkaraoui/ToM-protocol/commit/bcd6c6cd2d7b5935cabbe0576cc31dbb89eedec4))
* add LLM-first documentation and MCP server (Stories 8.1 & 8.2) ([41893a5](https://github.com/malikkaraoui/ToM-protocol/commit/41893a57bb167348d4143e3373a3eb7c423d150d))
* add multi-relay message traversal (Story 5.1) ([21d833b](https://github.com/malikkaraoui/ToM-protocol/commit/21d833ba26fa397f8231e40d2faa7c16ae9d95de))
* add reactive UIStateManager for centralized state updates (Action 3) ([fb29f5e](https://github.com/malikkaraoui/ToM-protocol/commit/fb29f5e70c60480ce3a478e1c7be34d8a82286dc))
* add robust E2E test infrastructure for Phase 6 ([ba9dfc9](https://github.com/malikkaraoui/ToM-protocol/commit/ba9dfc930c3f81b4a7574a9c926ab5937f119cb5))
* add VS Code extension and demo launcher (Story 8.3) ([5f1bef8](https://github.com/malikkaraoui/ToM-protocol/commit/5f1bef8243d63d8c869a71047b39b40b0dc8aa9c))
* complete phase 1 - docs, issues, and td-001 countdown sync ([9024225](https://github.com/malikkaraoui/ToM-protocol/commit/9024225a51904558ff9bc153a117f0d5c18492cd))
* display contribution/usage equilibrium score in demo UI (Story 5.4) ([fa74ddd](https://github.com/malikkaraoui/ToM-protocol/commit/fa74ddd6d61a0061174d6413e8d8b950fd7f7487))
* implement micro-session contribution model (Story 8.4) ([6b1020f](https://github.com/malikkaraoui/ToM-protocol/commit/6b1020ff427e59d5f79614da2008cbc61cdfc157))
* implement robust group invitations (Consolidation Action 2) ([49ad5bd](https://github.com/malikkaraoui/ToM-protocol/commit/49ad5bd69123464dda5abef0f0580e9b04bd3758))
* implement self-sustaining alpha network (Epic 7) ([2ab543d](https://github.com/malikkaraoui/ToM-protocol/commit/2ab543d5f1c6d0f9c2d9360a8d7c718dea4ec25d))
* **test:** add Playwright E2E testing framework (Action 4) ([a51a0fa](https://github.com/malikkaraoui/ToM-protocol/commit/a51a0fa0d45704ce132153e083af792ee9ccde41))


### Bug Fixes

* address code review findings from Copilot review ([46b822e](https://github.com/malikkaraoui/ToM-protocol/commit/46b822e852c1ac976bc2ac51843470b251209626))
* e2e tests start full demo stack (demo + signaling) ([7bf15a5](https://github.com/malikkaraoui/ToM-protocol/commit/7bf15a520dd5d0668f922c89ceebb12d980c8bd0))
* ensure direct connection before sending group invitations ([7994911](https://github.com/malikkaraoui/ToM-protocol/commit/79949114fbf0596e2771a1b82eaee64d36375666))
* prevent relay nodes from processing group payloads not addressed to them ([c074fe3](https://github.com/malikkaraoui/ToM-protocol/commit/c074fe36bba6193c9fb05d2cbe354c709b54c2ca))
* **security:** replace weak Math.random() with crypto APIs (CVSS 7.5) ([20a5c28](https://github.com/malikkaraoui/ToM-protocol/commit/20a5c28c1893d8d09fa451b176f55dc3df127294))
* **snake:** gpt-5.2 security hardening and collision edge cases ([c4057c8](https://github.com/malikkaraoui/ToM-protocol/commit/c4057c8c43e9eec9f8baa311880f0790a10df76d))

## 1.0.0 (2026-02-05)


### Features

* add automatic recipient acknowledgment (ACK) ([1961fcf](https://github.com/malikkaraoui/ToM-protocol/commit/1961fcf114d4db160baee84c37b9895a45394ac9))
* add Ed25519 node identity generation and persistence ([25c36ce](https://github.com/malikkaraoui/ToM-protocol/commit/25c36ceeb5ed88febb8bf54e345dd09add6a4b00))
* add group chat UI with French localization and self-hub support ([37e1c15](https://github.com/malikkaraoui/ToM-protocol/commit/37e1c151aac356e7a4f4a9f8d5962e71b69b0799))
* add group invite functionality ([e9d424b](https://github.com/malikkaraoui/ToM-protocol/commit/e9d424bd0dd7f17740af338a47eaea89774a0d1b))
* add message router for relay-based forwarding ([383264a](https://github.com/malikkaraoui/ToM-protocol/commit/383264a36e589353fc6689dcf9bf91669fe40241))
* add shared types (MessageEnvelope, TomError, events) ([500e9b6](https://github.com/malikkaraoui/ToM-protocol/commit/500e9b696770c928d285cbaa22f291b456d7222e))
* add transport layer abstraction for WebRTC DataChannel ([f668374](https://github.com/malikkaraoui/ToM-protocol/commit/f6683742ad2f98aab24545931bed7ca2f19209c7))
* **demo:** complete story 2.5 demo chat UI with SDK client ([5c3b4f2](https://github.com/malikkaraoui/ToM-protocol/commit/5c3b4f2fdb285f56759a78c9f0fc9ce57f4a5046))
* fix relay ACK delivery and improve mobile UI ([b96e801](https://github.com/malikkaraoui/ToM-protocol/commit/b96e8019b4e69e2de8ae7efa33f5cdfaa9ee2de1))
* implement automatic relay selection (Story 3.3) ([c8fbb85](https://github.com/malikkaraoui/ToM-protocol/commit/c8fbb852a6e7aaf574e2edc0fe9bfafc01515231))
* implement deterministic relay consensus (Story 3.2 fixes) ([4a2593a](https://github.com/malikkaraoui/ToM-protocol/commit/4a2593a4f30e295d35698f0794f8eee8a1de90e6))
* implement dual-role node with relay stats (Story 3.4) ([4c54894](https://github.com/malikkaraoui/ToM-protocol/commit/4c5489484a9aea07f0361c6085a6ee814f1e8d56))
* implement dynamic role assignment (Story 3.2) ([e772bbf](https://github.com/malikkaraoui/ToM-protocol/commit/e772bbfc92d4ee9689877db188281adfceea1e12))
* implement message path visualization (Story 4.3) ([15be99b](https://github.com/malikkaraoui/ToM-protocol/commit/15be99bf9fa7d6b648440fc1837e390fa5b640e4))
* implement multiplayer Snake game (Story 4.5) ([968d41c](https://github.com/malikkaraoui/ToM-protocol/commit/968d41c4b4aecacef8669af0c7a13c3b7f3aae45))
* implement peer discovery protocol (Story 3.1) ([284abea](https://github.com/malikkaraoui/ToM-protocol/commit/284abea64107373490e93d6c91ea6d8062b4a7c4))
* implement Stories 3.2-3.5 with GPT 5.2 security hardening ([428797c](https://github.com/malikkaraoui/ToM-protocol/commit/428797c8d707da0adf86eb0992bf870601948360))
* implement Stories 4.1, 4.2, 4.4 with GPT 5.2 security hardening :) ([faed97a](https://github.com/malikkaraoui/ToM-protocol/commit/faed97a7683a5e4299ad8bf4cf0cf29a197198e7))
* implement Story 3.5 Bootstrap & Participation Vectors ([3cd33e3](https://github.com/malikkaraoui/ToM-protocol/commit/3cd33e3818bb1255c5a9a0e96617ceb9ff6335c1))
* implement WebSocket signaling server bootstrap ([a1f09b1](https://github.com/malikkaraoui/ToM-protocol/commit/a1f09b19230540399d4300f29991b90621b700d4))
* initialize tom-protocol monorepo with full project scaffold ([ca5a272](https://github.com/malikkaraoui/ToM-protocol/commit/ca5a2722e0eed86dfe0c5c7f3cb641d8666f96a3))
* toroidal grid and longest-snake-wins collision rules ([1be7c35](https://github.com/malikkaraoui/ToM-protocol/commit/1be7c35f31d654a1a25c2314aa0ea083ff35751f))


### Bug Fixes

* add missing group type exports for CI build ([a9e774c](https://github.com/malikkaraoui/ToM-protocol/commit/a9e774cfd09304a0777be57ad8d67bd12cb023e1))
* add missing hubRelayId in test handleInvite calls ([14cb59c](https://github.com/malikkaraoui/ToM-protocol/commit/14cb59c3264c24382ac9239f70c92c8beecbeb1b))
* don't remove peers from topology on heartbeat timeout ([4bf6986](https://github.com/malikkaraoui/ToM-protocol/commit/4bf698612e424ae607a5666035eebf5860abac41))
* group invitations via direct 1-to-1 channels only ([372bbc2](https://github.com/malikkaraoui/ToM-protocol/commit/372bbc2d1e4e938001d5297fcfb5e159688999a3))
* heartbeat timeout must be greater than send interval ([12d89be](https://github.com/malikkaraoui/ToM-protocol/commit/12d89be55054a5ef88a6d5a6ed6466703eb1c487))
* improve game session management and edge cases ([3de417a](https://github.com/malikkaraoui/ToM-protocol/commit/3de417acfef7762931bf11768abb47144ba0fe7c))
* keep peers alive via heartbeat and periodic UI refresh ([8eaa760](https://github.com/malikkaraoui/ToM-protocol/commit/8eaa76098b1d12ab2073915a4ba1c9e0a5e79056))
* make chat UI responsive for mobile devices ([1fe1d1e](https://github.com/malikkaraoui/ToM-protocol/commit/1fe1d1ea22f69f8d6c8e2a82e2fc0542534e1359))
* prevent duplicate group joins and ensure member sync consistency ([9a56a82](https://github.com/malikkaraoui/ToM-protocol/commit/9a56a823ddff96b65e19b2cead7506cda756c47e))
* **sdk:** fix message relay, mobile crypto fallback, and Enter key UX ([0ed0194](https://github.com/malikkaraoui/ToM-protocol/commit/0ed01949cbdc55e7357916c39d8f7447fbca5905))
* security hardening for Snake game (Story 4.5) ([8ba7a3e](https://github.com/malikkaraoui/ToM-protocol/commit/8ba7a3ec8febebe82ddff6572f452fe507ee902a))
* sync topology with participants list on connect ([4e6a61f](https://github.com/malikkaraoui/ToM-protocol/commit/4e6a61f98d154640203e4baf692f50695c66f94b))
* update group-manager test for new acceptInvite behavior ([4ff9984](https://github.com/malikkaraoui/ToM-protocol/commit/4ff9984d3b805f4f81df43407083f876722348b1))
