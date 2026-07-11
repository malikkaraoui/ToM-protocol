# L1-003 — Vue signée du relais (présence scopée pour appareils faibles)

**Jalon L1-003 : §5 de l'ADR-011 (« consommation par un appareil faible »)**

Voir : [`POP-PROOF-OF-PRESENCE.md`](./POP-PROOF-OF-PRESENCE.md) §5 · [`L1-001-attestation-presence.md`](./L1-001-attestation-presence.md)

> **Statut : conception (avant implémentation). Doc à valider AVANT de coder.**
> 2026-07-11 · Écrit après fermeture des kill-shots red-team #1/#4 (build 33).

---

## 1. Le problème (ADR-011 §5, mot pour mot)

> *L'appareil faible **arrête de calculer** la présence de N pairs. Il **s'abonne** à la vue signée de son relais, **scopée à ses groupes**.*

Aujourd'hui, un appareil faible (Apple TV, ~500 Mo) calcule la présence de **tous** les pairs `Online` (jusqu'à 50 sur le LAN de test, potentiellement 1M à l'échelle). C'est exactement la charge O(N) que PoP est né pour tuer. §5 inverse : le relais — appareil fort, déjà sur le chemin — **observe** et **publie une vue signée**, l'appareil faible **s'abonne** et ne garde que : ses **propres preuves directes** + la vue du relais, bornée à « pairs de mes groupes à qui je parle ».

## 2. Ce qui existe déjà (vérifié, réutilisable — file:line)

| Brique | État | Emplacement |
|---|---|---|
| Présence dyadique signée (challenge→attestation) | ✅ livré | `state.rs:1795` `handle_presence_attestation`, `presence/mod.rs:272` `consume_and_store` |
| Hook « témoin tiers signé » | ⚠️ **défini, stubbé, inutilisé** | `presence/attestation.rs:88` `RelayProofType::ObserverSigned`, champ `observer_signature` |
| ACK signés (les preuves que la vue référencera) | ✅ livré (verrou #1) | `state.rs:835/846/871` émission, `:898` gate réception |
| Crédit relais témoin gaté | ✅ livré (FINDING #7) | `state.rs:920-926` `record_relay` si `recipient_of(msg) != from` |
| Lecture présence appareil faible (à remplacer) | présent | `topology.online_count()`, `presence.accepted_count()` |

**Conséquence :** L1-003 n'invente PAS la crypto ni le transport. Il branche le chemin `ObserverSigned` déjà anticipé et ajoute un nouveau type wire de **vue agrégée**.

## 3. Le nouveau type wire

```
RelayPresenceView {
    witness_id:  NodeId,          // le relais qui signe cette vue
    epoch_ms:    u64,             // horloge du témoin à l'émission
    scope:       GroupId | Peers, // groupes/pairs couverts (borne la taille)
    present: [ PresenceEntry ],   // les pairs vus vivants par ce témoin
    signature:  [u8; 64],         // sig Ed25519 du témoin sur (tout sauf signature)
}

PresenceEntry {
    peer_id:    NodeId,
    proof_ref:  MessageId,   // référence à un ACK signé RÉEL (verrou #1) observé par le témoin
    proof_type: RelayForwarded | RecipientReceived,
    seen_at_ms: u64,         // quand le témoin a constaté la preuve
}
```

Règles :
- `proof_ref` DOIT pointer vers un ACK signé que le témoin a réellement relayé/observé (pas une déclaration). Le témoin ne met dans `present` que ce qu'il a **constaté de première main** — jamais du ouï-dire (ADR-011 §2 : « vu-par-X », pas vérité globale).
- Signé du témoin → l'origine est prouvée. Mais la signature prouve QUI parle, pas que le contenu est VRAI (c'est l'objet du quorum, §4).
- Éphémère (TTL aligné sur `PRESENCE_TTL_MS` = 30s), jamais persisté (LOCKED #2).

## 4. Quorum de témoins — NON-NÉGOCIABLE dès le départ (kill-shot #3)

Le red-team a établi : `select_path` (`relay.rs`) rend UN relais → un relais Sybil unique = **toute la vue de présence** d'une victime faible (eclipse). Donc L1-003 **ne doit jamais** faire confiance à une vue de témoin unique.

**Règle :** un pair n'est considéré `Online` par un appareil faible que si **≥ N témoins distincts** (parmi ses relais, pluriels par design) publient une `PresenceEntry` concordante (même `peer_id`, `proof_ref` vérifiable, fenêtre de fraîcheur commune). Un seul témoin = `Known` au mieux, jamais `Online`. **N est dynamique** (D1) : `required_witnesses(density, activity)` bornée `[2, 4]`, plancher dur 2.

Corollaire : l'appareil faible doit s'abonner à **≥ 2 relais** (déjà le cas potentiel — relais pluriels, remplaçables). Si un seul relais disponible → présence dégradée en `Known`, pas de faux `Online`. C'est un choix conscient : mieux vaut sous-estimer la présence que se faire eclipser.

## 5. Décisions — TRANCHÉES (Malik, 2026-07-11)

**D1 — Quorum DYNAMIQUE, pas figé. ✅ Tranché : démarrer à 2, monter 3 puis 4 selon densité/activité.**
Le quorum n'est PAS une constante mais une **fonction de la densité et de l'activité du réseau** : plus le réseau est dense/actif, plus le quorum monte (2→3→4) pour augmenter puissance, résilience et sécurité. Sur petit réseau (peu de relais dispo) il reste à 2 (disponibilité). Implémentation : `required_witnesses(density, activity) -> usize` bornée `[2, 4]`, pas un `const`. Plancher dur = 2 (jamais de témoin unique, kill-shot #3). À calibrer avec les données flotte réelles.

**D2 — Push périodique par le relais. ✅ Tranché : l'appareil fort paie toujours.**
Logique du réseau = exploiter au mieux la **puissance de bord** (temps, calcul, bande passante). Le relais (fort) pousse sa vue au tick présence (30s), l'appareil faible consomme. **Vision long terme (notée, hors périmètre L1-003)** : à terme le nœud **déclare ses capacités réelles** (stockage dispo, CPU, connexion : fiabilité/débit/latence, historique) et le réseau route la charge en fonction — le « qui paie » devient dérivé des capacités déclarées+constatées, pas d'un rôle figé. L1-003 pose la première pierre (relais = payeur), l'auto-déclaration de capacités est un chantier ultérieur.

**D3 — Abonnement explicite. ✅ Tranché : on fait simple.**
L'appareil faible envoie sa liste de pairs-d'intérêt au relais (`scope`). Pas de fuite NOUVELLE : le relais est déjà sur le chemin de routage, il voit déjà le `from`/`to` des enveloppes qu'il relaie (contenu chiffré E2E, adresses non). Donc il connaît déjà le graphe.
> ⚠️ **DETTE / FAILLE NOTÉE — à corriger définitivement (durcissement futur).** Le fait que le scope soit *connu du relais*, et surtout **remis à jour et confirmé** en continu, est une surface d'attaque potentielle (un relais menteur peut mentir sélectivement sur les pairs du scope ; la mise à jour du scope est un canal à intégrité à garantir). Acceptable pour démarrer (le quorum ≥2 §4 limite déjà le mensonge d'un relais isolé), mais **à remédier définitivement** : piste = scope à divulgation minimale (le relais rapporte sur le groupe sans énumérer les individus / filtre de Bloom / preuve que le scope n'a pas été altéré). NE PAS clore L1-003 en prétendant cette faille fermée.

## 6. Ce que L1-003 NE fait PAS (bornes explicites)

- Ne touche PAS `online_relays()` / élection de hub (reste strict Online local).
- Ne remplace PAS la présence dyadique directe (l'appareil faible garde ses **propres** preuves de première main — la vue relais s'y **ajoute**).
- Ne persiste rien, ne dépasse pas le TTL 30s.
- Ne fait PAS confiance à un témoin unique (§4).
- Ne résout PAS le subset-grinding de l'agrégateur (OPEN, différé L1-002 — ne pas le prétendre).

## 7. Plan d'implémentation (après validation de ce doc)

1. Type wire `RelayPresenceView` + `PresenceEntry` (`presence/relay_view.rs`), sérialisation MessagePack, tests roundtrip.
2. Côté témoin (relais) : construire la vue depuis les ACK observés (réutilise le crédit `record_relay` gaté), signer, publier selon D2.
3. Côté appareil faible : consommer, vérifier signature témoin + `proof_ref`, appliquer le quorum §4, promouvoir `Known → Online` seulement si quorum atteint.
4. Anti-abus : borne mémoire globale (pattern 347421b), cap par témoin, purge TTL.
5. Tests adversariaux : témoin unique menteur (rejeté), 2 témoins complices (D1), `proof_ref` forgé (rejeté car ACK invérifiable), flood de vues (borné).
6. Validation flotte réelle (iPad/Apple TV/NAS) — l'appareil faible = Apple TV, le témoin = NAS.

## 8. Cohérence avec les 7 décisions LOCKED

#1 livraison=ACK (la vue référence des ACK signés) · #2 TTL (30s, éphémère) · #3 L1 rapporte, n'arbitre pas (le témoin publie ce qu'il a vu, ne juge pas) · #4 réputation en fondu (présence dérivée du score, décroît) · #5 anti-spam progressif (bornes mémoire) · #6 rôles imposés (le relais est témoin parce que le réseau l'a mis sur le chemin) · #7 fondation universelle.
