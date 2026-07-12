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

## 9. Addendum — Durcissements post-étape-3 (capacité + crypto spot-check)

Après livraison des étapes 1-3 (types wire + câblage témoin + quorum basique), deux durcissements ont été implémentés pour L1-003 :

### 9.1. Cap par témoin (anti-abus mémoire Sybil)

**Problème :** `QuorumAggregator` et `WitnessLog` avaient un cap global (`MAX_TRACKED_PEERS` = 256) mais AUCUN cap par témoin. Un relais Sybil pouvait, en une seule vue, remplir la table entière de 256 pairs fictifs et évincer les attestations légitimes d'autres témoins.

**Solution implémentée :**
- Ajout d'une constante `MAX_PEERS_PER_WITNESS = MAX_TRACKED_PEERS / 4` (64 pairs max par témoin).
- Tracking par-témoin : `HashMap<NodeId, HashSet<NodeId>>` dans `QuorumAggregator`.
- Éviction stratifiée : quand un témoin dépasse son quota, ses **propres** pairs les plus anciens sont évincés EN PRIORITÉ (pas ceux d'autres témoins).
- Garantie : aucun témoin Sybil ne peut évincer les attestations valides d'autres témoins distincts.

**Impact mémoire :** ~+64 octets par témoins distinct (HashSet<NodeId>). Acceptable : la plupart des appareils faibles n'ont ≤ 4 relais, soit <1 KB.

**Tests :** témoin A sature son quota (64 peers), témoin B légal atteste un peer → le peer de B survit, seuls les vieux pairs de A sont évincés.

### 9.2. Spot-check crypto du proof_ref (vérification real ACK, pas assertion)

**Problème :** `PresenceEntry.proof_ref` était une simple String assertée « c'est un id d'ACK réel ». Aucune vérification — un témoin pouvait prétendre avoir vu n'importe quel ACK sans preuve. La doc disait « consumer verifies this points to genuine evidence », mais c'était stubé.

**Solution implémentée :**

1. **Ajout champ `ack_proof: Vec<u8>`** à `PresenceEntry` (avec `#[serde(with = "serde_bytes")]` pour éviter l'inflation MessagePack ×8).
   - Stocke les bytes bruts de l'`Envelope` ACK signé que le témoin a réellement relayé.
   - Réutilise les primitives existantes (`Envelope::to_bytes()`, `Envelope::verify_signature()`, `AckPayload::from_bytes()`) — zéro duplication de crypto.

2. **Point d'origine (witness side)** : dans `state.rs` lignes ~898-909 (relais qui forward un ACK signé), `envelope.to_bytes()` est capture et passé à `witness_log.record()`.

3. **Vérification (consumer side)** : dans `handle_relay_presence_view()`, **avant** d'enregistrer dans `quorum.record()`, chaque entrée passe par un gate cryprographique :
   - `Envelope::from_bytes(&entry.ack_proof)` parse correctement
   - `envelope.msg_type == MessageType::Ack`
   - `envelope.from == entry.peer_id` (le signataire EST le pair attesté)
   - `envelope.verify_signature().is_ok()` (Ed25519 valide)
   - `AckPayload::from_bytes()` décode et `.original_message_id == entry.proof_ref`, `.ack_type == entry.proof_type` (cohérence)
   - `now - envelope.timestamp < PRESENCE_TTL_MS` (fraîcheur, empêche relay d'un vieil ACK réel avec timestamp frais menti)
   - **Défaillance gracieuse :** une entrée qui échoue toute vérification est **silencieusement ignorée** ; les autres entrées de la même vue sont traitées normalement.

**Garantie :** un ACK forgé, périmé, malformé, ou incohérent ne compte PAS pour le quorum.

**Coût :**
- Taille : `ack_proof` (~128 B/ACK) → vue de 256 pairs ~32 KB bruts. Avec compression MessagePack, ~10-15 KB/view (acceptable, un appareil faible reçoit ≤ 1 view/30s par témoin).
- CPU : désérialisation + 1 vérif Ed25519/entrée. Sur 256 entrées, ~256 vérifs = ~10-30 ms par appareil faible (acceptable pour un tick 30s).

**Non fermé :** cette vérification empêche un ACK **forgé de novo**, mais pas un ACK réel d'un pair MORT que le témoin rejoue. Mitigation : TTL 30s + le pair mort ne confirmera pas dans le quorum par d'autres témoins frais.

### 9.3. Tests adversariaux ajoutés

- ✅ Témoin Sybil flood : 65+ peers, max 64 admis, seul le plus ancien du Sybil évincé.
- ✅ Plusieurs témoins complices : quorum stable même si 1 témoin sature (ne starve pas les autres).
- ✅ ACK forgé/signature invalide dans `ack_proof` → entrée rejetée, traitement des autres entrées inchangé.
- ✅ ACK périmé (`timestamp` > 30s) → entrée rejetée.
- ✅ Incohérence `proof_ref`/`proof_type` vs contenu réel de `ack_proof` → entrée rejetée.
- ✅ Vue mixte valides+invalides → seules les valides comptent pour quorum, pas de crash.
