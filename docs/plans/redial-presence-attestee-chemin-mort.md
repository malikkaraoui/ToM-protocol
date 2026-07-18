# Re-dial sur « présence attestée + chemin local mort » — doc de conception

> Statut : **PROPOSITION — en attente de validation utilisateur avant tout code.**
> Origine : finding ① de la campagne du 18/07 (T1), re-vérifié sur pièces le jour même
> (contre-enquête : les findings ② et ③ de la même campagne étaient faux — celui-ci tient).
> Garde-fous hérités : design-first (mémoire `design-doc-before-coding-protocol-features`),
> instrument avant théorie (17/07), canari avant flotte.

## 1. Constat vérifié (pièces, 18/07 09:34:50 → 09:36:34)

Scénario T1 : iPhone-Malik (`80eb9196`) bascule WiFi → data cellulaire, trafic continu.

| Fait | Preuve (collecteur, fenêtre bornée par lignes 735000-765718) |
|---|---|
| L'ATV oscille `🟢 Pair en ligne ↔ ⚠️ Pair vieilli` sur l'iPhone pendant ~90 s | lignes 09:34:56 → 09:36:10, up=205-280 s (jour confirmé par uptime) |
| Un seul échec applicatif | `⏱️ Livraison échouée` + `❌ échec → iPhone` à 09:35:06 |
| L'ATV **tente** des dials, ils n'aboutissent pas | 2 × `Connexion au pair 80eb9196…` dans la fenêtre, aucun rétablissement |
| Pendant ce temps les pairs à chemin chaud livrent | Mac 3 ×, iPad 4 × `délivré → iPhone · 80eb9196` dans la même fenêtre |
| Aucune bascule relay tentée côté ATV | 0 ligne `→ RELAY` / flip de path dans la fenêtre |
| Retour WiFi → tout reconverge | 09:36:34, re-upgrade DIRECT en rafale |

Lecture : le réseau **sait** que le pair est vivant (les témoins l'atteignent, le mesh
propage cette évidence), le nœud **sait** que son propre chemin est mort (échec + vieillissement),
et il re-diale les **mêmes adresses mortes** (la LAN IPv4 que le téléphone n'a plus)
au lieu d'aller chercher l'adressage frais que les témoins, la DHT ou un relay connu possèdent.

## 2. Mécanique actuelle (ancres vérifiées dans le code, 2026-07-18)

- **Évidence « vivant » (mesh)** : le tracker heartbeat est nourri par
  `record_heartbeat_with_source` depuis 3 chemins — PeerAnnounce gossip
  (`runtime/state.rs:1551`, source `Direct`), `RuntimeCommand::UpsertPeer`
  (`state.rs:2600`), résultat de lookup DHT (`state.rs:2851`, source `Dht`).
  Les transitions Alive↔Stale sortent de `discovery/heartbeat.rs` (`check_all` :142,
  seuils :45) → `DiscoveryEvent::PeerStale/PeerOnline` → événements protocole
  (`state.rs:476/498`, aussi :2138/:2251). C'est l'oscillation observée.
- **Évidence « présence » (PoP L1-003)** : sous-système `presence/` distinct
  (attestations signées de témoins, `witness_log.record` `state.rs:984`,
  agrégation `presence/aggregator.rs`). Fraîcheur disponible mais **pas** consommée
  par la logique de reconnexion.
- **Échec applicatif** : `tracker.mark_failed` (`state.rs:440`).
- **Reconnexion existante** : tick 15 s (`runtime/loop.rs:~755`) qui re-diale les pairs
  de la topologie **avec leurs adresses connues** (donc mortes dans ce scénario) ;
  `NeighborDown` gossip (`loop.rs:613`) ; récupération d'isolement (`on_isolated`) —
  aucune n'est déclenchée par le désaccord « attesté-vivant ∧ chemin-mort », et aucune
  ne rafraîchit l'adressage avant de dialer.

**Le trou** : le signal composite `(évidence vivant fraîche) ∧ (chemin local mort)` n'a
aucun consommateur. C'est un signal riche : il dit précisément « ce pair est joignable,
mais pas par les adresses que TU connais ».

## 3. Non-buts (décisions LOCKED respectées)

- **Ne PAS toucher au quorum/attestation L1-003** (ni format wire, ni seuils) —
  décision #3 : L1 ancre, n'arbitre pas. Le quorum reste un capteur, jamais un acteur.
- **Pas de dial-storm** — décision #5 (charge progressive) : throttle strict, jamais
  de re-dial en boucle sur ce déclencheur.
- **Invisible** — décision #6 : mécanisme protocolaire, zéro état visible utilisateur.
- **Pas de wire change** : la variante « embarquer l'addr_info du pair dans
  l'attestation » (le témoin connaît les adresses fraîches) est **rejetée pour l'instant**
  — elle modifie le payload signé d'attestation (LOCKED-adjacent). Réévaluable plus tard.

## 3bis. Prérequis sécurité P0 (sortis de la review-oracle du 18/07)

- **P0-1 — Binding évidence ⟷ identité authentifiée (constat vérifié sur pièces)** :
  `PeerAnnounce` ne porte AUCUNE signature propre (`discovery/types.rs:37-51` — champs
  node_id/username/app_build/roles/encryption_key/timestamp, rien d'autre) et le handler
  QUIC direct `handle_peer_announce` (`state.rs:1543-1563`) crédite heartbeat + topologie
  `Online` pour `announce.node_id` sur simple validité de timestamp — sans lier ce
  node_id à l'identité de l'émetteur. Contraste : `RoleAnnounce` (`state.rs:757`) et
  `RelayReadyAnnounce` (`state.rs:824`) vérifient leur signature. La voie GOSSIP est
  déjà fermée par ADR-011 (`mark_known` sans crédit) ; la voie QUIC directe ne l'est pas.
  **Avant M1 (étape 0)** : trancher l'exploitabilité de bout en bout (l'enveloppe
  non signée est-elle rejetée en amont ? `envelope.from` est-il lié à l'identité QUIC
  de la connexion ?) puis imposer `announce.node_id == envelope.from` (ou signature
  propre, pattern RoleAnnounce). Sans ce binding, l'évidence « vivant » consommée par
  M1 est forgeable — le redial reste borné par le throttle, mais username/Online
  spoofables est un trou pré-existant à fermer de toute façon.
- **P0-2 — Adresses candidates sans confiance (invariant existant, à réaffirmer)** :
  un dial n'accorde AUCUNE confiance protocolaire ; l'identité du pair est prouvée au
  handshake QUIC (clé Ed25519 = adresse réseau, ADR-005). Un empoisonnement DHT/annuaire
  coûte au pire un dial gaspillé + une fuite de méta-données (timing/IP source) —
  pré-existant pour bootstrap/tick 15 s, pas une régression M1 (cf. Known Limitations #6).

## 4. Conception proposée — M1 « redial ciblé sur désaccord »

### Déclencheur (dans `RuntimeState`, pur, testable)
Transition détectée quand, pour un pair P :
- événement `PeerStale { P }` émis **ou** `mark_failed` d'un message vers P, **et**
- une évidence « vivant » fraîche existe pour P (heartbeat sourcé mesh/DHT **ou**
  attestation présence < fenêtre F ; F à fixer à l'étape 0 en lisant les seuils réels
  de `heartbeat.rs:45`).

### Effet (nouveau `RuntimeEffect`, exécuté par la boucle)
`RedialStalePath { peer }` →
1. **Rafraîchir l'adressage avant de dialer** : lookup DHT ciblé du pair (chemin
   existant `state.rs:2851` réutilisé) + re-seed de la route relay connue
   (même mécanique que le seed bootstrap, source `"presence-redial"`).
2. **Dial 1 ×** via le chemin normal (dial-hors-verrou du 17/07 inchangé).

### Throttle (décision #5)
- 1 redial max par pair par fenêtre de 60 s — **60 s n'est pas une constante neuve** :
  c'est la période du cycle rendezvous existant (`runtime/loop.rs:101`), le rythme
  auquel l'adressage frais peut de toute façon se renouveler,
- reset du throttle sur succès (trafic entrant direct de P),
- cap global simultané (ex. 3 redials en vol) — classe « budget per-identité + cap
  global » (leçon red-team `tom-redteam-loop-2026-07-06`) ; au-delà du cap :
  **éviction FIFO documentée** (pas de file d'attente — un pair non servi sera repris
  par sa prochaine transition ou par le tick 15 s),
- **flap de masse** (N pairs stale simultanés, essaim Sybil attesté) : le cap global
  protège l'appareil ; la convergence de masse reste portée par le tick 15 s existant
  — M1 est un accélérateur ciblé, jamais l'unique chemin de récupération (zéro
  régression vs aujourd'hui si le cap est atteint).

### Ce que M1 ne fait PAS
Pas de nouveau message réseau, pas de nouveau rôle, pas d'état persisté : uniquement
un consommateur local d'évidences déjà présentes + les chemins de seed/dial existants.

## 5. Articulation avec le « chantier #4 auto-guérison Swift » (handoff nuit2)

**Tranché : UN mécanisme, protocolaire (celui-ci).** Le filet Swift envisagé
(forceReset si envois en échec + découverte > X min) traiterait le même symptôme au
mauvais étage (marteau : reset complet) et violerait l'esprit #6 (logique réseau dans
l'UI). Il est **abandonné comme mécanisme dédié** ; le watchdog Swift existant reste,
inchangé, en dernier recours générique. Si M1 est validé et livré, le cas « pair
joignable mais chemin mort » ne doit plus jamais atteindre le watchdog.

## 6. Recette et validation (dans l'ordre, canari avant flotte)

1. **Étape 0 — instrument avant théorie** : identifier l'émetteur EXACT du
   `⚠️ Pair vieilli` du scénario T1 (heartbeat `check_all` vs `state.rs:2138`) par un
   test unitaire reproduisant la séquence d'évidences ; ajuster le déclencheur si besoin.
2. **Unit (déterministes)** : transition (attesté ∧ stale) → 1 effet redial, pas 2 ;
   throttle 60 s ; reset sur succès ; cap global ; aucune émission si évidence périmée.
   Cas limites imposés par la review-oracle :
   - horloge qui recule pendant le throttle → refus silencieux, jamais de panique
     (arithmétique `saturating_sub` comme le throttle role-announce `state.rs:751` ;
     rappel orage intervalles #28 : comportement Skip, pas Burst) ;
   - `PeerStale` ET `mark_failed` dans le même tick → **1 seul** redial en vol ;
   - évidence qui périme PENDANT la fenêtre de throttle → le redial suivant est
     refusé (l'évidence se re-prouve, elle ne se mémorise pas) ;
   - pair revenu (trafic entrant) pendant un redial en vol → reset du throttle, le
     dial en vol se termine sans effet de bord (idempotent) ;
   - redial pendant shutdown → effet ignoré proprement ;
   - cap atteint (10 pairs stale, cap 3) → 3 dials exactement, éviction FIFO, zéro file ;
   - DHT en unit : stub/`SharedDht` mocké (le lookup réel `state.rs:2851` exige un
     réseau vivant → réservé au labo étape 4, jamais dans les units).
3. **Métriques de recette** (exposées :9091, comme les compteurs existants) :
   `redial_presence_triggered`, `redial_presence_success`, latence trou-de-livraison.
4. **Repro labo** : flotte locale hermétique, couper le chemin d'UN nœud (iptables/toggle
   interface) en gardant les autres chauds → cible : re-livraison **< 15 s** (vs 90 s T1).
5. **Canari** : 1 appareil (ATV, la victime historique), rejouer T1 réel
   (iPhone → data 60 s), comparer les métriques AVANT/APRÈS sur le même scénario.
6. **Flotte** entière seulement après canari vert.

## 7. Questions — état après review-oracle (18/07)

1. **Tranché (review)** : F = seuil stale existant de `heartbeat.rs:45` — zéro
   constante neuve. Une évidence plus vieille que le seuil stale n'est pas « fraîche ».
2. **Proposé, à valider** : déclencheur = `PeerStale` ET `mark_failed` (même throttle).
   Garde-fou review : si le labo (étape 4) montre des redials vers des pairs réellement
   morts via `mark_failed` + évidence limite, resserrer F à ~10 s pour ce seul
   déclencheur — mesure d'abord, pas de constante préventive.
3. **Ouvert (pour Malik)** : cible de recette < 15 s acceptable, ou viser < 10 s
   (aligné sur la reconvergence post-SIGKILL de 8 s mesurée en campagne) ?
4. **Ouvert (pour Malik, P0-1)** : le binding announce ⟷ émetteur (§3bis) se corrige-t-il
   DANS le chantier M1 (même livraison) ou en chantier sécurité séparé AVANT M1 ?
