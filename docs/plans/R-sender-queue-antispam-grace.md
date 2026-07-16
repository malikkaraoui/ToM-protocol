# Conception — File de sortie maîtrisée + grâce reconnexion (chantier #31)

> Statut : conception (doc-avant-code, LOCKED-adjacent : décision #5 antispam, décision #1 livraison, ADR-009 backup). Rédigé 2026-07-16.

## Problème (observé terrain)

Au join d'un pair qui redémarre (ex iPhone WiFi), le collecteur montre l'iPhone émettant **des dizaines de « 🚨 Pair ralenti MacBook Pro (antispam) » dans la même seconde**. Cause : le récepteur (iPhone) a **perdu les scores de contribution** de ses pairs au restart → le Mac redevient « étranger score 0 » plafonné à **30 msg/s** (`roles/antispam.rs`, token-bucket). L'expéditeur (Mac) lui envoie alors une **rafale de rattrapage** (probes présence, pings, rejoin, messages en file) qui dépasse 30/s → throttle → fausse alerte sur un pair ami, et **ralentit le join** (le trafic qui établit la connexion est rate-limité).

Le join aboutit quand même (~77s), donc c'est du **bruit + délai**, pas un blocage. Mais deux vrais défauts : fausse alerte spammy, et antispam qui freine une reconnexion légitime.

## Décisions LOCKED respectées

- **#5 antispam « sprinkler gets sprinkled »** : on ne bannit rien, on ne fait que rendre l'expéditeur plus poli + accorder une grâce aux pairs connus. Toujours progressif.
- **#1 livraison (ACK)** : ne rien perdre — dédupliquer/pacer/différer, jamais dropper un message non livré non périmé.
- **ADR-009 backup TTL 24h** : les messages backup gardent leur TTL ; on change QUAND on les envoie (attendre le direct), pas leur durée de vie.

## Tranche 1 — Récepteur : grâce + dédup alerte (bounded, immédiat)

1. **Grâce reconnexion** : un pair **connu** (présent en topology / avec historique de contribution) qui se reconnecte ne doit PAS être traité en « étranger 30/s ». Options : (a) semer un score minimal non nul pour un pair déjà vu, (b) fenêtre de grâce (burst capacity relevée) au premier contact après un gap. But : absorber la rafale de rattrapage légitime sans throttle.
2. **Dédup de l'alerte** `SenderThrottled` : émettre **au plus 1 fois par pair par fenêtre** (ex 10 s), pas une par message throttlé. Fini le spam de 30 lignes (principe « info persistante ≠ spam d'activité »).

## Tranche 2 — Expéditeur : file de sortie maîtrisée (le fix racine)

L'expéditeur (le Mac) doit **posséder sa queue de sortie** et ne pas la vider en rafale :
1. **Dédupliquer** la queue (pas deux fois le même message/id).
2. **Drop les périmés** (TTL dépassé → inutile de les envoyer, cohérent ADR-009).
3. **Pacer** l'envoi vers un pair sous le seuil antispam (≤ ~30 msg/s / pair, avec un peu de marge), plutôt qu'un burst.
4. **Backup → attendre le direct** : les messages backup destinés à un pair ne partent QUE sur une connexion **DIRECTE** établie (pas un blast via relais dès la découverte). Ça évite le pattern spam ET économise le relais.

## Red-team abus

- La grâce récepteur ne doit s'appliquer qu'à un pair **authentifié/connu** (node_id vu + signature valide), jamais à un flot d'identités fraîches (sinon on rouvre le sybil que le cap global `GLOBAL_STRANGER_RATE` ferme). Un attaquant ne peut pas « se faire passer pour connu » sans historique réel.
- Le pacing/dédup expéditeur est purement local (aucune décision de trust), zéro surface d'abus.

## Portée

- Tranche 1 : `crates/tom-protocol/src/roles/antispam.rs` (grâce/score connu) + le site d'émission de `SenderThrottled` (dédup) — trouver via grep `SenderThrottled`.
- Tranche 2 : le chemin d'envoi sortant (executor/runtime) + le backup coordinator (`backup/`) : dédup + drop-TTL + pacing + gate direct-vs-relais pour le backup.

## Validation

Unit tests antispam (grâce connu vs étranger, dédup alerte) + tests file de sortie (dédup, drop périmé, pacing). **Global** : re-lancer `presence-attack` + `chaos-monkey` (ne pas régresser les défenses) + observer terrain (plus de rafale « Pair ralenti » au join). Cross-crate → gate workspace + check-ffi si event FFI touché.
