# Handoff — Transport : port QUIC fixe via --bind-port/--bind-addr

> Date : 2026-06-09
> Type : review
> Priorité : moyenne (feature déployée sur NAS, en production)
> reviewedRange: aa1374e..3c7b0b5

---

## De : Claude (Sonnet 4.6)

### Contexte

Commit `3c7b0b5` — `feat(transport): port UDP QUIC fixe via --bind-port/--bind-addr`

Le NAS (Freebox, IPv6 public) nécessite un port UDP stable pour une règle
pare-feu entrante durable. Sans port fixe, QUIC bind sur un port éphémère à
chaque démarrage → la règle pare-feu Freebox devient invalide.

Implémentation :
- `TomNodeConfig.bind_addr: Option<SocketAddr>` + builder `.bind_addr()`
- Variable d'environnement `TOM_BIND_ADDR` (format `[::]:PORT`)
- Flags CLI dans `tom-tui` : `--bind-addr <addr>` et `--bind-port <port>`
  (raccourci dual-stack : `--bind-port 43925` → `[::]:43925`)
- Branchement sur `Endpoint::bind_addr()` dans `TomNode::bind()`
- Test `bind_addr_binds_fixed_port` : vérifie `bound_sockets()` post-bind

Déployé sur NAS : port `43925` stable, `node_id` persistant via `--key-path`.

### Fichiers à lire

- `crates/tom-transport/src/config.rs` — champ `bind_addr` + builder + env parsing
- `crates/tom-transport/src/node.rs` — intégration dans `TomNode::bind()`
- `crates/tom-tui/src/main.rs` — flags CLI `--bind-addr` / `--bind-port`

### Question précise

1. **Dual-stack IPv4/IPv6** : `[::]:43925` bind en dual-stack sur Linux (NAS Debian)
   mais pas toujours sur macOS (`IPV6_V6ONLY` par défaut à 1 sur BSD).
   Est-ce que le comportement actuel est correct pour les deux OS, ou faut-il
   gérer explicitement le bind IPv4 + IPv6 séparément ?

2. **Collision de port** : si le port `43925` est déjà occupé au démarrage,
   `Endpoint::bind_addr()` retourne une erreur. Cette erreur est-elle bien
   propagée jusqu'à l'utilisateur (log, exit code) ou avalée silencieusement ?

3. **Test de robustesse** : le test `bind_addr_binds_fixed_port` vérifie que
   `bound_sockets()` contient le port cible. Vérifie-t-il le bon port sur la
   bonne interface ? Y a-t-il un risque de faux positif si le système assigne
   un port proche ?

4. **`--bind-port` sans `--bind-addr`** : le raccourci génère `[::]:PORT`.
   Y a-t-il un cas où l'utilisateur voudrait `0.0.0.0:PORT` plutôt que `[::]:PORT` ?
   Le comportement actuel est-il documenté ?

### Contraintes / hors scope

- Ne pas proposer de refactorer TomNodeConfig en builder pattern complet
- Le NAS tourne Debian ARM64 musl — QUIC via iroh-quinn-udp (non forké)
- L'objectif est pare-feu Freebox IPv6 entrant : stabilité du port uniquement

---

## Réponse de : Copilot/GPT

**INSTRUCTION POUR LE LLM QUI RÉPOND :**
Tu DOIS écrire ta réponse directement dans **ce même fichier `.md`**, à la
suite du document existant, dans la section `## Réponse de : Copilot/GPT`.
Tu ne dois PAS répondre dans le chat, ni dans le prompt, ni dans un autre fichier.
Ce document est le vase communicant entre GPT et Claude. Utilise Edit/Write.
Quand tu as fini, dis : "J'ai répondu dans `docs/handoffs/2026-06-09-transport-bind-port-quic-fixe.md`."

---

## Intégration
<!-- Claude remplit après lecture de la réponse -->
