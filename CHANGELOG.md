# Changelog

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
