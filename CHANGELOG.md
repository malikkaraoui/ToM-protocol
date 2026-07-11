# Changelog

## [0.7.0](https://github.com/malikkaraoui/ToM-protocol/compare/v0.6.0...v0.7.0) (2026-07-11)


### Features

* **apps:** mode campagne + affichage taille lisible (build 8) ([76e0cd8](https://github.com/malikkaraoui/ToM-protocol/commit/76e0cd88bc64e60d41da172baccf20bd90c6ced2))
* **l1-003:** câblage témoin — le relais nourrit WitnessLog depuis les ACK relayés ([698eaef](https://github.com/malikkaraoui/ToM-protocol/commit/698eaef9e404066388e10c6efb42e502be4a6a06))
* **l1-003:** côté témoin — WitnessLog (observations bornées + build view) ([2a4c9c0](https://github.com/malikkaraoui/ToM-protocol/commit/2a4c9c01140e4cebdfe00e2158487d4b59ea5e31))
* **l1-003:** fondation vue signée relais — type wire + doc conception ([7df381b](https://github.com/malikkaraoui/ToM-protocol/commit/7df381b62a9000fcde9f2882c580d93e622bf9ea))
* **l1:** attestation de présence (story L1-001) — module presence + validation réseau réel ([dca0fa5](https://github.com/malikkaraoui/ToM-protocol/commit/dca0fa555ca3af05587e3304aa0c0410e5fd043e))
* **l1:** presence exposée à la flotte — FFI/Swift/apps + runbook tests réels ([5132ce2](https://github.com/malikkaraoui/ToM-protocol/commit/5132ce2cb47715ed57ef7ccec9e40aed5d00abbc))
* **phase-c:** api groupe FFI + serveur de controle in-app (test R13 device) ([3a3c005](https://github.com/malikkaraoui/ToM-protocol/commit/3a3c005b18eb36a2da7574027ce58d4a0b3f1403))
* **r13:** validation reelle offline gap-fill groupe + API de test ([fc1d0ea](https://github.com/malikkaraoui/ToM-protocol/commit/fc1d0ea3601ec2f965c614d28fcd076c57be41a8))
* **redteam:** attaquant presence sur QUIC reel (forge/replay/malformed/flood) ([df93f0c](https://github.com/malikkaraoui/ToM-protocol/commit/df93f0cd2a983d332be87b169dfaa17ed266fe22))
* **redteam:** boîte à outils stress présence + Simian Army + protocole red-team (build 21) ([877a0e6](https://github.com/malikkaraoui/ToM-protocol/commit/877a0e6bc497547c3c342ec454dacf49d6b9e335))
* **tom-tui:** API contrôle enrichie (burst, sendall, metrics) + build 10 ([5488b92](https://github.com/malikkaraoui/ToM-protocol/commit/5488b9287e5418f612cee0147b25a7f2f1cd8b24))
* **tom-tui:** API de contrôle HTTP (--control-port) pour l'orchestration ([6970b7b](https://github.com/malikkaraoui/ToM-protocol/commit/6970b7b9a0e722e05178a7c2b6cb622744cb215d))
* **tom-tui:** mode --campaign + affichage compact des payloads ([542edcd](https://github.com/malikkaraoui/ToM-protocol/commit/542edcd3f04fbd55c28208b400177164fd852b22))
* **tom-tui:** rampe de taille de message scriptée (--size-ramp) ([f53716a](https://github.com/malikkaraoui/ToM-protocol/commit/f53716ac15254033fe0ee53f683a5e6ff9fd99b5))
* **transport:** relais fallback privé injectable à la compilation ([8a1c0c8](https://github.com/malikkaraoui/ToM-protocol/commit/8a1c0c875079a16d02cf37e54b838827296ebfdc))
* **transport:** segmentation des gros messages (jusqu'à 64 Mo) ([0f49941](https://github.com/malikkaraoui/ToM-protocol/commit/0f49941966141f73fa1e9b889f12e30091a9ac54))


### Bug Fixes

* **antispam:** cap global inconnu anti-swarm ingress (build 27) ([3f7ad7c](https://github.com/malikkaraoui/ToM-protocol/commit/3f7ad7c54f0a82afab84ee8036d7686053a7d4d3))
* **apps:** l'écho répond à tout message sauf aux échos (build 7) ([6a1519d](https://github.com/malikkaraoui/ToM-protocol/commit/6a1519d6caaffddb91f48d20ad2f6744852e73c1))
* **apps:** stopper la conso CPU/mémoire injustifiée qui fait tuer l'app par iOS (build 15) ([dbf7d7c](https://github.com/malikkaraoui/ToM-protocol/commit/dbf7d7c752c4e458a660ec33752d585b4283b74e))
* **apps:** tempête d'échos — n'écho-répondre qu'aux sondes PING: ([13f5c10](https://github.com/malikkaraoui/ToM-protocol/commit/13f5c10f39bba8121b55ad997d8df46eedf04ae0))
* **backup:** borne + eviction equitable du store replica — anti-flood (build 30) ([87c3567](https://github.com/malikkaraoui/ToM-protocol/commit/87c3567382f11fdfb6e53d84123b930a36e4cecc))
* **backup:** borner replicated_to reçu d'un pair (anti-DoS mémoire) ([2d7738a](https://github.com/malikkaraoui/ToM-protocol/commit/2d7738a87bb3c4545b3797b39797b6a11002f745))
* budget deux-voies. les pairs connus (preuve de relais, score local ([fa61b53](https://github.com/malikkaraoui/ToM-protocol/commit/fa61b53005a07ce32549cbb8262e512f8d8108cd))
* cap global inconnu deux-voies (meme motif que presence [#5](https://github.com/malikkaraoui/ToM-protocol/issues/5)). budget ([3f7ad7c](https://github.com/malikkaraoui/ToM-protocol/commit/3f7ad7c54f0a82afab84ee8036d7686053a7d4d3))
* **ci:** clippy stable 1.97 sur tom-quinn-proto + tom-relay ([048ced7](https://github.com/malikkaraoui/ToM-protocol/commit/048ced73cc298d0a59bab210fb101ce5a07998c2))
* **ci:** clippy::manual_is_multiple_of dans stability_2min ([2ca3829](https://github.com/malikkaraoui/ToM-protocol/commit/2ca38293fb7e5fc9a14c1a457a072cbca9545757))
* **ci:** sérialise les tests d'intégration réseau (--test-threads=1) ([455b9de](https://github.com/malikkaraoui/ToM-protocol/commit/455b9de16062784fa40caf169cd28f28d2e437da))
* **ci:** tests integration + advisory crossbeam — CI verte ([d97888b](https://github.com/malikkaraoui/ToM-protocol/commit/d97888b01c4e82eccb3e995c2845908892cca758))
* **ci:** triage RUSTSEC-2026-0194/0195 (quick-xml, DoS XML non fiable) ([ddf42f7](https://github.com/malikkaraoui/ToM-protocol/commit/ddf42f7c1996b737e77a0ea075b7ca153019b332))
* **delivery:** lier l ack au destinataire prevu — verrou [#1](https://github.com/malikkaraoui/ToM-protocol/issues/1) (build 25) ([ea9da38](https://github.com/malikkaraoui/ToM-protocol/commit/ea9da38cb4f4de37026fa97ab2d846d516b11800))
* **dht:** borner les adresses acceptées d'un enregistrement DHT (anti-DoS) ([3d8cdfb](https://github.com/malikkaraoui/ToM-protocol/commit/3d8cdfbf37428b8b244e2b92bccf942dc34bdfcc))
* ensure!(content.len() &gt;= EndpointId::LENGTH, Error::InvalidFrame) avant ([0d79438](https://github.com/malikkaraoui/ToM-protocol/commit/0d794386d1096129baceb48ab478050ca209f5b1))
* **ffi:** régénérer le header C (dérive doc-comment tom_node_free) ([e6d3501](https://github.com/malikkaraoui/ToM-protocol/commit/e6d3501bbc3124e3f50c1a6895e858048b867d08))
* **ffi:** test unitaire cassé — champs relais manquants dans RuntimeConfigFFI ([d512e01](https://github.com/malikkaraoui/ToM-protocol/commit/d512e01a948683f6f92f6d4c2102ecc61ca8107a))
* **group+dht:** durcissement groupes + filtre multicast + verify_strict (build 31) ([5459528](https://github.com/malikkaraoui/ToM-protocol/commit/5459528f13e28f92b37c3349c973d8feb4407418))
* **group:** borner les members reçus dans HubShadowSync (anti-DoS mémoire) ([1656abb](https://github.com/malikkaraoui/ToM-protocol/commit/1656abbecd5d18cf2829bf474aec93ef026609cb))
* **ios,tvos:** Stop toujours obéi — course start/stop éliminée ([2007253](https://github.com/malikkaraoui/ToM-protocol/commit/20072533586bc7444454259ee057ee636dce516f))
* **ios:** appeler handleEnterBackground sur la vraie transition .background ([3fa8ebe](https://github.com/malikkaraoui/ToM-protocol/commit/3fa8ebe94279460da3083bc65ea239890f8b8b9a))
* mark_delivered/mark_read exigent entry.to == from (destinataire prevu), ([ea9da38](https://github.com/malikkaraoui/ToM-protocol/commit/ea9da38cb4f4de37026fa97ab2d846d516b11800))
* **pop:** exiger un travail soutenu (pas juste 1 relais) pour KNOWN ([5104b03](https://github.com/malikkaraoui/ToM-protocol/commit/5104b0378c768195d01f49fc3a5b16bef47d8098))
* **pop:** gater la credence de presence sur signature_valid ([00e7a4e](https://github.com/malikkaraoui/ToM-protocol/commit/00e7a4e250698bec692a641bb6c453acd41316cc))
* **pop:** separer Known (adresse decouverte) de Online (travail prouve) ([f0e11cc](https://github.com/malikkaraoui/ToM-protocol/commit/f0e11ccb00e470934b381f6cdde08a213847670b))
* **presence:** budget repondeur deux-voies anti-famine (build 24) ([fa61b53](https://github.com/malikkaraoui/ToM-protocol/commit/fa61b53005a07ce32549cbb8262e512f8d8108cd))
* **presence:** plafond global responder anti-sybil-swarm (build 23) ([aa220eb](https://github.com/malikkaraoui/ToM-protocol/commit/aa220eb0f1da02e17738f5004b0ee9e843afad68))
* **protocol:** corrige l'inflation ×8 des payloads (DoS gros messages) ([94a7851](https://github.com/malikkaraoui/ToM-protocol/commit/94a7851c386e8645af5f65fd71cfe16a3e38f9a6))
* **protocol:** failover hub réel bloqué par boucle runtime + event mort ([a95175a](https://github.com/malikkaraoui/ToM-protocol/commit/a95175ad3824f9eb8971a85fb0858e63c7931dbd))
* **protocol:** le garde anti-spam 256 Ko tuait les gros messages chunkés ([7a008a1](https://github.com/malikkaraoui/ToM-protocol/commit/7a008a18542559fa47d0a6f0547b9b0d69d040bf))
* **protocol:** ré-ACK des doublons pour survivre à la perte d'ACK (audit [#5](https://github.com/malikkaraoui/ToM-protocol/issues/5)) ([04a0add](https://github.com/malikkaraoui/ToM-protocol/commit/04a0add576e8eec6de8bb95aaf271709fd88e9e2))
* record_relay credite seulement si tracker.recipient_of(msgid) existe ([4964ac9](https://github.com/malikkaraoui/ToM-protocol/commit/4964ac94ceb83f66bfb5e7cfaa6bc3e8c8094e74))
* **redteam:** harnais enable_dht=false (stop pollution rendez-vous mondial) ([f619402](https://github.com/malikkaraoui/ToM-protocol/commit/f6194029209abad1c9d57b4377c2de2059ee3dc4))
* **relay:** bounds-check frame datagram client — anti-panic distant (build 29) ([0d79438](https://github.com/malikkaraoui/ToM-protocol/commit/0d794386d1096129baceb48ab478050ca209f5b1))
* **roles:** anti-pumping du score de relais via ack forge (build 26) ([4964ac9](https://github.com/malikkaraoui/ToM-protocol/commit/4964ac94ceb83f66bfb5e7cfaa6bc3e8c8094e74))
* **roles:** borne le bandwidth_ratio anti-inflation de score (red-team PoP) ([78f7980](https://github.com/malikkaraoui/ToM-protocol/commit/78f79808cb2c06bdc9a3f2a20d457677a9ca9cdd))
* **runtime:** finding [#1](https://github.com/malikkaraoui/ToM-protocol/issues/1) — borne les ops recovery du select! loop (build 22) ([61d8723](https://github.com/malikkaraoui/ToM-protocol/commit/61d8723f1421dd2b3e3f2f860d2b2156c0595dcd))
* **runtime:** stopper le busy-spin du select! sur canal fermé (CPU 100%) ([68a7022](https://github.com/malikkaraoui/ToM-protocol/commit/68a7022b536f3d3d2b34da9c1605aad301d59aed))
* **sécurité:** ferme le trou hub-hijack résiduel (shadow signé diffusé) ([33aba6a](https://github.com/malikkaraoui/ToM-protocol/commit/33aba6ac54f3fd4f517e475821ec6d26c57ee7a7))
* **sécurité:** ferme les verrous [#1](https://github.com/malikkaraoui/ToM-protocol/issues/1)/[#2](https://github.com/malikkaraoui/ToM-protocol/issues/2) + failover hub réel + hijack + anti-squat DHT ([c3b7f9a](https://github.com/malikkaraoui/ToM-protocol/commit/c3b7f9a782e063ddc5d386c09903dc6858d19708))
* **tests:** hermétise multi_node — mDNS coupé sur les nodes de test ([22e133f](https://github.com/malikkaraoui/ToM-protocol/commit/22e133fda8626b0c96444510a63619eefa7ca01c))
* **tom-protocol:** log warning quand send_with_retry épuise ses tentatives ([6c8da16](https://github.com/malikkaraoui/ToM-protocol/commit/6c8da16f3bd6b79a856f54ff0c08d9c75142d341))
* **tom-transport:** borne à 3s le lookup DNS TXT fallback ([dc0034d](https://github.com/malikkaraoui/ToM-protocol/commit/dc0034d3a1db9ad1bf67b0c1b92c89fa9f5752a2))
* **tom-tui:** le mode bot avalait silencieusement ProtocolEvent::Error ([8c85cf9](https://github.com/malikkaraoui/ToM-protocol/commit/8c85cf96cf28290f81434af16937409c41bbc848))
* **topology:** eviction du plus-ancien-offline quand plein — anti-swarm decouverte (build 28) ([1efe6c0](https://github.com/malikkaraoui/ToM-protocol/commit/1efe6c0ba40c68d0865a8b5b2e3dd672a00c8de8))
* **transport:** borner la mémoire de réassemblage — anti-amplification + budget global ([347421b](https://github.com/malikkaraoui/ToM-protocol/commit/347421b0dacdf9ccb8b318cdfe07b520fd0bde84))
* **transport:** cap transferts concurrents/pair + TTL sur partiels (anti-DoS) ([a3cd0ef](https://github.com/malikkaraoui/ToM-protocol/commit/a3cd0efa53c40f01fcf5be17e075820f79750229))
* **tvos:** appeler handleEnterBackground sur la vraie transition .background ([d21f79d](https://github.com/malikkaraoui/ToM-protocol/commit/d21f79db2c7daf2386338044d08b3abdc76131a1))
* **tvos:** référence explicite du package local TomProtocolKit ([327fb4e](https://github.com/malikkaraoui/ToM-protocol/commit/327fb4e3f4755fca84bcff06b7e574b2383f0626))
* **ui:** borner les chaînes passées au typographe SwiftUI (anti-watchdog) ([81b244a](https://github.com/malikkaraoui/ToM-protocol/commit/81b244afb66f3a4cdb8efb67972ab0cbe6f023bc))

## [0.6.0](https://github.com/malikkaraoui/ToM-protocol/compare/v0.5.0...v0.6.0) (2026-06-23)


### Features

* **dashboard:** redesign UX visualisation réseau P2P zero-config ([5be51f9](https://github.com/malikkaraoui/ToM-protocol/commit/5be51f94893d817638f9597547a537089a3cebab))
* **dht:** rendez-vous DHT partagé pour découverte zéro-config ([3858098](https://github.com/malikkaraoui/ToM-protocol/commit/385809875a87c453796ecb3c24ee47963f8ac2d1))
* **gateway:** commande teardown des règles NAT du relais ([3f3c6f8](https://github.com/malikkaraoui/ToM-protocol/commit/3f3c6f8f1a1f1bb113ac7ba0d686e7e670937a5d))
* **protocol:** runtime — récupération d'isolement + rendez-vous DHT ([555f5d4](https://github.com/malikkaraoui/ToM-protocol/commit/555f5d4ed8eaf6368a0855f4f829130a0bc41778))


### Bug Fixes

* **dht,protocol:** preuve-de-possession du rendez-vous DHT (audit [#2](https://github.com/malikkaraoui/ToM-protocol/issues/2)) ([7447324](https://github.com/malikkaraoui/ToM-protocol/commit/7447324f9e21e6fea8b10f5a97a0dece87a198ae))
* **dht:** mettre à jour les lockfiles après ajout de sha2 à tom-dht ([55cde19](https://github.com/malikkaraoui/ToM-protocol/commit/55cde19ecdbc64d368be035df6e7bec6ba445c1f))
* **ffi:** ajouter les champs full-node à RuntimeConfigFFI ([1096981](https://github.com/malikkaraoui/ToM-protocol/commit/1096981806e64e9089a6ee36ee92be301178f2fb))
* **ffi:** teardown détaché — Stop ne bloque plus jamais ([61ca8f1](https://github.com/malikkaraoui/ToM-protocol/commit/61ca8f134836774a98c553ef72d466553b076cb5))
* **gateway:** demander toutes les permissions Freebox à l'auth ([807ed7a](https://github.com/malikkaraoui/ToM-protocol/commit/807ed7a409eedcfe19e83d76fdf3fc3f3734de87))
* **ios,tvos:** keepalive anti-veille résilient aux interruptions audio ([185f1b8](https://github.com/malikkaraoui/ToM-protocol/commit/185f1b84763d6405ad03f5bc4b9bb3f804490930))
* **ios,tvos:** Stop instantané — l'UI ne bloque plus sur 'Stopping' ([9fe6121](https://github.com/malikkaraoui/ToM-protocol/commit/9fe6121b56085b018990df0ed628c771e19714ac))
* **ios:** redémarrer le nœud seulement après un vrai arrière-plan ([b3353d2](https://github.com/malikkaraoui/ToM-protocol/commit/b3353d288a9f48a5044b9b5faae8d8aad47041a5))
* **macos,tvos:** redémarrer le nœud seulement après un vrai arrière-plan ([893ef1d](https://github.com/malikkaraoui/ToM-protocol/commit/893ef1d22d0116f3beb3fefb5232ccbe2047bc0b))
* **protocol,ffi:** arrêt borné — le bouton Stop ne bloque plus ([3eefe65](https://github.com/malikkaraoui/ToM-protocol/commit/3eefe655a4357977a926a70c9f1fa7e1da5b9e20))
* **protocol:** détecter les connexions zombies via la vivacité (audit [#1](https://github.com/malikkaraoui/ToM-protocol/issues/1)) ([0b37023](https://github.com/malikkaraoui/ToM-protocol/commit/0b370235aafd3fd9e294dbcbbf463bf1e706002d))
* **protocol:** filtrer les adresses directes DHT non joignables (audit [#6](https://github.com/malikkaraoui/ToM-protocol/issues/6)) ([007c0d5](https://github.com/malikkaraoui/ToM-protocol/commit/007c0d5a1815766b457ac433c3b4c590efeba37e))
* **protocol:** ne jamais publier au gossip un relai non joignable globalement ([498a76b](https://github.com/malikkaraoui/ToM-protocol/commit/498a76b10dd41ac46e455a656790505796eb50bd))
* **protocol:** phase d'amorçage réversible sur isolement ([0ceaf2f](https://github.com/malikkaraoui/ToM-protocol/commit/0ceaf2f7247e75197731d5d7027c5ed01cfa6ccc))
* **protocol:** purge TTL du cache anti-replay nonce (audit [#7](https://github.com/malikkaraoui/ToM-protocol/issues/7)) ([e01a443](https://github.com/malikkaraoui/ToM-protocol/commit/e01a443441544c8ef3ddba41807637e226a35064))
* **tom-sdk:** observabilité des événements droppés (review S0→S3) ([adb7935](https://github.com/malikkaraoui/ToM-protocol/commit/adb7935484ce14f4dd1f9510b2b3d0c80716b8eb))
* **tom-sdk:** tracer les evenements protocole droppes par le merger ([c8fc481](https://github.com/malikkaraoui/ToM-protocol/commit/c8fc48127d35e925e02e93c072bab37e6721e09f))

## [0.5.0](https://github.com/malikkaraoui/ToM-protocol/compare/v0.4.0...v0.5.0) (2026-06-12)


### Features

* **ios:** migrer TomNode iOS vers le package TomProtocolKit (S2.4) ([9345774](https://github.com/malikkaraoui/ToM-protocol/commit/9345774d5d45d2b9286389183543272e09f9c193))
* S2.4 — migration des apps iOS/tvOS/macOS vers TomProtocolKit ([96fa1a9](https://github.com/malikkaraoui/ToM-protocol/commit/96fa1a90ab4aba15eecca772b60526f20b03caf7))
* **sdk-swift:** porter les exigences de link de la lib Rust dans le package ([6deb282](https://github.com/malikkaraoui/ToM-protocol/commit/6deb2820cf5a23eb7fb572078edeae2377349c61))
* **tvos:** sortir du pbxproj manuel et migrer vers TomProtocolKit (S2.4) ([e909074](https://github.com/malikkaraoui/ToM-protocol/commit/e9090743af592f71db6ba81753fa8620409ae228))


### Bug Fixes

* **s2.4:** traiter les findings de la review adversariale ([c210848](https://github.com/malikkaraoui/ToM-protocol/commit/c210848543cbdd7cea25bacdca21845bef4231c1))
* **securite:** bump hickory-resolver 0.24/0.25 vers 0.26.1 + swarm-discovery 0.6.1 ([c7d7370](https://github.com/malikkaraoui/ToM-protocol/commit/c7d7370a496969bc2f5ad961e335bb531ddcebde))
* **securite:** hickory 0.26 — RUSTSEC-2026-0119 + 0118 corrigées (PR [#45](https://github.com/malikkaraoui/ToM-protocol/issues/45)) ([463c779](https://github.com/malikkaraoui/ToM-protocol/commit/463c779553392e40df7de44651d6fa6eccd52057))
* **securite:** retire les ignores hickory de deny.toml + resynchronise le lock FFI ([5e89d6c](https://github.com/malikkaraoui/ToM-protocol/commit/5e89d6cd0a55b08d4a7f7747c805067c245f0ce9))
* **tom-connect,tom-transport:** adapte serveur DNS de test et resolver à hickory 0.26 ([4e285a6](https://github.com/malikkaraoui/ToM-protocol/commit/4e285a61a99df638e2c24d987b03a9675abc7b70))
* **tom-relay:** adapte le resolver DNS à l'API hickory 0.26 ([7cbc8f2](https://github.com/malikkaraoui/ToM-protocol/commit/7cbc8f2ea9450c88e10d503ffa9cb3799fb49b83))

## [0.4.0](https://github.com/malikkaraoui/ToM-protocol/compare/v0.3.0...v0.4.0) (2026-06-12)


### Features

* **ffi:** header c généré par cbindgen + check de drift en ci ([02eea51](https://github.com/malikkaraoui/ToM-protocol/commit/02eea51643061e99e68a36c3af113226879e2bd3))
* **sdk-swift:** package TomProtocolKit — wrappers dé-dupliqués ([3194ed2](https://github.com/malikkaraoui/ToM-protocol/commit/3194ed2f6113f60ebf1114d1ae8d71ffea799cb7))
* **spec:** test vectors protocole — générateur déterministe auto-vérifié ([25da859](https://github.com/malikkaraoui/ToM-protocol/commit/25da8595db3f3de2a1a4dd2e3368727936ee0af6))


### Bug Fixes

* **ffi:** resynchronise le Cargo.lock de tom-protocol-ffi après unification lru 0.16.3 ([0bf440c](https://github.com/malikkaraoui/ToM-protocol/commit/0bf440c5d9bb0508528eedd10065e264206fa345))
* **securite:** 4 vulnérabilités rustls-webpki corrigées — RUSTSEC 0049/0098/0099/0104 (PR [#42](https://github.com/malikkaraoui/ToM-protocol/issues/42)) ([a2496c6](https://github.com/malikkaraoui/ToM-protocol/commit/a2496c604a4f6dd0497b07154a530338ad7e6d23))
* **securite:** aligne rustls-webpki 0.103.13 dans le lock de tom-protocol-ffi ([54f23ca](https://github.com/malikkaraoui/ToM-protocol/commit/54f23cab085866ea49364b8de12ca7d8b2d24a76))
* **securite:** batch advisories aws-lc du 2026-06-11 — ci débloquée ([f892be4](https://github.com/malikkaraoui/ToM-protocol/commit/f892be4c38ac0fc0602e7c58b2c0d7b52d0d2879))
* **securite:** batch advisories aws-lc du 2026-06-11 + gcc unmaintained (PR [#44](https://github.com/malikkaraoui/ToM-protocol/issues/44)) ([375193d](https://github.com/malikkaraoui/ToM-protocol/commit/375193d95f49dbee1e028940dee643af5f544eb5))
* **securite:** corrige 4 vulnérabilités rustls-webpki via cargo update ([21fe1d2](https://github.com/malikkaraoui/ToM-protocol/commit/21fe1d2504f6794647cd80f4991489ed5dcbb879))
* **securite:** pin lru &gt;=0.16.3 — applique la review copilot (PR [#43](https://github.com/malikkaraoui/ToM-protocol/issues/43)) ([c5b4e95](https://github.com/malikkaraoui/ToM-protocol/commit/c5b4e950c60757e0f7e9f9b86e12bc51d8723393))
* **securite:** supprime la dépendance morte gcc 0.3.55 de tom-quinn ([c8c5dc3](https://github.com/malikkaraoui/ToM-protocol/commit/c8c5dc3caffc50d0919ea7510ce8641ef2d127a8))
* **securite:** triage rand + lru — RUSTSEC-2026-0002 corrigé (PR [#43](https://github.com/malikkaraoui/ToM-protocol/issues/43)) ([084dde7](https://github.com/malikkaraoui/ToM-protocol/commit/084dde7c389e06bcada1ddf4a1a11faf7fe1dc3d))
* **securite:** triage rand + lru — usage direct lru corrigé (0.12→0.16) ([5c4163b](https://github.com/malikkaraoui/ToM-protocol/commit/5c4163b27a9b553c0d38bc3dcf5911a65cbd7249))

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
