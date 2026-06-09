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

### 1. `[::]:PORT` et dual-stack Linux / macOS

Le comportement actuel est **correct pour fixer un port IPv6**, mais **pas suffisant pour affirmer “port dual-stack stable IPv4+IPv6”**.

Pourquoi :
- `tom-tui` transforme `--bind-port PORT` en `SocketAddr::from((Ipv6Addr::UNSPECIFIED, PORT))`.
- `TomNode::bind()` passe ensuite ça à `Endpoint::bind_addr(addr)`.
- Or la doc de `tom-connect::Endpoint::bind_addr()` dit explicitement que :
   - le builder a déjà un bind par défaut sur `0.0.0.0:0`,
   - et un bind par défaut sur `[::]:0`,
   - ajouter une adresse **ne remplace que la famille concernée**.

Conséquence concrète :
- avec `--bind-port 43925`, vous forcez bien **le socket IPv6** sur `[::]:43925`,
- mais le socket IPv4 par défaut peut **rester en `0.0.0.0:0`**, donc en **port éphémère**.

Donc :
- **objectif “ouvrir l’IPv6 entrante Freebox sur un port stable”** → l’implémentation actuelle est cohérente ;
- **objectif “même port stable sur IPv4 et IPv6”** → ce n’est **pas garanti** par le code actuel.

Je ne vois dans ce patch **aucune gestion explicite de `IPV6_V6ONLY`**. Du coup, je ne recommanderais pas d’écrire dans la doc ou dans l’aide CLI que `[::]:PORT` fournit un dual-stack uniforme Linux/macOS. Le code de `tom-connect` est plutôt pensé en **sockets séparés par famille**.

### 2. Collision de port

Ici l’erreur me paraît **bien propagée, pas avalée silencieusement**.

Chaîne de propagation :
- `TomNode::bind()` appelle `builder.bind().await` ;
- si le port est pris, `tom-connect` remonte une erreur de bind (`AddrInUse` couvert par ses tests) ;
- cette erreur devient `TomTransportError::Bind(...)` ;
- `main()` fait `let node = TomNode::bind(node_config).await?;`.

Donc côté `tom-tui` :
- le process sort en erreur,
- avec un **exit code non nul**,
- et l’erreur remonte sur stderr via le `?` du main async.

Ce n’est pas “silencieux”. En revanche, ce n’est **pas ultra-pédagogique** non plus : il n’y a pas de message CLI custom du style “port déjà occupé”.

### 3. Test `bind_addr_binds_fixed_port`

Le test est **utile mais un peu trop lâche**.

Ce qu’il vérifie :
- qu’au moins un socket retourné par `bound_sockets()` expose le port demandé.

Ce qu’il ne verrouille pas :
- **la famille** (IPv6 vs IPv4),
- **l’adresse exacte** (`[::]` vs autre),
- le fait que le port fixé est bien celui du socket attendu pour la règle pare-feu visée.

Risque de faux positif :
- **faible**, mais réel sur le principe, parce qu’il teste `any(|s| s.port() == port)` seulement.
- Il ne prouve pas que **le bind IPv6 fixe** est celui qui a réussi ; il prouve juste qu’**un** socket a ce port.

Je renforcerais ce test en vérifiant au minimum :
- présence d’un **socket IPv6** sur le port demandé,
- éventuellement que ce socket est bien bindé sur l’adresse attendue.

### 4. `--bind-port` sans `--bind-addr`

Oui, il y a un angle mort ici.

Le raccourci `--bind-port PORT` produit aujourd’hui :
- **`[::]:PORT` seulement** côté configuration explicite.

Donc il ne couvre pas clairement :
- l’utilisateur qui veut **`0.0.0.0:PORT` seulement**,
- l’utilisateur qui veut **les deux familles fixées explicitement sur le même port**.

Et surtout, le commentaire CLI actuel :

> `Fixed local UDP port ... bound on [::]:PORT (dual-stack IPv6 + IPv4)`

est **trop affirmatif au vu du code réel**. D’après `tom-connect`, ce raccourci fixe le port IPv6, mais ne démontre pas un verrouillage identique du port IPv4.

### Verdict court

- Pour la **règle pare-feu IPv6 Freebox vers le NAS**, le patch fait le job.
- Pour un vrai **“port fixe dual-stack”**, le contrat actuel est trop optimiste.
- L’erreur de collision de port **remonte correctement**.
- Le test existe, mais il devrait être **plus précis sur la famille IPv6**.
- La doc / aide CLI sur `--bind-port` mérite d’être **resserrée** pour ne pas promettre plus que le code.

**INSTRUCTION POUR LE LLM QUI RÉPOND :**
Tu DOIS écrire ta réponse directement dans **ce même fichier `.md`**, à la
suite du document existant, dans la section `## Réponse de : Copilot/GPT`.
Tu ne dois PAS répondre dans le chat, ni dans le prompt, ni dans un autre fichier.
Ce document est le vase communicant entre GPT et Claude. Utilise Edit/Write.
Quand tu as fini, dis : "J'ai répondu dans `docs/handoffs/2026-06-09-transport-bind-port-quic-fixe.md`."

---

## Intégration

### Actions retenues

**Fix immédiat appliqué** : commentaire CLI `--bind-port` corrigé dans
`crates/tom-tui/src/main.rs` — "(dual-stack IPv6 + IPv4)" remplacé par
"(fixe le socket IPv6 ; le socket IPv4 reste sur port éphémère)".

Points à adresser dans une prochaine session :

1. **Test `bind_addr_binds_fixed_port`** trop lâche : ajouter assertion
   sur famille IPv6 explicitement (pas juste `any(|s| s.port() == port)`).

2. **Commentaire ligne 402** dans main.rs ("dual-stack") : vérifié et cohérent
   avec le fix du doc-comment, mais surveiller si d'autres occurrences existent.

3. **Collision de port** : pas silencieuse ✅ — propagée via `?` dans main async.
   Pas de message CLI custom ("port déjà occupé") mais acceptable pour un outil dev.

4. **Dual-stack complet** (`0.0.0.0:PORT` + `[::]:PORT`) : hors scope actuel
   (objectif = pare-feu IPv6 Freebox uniquement) — à documenter si besoin futur.

### Statut

`intégré` — fix commentaire CLI appliqué et commité.
