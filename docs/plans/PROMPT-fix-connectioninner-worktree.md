# PROMPT — Fixer la fuite `ConnectionInner` (OOM Freebox) en worktree, valider par load-8

> Colle ce fichier entier comme premier message d'une nouvelle session.
> Mission : **implémenter le fix de la fuite mémoire `ConnectionInner`, en worktree isolé, et le
> VALIDER empiriquement** — re-run `load-8` sur le NAS musl, le verdict = **`conns_quic` doit
> redescendre au nombre de pairs réels ET le RSS avec** (post-kill). Mode nuit : enchaîne.

---

## 0. Charge le contexte AVANT de coder (obligatoire)

1. **Mémoire** `tom-freebox-oom-carnet-rendezvous` — le verdict complet, le call-site file:line, le fix designé.
2. **Doc** `docs/plans/verdict-oom-connectioninner-2026-07-22.md` — preuve terrain, **4 pistes de fix**, file:line.
3. Mémoires connexes : `tom-pathidle-timer-inoperant` · `pool-lock-hostage-root-cause-2026-07-17`
   · `relaymode-disabled-pas-de-transport-relais` · `nas-dynamic-ip-and-device-diagnostics` · `validate-clean-worktree-before-push`.
4. **Vault-first** : `vault/30-discoveries.md`.

## 1. Le verdict (résumé — détail dans le doc)

Fuite mémoire RÉELLE, non bornée, ∝ churn de connexions QUIC. **Ce n'est ni le relais, ni le DHT,
ni le pool, ni les structures applicatives** (tous innocentés, vérifiés). C'est **`ConnectionInner`
(tom-quinn) jamais libérée** : une connexion QUIC **abandonnée** (handles applicatifs droppés) n'est
**jamais fermée activement**, car le fork a unifié le `ref_count` (handles + driver + paths dans un
seul `AtomicUsize`). `implicit_close` n'est appelé que par `Drop for ConnectionRef` à `prev==1`
(`tom-quinn/src/connection.rs:1286`), or le **driver garde une ref tant qu'il vit** → `ref_count`
n'atteint jamais 0 tant que le driver tourne → la connexion attend un idle timeout (10 s) qui sous
kill-brutal/churn ne draine pas → `ConnectionInner` (~250-400 Ko) s'accumule → OOM.

**Preuve terrain (NAS musl instrumenté)** : `conns_quic` 131→991 sous charge, et **RSS figé à 293 Mo
même quand le proto purge 660 connexions** post-kill (`conns_quic` 991→330). Sur musl (qui rend la
mémoire), un RSS bloqué = vraie fuite. CSV : `/tmp/nas-load.csv` (de la session précédente).

## 2. Le fix — 2 pistes (ordre recommandé)

### ▶ PISTE 2 d'abord (pool-close, la MOINS risquée — ne touche pas le ref_count)
Le churn passe par le POOL (`tom-transport/src/connection.rs`). Quand une nouvelle connexion d'un
pair arrive, l'ancienne est **écrasée** (`register_inbound` l.81-93 `retain`+`insert` ; `get_or_connect`
l.251 `insert`) : **l'ancien handle `Connection` est droppé mais jamais fermé** (`close_reason()`
reste None → même bug). **Fix** : avant d'écraser/retirer une ancienne connexion vivante, appeler
`old.close(0u32.into(), b"")` (API publique tom-connect) → fermeture active → drained → libérée.
Points : `register_inbound` (avant l'insert qui écrase), `get_or_connect` l.251 (avant insert),
`remove`, `unregister_inbound`.
⚠️ **Logique subtile #46b/#46c** (dial mutuel, fusion QUIC, quelle connexion LIT les entrants) : ne
ferme QUE l'ancienne entrée réellement remplacée par une NOUVELLE du même pair ; ne ferme jamais
celle qu'on vient d'insérer. Vérifie qu'on ne casse pas la réception (le test load-8 doit garder
`sends OK` élevé).

### ▶ PISTE 1 si la 2 est insuffisante (app_handle_count — le fix COMPLET, cœur tom-quinn)
Restaure la distinction handle applicatif ↔ driver, **sans modifier** le `ref_count` atomique existant :
1. Ajoute `app_handle_count: AtomicUsize` dans `ConnectionInner` (`tom-quinn/src/connection.rs:1300`).
2. Incrémente/décrémente-le **UNIQUEMENT** dans `Connection` (le handle applicatif, l.316) — Clone/Drop
   **manuels** (il a un `#[derive(Clone)]` à retirer). Le driver et les `Path` n'y touchent pas.
3. Quand `app_handle_count` atteint 0 → notifie le driver via `shared.closed` (`Notify`, existe l.1342)
   **hors verrou** (anti-deadlock préservé).
4. Le driver, au poll suivant, voit le signal → `implicit_close` sous SON verrou → drained → libérée immédiatement.

> ⚠️⚠️ Le `ref_count` est le **cœur anti-deadlock** qui a résolu les gels terrain 13/07 et iOS 17/07
> (`connection.rs:1303-1307`). **N'y touche PAS.** N'AJOUTE qu'un compteur séparé. Une autre fenêtre
> a pu travailler ce fichier — vérifie `git log`/le diff avant.

## 3. Étape 0 — sécuriser l'instrumentation + créer le worktree

L'instrumentation `conns_quic_live` (le JUGE) est déjà écrite mais **NON commitée** dans le working
tree principal (5 fichiers). Un `git worktree` part de HEAD → il faut d'abord la committer, sinon le
binaire du worktree n'exposera pas `conns_quic` et le test sera aveugle.

```bash
cd /Users/malik/Documents/tom-protocol
# Committer UNIQUEMENT l'instrumentation + doc + scripts (PAS .claude/CLAUDE.md ni vault ni tom-stress) :
git add crates/tom-connect/src/endpoint.rs crates/tom-transport/src/node.rs \
        crates/tom-protocol/src/runtime/metrics.rs crates/tom-protocol/src/runtime/loop.rs \
        crates/tom-tui/src/main.rs \
        docs/plans/verdict-oom-connectioninner-2026-07-22.md \
        scripts/chaos/load-8-nas.sh scripts/chaos/deploy-nas.sh
git commit -m "diag(oom): instrumentation conns_quic_live + handshakes_accepted au /status"
# ⚠️ le hook guard-loop-master peut bloquer (>1 fichier). Si oui : c'est du DIAGNOSTIC, pas une
#    feature — touch le flag attendu par le hook, OU passe par /loop-master, selon ce que le hook demande.

# Worktree isolé pour le fix (part du commit qui contient l'instrumentation) :
git worktree add ../tom-fix-connectioninner -b fix/connectioninner-leak HEAD
cd ../tom-fix-connectioninner
```

## 4. Étape 1 — implémente le fix (dans le worktree)

Implémente la PISTE 2 (ou 1). Valide par crate au fur et à mesure :
```bash
cargo build -p tom-transport   # (piste 2) ou -p tom-quinn (piste 1)
cargo build -p tom-tui --bin tom-chat   # tire toute la chaîne
cargo clippy -p <crate touché> -- -D warnings
```

## 5. Étape 2 — rebuild ARM + deploy sur le NAS

⚠️ **Le binaire du worktree est dans `<worktree>/target/…`, PAS dans le repo principal.** Édite la
variable `BIN` de `scripts/chaos/deploy-nas.sh` pour pointer le target du WORKTREE, ou cp le binaire.

```bash
# Depuis le worktree :
cargo zigbuild -p tom-tui --bin tom-chat --target aarch64-unknown-linux-musl --release
# BIN attendu par deploy-nas.sh = <worktree>/target/aarch64-unknown-linux-musl/release/tom-chat
bash scripts/chaos/deploy-nas.sh   # (édite BIN dedans d'abord) — stop→backup→scp→md5→start→vérif instrumentation
```
Le deploy vérifie que `conns_quic_live` apparaît dans `/status` (sinon le binaire n'a pas l'instrumentation).

## 6. Étape 3 — re-run load-8 + oracle (LE verdict)

⚠️ Le binaire de CHARGE (les 8 nœuds) est le `tom-chat` **debug** local — build-le dans le worktree :
`cargo build -p tom-tui --bin tom-chat` (édite `BIN` de `load-8-nas.sh` vers `<worktree>/target/debug/tom-chat`).

```bash
# garde-fou timeout + trap durci (déjà dans le script) :
timeout 900 bash scripts/chaos/load-8-nas.sh 8 12 5 > /tmp/nas-load-fix.log 2>&1 &
# surveille conns_quic en direct (Monitor sur /tmp/nas-load-fix.log), ou attends la fin + lis /tmp/nas-load.csv
```

### ✅ Definition of Done (critère de succès, sans ambiguïté)
1. **`conns_quic` post-kill redescend au ~nombre de pairs réels** (≈ pairs de la flotte, PAS des
   centaines de fantômes). Avant fix : restait à ~330-991. Après fix : doit tomber à ~poignée.
2. **RSS post-kill redescend** vers la baseline (~40-90 Mo), ne reste PAS figé à ~290 Mo.
   L'oracle du script doit sortir `PAS DE FUITE nette (RSS redescend post-kill)`.
3. **Pas de régression réception** : `sends OK` reste élevé pendant la charge (le fix ne casse pas
   l'envoi/réception — piège #46b/#46c de la piste 2).
4. Endurance : idéalement laisser le NAS tourner quelques heures / une nuit sans OOM (`free -m`,
   `VmRSS` stable). Le NAS est en `Restart=always`, kernel OOM à ~920 Mo, pas de `MemoryMax`.

## 7. Pièges (déjà payés — ne les rejoue pas)

- **IP NAS DYNAMIQUE** : `192.168.0.83` au 22/07 (bail DHCP). Si `ssh`/`curl` timeout : `arp -a` →
  cherche le hostname base32 sur une autre IP, mets à jour `NAS=` dans les 2 scripts. Témoin de vie : `nc 82.67.95.8 3340`.
- **Cross-compile** : `cargo zigbuild … --target aarch64-unknown-linux-musl --release`. `scp` d'un binaire
  pendant que le process tourne → « dest open Failure » : `deploy-nas.sh` stoppe le service AVANT (OK).
- **`cargo … | tail` masque l'exit code** — utilise `> log 2>&1; echo EXIT=$?`.
- **Bash `wait` sans arg** attend les nœuds bots infinis → hang. `load-8-nas.sh` est déjà corrigé (curls séquentiels).
- **Tuer les process de charge AVANT et APRÈS** (`pkill -9 -f target/debug/tom-chat`). Le trap du script le fait.
- **`target/` ≤ 20 Go** (non négociable). Le worktree a son propre target → surveille `du -sh`, `cargo clean` si besoin.
- **loopback = tout DIRECT** (le relais n'est pas sollicité) → le vrai churn se voit sur le LAN Mac↔NAS, pas en local.
- **FFI** : si tu touches un crate côté FFI (`tom-connect`/`tom-quinn`), valide `bash scripts/check-ffi.sh` (hors workspace).
- **Ne PAS push sans la gate** : `bash scripts/pre-push-gate.sh` + `/review-oracle` (hook bloque si diff ≥ 50 lignes).
  Commits en français, jamais signés. Valide en **worktree propre à HEAD** avant push (le working copy masque les erreurs).

## 8. Si les deux pistes échouent / doute

Ne pas itérer à l'identique (§6). Reviens au doc `verdict-oom-connectioninner-2026-07-22.md` (pistes 3 churn
/ 4 idle-timer), ou remonte à Malik avec : ce qui a été tenté, le CSV `conns_quic`/RSS obtenu, l'hypothèse suivante.

## État au départ
- NAS propre (~35 Mo), tourne le binaire **instrumenté** (sans fix) — `TOM_RELAY_URL=http://192.168.0.83:3340`.
- Instrumentation `conns_quic_live`+`handshakes_accepted` : 5 crates dans le working tree principal (à committer, étape 0).
- Scripts persistés : `scripts/chaos/load-8-nas.sh` + `scripts/chaos/deploy-nas.sh`.
- Mémoires + doc à jour.
