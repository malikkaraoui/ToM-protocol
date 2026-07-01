# 🎯 MISSION — ToM Protocol : atteindre la cible finale

> Document maître. Lu en premier par tout agent (Claude Code / Fable 5) qui reprend le projet.
> Rédigé 2026-06-26. Ne PAS diluer la cible. Si un choix s'éloigne de la cible → il est refusé.
> Sources : `_bmad-output/planning-artifacts/product-brief-tom-protocol-2026-01-30.md`, `CLAUDE.md` (7 verrous), `docs/audits/AUDIT-2026-06-26.md`.

---

## 0. Comment lire ce document

1. **§1 = LA CIBLE.** Non négociable. C'est le nord. Tout le reste sert §1.
2. **§2 = les formes de livraison** (SDK + au-delà).
3. **§3 = où on en est** (état réel, audit).
4. **§4 = comment tu travailles** (agents, skills, journal, boucle, tests durs, auto-critique).
5. **§5 = « cible atteinte » = quoi exactement** (Definition of Done mesurable).
6. **§6 = garde-fous** (les 7 verrous, ne pas changer de cap).

---

## 1. LA CIBLE FINALE (détail complet — à ne jamais oublier)

**ToM (The Open Messaging) est une COUCHE DE TRANSPORT universelle — comme TCP/IP — pas une application.** Elle transporte des octets de A à B, librement, sans permission, sans serveur, sans péage, sans trace. *(brief l.14, l.60)*

La cible est atteinte quand le réseau possède **simultanément** toutes ces propriétés :

### 1.1 Réseau complètement distribué — chacun sa part de travail
- **L'infrastructure, ce sont les utilisateurs.** Aucun serveur, aucun serveur fédéré, aucun relais bénévole, aucun nœud de bootstrap contrôlé par une entité unique. *(brief l.54, l.66)*
- Tout appareil avec CPU + connectivité + un minimum de stockage devient un nœud : smartphone, box internet, routeur, TV, terminal de paiement, objet connecté. *(brief l.50)*
- **Utiliser = contribuer.** Participation automatique et invisible. Pas d'opt-in, pas de « merci de seeder ». *(brief l.53)*

### 1.2 Rôle NON choisi — assigné par le besoin du réseau
- Les rôles (client, relais, hub, backup…) sont **imposés dynamiquement et de façon imprévisible**, au dernier moment, en fonction du **besoin du réseau** et de la **capacité de l'appareil** — jamais choisis par l'utilisateur. *(brief l.65, verrou #3, ADR-006)*
- Impossible à truquer : on ne peut pas décider d'être relais ni d'y échapper.

### 1.3 Indébranchable — propagation « virus » proportionnelle à la demande
- **Plus il y a de demande, plus l'effort se propage** à travers le réseau, **sans pouvoir être arrêté ni débranché.** Couper un nœud ne coupe rien : les messages se re-routent, les rôles se réassignent. *(brief l.194, ADR-009)*
- **Métaphore virus (backup)** : les messages pour destinataires hors-ligne **se répliquent** sur des nœuds de secours, puis **s'auto-détruisent** à la livraison ou après TTL. Résilience par réplication, pas par centralisation. *(ADR-009)*
- **Aucun point de coupure = aucun point de contrôle.** C'est le cœur politique du projet. *(brief l.24, l.33)*

### 1.4 Toujours chiffré (E2E obligatoire)
- **Toutes** les données sont chiffrées de bout en bout. Aucun nœud de la chaîne ne peut lire le contenu. *(brief l.172, ADR-004)*
- Stack : Ed25519 (signature) + X25519 (DH) + XChaCha20-Poly1305 (AEAD) + HKDF-SHA256. Encrypt-then-sign. *(ADR-004)*

### 1.5 Purge par défaut — le réseau oublie
- **Message livré = message supprimé.** L'état passé est compacté puis effacé. TTL 24h max, purge globale sans exception. Le réseau reste léger quel que soit son âge. *(brief l.55, verrou #2, ADR-002)*
- **Livraison = ACK du destinataire final.** Un message est livré ⟺ le destinataire émet un ACK (signé). *(verrou #1)*

### 1.6 Invisible & multi-plateforme
- **Intégration invisible** : livré comme couche SDK/protocole. L'utilisateur ouvre son app, ça marche. Zéro config, zéro friction. Personne ne sait qu'il utilise ToM — et c'est pour ça que ça marche. *(brief l.56, l.146, verrou #6)*
- **Multi-plateforme par nature** : navigateur (WASM), iOS/tvOS/macOS, Android, Linux/Windows, IoT/routeurs/embarqué. Un cœur, des bindings partout.

### 1.7 Économie non spéculative
- Équilibre interne contribution/usage. **Pas de token, pas de frais, pas de blockchain, pas d'actif convertible.** Le score n'est pas un actif : c'est une mesure d'équilibre. *(brief l.67, verrou #4/#5)*
- **Rien à voler** : pas de token, pas de datastore central. Attaquer ToM ne rapporte rien. *(brief l.58)*

### 1.8 Souverain & libre
- **N'appartient à personne.** Open source radical, n'évolue que par la communauté. Né d'un fork d'iroh 0.96 mais **autonome et souverain** (namespace `tom-*`, PAS compatible iroh — assumé). *(CLAUDE.md Fork Status, brief l.14, l.69)*
- **Proposé gratuitement**, sous la forme la plus pertinente et la plus accessible possible.

> **Résumé en une phrase** : un bus de données planétaire, chiffré, distribué sur les appareils des gens, où les rôles s'imposent selon le besoin, qui se propage comme un virus proportionnellement à la demande, qu'on ne peut ni arrêter ni débrancher, qui oublie ce qu'il a livré, et que personne ne possède.

---

## 2. FORMES DE LIVRAISON (SDK + au-delà — on peut TOUT faire)

La cible impose « la forme la plus pertinente et accessible ». Ce n'est **pas un choix unique** : on multiplie les portes d'entrée. Priorité au SDK, mais tout ce qui augmente l'accessibilité est légitime.

| Forme | Cible d'intégrateur | Base existante | Priorité |
|---|---|---|---|
| **SDK cœur Rust** (`tom-sdk`) | Toute app native | `crates/tom-sdk` existe | 🥇 P0 |
| **Binding Swift** (iOS/tvOS/macOS) | Apps Apple | `tom-protocol-ffi` + XCFramework CI | 🥇 P0 |
| **Binding WASM/JS** (navigateur + Node) | Web, sans install | packages TS legacy à moderniser | 🥇 P0 |
| **Binding Kotlin/JNI** (Android) | Apps Android | à créer via FFI | 🥈 P1 |
| **Binding Python** | Backends, prototypage, IoT | à créer via FFI | 🥈 P1 |
| **Extension navigateur** (nœud persistant, cadenas bleu) | Utilisateur final web | roadmap brief l.201 | 🥈 P1 |
| **Daemon / CLI** (`tom-tui`, `tom-node`) | Serveurs, power users, tests | `tom-tui` existe | 🥈 P1 |
| **Module embarqué / routeur / box** | IoT, Freebox | `tom-gateway` (Freebox) existe | 🥉 P2 |
| **App de référence** (chat démo multi-plateforme) | Preuve vivante, adoption | apps tvOS existantes | 🥉 P2 |
| **Spécification publique** (`docs/spec/`) | Ré-implémenteurs tiers | `tom-wire-v1`, `tom-crypto-v1` existent | 🥈 P1 |
| **Packages publiés** (crates.io, npm, SPM, Maven, PyPI) | Distribution zéro-friction | à faire | 🥈 P1 |

> Règle : chaque nouvelle forme **doit réutiliser le cœur Rust unique** (un seul protocole, N façades). Jamais de ré-implémentation divergente du protocole par plateforme.

**Si tu identifies une meilleure forme d'accessibilité, propose-la** dans le journal (§4.3) — on ne se limite pas à cette liste.

---

## 3. OÙ ON EN EST (point de départ — audit 2026-06-26)

Réf. complète : `docs/audits/AUDIT-2026-06-26.md`. Résumé :

- ✅ **Cap verrouillé** : 7 verrous → **5 ✅, 1 ⚠️ (#2), 1 ❌ (#1)**. Les écarts sont des **bugs localisés**, pas des dérives de conception.
- 🟢 **Solide** : crypto E2E, transport/NAT (holepunch 100 % terrain), relais stateless.
- 🔴 **Bugs à fermer en priorité** (déjà localisés, `fichier:ligne` dans l'audit) :
  1. **ACK entrant non vérifié** (verrou #1) — `state.rs:872`.
  2. **Purge SQLite hub jamais déclenchée** (verrou #2) — `state.rs:536`.
  3. **Failover hub mort au runtime** — `manager.rs:846,880`.
  4. **Split-brain / hub hijack** — `manager.rs:449-465`.
  5. **Double-version `ed25519-dalek`** ; **pre-push-gate ignore Rust** ; **tom-connect/dht/integration absents de CI**.
- 🚧 **Manque pour un test réel hors-domicile** (voir aussi `docs/plans/2026-03-30-organic-seed-handoff-test.md`) : découverte zéro-infra validée avec un inconnu, client installable par un non-dev, relais public non-perso.

**La cible n'est pas atteinte. Ne jamais prétendre « complete ».**

---

## 4. COMMENT TU TRAVAILLES (protocole opératoire de l'agent)

### 4.1 Agents à gogo (orchestration massive)
- **Par défaut, délègue.** Utilise le tool `Workflow` pour orchestrer des flottes d'agents (fan-out → vérifie → synthétise) et le tool `Agent` pour les sous-tâches isolées.
- Patterns imposés selon la tâche :
  - **Explorer** (`Explore`) pour cartographier avant de coder.
  - **Planifier** (`Plan`) avant tout changement > 1 fichier.
  - **Fan-out par dimension** (bugs / perf / sécurité / conformité verrous) puis **vérification adverse** (plusieurs sceptiques qui tentent de RÉFUTER chaque trouvaille ; on ne garde que ce qui survit au vote majoritaire).
  - **Loop-until-dry** pour la découverte de bugs/edge-cases : on relance des chercheurs jusqu'à 2 tours consécutifs sans rien de neuf.
- **Isolation `worktree`** dès que plusieurs agents modifient des fichiers en parallèle.
- Règle d'or : *un agent produit de la donnée vérifiée, pas une opinion.* Toujours `fichier:ligne`. Zéro hallucination (CLAUDE.md §5).

### 4.2 Skills pour se perfectionner
Utilise les skills disponibles comme multiplicateurs de qualité, pas comme décoration :
- `/code-review` (correctness + reuse) et `/simplify` (nettoyage) sur chaque diff.
- `/angle-mort` (review anti-complaisance : cherche le fragile, le manquant, le faux-confort — ne félicite pas).
- `/verify` et `/run` pour prouver qu'un changement marche pour de vrai (pas seulement les tests).
- `/security-review` avant tout push touchant crypto/réseau.
- `/review-copilot` + `docs/handoffs/` pour handoff inter-LLM (CLAUDE.md §25).
- Les agents BMAD (`bmad-*`) pour architecture, stories, retrospectives quand on ouvre un gros chantier.
- `/deep-research` quand une décision dépend d'un état de l'art externe.

> Avant une tâche : demande-toi « quel skill rendrait ce rendu meilleur ? » et utilise-le.

### 4.3 Journal — note TOUT (pour reprendre + expliquer)
Tenue obligatoire d'un journal, pour que n'importe qui (humain ou agent) reprenne sans contexte perdu :
- **`docs/journal/JOURNAL.md`** — registre append-only. Une entrée par session/chantier :
  ```
  ## [YYYY-MM-DD HH:MM | MODEL-ID] — <titre du chantier>
  ### Objectif (lien vers la propriété §1 visée)
  ### Ce que j'ai fait (décisions + POURQUOI, pas seulement quoi)
  ### Fichiers touchés (chemin:ligne)
  ### Résultats de tests (chiffres réels, liens vers rapports)
  ### Ce qui reste / prochain [→]
  ### Auto-critique (ce qui est fragile, ce dont je doute)
  ```
- **Explique ce que tu fais** en continu : chaque décision non triviale est justifiée dans le journal.
- Les todos survivent aux compactions (CLAUDE.md §17). Reprise = dernier `[→]` ou premier `[ ]`.
- Tout chantier qui finit un objectif → entrée de journal + mise à jour de la **checklist §5**.

### 4.4 Boucle « itérer jusqu'à la cible » (mode réitération)
C'est le mode de fonctionnement par défaut. Pour chaque objectif tiré de §1 :

```
1. DÉFINIR l'état-cible mesurable (un critère de §5).
2. MESURER l'écart réel (agents d'exploration + tests). Chiffrer.
3. PLANIFIER le minimum viable qui réduit l'écart (skill /Plan).
4. IMPLÉMENTER (Edit ciblé, jamais réécriture si >20 lignes intactes).
5. TESTER DUR (§4.5) + simuler la prod.
6. DOCUMENTER les résultats obtenus (journal + rapport de test).
7. AUTO-CRITIQUE (skill /angle-mort) : qu'est-ce qui casse encore ?
8. SI critère non atteint → retour 2 avec les nouveaux constats.
   SI atteint → cocher §5, entrée journal, objectif suivant.
```

- Pour l'exécution récurrente/non-supervisée, tu peux utiliser le skill `/loop` (réitère un objectif à intervalle jusqu'à la condition de sortie).
- **Condition d'arrêt d'un objectif = son critère §5 vérifié par un test reproductible**, pas « ça a l'air bon ».
- **Ne jamais élargir le scope en douce.** Une itération = un pas vers un verrou de §1.

### 4.5 Tests DURS — couvrir l'entièreté des fonctions
Objectif : **couvrir l'intégralité des fonctions**, pas « valider l'algo heureux ». Chaque fonction publique + chaque chemin d'erreur.

Taxonomie imposée (documente chaque catégorie) :
1. **Unitaires isolés** : chaque fonction testée seule, y compris branches d'erreur, entrées limites, valeurs nulles/max, unicode, tailles extrêmes.
2. **Property-based** (proptest) : invariants (ex : `decrypt(encrypt(x)) == x`, `signing_bytes` stable, dedup idempotent). Génère des milliers de cas.
3. **Adversariaux / sécurité** : nonce rejoué, ACK forgé, signature malléable, slot DHT squatté, hub usurpé, message expiré, TTL dépassé. Le test **doit prouver le rejet**.
4. **Fuzzing** du wire format (envelope MessagePack) et du handshake.
5. **Simulation de mise en production (chaos)** : multi-nœuds réels, NAT symétrique, CGNAT, churn (nœuds qui entrent/sortent), blackout réseau puis récupération, partition, isolation → reconvergence. Réutilise/étends `tom-stress` et `tom-integration-tests`.
6. **Parties isolées en conditions réelles** : chaque crate testable seule ET en intégration ascendante (tom-connect → tom-transport → tom-protocol → tom-sdk).
7. **Multi-plateforme** : FFI (`bash scripts/check-ffi.sh`), WASM, bindings — un chemin de test par façade.
8. **Non-régression des 7 verrous** : une suite dédiée qui échoue si un verrou est violé (ex : un message livré non purgé, un ACK non signé accepté, un ban permanent, un rôle choisi manuellement).

Règles :
- **Coverage visé : maximal et mesuré** (ajouter `cargo llvm-cov` en CI). Documenter le % réel, jamais l'inventer.
- Un bug trouvé → d'abord un **test qui échoue** le reproduit, ensuite le fix.
- **Documenter ce que les tests révèlent** dans `docs/test-reports/AAAA-MM-JJ-<sujet>.md` : ce qui passe, ce qui casse, les chiffres, les surprises.
- **Se remettre en question** : si un test passe trop facilement, il est probablement faible → durcis-le (skill `/angle-mort`).

### 4.6 Gate avant push (non négociable, CLAUDE.md §24)
```bash
cargo clippy --workspace -- -D warnings
cargo test --workspace
bash scripts/check-ffi.sh          # FFI hors workspace
```
Jamais `--no-verify`. Si rouge → corriger avant push. Commits atomiques, messages français, **jamais** signer.

---

## 5. « CIBLE ATTEINTE » = QUOI EXACTEMENT (Definition of Done mesurable)

La cible §1 est atteinte quand **tous** ces critères sont vérifiés par un test/démo reproductible. Coche uniquement avec preuve (`fichier:ligne` ou rapport).

### Conformité protocole (les 7 verrous)
- [ ] **#1** Un message est marqué livré **⟺** ACK signé du destinataire vérifié (ACK forgé rejeté). *(auj. ❌)*
- [ ] **#2** Tout message livré/expiré est purgé partout ≤ TTL 24h — y compris `hub_message_history` SQLite. *(auj. ⚠️)*
- [ ] **#3** Aucun nœud (relais/L1) n'arbitre ; pass-through pur. *(auj. ✅ — garder)*
- [ ] **#4** Réputation en fade progressif, aucun ban permanent, rejoin immédiat. *(auj. ✅)*
- [ ] **#5** Anti-spam progressif, jamais exclusion binaire. *(auj. ✅)*
- [ ] **#6** Zéro état protocolaire visible par l'utilisateur final. *(auj. ✅)*
- [ ] **#7** Aucune logique produit dans le transport ; transport d'octets pur. *(auj. ✅)*

### Résilience « virus / indébranchable »
- [ ] Couper N-1 nœuds sur N ne perd aucun message (re-route + backup virus prouvés).
- [ ] Rôle réassigné automatiquement à la perte d'un hub (failover réel au runtime, pas seulement en test). *(bloqué par bug #3)*
- [ ] Isolation totale d'un nœud → reconvergence automatique au retour.
- [ ] Charge ↑ → propagation de l'effort mesurée (plus de demande = plus de nœuds mobilisés).

### Réseau réellement distribué & zéro-config
- [ ] Deux inconnus, sur deux réseaux différents, **sans aucune infra perso hardcodée**, échangent un message chiffré (découverte via DHT rendezvous + Pkarr uniquement).
- [ ] Aucun nœud privilégié, aucun bootstrap contrôlé indispensable.

### Livraison / accessibilité (formes §2)
- [ ] SDK Rust `tom-sdk` documenté + testé + publiable (crates.io).
- [ ] Au moins un binding non-Rust intégrable en < 30 min par un dev tiers (Swift **et** WASM/JS visés).
- [ ] Un non-développeur peut installer un client et rejoindre (TestFlight / paquet signé / page web).
- [ ] Spec publique à jour permettant une ré-implémentation indépendante.

### Multi-plateforme & chiffrement
- [ ] Le même protocole tourne et interopère sur ≥ 3 familles de plateformes (Apple, navigateur/WASM, Linux ; Android en cible).
- [ ] 100 % du trafic E2E ; vecteurs de test crypto à jour et verts.

### Qualité
- [ ] Toutes les fonctions publiques couvertes ; suite de non-régression des 7 verrous verte.
- [ ] Simulation de prod (chaos/churn/NAT/blackout) documentée et stable.
- [ ] CI couvre tous les crates (dont tom-connect, tom-dht, integration, FFI, WASM) ; coverage mesuré.

> Tant qu'une case reste vide, la mission continue. Le journal (§4.3) trace la progression.

---

## 6. GARDE-FOUS (ne pas changer de cap)

1. **§1 prime tout.** Une feature qui ne sert pas une propriété de §1 est refusée.
2. **Anti-hallucination absolu** (CLAUDE.md §5) : jamais inventer un fait/API/chiffre. Incertain → « je ne peux pas l'affirmer » + hypothèses + comment vérifier.
3. **Wire invariants `tom-*` sacrés** entre versions ToM (préfixe DNS `_tom`, SNI `.tom.invalid`, `X-Tom-*`, ALPN `tom-protocol/transport/0` et `/tom-gossip/1`). ToM n'est PAS compatible iroh — assumé.
4. **Jamais de** : serveur central, token/blockchain, ban permanent, rôle choisi manuellement, persistance au-delà du TTL, état protocolaire visible. Chacun viole un verrou.
5. **Doc = code.** Si un écart doc/réalité est trouvé, corriger la doc immédiatement (comme fait pour les wire invariants le 2026-06-26).
6. **Un chantier n'est PAS fini** tant que clippy + test workspace + FFI ne passent pas et que le critère §5 visé n'est pas coché avec preuve.

---

## 7. Premier pas recommandé pour l'agent qui reprend

1. Lire ce fichier, puis `docs/audits/AUDIT-2026-06-26.md`.
2. Créer `docs/journal/JOURNAL.md` (première entrée : reprise).
3. Ouvrir le chantier **« fermer les 2 bugs de verrous »** (#1 ACK entrant, #2 purge SQLite) — effort faible/trivial, fait passer 2 verrous au vert. Écrire d'abord les tests qui échouent (verrou #1 : ACK forgé doit être rejeté ; verrou #2 : table purgée ≤ TTL).
4. Puis chantier **failover hub réel** (#3), puis **anti-squat DHT** + **CI/coverage complète**.
5. En parallèle (agents à gogo), lancer le chantier **formes de livraison §2** (durcir `tom-sdk`, binding WASM, TestFlight).
6. Après chaque chantier : `/angle-mort` + `/code-review`, entrée journal, cocher §5.

**Le cap est vert. On le garde. On itère jusqu'à ce que toutes les cases de §5 soient cochées — pas avant.**
