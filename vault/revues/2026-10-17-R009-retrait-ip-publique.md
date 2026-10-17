---
verdict: GO
tip: 3ef96eabd8b7d80ccc7871827db01256e75e52ba
niveau: N2
---

# R009 — retrait IP publique (doublage N2 court)

Verdict provisoire : **GO** sur les points (a) à (d). Point (e) en attente (sorties de `scripts/ci-local.sh` à fournir par la session mère). Aucune valeur interdite n'est affichée ci-dessous : comptes, fichier:ligne, masques uniquement.

## (a) Aucune occurrence des valeurs retirées au tip (hors exemption)

Contrôle indépendant sur les objets git (`git ls-tree -r -z HEAD` + `git cat-file --batch`, 3 hashs chargés depuis `scripts/ip-interdites.sha256`, canonicalisation IPv4 et préfixe IPv6 /64, hachage, comparaison).

Script : `/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/df01a218-b24a-43e8-8d1d-28853917740e/scratchpad/r009/scan_objects.py`

Sortie :
```
hashs interdits charges: 3
fichiers blob scannes: 1499 | binaires (ignores, NUL): 3
noms de fichiers interdits: 0
occurrences hors exemption: 0 | dans exemption: 7
```

- Exemption : les 7 occurrences sont dans `vault/30-discoveries.md` (seul fichier exempté), conformément à `scripts/ip-interdites.exemptions`.
- Auto-test du scanner (valeurs synthétiques, hashs injectés en mémoire) : détecte une IPv4 `a.b.c.d`, une IPv4 à zéros de tête, un /64 en forme compressée ; ne déclenche pas sur une IPv4 voisine. Sortie : `True / True / True / False`.
- Binaires : `apps/tom-node-tvos/.../icon_1024.png` (non modifié par la branche) et `docs/plans/2026-04-01-plan-maitre-reseau-vivant-v2.pdf` (non modifié) ne sont pas scannés (limite connue de la garde, binaires).
- `docs/article ToM protocol.pdf` (modifié par la branche, 754 968 → 717 780 octets) : extraction texte `pdftotext -layout` puis même contrôle par hash : **0 occurrence au tip**, contre **1 occurrence sur main** (99aa090). Le retrait est donc causal sur ce fichier.

Garde :
```
bash scripts/verifier-ip-interdites.sh
verifier-ip-interdites: OK (aucune IP interdite)
rc=0
```

Mot de passe NAS :
- `git grep` d'un `sshpass -p` suivi d'un littéral : **aucun résultat**.
- Affectations `TOM_NAS_SSH_PASS=`, `PASSWORD=`, etc. hors exemption : seulement `.env.example:12` (valeur **vide**) et un exemple générique `password=` (valeur vide) dans `.claude/orchestration/parallelization.md:60`.
- `scripts/car-test.sh:20` : `TOM_HOME_HOST="${TOM_HOME_HOST:?…}"` ; `scripts/car-test.sh:22` : `NAS_SSH_PASS="${TOM_NAS_SSH_PASS:?…}"` (forme `${VAR:?}`, aucun défaut). Les `sshpass -p "$NAS_SSH_PASS"` (l.27, l.71) passent par la variable.

## (b) Scripts sans valeur publique par défaut, `bash -n`

- `bash -n` : `car-test.sh` OK, `deploy-nas-node.sh` OK, `nas-node-ctl.sh` OK.
- Lecture : `deploy-nas-node.sh:17` `SSH_HOST="${1:-${TOM_HOME_HOST:?…}}"` (le port 2222 est un défaut de port, pas d'hôte) ; `nas-node-ctl.sh:13` `TOM_HOME_HOST="${TOM_HOME_HOST:?…}"`. Dans les trois scripts, la vérification de variable précède toute commande réseau (`car-test.sh` : l.1-19 ne contiennent que des affectations et des commentaires).
- Exécution sans variables (`env -u TOM_HOME_HOST -u TOM_NAS_SSH_PASS timeout 10 bash <script>`) :
```
scripts/car-test.sh rc=1            (message : TOM_HOME_HOST absent …, avant tout accès réseau)
scripts/deploy-nas-node.sh rc=1     (idem, l.17)
scripts/nas-node-ctl.sh rc=1        (idem, l.13)
```

## (c) Test `state.rs` (vers l.7714)

- Test : `crates/tom-protocol/src/runtime/state.rs:7698` `fn embedded_relay_port_mapped_publishes_global_url()`.
- `git grep -n -E 'std::env|env!\(|option_env!|env::var'` sur `state.rs` : **aucun résultat** (rc=1).
- Diff vs main sur ce fichier : une seule ligne modifiée (l.7714, `.parse().expect("valid public SocketAddr")` avec une adresse changée). La nouvelle adresse est `1.2.3.<dernier octet>:58870` : IPv4 d'exemple neutre (plage générique, non interdite — vérifiée par le scan (a)).

## (d) JSONL `experiments/iroh-poc/results/*.jsonl`

```
fichiers jsonl: 4
lignes non vides: 3451 | invalides: 0
```
(`python3 -I` + `json.loads` ligne à ligne.) Les 4 fichiers modifiés par la branche (1 à 2 lignes chacun) restent valides.

## (e) Merge de `main` — en attente

- `git grep -n -E '^(<<<<<<<|=======|>>>>>>>)' HEAD` : **aucun marqueur** (rc=1).
- `git diff 99aa090 3ef96ea --stat` : 78 fichiers, 274 insertions, 154 suppressions. Le périmètre est celui du retrait (scripts `car-test`, `deploy-nas-node`, `nas-node-ctl`, garde + sha256 + exemptions, `state.rs` et `fleet_probe.rs` pour une IP d'exemple, docs/plans, résultats et logs contenant les IP, `.env.example`, `CLAUDE.md`, vault). Aucun fichier hors de ce périmètre.
- Journal `scripts/ci-local.sh` relu par moi (`grep -a`, `/private/tmp/claude-501/-Users-malik-Documents-tom-protocol/df01a218-b24a-43e8-8d1d-28853917740e/scratchpad/ci-local-1.log`, 11 678 lignes) :
  - en-tête : `base de comparaison : 99aa090 → HEAD 3ef96ea` (correspond au tip) ;
  - ligne 11678 : `[ci-local] TOUT VERT.` (unique) ;
  - 34 lignes `test result`, somme recalculée : **passed=1354, failed=0** ;
  - 0 occurrence de `FAILED` ou `panicked`.
- (e) : OK.

## Réserves R007 

Les réserves R007 (garde disque vs objets : noms de fichiers, cibles de liens, index vs disque ; placeholders Markdown non échappés) ont été acceptées par le fondateur le 2026-10-17 20:43 → issue #67.

## Remarques (hors réserves)

1. **Test de détection de la garde non exécuté par moi.** J'ai prévu de vérifier que `verifier-ip-interdites.sh` sort en 1 sur une valeur synthétique plantée dans un dépôt jetable ; la commande a été refusée par la permission Bash et je ne l'ai pas contournée. Ce n'est pas une réserve (la garde a bien tourné en OK sur le tip), mais la preuve de détection par la garde elle-même reste à faire si le fondateur la veut.
2. **Valeurs encore présentes dans l'historique git public** (commits antérieurs au retrait, dont le mot de passe NAS de l'ancien `car-test.sh`). Le retrait du tip ne les supprime pas. Une rotation du mot de passe NAS est à considérer si ce n'est pas déjà fait ; hors périmètre de R009.
3. **Exemption `vault/30-discoveries.md`** : 7 occurrences conservées (exemption assumée). Elle reste à retirer dès nettoyage, selon le commentaire du fichier d'exemptions.
4. **Fichiers du vault touchés par la branche** : `vault/10-mailbox.md`, `vault/40-roadmap.md`, `vault/reprise/2026-10-17-ETAT-DES-LIEUX.md`, `vault/reprise/F03.md` apparaissent dans le diff. Je ne les ai pas relus au fond (hors périmètre) ; je n'ai pas touché `vault/runtime/` ni `vault/reprise/`.
5. **Scan des binaires limité** : PNG d'icône et PDF du plan maître non scannés (le PDF article l'a été par extraction texte).
6. Aucune écriture dans le worktree, aucun commit, aucun build ni test cargo.
