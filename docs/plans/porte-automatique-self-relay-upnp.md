# Porte d'entrée automatique — mapping UPnP du relais embarqué

> ⚠️ Ne pas confondre avec `docs/plans/R13-offline-delivery.md` (autre chantier,
> même numéro "R13" réutilisé par le roadmap arbitré 2026-07-03 pour un sujet
> différent — collision de nommage historique, pas la même feature).

## Contexte

Roadmap (`vault/40-roadmap.md`, "R13 — Porte d'entrée automatique") : constat
qu'il n'existe qu'UNE porte publique fixe (le NAS perso) = SPOF de fait.
Objectif : que tout nœud installé (Mac, iPad, Pi…) devienne une porte
complète, publiée et recrutée par le gate ADR-010, sans manip manuelle.

**Étape 1 vérifiée empiriquement le 2026-07-12** (voir `vault/40-roadmap.md`) :
le mapping UPnP fonctionne déjà pour le port QUIC principal, via
`portmapper::Client::procure_mapping()` (`crates/tom-connect/src/socket.rs:610`).
Preuve réelle sur la Freebox de l'utilisatrice (log `RUST_LOG=debug`) :
```
getting a port mapping for 192.168.0.70:49850 -> None
new port mapping Some(Upnp(Mapping {
    gateway: http://192.168.0.254:5678/control/wan_ip_connection,
    external_ip: 82.67.95.8,
    external_port: 58870,
}))
```
La Freebox répond à UPnP IGD par défaut. Ce mécanisme n'est PAS câblé pour le
port du relais embarqué — c'est l'objet de ce doc.

## Ce qui manque (étape 2)

Le relais embarqué (`crates/tom-protocol/src/runtime/embedded_relay.rs`,
`LocalEmbeddedRelayState`) écoute sur son propre `bind_addr` (port distinct du
socket QUIC principal). Aujourd'hui :
- Aucun appel à `portmapper::Client::procure_mapping()` n'existe pour ce port.
- `relay_url_is_globally_reachable` (`crates/tom-protocol/src/runtime/loop.rs`
  ou `state.rs`, cf. ADR-010) décide si l'URL du relais embarqué est publiée
  au gossip global — actuellement basé sur l'adresse locale (privée/loopback
  filtrée), PAS sur un mapping UPnP obtenu.

Sans ce câblage, un nœud ne peut devenir une porte complète que si son port de
relais est déjà joignable publiquement par un autre moyen (port forwarding
manuel côté box) — exactement la friction que R13 doit éliminer.

## Design proposé

### 1. Obtenir un mapping pour le port du relais

`tom-connect::MagicSock` possède déjà UN `portmapper::Client` lié à `Actor`
(le socket QUIC). Deux options :

- **(a) Deuxième `portmapper::Client` dédié**, instancié côté
  `tom-protocol::runtime::embedded_relay` au démarrage du relais embarqué,
  avec son propre `update_local_port(relay_bind_port)` +
  `procure_mapping()` + `watch_external_address()`.
- **(b) Réutiliser le client existant** de MagicSock avec un second mapping
  (si l'API `portmapper` v0.13 le permet — **à vérifier en tout début
  d'implémentation**, pas supposé ici : inspecter si `Client` supporte
  plusieurs mappings concurrents ou est mono-port par construction).

Recommandation : **(a)**, plus simple à raisonner (le relais embarqué est déjà
un sous-système avec son propre cycle de vie dans `embedded_relay.rs`,
l'isoler évite un couplage supplémentaire avec `tom-connect::socket::Actor`
qui n'a pas vocation à connaître le port du relais).

### 2. Câbler le résultat sur la publication ADR-010

Une fois `watch_external_address()` du relais retourne `Some(Mapping { external_ip, external_port, .. })` :
- Construire l'URL publique du relais (`http://{external_ip}:{external_port}`
  ou équivalent selon le schéma d'URL relay existant).
- Faire passer cette URL (mappée) à `relay_url_is_globally_reachable` — ou
  court-circuiter ce gate quand la source de l'adresse est un mapping UPnP
  confirmé (un mapping UPnP actif EST une preuve de joignabilité globale,
  contrairement à une adresse locale non vérifiée).
- Publier au gossip global (même chemin que l'URL de relais manuelle
  existante) + republier au DHT rendez-vous (ADR-010) au même titre qu'un
  relais à IP fixe.

### 3. Dégradation propre si pas de mapping

Pas d'UPnP disponible (box sans IGD, mapping refusé, double NAT) → le relais
reste LAN-only comme aujourd'hui, aucune régression. Le mapping est un bonus
opportuniste, jamais un prérequis.

## Questions ouvertes à trancher AVANT de coder

1. ✅ **TRANCHÉE (2026-07-12)** — `portmapper::Client` (v0.13) ne supporte
   PAS plusieurs mappings simultanés : `Service::local_port` est un
   `Option<NonZeroU16>` unique (`portmapper-0.13.0/src/lib.rs:438`),
   `update_local_port` REMPLACE l'ancien port par le nouveau
   (`lib.rs:593-610`, pas un ajout). **Confirme l'option (a)** : il faut une
   deuxième instance dédiée de `portmapper::Client` pour le port du relais
   embarqué, pas de réutilisation possible du client de MagicSock.
2. Le TTL/renouvellement du mapping UPnP (bail limité côté box) est-il déjà
   géré en interne par `portmapper::Client` (renouvellement auto) ou faut-il
   un tick applicatif côté `embedded_relay.rs` ?
3. Que fait-on si le nœud est DERRIÈRE le NAS ET a un mapping UPnP actif en
   même temps (cas Mac de ce jour) ? Publier LES DEUX URLs (NAS + mapping
   propre) ou une politique de préférence ? Impact sur `RelaySelector`
   (répartition de charge, décision #5 anti-spam "sprinkler gets sprinkled").
4. Sécurité : un mapping UPnP expose le port du relais embarqué directement à
   Internet — le relais est déjà conçu stateless/pass-through (pas de
   confiance requise), mais vérifier qu'aucune surface d'attaque
   supplémentaire n'est ouverte (le relais accepte déjà des connexions
   inconnues par design — ADR-001 — donc pas de changement de modèle de
   menace attendu, à confirmer).

## Plan d'implémentation (une fois les questions ci-dessus tranchées)

1. `embedded_relay.rs` : instancier un `portmapper::Client` dédié au
   démarrage du relais, mappé sur `relay_config.bind_addr` port.
2. Watcher l'adresse externe, exposer le résultat via un nouveau
   `RuntimeEffect`/état (suivre le pattern effet pur existant).
3. Câbler sur `relay_url_is_globally_reachable` / publication ADR-010.
4. Tests : mapping obtenu → URL publiée ; pas de mapping → comportement LAN
   actuel inchangé (non-régression) ; renouvellement/expiration du mapping.
5. Validation `tom-stress` + test d'acceptation réel (étape 3 du roadmap) :
   iPhone en 5G/data ↔ maison SANS le NAS, uniquement via le Mac/iPad comme
   porte auto-mappée.

## Statut

- [x] Étape 1 (mapping QUIC principal) — vérifiée empiriquement 2026-07-12.
- [x] **Étape 2 — implémentée (2026-07-13)** — option (a) retenue (client
  `portmapper` dédié dans `EmbeddedRelayService`), câblage complet :
  `embedded_relay.rs` (client + watcher exposé) → `loop.rs` (observation dans
  le `select!` principal, 3 sites de démarrage/arrêt du relais couverts) →
  `state.rs` (`RuntimeCommand::EmbeddedRelayPortMapped` → `build_relay_publication`,
  passe par le même gate `relay_url_is_globally_reachable` que le chemin
  existant, pas de bypass). 2 tests ajoutés (`embedded_relay_port_mapped_publishes_global_url`,
  `embedded_relay_port_mapped_with_private_ip_not_published`).
  **2 bugs de robustesse trouvés en review et corrigés avant commit** (voir
  [[verify-subagent-security-shortcuts]]) :
  1. Le premier jet recréait un `watch::Receiver` À L'INTÉRIEUR du bloc async
     de la branche `select!`, DONC à chaque itération de la boucle — un
     receiver fraîchement créé ne voit que les changements FUTURS depuis sa
     création, donc un mapping obtenu entre deux itérations pouvait être
     silencieusement perdu (le nouveau receiver de l'itération suivante
     démarre déjà "à jour" par rapport à ce changement passé). Fix : watcher
     stable déclaré une fois en dehors de la boucle (`embedded_relay_mapped_watcher`),
     réutilisé via `.as_mut()`, mis à jour uniquement aux 3 points où le
     relais démarre avec succès, remis à `None` aux 2 points d'arrêt.
  2. Aux 3 points de démarrage, le watcher était assigné APRÈS `node.reprobe_relays().await`
     — fenêtre théorique où un mapping résolu pendant ce await ne serait
     capturé par aucun watcher encore vivant. Fix : assignation déplacée
     AVANT le `await`, dans les 3 sites.
  Les deux trouvés par relecture indépendante (moi-même pour #1, un
  sous-agent Relecteur pour #2, confirmé et appliqué).
- [ ] Étape 3 (test d'acceptation réel : iPhone data ↔ maison sans le NAS).
