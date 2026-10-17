---
date: 2026-10-17
tags: [securite, ip, ssh, hygiene]
session: F03
mandat: retrait-ip-publique-et-audit-ssh
mandat_id: M0009
statut: reponse-disponible
commande: cd '/Users/malik/Documents/tom-protocol' && claude --disallowedTools ScheduleWakeup,CronCreate --settings '/Users/malik/.harnais/hooks.json' --model 'opus' --effort 'medium' 'Va lire ton prompt dans vault/echanges/F03.md et exécute-le. Applique strictement les rituels décrits.'
worktree: /Users/malik/Documents/tom-protocol/.claude/worktrees/F03-M0009
branche: fix/retrait-ip-publique
niveau: N2
derniere_maj: 2026-10-17T16:51:33+0200
---

# F03 / M0009 — Retirer l'IP publique du domicile du dépôt (sans réécrire l'historique) + audit de l'accès SSH

> **IMPÉRATIF (fondateur) : QMD d'abord pour tout `.md`** (`mcp__qmd__query` / `mcp__qmd__get`, https://github.com/tobi/qmd ; index `tom` à jour depuis M0006) ; `Read` sur `.md` seulement à ligne connue (offset + limit). **GitHub = `gh` CLI**, jamais le connecteur MCP GitHub.
> **IMPÉRATIF (fondateur, 2026-10-17) : on ne pousse pas si ce n'est pas nickel.** Avant CHAQUE `git push`, tu rejoues en local les contrôles de la CI (`.github/workflows/ci.yml`) qui couvrent ce que tu as touché, et tu colles leurs sorties. Rouge = pas de push.

**Tu ne poses JAMAIS de question et tu n'attends JAMAIS de réponse.** Personne n'est devant l'écran. Si tu es sur le point de demander un arbitrage, c'est un **STOP** : écris dans ton rapport la question que tu aurais posée, les options que tu voyais et celle que tu aurais choisie avec sa raison, puis applique les rituels de fin. Un STOP propre est un rendu valable et laisse l'orchestrateur décider ; une question laisse un mandat mort. Vaut aussi pour une demande d'autorisation d'outil : n'insiste pas ; contourne par une commande plus simple répondant au même besoin (lecture seule, `git show`, `git -C`, chemin explicite dans le dépôt) ; si aucun contournement n'existe, c'est un STOP propre avec rapport — jamais une attente. **Une forme interdite qui t'est refusée ne se contourne jamais par une forme plus dangereuse : c'est un STOP.**

## Contexte (autonome)

Le dépôt `malikkaraoui/ToM-protocol` est **public**. L'état des lieux M0006 a trouvé l'IP publique du domicile du fondateur dans ~35 fichiers suivis, dont `ssh root@<IP>` dans `CLAUDE.md`. Décision du fondateur, 2026-10-17 16:50 : **retirer l'IP du code actuel, sans réécrire l'historique git**, et **sécuriser l'accès SSH**.
Liste de départ (orchestrateur) : `git grep -l -E '82\.67\.[0-9]+\.[0-9]+'` — à recalculer toi-même. Cherche aussi les autres IP publiques du domicile/de la flotte, noms d'hôte DNS dynamiques et ports SSH exposés (`git grep -n -E '([0-9]{1,3}\.){3}[0-9]{1,3}'` filtré : garde les IP publiques, écarte privées 10/8, 172.16/12, 192.168/16, 127/8, documentaires 192.0.2/24, 198.51.100/24, 203.0.113/24, et les versions/numéros).

## Fenêtres parallèles

F01/M0007 touche : `crates/tom-quinn/build.rs`, `rust-toolchain.toml`, `package.json`, `pnpm-lock.yaml`, `scripts/ci-local.sh`, `.husky/pre-push`, `CONTRIBUTING.md`. F02/M0008 touche : `Cargo.toml` des crates TLS, `Cargo.lock`, `deny.toml`, lock FFI. **Tu n'y touches pas** ; un de ces fichiers contient l'IP → laisse-le, liste-le au rapport (traité après leur merge).

## Gardes de précondition

1. `git fetch origin` ; `git merge-base --is-ancestor 9278dcab25e69b3b4a05e980542997cd870bc3c5 origin/main` (code 0), sinon STOP.
2. Worktree depuis `origin/main` : `git worktree add -b fix/retrait-ip-publique .claude/worktrees/F03-M0009 origin/main`. Jamais de `checkout` à la racine.

## Mission

1. Rituel de début : `statut: en-cours` (dans `vault/echanges/F03.md`, à la racine).
2. **Inventaire** (tableau au rapport : fichier:ligne, type — code / script / doc vivante / archive / données d'essai / vault — et traitement choisi). Ne recopie l'IP **nulle part** dans le rapport ni dans un commit : écris `<IP_DOMICILE>`.
3. **Remplacement** :
   - code et scripts (`crates/…`, `scripts/…`) : valeur lue dans une variable d'environnement (ex. `TOM_HOME_HOST`) ou un fichier local gitignoré, **sans valeur par défaut publique** ; message d'erreur clair si absente. Documente la variable dans `.env.example` (sans valeur). Vérifie que `.env` est gitignoré (`git check-ignore .env`) ; s'il est suivi, STOP.
   - docs, archives, plans, articles, journaux, `vault/` (sauf intouchables) : remplacer par `<IP_DOMICILE>` (ou `<HOST_SSH>` pour un hôte).
   - données d'essai `experiments/**/results/*.jsonl` : remplacer la valeur par `<IP_DOMICILE>` en gardant le JSON valide (vérifie chaque ligne avec `python3 -I -c 'import json,sys;[json.loads(l) for l in open(sys.argv[1])]' <fichier>`).
   - **Intouchables qui contiennent l'IP** (`vault/30-discoveries.md`, `scripts/pre-push-gate.sh`, `.claude/settings.json`) : ne pas toucher, lister au rapport.
   - `CLAUDE.md` : remplacer sur place, sans ajouter de lignes ; budget 40 000 caractères / 400 par ligne.
4. **Garde anti-retour** : un test simple qui échoue si une IP publique du domicile réapparaît — sans écrire l'IP en clair dans le dépôt (compare un **hash** SHA-256 de chaque IPv4 trouvée par `git grep` à une liste de hashs stockée dans `scripts/ip-interdites.sha256`, et script `scripts/verifier-ip-interdites.sh`). Ne le branche dans aucun hook (M0007 s'en charge plus tard) ; il doit être exécutable seul et sortir en code ≠ 0 si une IP interdite est trouvée. Teste-le avant/après (sorties collées, IP masquée).
5. Contrôles CI concernés (impératif pre-push) : `cargo build/test/clippy` des crates touchés (`tom-protocol`, `tom-stress`…), `shellcheck` des scripts touchés si présent, `pnpm lint` si des fichiers biome sont touchés. Colle les sorties.
6. Commits pathspec explicites, sans trailer : `fix(securite): retrait de l'IP publique du domicile (historique non reecrit)` (+ commit séparé pour la garde). Push branche + `ls-remote`.
7. **Audit SSH — LECTURE SEULE, aucune modification sur aucune machine** : à partir des scripts/docs du dépôt (`scripts/deploy-nas-node.sh`, `scripts/nas-node-ctl.sh`, `scripts/car-test.sh`, `docs/freebox-sdk-workflow.md`, `docs/archive/rapport-diagnostic-ssh-wan-2026-03-02.md`), décris quelles machines sont joignables en SSH depuis Internet, sur quel port, avec quel utilisateur. Si et seulement si une connexion par clé existe déjà sans mot de passe, tu peux lancer **en lecture** `ssh -o BatchMode=yes <hôte> 'sudo -n sshd -T 2>/dev/null | grep -E "^(permitrootlogin|passwordauthentication|port|kbdinteractiveauthentication|pubkeyauthentication) "'` (rien d'autre) ; échec = note-le, pas d'insistance. Rends une **liste de durcissements proposés** (root interdit, clé seule, port, pare-feu/redirection box, fail2ban ou équivalent, IP fixe vs nouvelle IP auprès du FAI) avec, pour chacun, le risque de se bloquer dehors et la procédure de retour arrière. **Aucun changement** : le fondateur décidera.
8. **Doublage N2** : sous-agent FRAIS, worktree détaché lecture seule `.claude/worktrees/F03-R007`, rapport `vault/revues/2026-10-17-R007-retrait-ip-publique.md` (frontmatter : exactement `verdict: GO|RESERVE`, `tip: <sha 40>`, `niveau: N2`). Il vérifie : plus aucune occurrence hors intouchables et fichiers des autres fenêtres (il relance la garde et `git grep`), code qui ne contient plus de valeur par défaut publique, JSONL valides, `CLAUDE.md` dans le budget, rien de l'IP dans les messages de commit ni dans les rapports.
9. Merge (bloc ci-dessous).

## Règles des sous-agents (verbatim)

(1) lecture/recherche parallele = agents FRAIS (jamais fork) ; (2) aucun sous-agent ne commit, ne pousse, ne modifie vault/runtime/ ni vault/reprise/, ni ne tue un autre agent ; (3) tout rapport de sous-agent est VERIFIE (fichier:ligne recontroles) ; (4) apres tout incident : git status/diff/log/reflog AVANT toute autre action ; (5) ATTENDRE LE RETOUR DE TOUS les sous-agents AVANT le premier rituel de fin, jamais sleep ni sondage ; echec de l'un = STOP propre. **Aucun sous-agent ne lance de build ou de test cargo** (incident M0006 : `target/` à 23 Go) : les builds restent à la session mère.

## Hygiène disque

Plafond `target/` 20 Go (`CLAUDE.md` racine) : `du -sh target/` avant et après chaque build lourd ; au-delà de 15 Go, `cargo clean` avant de continuer. Rien ne tourne en fond à la sortie.

## Intouchables

`scripts/pre-push-gate.sh` (modifié par le fondateur, non committé), `vault/30-discoveries.md`, `.claude/settings.json`, les `vault/echanges/F0x.md` vivants (jamais committés), tout fichier de l'autre fenêtre (voir « Fenêtre parallèle »).

## Interdits permanents

Jamais `sleep` ; jamais de tâche de fond vivante à la sortie ; jamais `stash` / `reset --hard` / `checkout -- .` / `update-ref` / `pull --rebase --autostash` à la racine ; `rm -rf` seulement sous forme gardée ; jamais `--no-verify` ; jamais de force-push.

## Merge (gabarits §9) et intégration

Scratch détaché sur `origin/main` à jour (`git fetch`), `merge --no-ff <tip> -m "Merge <branche> [<Rxxx>]"`, **contrôles CI concernés rejoués sur le scratch** (sorties collées), copie + commit du rapport de doublage (`docs(revues): <Rxxx> -- GO`). Puis : racine sur `main` et `git status --short` = intouchables seuls → `git merge --ff-only <HEAD scratch>` à la racine + `git push origin main` ; racine occupée → `git push origin <HEAD scratch>:refs/heads/main` depuis le scratch (fast-forward seulement). `origin/main` a bougé → refaire le scratch UNE fois. `git ls-remote origin refs/heads/main` collé. Après le push : `gh run list --branch main -L 3` et `gh run watch <id> --exit-status` (attente bornée à 30 min) — sortie collée ; le résultat CI réel va au rapport tel quel. Worktrees retirés à la fin.

## Rituels de fin, dans l'ordre

1. Rapport ci-dessous (`## Rapport M0009`), footer de statut (gabarits §4) en toute dernière ligne.
2. `vault/reprise/F03.md` (5 lignes au plus).
3. `vault/runtime/events.jsonl` (`src: "fenetre"`, `f: "F03"`, `mandat: "M0009"`) et `vault/runtime/state.json` (`windows.F03` seulement, relu juste avant d'écrire) : jamais committés.
4. Commit vault sur `main` (racine si elle est sur `main`, sinon scratch détaché) : `vault/reprise/F03.md` ; `docs(vault): reprise F03 M0009` ; push ; `ls-remote` collé.
5. **Dernier geste** : `statut: reponse-disponible`.


## Rapport M0009

**Résultat : STOP propre — rien n'est mergé sur `main`.** Le retrait est fait, vérifié et poussé sur `fix/retrait-ip-publique` (tip `e7f6ce8`). Mais le doublage R007 rend RESERVE pour la 4ᵉ fois, toujours sur la garde anti-retour, et la règle du tronc commun s'applique (« deux correctifs de suite sur la même classe de défaut : le mécanisme est en cause, on s'arrête et on le dit »).

Scellé tests : sans objet (`scripts/verifier-rendu.mjs` absent du dépôt)
Contre-témoins : aucun

### 0. Arbitrage demandé (la question que j'aurais posée)

**Question** : la garde `scripts/verifier-ip-interdites.sh` lit les fichiers sur le disque, pas les objets git. Elle ne voit donc ni une IP placée dans un NOM de fichier, ni une IP dans la cible d'un lien symbolique, ni un fichier suivi supprimé du disque mais encore indexé (réserve R1 du tour 4, vérifiée en relisant mon code). Faut-il refaire la garde avant le merge, ou merger le retrait tout de suite ?

| Option | Contenu | Coût / risque |
|---|---|---|
| **A (mon choix)** | Merger la branche telle quelle (retrait + garde v4), en écrivant la limite R1 dans l'en-tête de la garde. Refaire la garde sur les objets (`git ls-files -s` + `git cat-file --batch`, noms de fichiers compris) dans le mandat qui la branche aux hooks (M0007 ou suivant). | L'IP et le mot de passe quittent le tip public tout de suite. La garde n'est branchée nulle part : sa limite ne bloque rien aujourd'hui. |
| B | Refaire la garde sur les blobs dans cette fenêtre, puis un 5ᵉ tour R007. | Le tip public garde l'IP du domicile et le mot de passe root pendant ce temps. Une 5ᵉ itération sur la même classe va contre la règle du harnais. |
| C | Ne merger que le retrait (`0b24450` + `ec8b180`, choisis avec soin) et sortir la garde dans un mandat dédié. | Le retrait part vite, mais l'exigence n°4 du mandat (la garde) n'est pas livrée. |

Raison du choix A : l'urgence est d'enlever du tip public le mot de passe root en clair et l'adresse du domicile. La garde est un outil d'appoint, non branché. Un 5ᵉ correctif dans cette fenêtre reproduirait le schéma que la règle veut arrêter. Il faudra peut-être un doublage court GO sur A (rien d'autre à revoir que la note de limite).

### 1. ⚠️ Constats critiques (hors tableau, à traiter en priorité par le fondateur)

1. **Le mot de passe SSH root du NAS était en clair dans `scripts/car-test.sh`**, au tip public `main` (variable `NAS_SSH_PASS`, ligne 20 à la base `9278dca`, introduit par au moins un commit de l'historique). Je l'ai sorti du tip (variable d'env `TOM_NAS_SSH_PASS` sans défaut). **Il reste dans l'historique public : il est compromis et doit être changé.**
2. **Lecture `sshd -T` du NAS (BatchMode, clé existante, lecture seule, rien modifié)** : `port 22`, `permitrootlogin yes`, `pubkeyauthentication yes`, `passwordauthentication yes`, `kbdinteractiveauthentication no`. Joint au point 1 et à la redirection WAN 2222 → 22 : **n'importe qui peut tenter, et sans doute réussir, une connexion root par mot de passe depuis Internet.** C'est la priorité n°1, avant le merge.

### 2. Inventaire (valeurs jamais recopiées)

Valeurs trouvées : (a) `<IP_DOMICILE>`, IPv4 publique du domicile (une seule valeur distincte, 61 occurrences) ; (b) `<IP_FLOTTE_2>`, IPv4 publique de la box de la belle-famille, avec les 2 ports d'administration distante Freebox OS, le nom de la box et un fragment de domaine `fbxos` (`docs/INFRA-LOCAL-DEPLOYMENT-GUIDE.md:1560-1572`) ; (c) `<IPV6_DOMICILE>`, préfixe IPv6 /64 fixe du FAI au domicile (91 occurrences, 43 fichiers ; ajouté au tour 2 de R007, j'avais d'abord cherché l'IPv4 seulement). Autres IPv4 publiques écartées après vérification : DNS publics, exemples (`1.2.3.4`…), fixture amont iroh (`tom-relay/src/endpoint_info.rs`), relais n0 dans des journaux, `4.3.2.2` (numéro de section RFC). Aucun nom DNS dynamique trouvé.

Traitement par type :
- **code** : `crates/tom-protocol/src/runtime/state.rs:7714` est un test UPnP qui demande « une IP publique ». Remplacé par `1.2.3.4`, exemple public déjà utilisé dans `runtime/loop.rs`. Une variable d'env casserait ce test unitaire. `crates/tom-stress/src/fleet_probe.rs:21` est un commentaire de doc : placeholder.
- **scripts** : `car-test.sh`, `deploy-nas-node.sh`, `nas-node-ctl.sh` lisent `TOM_HOME_HOST` (et `TOM_NAS_SSH_PASS`) via `${VAR:?message}` : aucune valeur par défaut, sortie 1 avant tout réseau. Variables documentées sans valeur dans `.env.example`. `.env` est ignoré (`.gitignore:14`) et non suivi.
- **docs, archives, plans, articles, journal, vault** : `<IP_DOMICILE>`, `<HOST_SSH>` après `root@`, `<IPV6_DOMICILE>` (suffixes d'interface conservés). Placeholders échappés en HTML (`&lt;…&gt;`).
- **PDF** `docs/article ToM protocol.pdf` : régénéré avec Chrome headless depuis `docs/article-ToM protocol-fr.html`. Texte identique mot à mot, sauf le placeholder et les coupures de mots en fin de ligne (39 pages, comme avant).
- **données d'essai** : 4 JSONL `experiments/iroh-poc/results/` ; chacun reste valide (`json.loads` sur chaque ligne).
- **`CLAUDE.md`** : 3 lignes remplacées sur place (396, 398, 404), aucune ajoutée ; 31 373 caractères. Les lignes modifiées font moins de 400 caractères ; des lignes plus longues existaient déjà ailleurs (83, 153, 457…).

| fichier | lignes (nouvelle numérotation) | type |
|---|---|---|
| `.env.example` | 6 | script |
| `CLAUDE.md` | 396,398,404 | doc vivante |
| `crates/tom-protocol/src/runtime/state.rs` | 7714 | code (test) |
| `crates/tom-stress/src/fleet_probe.rs` | 21 | code (doc-comment) |
| `docs/DEPLOY-APPLE.md` | 187 | doc vivante |
| `docs/INFRA-LOCAL-DEPLOYMENT-GUIDE.md` | 1562,1566,1571 | doc vivante |
| `docs/archive/PROOF-CROSS-MACHINE-2026-03-27.md` | 95 | archive |
| `docs/archive/RCA-PEERPRESENT-RACE-2026-03-30.md` | 28 | archive |
| `docs/archive/RECAP-DEV-2026-03-05.md` | 20,26 | archive |
| `docs/archive/freebox-bug-report-arp-wifi-vm.md` | 48 | archive |
| `docs/archive/rapport-diagnostic-ssh-wan-2026-03-02.md` | 13,31,91,115,127 | archive |
| `docs/article ToM protocol.pdf` | binaire | doc vivante |
| `docs/article-ToM protocol-en.html` | 782 | doc vivante |
| `docs/article-ToM protocol-en.md` | 591 | doc vivante |
| `docs/article-ToM protocol-fr.html` | 771 | doc vivante |
| `docs/article-ToM protocol.md` | 591 | doc vivante |
| `docs/freebox-sdk-workflow.md` | 221 | doc vivante |
| `docs/journal/JOURNAL.md` | 65 | doc vivante |
| `docs/plans/2026-03-30-apple-tv-3rd-node.md` | 10,19,30,40,89,165 | doc vivante |
| `docs/plans/2026-06-09-dev-metrics-dashboard.md` | 39 | doc vivante |
| `docs/plans/L1-001-matrice-flotte.md` | 37 | doc vivante |
| `docs/plans/PROMPT-fix-connectioninner-worktree.md` | 130 | doc vivante |
| `docs/plans/R33-path-observability.md` | 13,55 | doc vivante |
| `docs/plans/capstone-run-checklist.md` | 38 | doc vivante |
| `docs/plans/hermeticite-etage-l.md` | 13 | doc vivante |
| `docs/plans/porte-automatique-self-relay-upnp.md` | 22 | doc vivante |
| `docs/plans/r14-ipv6-first-class.md` | 18,101 | doc vivante |
| `experiments/iroh-poc/results/00-baseline-home-wifi.jsonl` | 4 | données d'essai |
| `experiments/iroh-poc/results/01-commute-fr-ch-_en_suisse_Salt_operator_part2:2.jsonl` | 4 | données d'essai |
| `experiments/iroh-poc/results/01-commute-fr-ch3_ecole.jsonl` | 4 | données d'essai |
| `experiments/iroh-poc/results/01-commute-fr-ch_en_france_jusqua_la_frontiere_freemobile_operator_part1:2.jsonl` | 4,418 | données d'essai |
| `results/4g-car-continuous.log` | 6,12,19,25,46,159 | données d'essai |
| `results/4g-car_20260213-074258/burst.log` | 8 | données d'essai |
| `results/4g-car_20260213-074258/continuous.log` | 6 | données d'essai |
| `results/4g-car_20260213-074258/ladder.log` | 8 | données d'essai |
| `results/4g-car_20260213-074258/ping.log` | 6 | données d'essai |
| `results/4g-car_20260213-074505/burst.log` | 8 | données d'essai |
| `results/4g-car_20260213-074505/continuous.log` | 6 | données d'essai |
| `results/4g-car_20260213-074505/ladder.log` | 8 | données d'essai |
| `results/4g-car_20260213-074505/ping.log` | 6 | données d'essai |
| `results/4g-car_20260213-074655/burst.log` | 8 | données d'essai |
| `results/4g-car_20260213-074655/continuous.log` | 6 | données d'essai |
| `results/4g-car_20260213-074655/ladder.log` | 8 | données d'essai |
| `results/4g-car_20260213-074655/ping.log` | 6 | données d'essai |
| `results/4g-car_20260213-074835/burst.log` | 8 | données d'essai |
| `results/4g-car_20260213-074835/continuous.log` | 6 | données d'essai |
| `results/4g-car_20260213-074835/ladder.log` | 8 | données d'essai |
| `results/4g-car_20260213-074835/ping.log` | 6 | données d'essai |
| `results/4g-car_20260213-075208/burst.log` | 8 | données d'essai |
| `results/4g-car_20260213-075208/continuous.log` | 6 | données d'essai |
| `results/4g-car_20260213-075208/ladder.log` | 8 | données d'essai |
| `results/4g-car_20260213-075208/ping.log` | 6 | données d'essai |
| `results/4g-car_20260213-075352/burst.log` | 8 | données d'essai |
| `results/4g-car_20260213-075352/continuous.log` | 6 | données d'essai |
| `results/4g-car_20260213-075352/ladder.log` | 8 | données d'essai |
| `results/4g-car_20260213-075352/ping.log` | 6 | données d'essai |
| `results/4g-car_20260213-075650/burst.log` | 8 | données d'essai |
| `results/4g-car_20260213-075650/continuous.log` | 6 | données d'essai |
| `results/4g-car_20260213-075650/ladder.log` | 8,17,26 | données d'essai |
| `results/4g-car_20260213-075650/ping.log` | 6,13 | données d'essai |
| `results/4g-cgnat_20260212-205140/burst.log` | 8 | données d'essai |
| `results/4g-cgnat_20260212-205140/ping.log` | 23 | données d'essai |
| `results/4g-cgnat_20260212-213418/ping.log` | 6 | données d'essai |
| `results/4g-highway-v4/MacBook-4g_ping_20260217-072805.log` | 3,7 | données d'essai |
| `results/4g-highway-v4/MacBook-4g_ping_20260217-151139.log` | 2,14,28,60 | données d'essai |
| `results/4g-highway-v4/MacBook-4g_ping_20260217-154931.log` | 2,11 | données d'essai |
| `results/4g-retour-2.log` | 15,18,33 | données d'essai |
| `results/wifi-lan_20260212-203639/ping.log` | 6 | données d'essai |
| `scripts/car-test.sh` | 5,13,20 | script |
| `scripts/deploy-nas-node.sh` | 12,17 | script |
| `scripts/ip-interdites.exemptions` | 1 | script |
| `scripts/ip-interdites.sha256` | 1 | script |
| `scripts/nas-node-ctl.sh` | 13 | script |
| `scripts/verifier-ip-interdites.sh` | 1 | script |
| `vault/10-mailbox.md` | 49,108 | vault |
| `vault/40-roadmap.md` | 57,120,172,211,316 | vault |
| `vault/reprise/2026-10-17-ETAT-DES-LIEUX.md` | 19 | vault |


**Non traités** :
- `vault/30-discoveries.md` : intouchable. Il contient (a) et (c) sur 5 lignes, et il est exempté dans `scripts/ip-interdites.exemptions`. À nettoyer par le fondateur, puis retirer l'exemption.
- `scripts/pre-push-gate.sh` et `.claude/settings.json` (intouchables) : **ne contiennent pas l'IP** à la racine, contrairement à ce que dit le mandat.
- Fichiers des fenêtres F01/F02 : aucun ne contient ces valeurs. Les correspondances numériques dans `crates/tom-protocol-ffi/Cargo.lock`, `.playwright-mcp/*.log` et `results/*` sont des coïncidences de chiffres (hash, durées).

### 3. Garde anti-retour (commits `d913824`, `d93f57b`, `79e139b`, `a7912d1`, `e7f6ce8`)

`scripts/verifier-ip-interdites.sh` lit tous les fichiers suivis (`git ls-files -z`) en octets et ignore les binaires. Il ramène chaque candidat IPv4 ou IPv6 /64 à sa forme canonique, le hache en SHA-256 et le compare à `scripts/ip-interdites.sha256` (3 empreintes). Il n'affiche jamais la valeur trouvée et n'est branché dans aucun hook.

```
### AVANT (origin/main 9278dca + garde copiée)
verifier-ip-interdites: 148 ligne(s) avec une IP interdite
AVANT rc=1
### APRES (tip e7f6ce8), LC_ALL=C puis UTF-8
verifier-ip-interdites: OK (aucune IP interdite)
verifier-ip-interdites: OK (aucune IP interdite)
### hostiles (dépôt jetable), fr_FR.UTF-8 et LC_ALL=C : 9/9 détectés
brute · collée à des chiffres · zéro-paddée · URL 2ᵉ IP · 2ᵉ IP paddée · IPv6 [..]:port · IPv6 MAJUSCULES+zéros ::/64 · IPv6 collée · ligne Latin-1 CRLF
lignes neutres (1.2.3.4, 8.8.8.8, 2a01:e0a::1, 2001:db8:0:1, version à 5 composants) : non signalées ; fichier binaire : ignoré
### sans aucune IP : OK, rc=0
```
Limites écrites dans l'en-tête : binaires, fichiers non suivis, écritures exotiques. Les empreintes sont réversibles par force brute (2³² possibilités) : elles n'ajoutent rien de neuf, puisque les valeurs restent dans l'historique public. Pour les rendre vraiment secrètes, il faudrait un HMAC avec une clé hors dépôt. **Limite non écrite (réserve R1 du tour 4, cause du STOP)** : noms de fichiers, liens symboliques et fichiers suivis mais supprimés du disque.

### 4. Contrôles CI rejoués (worktree, avant le push)

```
cargo test -p tom-protocol         → test result: ok. 695 passed; 0 failed (lib) + 14 binaires de tests, tous « ok », aucune ligne FAILED/panicked
cargo clippy -p tom-protocol -- -D warnings → Finished `dev` profile … in 41.66s (0 warning)
cargo build -p tom-stress          → Finished `dev` profile … in 1m 20s
cargo clippy -p tom-stress -- -D warnings  → Finished `dev` profile … in 25.68s
cargo test … embedded_relay_port_mapped_publishes_global_url → ok
du -sh target → 6,6G
```
`shellcheck` n'est pas installé : remplacé par `bash -n` (OK sur les 4 scripts) et par l'exécution sans variable (sortie 1 avant tout réseau). `pnpm lint` (biome) ne concerne aucun fichier touché : biome ne lit ni md, ni html, ni jsonl, ni sh. Les commits postérieurs ne touchent que la garde (shell/Python) et des docs : cargo n'a pas été rejoué pour eux. Note : la variable `RC=` des journaux cargo était vide (zsh ne remplit pas `PIPESTATUS`), le verdict vient des sorties.

### 5. Commits et push

`0b24450` retrait IPv4 + mot de passe · `d913824` garde · `d93f57b` garde, IP suivie d'un chiffre · `ec8b180` retrait IPv6 · `79e139b` garde IPv6 + sortie sans IP · `a7912d1` garde, forme canonique · `e7f6ce8` garde, sans dépendance à la locale. Pas de trailer, aucune valeur dans les messages (vérifié).
```
e7f6ce8e6fa67a8045f9df5478d9c7ee88de2186	refs/heads/fix/retrait-ip-publique
```
Les diffs poussés montrent les anciennes valeurs en lignes supprimées : c'est le prix de « sans réécrire l'historique », décidé par le fondateur.

Commit vault (rituel 4) : `91b8041 docs(vault): reprise F03 M0009` sur `main` depuis la racine (chemin explicite), poussé :
```
91b80412ae948c5ef0cab72df7c2df684839ec55	refs/heads/main
```
Worktrees `F03-M0009` (6,6 Go de `target/`) et `F03-R007` retirés ; la branche reste sur `origin` (`e7f6ce8`). Aucun process ni tâche de fond laissé.

### 6. Doublage R007 (N2, sous-agents frais, worktree détaché `F03-R007`)

Tour 1 (`d93f57b`) RESERVE : IPv6 non retirée + garde en exit 1 sans IP → `ec8b180`, `79e139b`. Tour 2 (`79e139b`) RESERVE : IPv4 avec zéros de tête → `a7912d1`. Tour 3 (`a7912d1`) RESERVE : pré-sélection `git grep` qui dépend de la locale → `e7f6ce8`. Tour 4 (`e7f6ce8`) RESERVE : R1 garde disque/objets (confirmée, cause du STOP) ; R2 placeholders Markdown non échappés en prose (4 docs). R2 n'est pas une exigence du mandat : c'est une remarque de rendu GitHub, le source reste lisible. Le rapport du tour 4 est dans `vault/revues/2026-10-17-R007-retrait-ip-publique.md`, **volontairement NON committé** : aucune valeur en clair, mais il indique le commit public et la longueur du mot de passe exposé, ce qui guiderait un attaquant. À committer une fois le mot de passe changé, ou à expurger. Rapports des tours 1 à 3 : scratchpad de la session seulement. Chaque tour a confirmé : retrait complet hors intouchable, JSONL valides, `CLAUDE.md` dans le budget, messages propres, scripts sans défaut.

### 7. Audit SSH (lecture seule, aucune modification)

D'après le dépôt (`rapport-diagnostic-ssh-wan-2026-03-02.md`, `deploy-nas-node.sh`, `nas-node-ctl.sh`, `car-test.sh`, `2026-03-30-apple-tv-3rd-node.md`, `freebox-sdk-workflow.md`) :
- **NAS du domicile (VM Debian « SERVEUR » sur la Freebox)** : joignable depuis Internet en SSH sur `<IP_DOMICILE>`, **port WAN 2222 → LAN 22**, utilisateur **root**. La redirection WAN 22 directe est mentionnée par `car-test.sh` mais n'a pas été vérifiée (pas de scan, hors mandat). Les ports relais 3340 (TCP/UDP) et 7842 sont aussi redirigés. Les redirections SMB (137/139/445) ont été retirées le 2026-03-02.
- **Box de la belle-famille** : administration distante Freebox OS ouverte en HTTP et en HTTPS sur deux ports WAN. Pas de SSH documenté.
- Mac Air / Apple TV : SSH seulement en LAN dans les docs. `INFRA-LOCAL-DEPLOYMENT-GUIDE.md:873` cite un mot de passe par défaut `alpine` (LAN, iSH) : à vérifier.
- `~/.ssh/config` : lecture refusée par la permission, je n'ai pas insisté.

Durcissements proposés (aucun appliqué). Règle commune : **garder une session root ouverte pendant chaque changement, et tester une 2ᵉ connexion avant de fermer la 1ʳᵉ.** Recours si on se bloque dehors : console de la VM dans Freebox OS (accès local).

| # | Durcissement | Risque de se bloquer dehors | Retour arrière |
|---|---|---|---|
| 1 | **Changer tout de suite le mot de passe root** (`passwd`) | nul | ancien mot de passe (compromis) |
| 2 | `PasswordAuthentication no` + `KbdInteractiveAuthentication no` (clé seule) | moyen : tout appareil sans clé (iPhone, autre Mac) perd l'accès | remettre `yes`, `systemctl reload ssh` depuis la console VM |
| 3 | `PermitRootLogin prohibit-password`, puis un utilisateur dédié + `sudo` et `PermitRootLogin no` | moyen : les scripts du dépôt utilisent `root@` (à adapter : `deploy-nas-node.sh`, `nas-node-ctl.sh`, `car-test.sh`) | `PermitRootLogin yes` + reload |
| 4 | Redirection Freebox : couper le WAN 2222 et passer par un VPN (WireGuard de la Freebox) ; sinon limiter aux IP sources connues si la box le permet | élevé si on est loin sans VPN prêt | recréer la redirection 2222 → 22 dans Freebox OS (à distance si l'admin distante est active) |
| 5 | fail2ban (ou `sshguard`) sur sshd | faible (bannissement de sa propre IP après des échecs) | `fail2ban-client unban <ip>` depuis la console / désinstaller |
| 6 | IP publique : demander une nouvelle IP fixe à Free (ou passer en IP partagée) puisque l'actuelle est publique dans l'historique | faible, mais à reporter partout (`TOM_HOME_HOST`, appareils de la flotte) | aucun (irréversible côté FAI) |
| 7 | Box de la belle-famille : couper l'admin distante Freebox OS si inutile, sinon HTTPS seul | faible (perte de l'admin à distance) | réactiver dans Freebox OS en local |
| 8 | `car-test.sh` : passer de `sshpass` à une clé (supprimer `TOM_NAS_SSH_PASS`) | nul | — |

### 8. Remarques / angles morts

- L'identifiant d'interface IPv6 du NAS, dérivé de son adresse MAC (EUI-64), reste dans environ 90 lignes, en lien local `fe80::…` et en suffixe. Ce n'est pas une IP publique ; à trancher.
- Des adresses LAN `192.168.0.x` (58 fichiers), le port 2222 et l'utilisateur `root` restent dans les scripts et docs : hors périmètre IP publique.
- Le scratchpad de la session contient des dépôts de test jetables avec les valeurs en clair (local, temporaire).
- La garde n'est branchée ni dans la CI ni dans les hooks (prévu M0007).

M0009
Implementation      ✅
Tests               ✅
Commit              ✅
Branch push         ✅
Review              ❌
Merge main          ❌
Main push           ❌
TASK STATUS         BLOCKED
PROJECT STATUS      NOT_INTEGRATED
