# Étape 1 — Le canal « log » : groupe public auto-porté

> Design-first. But : un groupe **public** `log` où chaque nœud déverse ses logs, porté
> **organiquement** (rejoins-ou-crée), pour voir les logs de tout device — **iPhone en 5G
> compris** — via **notre propre réseau** (chiffré, pas de port exposé). Dogfooding des groupes.
>
> Étape 2 (après) = le « génome immortel » : réplication des paramètres via le backup service
> pour un portage collectif sans perte. Hors périmètre ici.

## 1. Décisions (simples, tranchées avec Malik)

| Point | Décision |
|---|---|
| Identité | `GroupId::from("log")` — **well-known constant**, tout le monde vise le même |
| Ouverture | **public** : créé avec `invite_only = false` → `hub.handle_join` accepte n'importe qui (vérifié `hub.rs`) |
| Amorçage | **rejoins-ou-crée** : au démarrage, broadcast `Join{"log"}` aux pairs ; si un `Sync` revient → rejoint ; sinon (timeout) → **crée** et devient porteur |
| Porteur tombe | plus de `Sync` → un autre **recrée** (`rens()` re-tente au reconnect). Résilience « recréation » (état minimal reperdu — le sans-perte, c'est l'étape 2) |
| Deux « log » (partition) | convergence par **election déterministe** (plus petit NodeId porte, l'autre s'y rallie) |
| Débit | **throttle** : 1 message de groupe / ~2 s regroupant les lignes accumulées (anti-inondation + anti-`sprinkler`) |
| Vue | les messages `log` sont **filtrés de l'UI Messages** (jamais affichés comme messages normaux) |
| Echo | le bot **ne renvoie pas** les messages de groupe (pas de boucle) |

## 2. Où vit quoi

```
┌─ Runtime (Rust, tom-protocol) ── PARTAGÉ apps + bot ─────────────┐
│  Orchestration « log group » : au start (après découverte ~5 s), │
│  join-broadcast → timeout ~3 s → create(public). Config flag.    │
│  Réutilise group_manager + GroupPayload::Join + handle_join.     │
│  Expose le group_id "log" (constant).                            │
└──────────────────────────────────────────────────────────────────┘
      ▲ FFI (activer + exposer log_group_id)         ▲ direct (bot)
┌─────┴───────────────────────┐        ┌─────────────┴──────────────┐
│ App Swift (iPhone/iPad/…)   │        │ Bot NAS (tom-tui, Rust)    │
│ appendLog → buffer → flush  │        │ membre du groupe ;         │
│ 2 s → send_group_message    │        │ route :9300/group/inbox    │
│ ("log", batch) ; filtré UI  │        │ (lecture par moi) ;        │
└─────────────────────────────┘        │ pas d'echo groupe          │
                                        └────────────────────────────┘
```

## 3. Lecture (moi) — la boucle de diagnostic

Les messages du groupe arrivent chez **tous les membres**, dont la Freebox. Le bot expose
`GET :9300/group/inbox?group=log&contains=…&limit=…` (miroir de `/inbox`). Je lis en SSH
(déjà accessible à distance). → **logs terrain de tous les devices, 5G comprise, par notre réseau.**

## 4. Fichiers touchés

- **Runtime** `crates/tom-protocol/src/runtime/` : module orchestration log-group + config `log_group`.
- **FFI** `tom-protocol-ffi` : activer le log-group + exposer `log_group_id` + (déjà) `send_group_message`.
- **App Swift** `TomNodeService.swift` : hook `appendLog` → buffer throttlé → `send_group_message` ; filtrage UI.
- **Bot** `crates/tom-tui/src/main.rs` : activer log-group + route `/group/inbox`.
- **Rebuild** : FFI xcframework → apps (iPhone/iPad/ATV/Mac) + bot NAS (musl).

## 5. Validation (done-when)

1. iPhone **en 5G** poste ses logs dans `log` → je les lis via `:9300/group/inbox` sur la Freebox.
2. iPad + ATV + Mac postent aussi → un **seul** groupe `log`, tous membres.
3. Je coupe le porteur → un autre **reprend** (recréation) → les logs reprennent (< ~15 s).
4. Aucun message `log` visible dans l'UI Messages. Pas de boucle d'echo. Débit throttlé.

## 6. Non-objectifs (→ Étape 2)

Réplication du **génome** (paramètres) via backup ; **reprise sans perte** d'historique ;
créateur qui revient comme simple porteur. Tout ça vient après, en design + red-team.
