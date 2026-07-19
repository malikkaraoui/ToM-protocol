# Fix CRITIQUE — budget mémoire en octets pour le backup store (build 126)

> 2026-07-19. Corrige le bug découvert le matin même (voir `r14-ipv6-first-class.md` §2.3ter
> et le vault du 19/07). Symptôme utilisateur rapporté par Malik : « certains soirs la box
> ventilait fort » quand des appareils tournaient avec l'app en foreground pour des campagnes
> de test — c'était la VM en pression mémoire.

## §1 Le bug

`backup/store.rs` bornait le stock de backup par `MAX_TOTAL_MESSAGES = 10_000`, un plafond en
**cardinalité**. `BackupEntry.payload` est un `Vec<u8>` gardé **entier en RAM** (backup ADR-009
= RAM pure, pas de SQLite). Aucun plafond en octets n'existait. ~100 messages de 8 MiB
atteignent donc 800 MiB **très loin** des 10 000 entrées.

Terrain (NAS, VM 920 MiB) : **688 MiB** après 13 h, `free` 54 MiB dispo, OOM-killer déjà passé
(`Killed process 20593 (tom-chat) anon-rss:771260kB`), **8 366 échecs d'envoi** (56 la veille),
**0 pair connecté** — tout en affichant `phase: "connecte"` et en étant vu « DIRECT v6 7 ms »
par le Mac. **Preuve A/B au redémarrage : 688 MiB → 24 MiB, 0 → 5 pairs, 8 366 → 0 échec**,
même binaire et même réseau.

Déclencheur : les campagnes de charge légitimes de la veille (échelle 1/2/4/8 MiB).
**Un test normal suffisait à tuer un nœud de production.**

3ᵉ récidive de la classe « borne par-unité sans budget global » (cf. `tom-large-message-dos`,
`tom-reassembly-memory-dos`).

## §2 Le correctif

- `MAX_TOTAL_BYTES = 64 MiB`. Justification : un nœud sain fait ~24 MiB de RSS total ; 64 MiB
  laisse un relais utile tout en gardant de la marge sur le plus petit hôte du parc (VM 920 MiB).
  Le plafond d'un message étant de 1 MiB (envelope), cela représente ~64 messages pleins.
- Champ `total_bytes` maintenu **incrémentalement** (jamais recalculé par balayage).
- **Point de mutation unique** : `insert_entry` / `remove_entry` sont les SEULS endroits où
  `messages` grandit ou rétrécit. Le couple `remove` + nettoyage d'index était dupliqué 4 fois ;
  centraliser rend le compteur **impossible à désynchroniser** (vérifié : `grep messages.insert|
  messages.remove` ne renvoie que ces deux fonctions).
- `make_room_for(incoming, depositor_known, now)` évince jusqu'à ce que ça rentre, appelé sur
  **les deux** chemins d'entrée (`store` local ET `store_replica` réseau).
- **Équité anti-flood préservée** (FINDING #9) : les strangers sont évincés d'abord, et un dépôt
  stranger ne peut jamais déplacer un backup légitime — la règle vaut désormais aussi pour la
  pression en octets, sinon l'attaque changeait juste d'unité.
- **Terminaison garantie** : un payload plus gros que le budget entier est refusé d'emblée, et
  la boucle sort si `evict_oldest` n'évince plus rien (store vide).

## §3 Observabilité — ne pas reproduire le péché qu'on corrige

Le bug a survécu une nuit **parce que rien ne le rapportait**. Deux ajouts :
1. `tick_backup` (`runtime/state.rs`) logge l'empreinte du store, en `warn!` dès **80 % du
   budget** — la saturation devient visible AVANT de mordre.
2. **Aucun refus silencieux** : le budget introduit un chemin de refus qui n'existait pas
   (`store()` évinçait toujours jusqu'au succès). Chaque refus est loggé en `warn!` avec le
   message, sa taille, le total et le budget, sur les deux chemins.
   Note LOCKED #1 : un message non mis en backup n'est pas « livré puis perdu » — l'émetteur
   apprend la vérité par l'absence d'ACK. Le filet n'a simplement pas joué, et cela doit se voir.
3. Accesseurs publics `total_bytes()` / `max_total_bytes()` — le compte seul mentait
   (le relais paraissait sain à 688 MiB).

## §4 Tests (5 ajoutés)

| Test | Ce qu'il prouve |
|---|---|
| `store_is_bounded_by_bytes_not_just_count` | 200 × 1 MiB reste sous le budget, **avec assertion explicite que le cap de COMPTE n'est pas ce qui sauve** |
| `replica_flood_is_bounded_by_bytes` | même garde sur le chemin réseau |
| `total_bytes_returns_to_zero_after_removals` | le compteur ne fuit sur aucune des 3 voies de suppression (délivré / expiré / auto-supprimé) |
| `payload_larger_than_budget_is_refused_not_looped` | refus net, pas de boucle d'éviction infinie, et les backups existants survivent |
| `stranger_cannot_evict_legit_backups_via_byte_pressure` | FINDING #9 tient en octets |

**Validation que les tests prouvent quelque chose** : budget neutralisé temporairement dans une
copie → **4 tests passent au rouge**, restauration → 22/22 verts. Un test qui ne peut pas
échouer ne garantit rien.

## §4bis Review adversariale — 6 findings, 1 seul a produit du code

Verdict brut de l'agent : « BLOQUANT ». Après contre-vérification sur pièces (leçon
[[verify-subagent-security-shortcuts]]), la réalité est plus nuancée :

| # | Annoncé | Réalité vérifiée |
|---|---|---|
| 1 | BLOQUANT — `record_replication` échappe aux helpers | **Auto-réfuté** : l'agent conclut lui-même « garder tel quel ». Il mute des métadonnées, jamais le payload ; le compteur ne suit que `payload.len()`. → invariant verrouillé par un test (`record_replication_does_not_move_the_byte_counter`) |
| 2 / 4 | BLOQUANT — refus silencieux | **Déjà corrigé** avant la fin de la review (§3) : `warn!` sur les deux chemins de refus. La lecture « viole LOCKED #1 » est excessive : le backup est un **filet**, pas la livraison ; l'émetteur apprend la vérité par l'absence d'ACK. Rien ne dit « envoyé ✓ » sur la foi du backup. |
| 3 | MAJEUR — `make_room_for` refuse après avoir évincé | **FAUX** — erreur de calcul de l'agent : il écrit « `0 == 1` → true », or c'est faux, donc la fonction ne retourne pas `false` et la boucle réussit. Vérifié par un test dédié plutôt que par du raisonnement : `eviction_of_the_only_entry_makes_room_instead_of_refusing` ✅ |
| 5 | MINEUR — budget non configurable | Valide, déjà listé en §5. Non bloquant pour le parc actuel. |
| 6 | MINEUR — tests du refus manquants | **Valide** → ajout de `refusal_by_saturation_stores_nothing_and_keeps_existing` (refus par saturation, pas seulement par taille unitaire) |

Bilan : la review a produit **3 tests supplémentaires** (dont un qui réfute son propre finding),
zéro changement de logique. Un verdict « BLOQUANT » ne vaut que ce que valent ses preuves.

## §4ter ⚠️ VALIDATION TERRAIN : le correctif est NÉCESSAIRE mais NE SUFFIT PAS

Déployé sur le NAS (build 126, md5 `cbbcd429` vérifié process vs binaire). Nœud sain au
démarrage : **20 Mo, 4 pairs, 0 échec**.

Test de charge : 2 vagues de 120 messages × 1 Mo (240 Mo au total) vers un pair **hors ligne**
(`75baa468`, nœud mort), c'est-à-dire le chemin qui remplit le backup.

| Mesure | Résultat |
|---|---|
| Après vague 1 | 229 Mo |
| Après vague 2 | 32 Mo… **mais `uptime = 22 s`** |

**Le « 32 Mo » était un faux positif : le nœud avait redémarré.** `dmesg` :
`Killed process 23685 (tom-chat) anon-rss:780256kB` — **l'OOM-killer a de nouveau frappé,
avec le binaire corrigé**, et `NRestarts=1`.

**Conclusion honnête : le budget backup tient (le bug §1 est réel et corrigé), mais il n'était
pas — ou pas seulement — la cause de l'OOM.** Une seconde source de consommation, non
identifiée, porte la RSS à 780 Mo sous 240 Mo de trafic sortant vers un pair injoignable
(facteur ×3 environ). Pistes non encore vérifiées : buffers d'envoi/chunking retenus tant que
rien n'est acquitté, files de retry, structures par-message du tracker, ou fragmentation de
l'allocateur musl. **À investiguer avant de considérer le sujet clos.**

Leçon de méthode (encore) : `uptime` doit être lu à CHAQUE mesure post-charge. Une mémoire qui
« redescend » toute seule est d'abord suspecte d'un redémarrage, pas d'une éviction réussie.

## §5 Reste ouvert

- Le budget est une **constante**, pas une fraction de la RAM de l'hôte. Suffisant pour le parc
  actuel ; à rendre configurable si un hôte très contraint (Pi Zero) ou très large apparaît.
- L'éviction choisit le plus ancien (`evict_oldest`), pas le plus gros. Sous pression, évincer
  un gros message libèrerait plus vite ; non fait volontairement (garder la politique existante,
  une seule variable à la fois).
