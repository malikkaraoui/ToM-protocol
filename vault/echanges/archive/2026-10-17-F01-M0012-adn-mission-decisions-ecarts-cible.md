---
date: 2026-10-17
tags: [adn, mission, cible]
session: F01
mandat: adn-mission-decisions-ecarts-cible
mandat_id: M0012
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'high' 'Va lire ton prompt dans vault/echanges/F01.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol-scratch-M0012
branche: docs/adn-mission-2026-10
niveau: N2
derniere_maj: 2026-10-17T20:45:09+0200
---

# F01 / M0012 — ADN : inscrire les décisions du fondateur dans `docs/MISSION.md`, mesurer les écarts du code, proposer la cible

> **IMPÉRATIF (fondateur) : QMD d'abord pour tout `.md`** (`mcp__qmd__query` / `mcp__qmd__get`, https://github.com/tobi/qmd ; index `tom` à jour depuis M0006) ; `Read` sur `.md` seulement à ligne connue (offset + limit). **GitHub = `gh` CLI**, jamais le connecteur MCP GitHub.
> **IMPÉRATIF (fondateur, 2026-10-17) : on ne pousse pas si ce n'est pas nickel.** Avant CHAQUE `git push`, tu rejoues en local les contrôles de la CI (`.github/workflows/ci.yml`) qui couvrent ce que tu as touché, et tu colles leurs sorties. Rouge = pas de push.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente. **Une forme interdite qui t'est refusée ne se contourne jamais par une forme plus dangereuse : c'est un STOP.**

## Contexte (autonome)

`docs/MISSION.md` (1ᵉʳ juillet 2026) est le texte qui **fait foi** (décision du fondateur : « le projet évolue, la dernière version fait foi »). L'état des lieux M0006 (`vault/reprise/2026-10-17-ETAT-DES-LIEUX.md` §5) a listé les écarts entre documents et entre documents et code. Le fondateur a tranché le 2026-10-17 à 18:24 (verbatim résumé, ses mots) :
1. **Jeton** : « un jeton, ça peut être un grain de riz » — une note interne au programme pour noter les personnes qui participent au réseau. Pas de l'argent, pas un actif.
2. **Rôles** : la personne qui rejoint le réseau (à l'ouverture de l'app de messagerie par exemple) reçoit son rôle **du réseau lui-même**, selon le besoin du moment, la **note** de ce client, et un **contexte** : modèle d'appareil (puissance), place disponible, note de connexion (fiable / pas fiable).
3. **Observateur / double dépense** : l'abandon du 12/07 (`vault/20-decisions.md`) ne vise que le rôle « présence passive » jamais codé. Idée du fondateur pour les témoins : « tout le monde s'observe (jusqu'à 7), comme le vol des étourneaux ». La double dépense reste un chantier **ouvert** (pas d'argent dans le périmètre actuel).
4. **Relais / amorçage** : la découverte externe actuelle sert à l'**amorçage** de son petit réseau (Freebox = VM, iPhone, Apple TV, MacBook Pro, iPad) quand tout démarre en même temps. **Zéro dépense externe.** À éclaircir : peut-on vraiment se passer de tout service externe quand seuls la VM Freebox et l'Apple TV tournent ? Le projet a un module « pompe d'amorçage » : `docs/plans/2026-03-30-bootstrap-pump-module.md` (+ `docs/plans/2026-03-30-claude-bootstrap-pump-mission.md`, `vault/40-roadmap.md` R17 « seeds optionnels » VPS, `CLAUDE.md` ADR-002 et « DHT rendezvous »).
5. **Sous-réseaux** : pas de règle fixe ; le réseau s'adapte au besoin et au nombre d'utilisateurs. Constat : chacun a un répertoire de contacts et parle très rarement en dehors → **un subnet = ce cercle réel**. Un subnet trop petit (en connexions possibles) s'appuie sur d'autres plus robustes : **vases communicants**, union des forces, partage de rôles.

Une autre fenêtre (F03/M0011) intègre en parallèle le retrait de l'IP publique (scripts, docs, code de test) : **tu ne touches à aucun fichier hors `docs/MISSION.md` et `vault/`**.

## Préparation obligatoire de tout worktree (leçons du 2026-10-17, issues #62, #64, #65)

- Worktrees et scratchs **HORS du dépôt** : `/Users/malik/Documents/tom-protocol-scratch-<Mxxxx>[-suffixe]` (un worktree imbriqué fait remonter cargo au `Cargo.toml` racine).
- **Avant tout commit dans un worktree** : `pnpm install --frozen-lockfile` puis `test -x .husky/_/pre-push && echo HOOKS_OK` — sans `HOOKS_OK`, aucun commit ni push (sinon le contrôle pre-push ne tourne pas : #64).
- Le push déclenche `scripts/ci-local.sh` (≈15-20 min) : pousse avec `GIT_SSH_COMMAND='ssh -o ServerAliveInterval=30'` (#65). Jamais `--no-verify`.
- Disque : `cargo clean` dans chaque scratch après son dernier run (≈28 Go par worktree neuf).

## Gardes de précondition

1. `git fetch origin` ; `git merge-base --is-ancestor 99aa090a38650d1dd3813d3a16c9c999c6605330 origin/main`. Sinon STOP.

## Mission (aucune modification de code)

1. Rituel de début : `statut: en-cours`.
2. Lis via QMD : `docs/MISSION.md` en entier, l'état des lieux §5, `vault/20-decisions.md`, `vault/00-brief.md`, la pompe d'amorçage et ses liens.
3. **Amorçage — réponse factuelle à la question du fondateur** (lecture du code, preuves fichier:ligne, `[VÉRIFIÉ]`/`[HYPOTHÈSE]`) : quels mécanismes de découverte existent aujourd'hui (mDNS/LAN, DHT Mainline publique, Pkarr/DNS `n0_discovery`, relais propres, seeds), lesquels sont **externes** (tiers), lesquels **coûtent** de l'argent, lesquels sont actifs **par défaut** ; scénario « seules la VM Freebox et l'Apple TV tournent » : qu'est-ce qui marche en LAN, qu'est-ce qui marche depuis Internet, sans aucun service externe. Ne lance aucun nœud.
4. **Mettre à jour `docs/MISSION.md`** (remplacer les passages périmés, ne pas empiler) pour y inscrire les 5 décisions ci-dessus, datées « 2026-10-17, fondateur », en gardant le style du document. Ce qui reste ouvert (double dépense, amorçage sans tiers, mathématiques des notes/rôles) va dans une section « Questions ouvertes » claire, sans trancher à la place du fondateur.
5. **Carte des écarts** `vault/decisions/2026-10-17-ADN-ecarts-et-cible.md` : tableau principe (MISSION mise à jour) / état du code / preuve fichier:ligne / écart / lot proposé. Puis **cible proposée** : 3 à 5 chantiers ordonnés (titre, but, critère de réussite mesurable, risques, irréversibles), le premier étant le plus petit pas vers la vision, réaliste sur le réseau actuel du fondateur. Aucun chantier lancé.
6. **Doublage N2** par sous-agent FRAIS (lecture seule, sans cargo) : fidélité au verbatim du fondateur, aucune décision inventée, preuves fichier:ligne recontrôlées, MISSION remplacée et non empilée. Rapport `vault/revues/2026-10-17-R010-adn-mission.md` (frontmatter : exactement `verdict: GO|RESERVE`, `tip: <sha 40>`, `niveau: N2`).
7. Branche `docs/adn-mission-2026-10`, push (hook complet), merge (bloc ci-dessous) avec `[R010]`.

## Règles des sous-agents (verbatim)

(1) lecture/recherche parallele = agents FRAIS (jamais fork) ; (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent ; (3) tout rapport de sous-agent est VERIFIE (fichier:ligne recontroles) ; (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ; (5) ATTENDRE LE RETOUR DE TOUS les sous-agents AVANT le premier rituel de fin, jamais sleep ni sondage ; echec de l'un = STOP propre. **Aucun sous-agent ne lance de build ou de test cargo** (incident M0006 : `target/` à 23 Go) : les builds restent à la session mère.

## Hygiène disque

Plafond `target/` 20 Go (`CLAUDE.md` racine) : `du -sh target/` avant et après chaque build lourd ; au-delà de 15 Go, `cargo clean` avant de continuer. Rien ne tourne en fond à la sortie.

## Intouchables

`scripts/pre-push-gate.sh` (modifié par le fondateur, non committé), `vault/30-discoveries.md`, `.claude/settings.json`, les `vault/echanges/F0x.md` vivants (jamais committés), tout fichier hors `docs/MISSION.md` et `vault/`.

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous forme gardée ; jamais `--no-verify` ; jamais de force-push.

## Merge (gabarits §9) et intégration

Scratch détaché sur `origin/main` à jour (`git fetch`), `merge --no-ff <tip> -m "Merge <branche> [<Rxxx>]"`, **contrôles CI concernés rejoués sur le scratch** (sorties collées), copie + commit du rapport de doublage (`docs(revues): <Rxxx> -- GO`). Puis : racine sur `main` et `git status --short` = intouchables seuls → `git merge --ff-only <HEAD scratch>` à la racine + `git push origin main` ; racine occupée → `git push origin <HEAD scratch>:refs/heads/main` depuis le scratch (fast-forward seulement). `origin/main` a bougé → refaire le scratch UNE fois. `git ls-remote origin refs/heads/main` collé. Après le push : `gh run list --branch main -L 3` et `gh run watch <id> --exit-status` (attente bornée à 30 min) — sortie collée ; le résultat CI réel va au rapport tel quel. Worktrees retirés à la fin.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0012`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F01.md` (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F01"`, `mandat: "M0012"`) et `vault/runtime/state.json` (`windows.F01` seulement, relu juste avant d'écrire) : jamais committés.
4. Commit vault sur `main` (racine si elle est sur `main`, sinon scratch détaché) : `vault/reprise/F01.md` + `vault/echanges/archive/2026-10-17-F01-M0010-integration-ci-verte-et-pre-push.md` ; `docs(vault): reprise F01 M0012` ; push ; `ls-remote` collé.
5. **Dernier geste** : `statut: reponse-disponible`.


## Rapport M0012

Scellé tests : sans objet (mandat documentaire, aucun test)
Contre-témoins : aucun

### 0. Résultat
`docs/MISSION.md` porte les 5 décisions du fondateur, datées « 2026-10-17, fondateur » (§1.1, §1.2, §1.7, §1.9, §1.10, §6.7) et une section **§8 Questions ouvertes** (7 questions, aucune tranchée). Anciens passages **remplacés**, pas empilés : ancien §1.2, §1.7 « Pas de token », §6.4 « token/blockchain », §7 périmé. Le mot « token » n'apparaît plus. Carte des écarts + cible : `vault/decisions/2026-10-17-ADN-ecarts-et-cible.md`. Portée de la décision du 12/07 tracée dans `vault/20-decisions.md` (2 lignes ajoutées, l'ancienne conservée). Aucun code modifié, aucun nœud lancé, aucun chantier lancé. Intégré sur `main` : `7af7a45c5e97fb7a55ff99c28a4b76678b5b1705`.

### 1. Amorçage : réponse au fondateur (détail et preuves : carte §1)
- **Zéro dépense : déjà vrai.** Aucun mécanisme ne coûte d'argent. Seul R17 (VPS + domaine) en coûterait. `tom-protocol.org` est **NXDOMAIN** (`dig SOA` → `status: NXDOMAIN`) : le repli DNS TXT `_relay._tcp.tom-protocol.org` (`tom-transport/src/config.rs:29`) ne sert à rien.
- **Tiers actifs par défaut** :
  - Pkarr/DNS n0 (`dns.iroh.link`) : `n0_discovery: true` (`config.rs:172`), app `TomNodeService.swift:164`, preset `tom-connect/src/endpoint/presets.rs:51-67`.
  - DHT Mainline publique, amorces `router.bittorrent.com`, `dht.transmissionbt.com`, `dht.libtorrent.org`, `relay.pkarr.org` (`mainline-6.1.1/src/rpc.rs:42-46`, `enable_dht` true `runtime/mod.rs:145`, FFI `lib.rs:364`).
- **Non tiers** : mDNS (`local_discovery: true`, `config.rs:173`) ; relais embarqué sur chaque app Apple (FFI `lib.rs:368-369`) avec UPnP ; relais NAS injecté **à la compilation** via `.cargo/config.toml` hors git (`config.rs:24`, `.gitignore:67`) = adresse publique de la Freebox.
- **Scénario VM Freebox + Apple TV sans aucun tiers.**
  - En **LAN** : oui en principe, par mDNS. Conditions `[HYPOTHÈSE]` : multicast à travers le pont de la VM, multicast brut autorisé sur tvOS (aucun entitlement dans le dépôt).
  - Depuis **Internet** : uniquement via un point d'entrée connu d'avance (adresse publique de la Freebox, port redirigé). **Aucun mécanisme du code ne permet une première rencontre à connaissance nulle sans tiers.** La DHT et n0 sont les seuls à le faire.
  - Le chantier C1 tranche par la mesure, sans code.
- Effet de bord : avec n0 actif par défaut, chaque appareil publie son identifiant et ses adresses chez n0 (tension avec §1.8).

### 2. Écarts majeurs relevés (carte §2)
- **Rôle auto-déclaré accepté** : `runtime/state.rs:943` `updated_peer.role = announce.new_role` (signature vérifiée l.922, mais le signataire est le pair lui-même). Contredit « impossible à truquer ».
- **Contexte d'appareil absent** : `HostFactors` du backup alimentés **seulement en test** (`backup/coordinator.rs:513` sous `cfg(test)` l.310, `store.rs:992,1012`, `tests/backup_integration.rs`). En production, défaut 50/50/50 (`backup/types.rs:208-215`). Le relais embarqué tourne sur tous les appareils Apple.
- **Témoins** 2 à 4 (`presence/relay_view.rs:33,38`) au lieu de « jusqu'à 7 » ; vue active gossip 5 (`hyparview.rs:202`).
- **Subnets** : bornes fixes 3-10 (`discovery/subnet.rs:18,21`), vue locale des seuls `Chat` reçus (`state.rs:1965-1969`), consommés par rien (`state.rs:3255-3259`). Pas de vases communicants.
- **Note** : vue locale par observateur, contribution seule (`roles/scoring.rs:87-120`), score auto-déclaré dans les annonces (`discovery/role_sync.rs:9-17`).

### 3. Cible proposée (aucun chantier lancé)
- **C1** : prouver l'amorçage maison sans tiers, sur le réseau actuel. Sans code, interrupteurs existants.
- **C2** : le rôle se constate, il ne se déclare plus. Document de conception d'abord.
- **C3** : le contexte de l'appareil entre dans la note et le rôle (après arbitrage §8 q.3).
- **C4** : témoins jusqu'à 7.
- **C5** : subnet = cercle réel, vases communicants.
Critères, risques et irréversibles : carte §3.

### 4. Doublage N2 — R010
- **Tour 1** (sous-agent frais, lecture seule, sans cargo), tip `a623a387ab7d9c963e8307bb456151345ee986c2` : **RESERVE**. 30 références `fichier:ligne` recontrôlées, toutes OK. Trois réserves :
  - (1) §6.7 étendait « zéro dépense externe » à tout le fonctionnement sous label fondateur : **fondée**, corrigée (ramenée à l'amorçage et aux relais, extension en §8 q.7).
  - (2) « la dernière version fait foi » serait non sourcée : **non fondée**, la phrase est dans ce mandat, le relecteur n'avait reçu que les 5 points.
  - (3) « fonction temporaire et tournante » sous label fondateur : **fondée**, attribuée au plan du 2026-03-30.
  - Étiquettes corrigées (ExecStart NAS et ports en `[HYPOTHÈSE]`, requête DNS TXT ajoutée au verdict d'amorçage et à C1).
- Corrections : commit `987e1693b95a93a1ddca0cf1191031ca49733cec`.
- **Tour 2** (second sous-agent frais), tip `987e169` : **GO**. R1 et R3 levées, R2 traitée correctement. Aucune IPv4, aucune décision inventée. Remarques non bloquantes : 2 déjà couvertes par le verbatim du mandat (« 18:24 », formulations des points 2 et 5) ; 3 versées en **issue #68** (sans nouveau tour, règle du harnais).
- Rapport `vault/revues/2026-10-17-R010-adn-mission.md` : frontmatter `verdict: GO`, `tip: 987e1693b95a93a1ddca0cf1191031ca49733cec`, `niveau: N2`.
- **Question pour l'orchestrateur** : le mandat date MISSION du « 1ᵉʳ juillet 2026 », le fichier dit « rédigé 2026-06-26 ». Laissé tel quel.

### 5. Git, CI locale, intégration
- Garde : `merge-base --is-ancestor 99aa090… origin/main` → `GARDE_OK`.
- Worktree `/Users/malik/Documents/tom-protocol-scratch-M0012`, branche `docs/adn-mission-2026-10`. `pnpm install --frozen-lockfile` → `HOOKS_OK`. Commits `a623a38`, `987e169` (auteur et committer `karaoui.malik@gmail.com`). Fichiers touchés : `docs/MISSION.md`, `vault/20-decisions.md`, `vault/decisions/2026-10-17-ADN-ecarts-et-cible.md`, puis `vault/revues/2026-10-17-R010-adn-mission.md`.
- Push de la branche (hook complet, `GIT_SSH_COMMAND='ssh -o ServerAliveInterval=30'`) :
```
[ci-local] TOUT VERT.
 * [new branch]      docs/adn-mission-2026-10 -> docs/adn-mission-2026-10
```
  `ls-remote` : `987e1693b95a93a1ddca0cf1191031ca49733cec refs/heads/docs/adn-mission-2026-10`.
- Merge, 1ʳᵉ tentative (scratch détaché sur `99aa090`, `e14ff43` + `2e609dd`). Hook vert, puis rejet : `origin/main` avait bougé pendant les ~20 min de CI (commit vault F03 `7bf72ce`) :
```
[ci-local] TOUT VERT.
 ! [remote rejected] HEAD -> main (cannot lock ref 'refs/heads/main': is at 7bf72ce8796e92ac80879003514ec77cd7a3c4ec but expected 99aa090a38650d1dd3813d3a16c9c999c6605330)
```
- Scratch refait **une fois** (règle du mandat) : `/Users/malik/Documents/tom-protocol-scratch-M0012-merge2`, détaché sur `7bf72ce`. `bafa64a` « Merge docs/adn-mission-2026-10 [R010] » (`--no-ff 987e169`), puis `7af7a45` « docs(revues): revue R010 -- GO ». Racine non utilisée : fichiers non suivis d'autres fenêtres, dont l'archive F03 que `7bf72ce` ajoute, donc un ff-only aurait échoué. Contrôles CI rejoués sur le scratch par le hook :
```
[ci-local] TOUT VERT.
   7bf72ce..7af7a45  HEAD -> main
```
  `git ls-remote origin refs/heads/main` → `7af7a45c5e97fb7a55ff99c28a4b76678b5b1705 refs/heads/main`.
- CI GitHub :
```
completed	success	docs(revues): revue R010 -- GO	Release	main	push	39066487677	1m35s	2026-10-17T20:23:12Z
completed	success	docs(revues): revue R010 -- GO	CI	main	push	39066487691	4m16s	2026-10-17T20:23:12Z
```
  `gh run watch 39066487691 --exit-status` → `WATCH_RC=0`, conclusion `success`, **12/12 jobs success**.
- Disque : `cargo clean` dans le worktree de branche (24,7 Gio) et dans le 1ᵉʳ scratch de merge (24,7 Gio).

### 6. Constats hors périmètre (signalés, non corrigés)
- `vault/00-brief.md` périmé.
- `vault/40-roadmap.md:293` (R17 « URLs déjà en défaut committé ») périmé.
- `CLAUDE.md` ADR-002 « Bootstrap Elimination DONE » alors que `n0_discovery` est actif par défaut.
- Critère MISSION §5 « Pkarr uniquement » : annoté, renvoie à §8 q.2.
- Leçon transverse (R5) : un push vers `main` gardé par un hook de ~20 min se fait doubler par toute fenêtre qui pousse entre-temps. Coût : un run CI complet perdu, ici un commit vault de F03. Piste : sérialiser les pushes vers `main` entre fenêtres, ou exempter les commits vault-only du hook complet. Candidate à la remontée globale.

### 7. Rituels de fin
- `vault/reprise/F01.md` (5 lignes) et `vault/echanges/archive/2026-10-17-F01-M0010-integration-ci-verte-et-pre-push.md` : commit `e1dc05a00467b226fe7c73851b25598ea9b79219` « docs(vault): reprise F01 M0012 », depuis le scratch détaché sur `7af7a45`. Push (hook complet) :
```
[ci-local] TOUT VERT.
   7af7a45..e1dc05a  HEAD -> main
```
  `git ls-remote origin refs/heads/main` → `e1dc05a00467b226fe7c73851b25598ea9b79219 refs/heads/main`.
- `vault/runtime/events.jsonl` (RAPPORT, `src: fenetre`) et `vault/runtime/state.json` (`windows.F01` seul, relu avant écriture) : écrits, non committés.
- Worktrees `tom-protocol-scratch-M0012` et `-M0012-merge2` retirés après `cargo clean` (35,2 Gio pour le second). Le premier scratch de merge avait été retiré plus tôt. Aucune tâche de fond vivante. La racine n'a pas été touchée : elle est sur `main` `7bf72ce`, avec les intouchables.

M0012
Implementation      ✅
Tests               ✅
Commit              ✅
Branch push         ✅
Review              ✅
Merge main          ✅
Main push           ✅
TASK STATUS         INTEGRATED
PROJECT STATUS      INTEGRATED
