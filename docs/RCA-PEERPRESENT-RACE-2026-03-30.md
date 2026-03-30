# RCA — PeerAnnounce 12/13 (race condition self-relay)

Date: 2026-03-30
Scope: investigation only (no fix)

## Résumé
Le publisher ne reçoit pas immédiatement `PeerPresent` quand il démarre avec relay embarqué local (`127.0.0.1:3340`), malgré une config relay correcte. La cause est une course temporelle entre le probe relay initial de l’Endpoint et le démarrage du relay embarqué.

## Séquence temporelle observée
1. `TomNode::bind()` crée l’`Endpoint` avec `configured_relays=[http://127.0.0.1:3340/]`.
2. L’Endpoint démarre l’actor avec `new_re_stun_timer(false)` -> probe netcheck immédiat.
3. Le probe vise `http://127.0.0.1:3340/` mais le relay embarqué n’est pas encore démarré -> échec, `preferred_relay=None`.
4. `bind()` retourne.
5. `ProtocolRuntime::spawn()` démarre la loop runtime.
6. Le relay embarqué est lancé ensuite (`embedded_relay.start()`), donc après le probe initial.
7. Le re-probe suivant est planifié à ~20–26s; avant cela, pas de `home is now relay`, donc pas de connexion client relay côté publisher.

## Preuves (code)
- `crates/tom-connect/src/magicsock/socket.rs:871`
  - `new_re_stun_timer(false)` -> probe immédiat.
- `crates/tom-connect/src/magicsock/socket.rs:1501`
  - retry re-stun randomisé ~20–26s.
- `crates/tom-protocol/src/runtime/loop.rs:121`
  - `embedded_relay.start()` est exécuté après `TomNode::bind()`.
- `crates/tom-relay/src/server/clients.rs::register()`
  - `PeerPresent` est échangé entre clients effectivement connectés au relay.

## Corrélation logs
- Publisher: `configured_relays=[http://127.0.0.1:3340/]`, mais pas de `home is now relay` dans la fenêtre d’observation.
- Observer: même config, puis `home is now relay` -> `Adding relay connection` -> `connected to relay`.
- Interprétation: l’observer démarre après que le relay du publisher est déjà UP, son premier probe réussit.

## Impact
- Fenêtre muette initiale côté publisher (jusqu’au prochain re-stun).
- Pendant cette fenêtre: pas de connexion client relay du publisher, découverte incomplète, scénario 3 commandes pouvant rater (12/13).

## Décision de cette investigation
Hypothèse 1: **confirmée**.
Cause principale candidate: race de démarrage entre probe relay initial et démarrage du relay embarqué.

## Prochain point (sans implémentation ici)
Valider la stratégie de mitigation en design:
- soit démarrer relay embarqué avant `TomNode::bind()`,
- soit forcer un re-stun/re-probe juste après `embedded_relay.start()`.

Aucune correction de code n’a été appliquée dans cette note.
