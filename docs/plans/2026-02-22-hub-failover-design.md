# Hub Failover — Design Document

**Date**: 2026-02-22
**Scope**: Jour 5 du plan hebdomadaire

## Objectif

Quand le hub d'un groupe tombe, le protocole doit automatiquement migrer le rôle hub vers un autre noeud, sans intervention humaine, en moins de 10 secondes.

## Principes directeurs

1. **Le hub est un pass-through (bus)** — il ne stocke aucun historique de messages. Seules la liste des membres et la config du groupe sont répliquées.
2. **Réplication virus** — le rôle hub se réplique en cascade sur le réseau pour survivre. Toujours au moins un backup prêt.
3. **Le réseau travaille en amont** — la détection ne dépend pas de l'activité utilisateur. Le shadow surveille activement dès le boot.
4. **Self-election déterministe** — tous les noeuds calculent le même résultat, aucune coordination nécessaire.

## Architecture : Chaîne de réplication

```
Primary ──sync──→ Shadow ──(identifie)──→ Candidate
   │                │                        │
   │ fan-out        │ standby                │ standby
   │ ping/pong      │ watchdog actif         │ prêt à devenir shadow
   │                │                        │
   ▼                ▼                        ▼
 [MORT]         PROMOTION              PROMOTION
             → Primary               → Shadow
                                    elect new Candidate
```

### Rôles

| Rôle | Responsabilité | État répliqué |
|------|---------------|---------------|
| **Primary** | Fan-out des messages, réponse aux pings du shadow | Membre list, config, candidate_id |
| **Shadow** | Watchdog actif (ping primary toutes les 3s), copie synchronisée | Membre list, config, candidate_id |
| **Candidate** | Identifié, prêt à devenir shadow si besoin | Aucun (connaît juste son rôle) |

### Ce que le shadow reçoit (HubShadowSync)

Le primary envoie au shadow à chaque changement d'état :

```rust
HubShadowSync {
    group_id: GroupId,
    members: Vec<GroupMember>,    // liste complète
    candidate_id: Option<NodeId>, // prochain shadow si shadow meurt
    config_version: u64,          // version monotone
}
```

Payload léger — quelques centaines d'octets pour un groupe de 50 membres.

## Détection : Watchdog actif + alertes membres

### Signal 1 : Shadow ping actif (toujours, H24)

```
Toutes les 3s:
  Shadow ──HubPing──→ Primary
  Shadow ←─HubPong──  Primary    ✓ vivant

Si 2 pings consécutifs sans pong (timeout 2s chacun):
  → PROMOTION (~8s worst case)
```

Le réseau clarifie la situation **en amont**, indépendamment de toute activité utilisateur. Dès l'ouverture du process, dès la connexion internet, le shadow travaille.

### Signal 2 : Alerte membre (accélérateur)

```
Alice envoie un message → hub ne répond pas après 3s
Alice ──HubUnreachable { group_id }──→ Shadow
Shadow reçoit 1 alerte + 1 ping raté → PROMOTION immédiate
```

### Résultat combiné

| Situation | Détection | Raison |
|-----------|-----------|--------|
| Groupe actif | **~3s** | Premier message échoué déclenche l'alerte |
| Groupe dormant | **~8s** | 2 pings ratés du watchdog |
| Cas mixte | **3-8s** | Premier signal qui arrive gagne |

## Cycle de vie de la chaîne

### Création (quand un groupe est créé)

1. Le primary (hub) exécute `elect_hub()` sur les membres pour identifier le **shadow**
2. Le primary envoie `HubShadowSync` au shadow élu
3. Le shadow exécute `elect_hub()` (excluant primary + shadow) pour identifier le **candidate**
4. Le shadow notifie le candidate de son rôle via `CandidateAssigned`

### Fonctionnement normal

```
Primary ──HubShadowSync──→ Shadow      (à chaque join/leave/config change)
Shadow  ──HubPing──→ Primary            (toutes les 3s)
Primary ──HubPong──→ Shadow             (réponse immédiate)
```

### Primary meurt

1. Shadow détecte (2 pings ratés ou alerte membre)
2. Shadow **se promeut Primary** (import_group avec l'état synchronisé)
3. Shadow envoie `HubMigration { new_hub_id: self }` à tous les membres
4. Ancien Candidate **se promeut Shadow**
5. Nouveau Shadow élit un nouveau Candidate
6. Chaîne restaurée en ~1s après détection

### Shadow meurt

1. Primary détecte (HubShadowSync non-ACK, ou heartbeat tracker)
2. Candidate **se promeut Shadow**
3. Primary envoie `HubShadowSync` au nouveau Shadow
4. Nouveau Shadow élit un nouveau Candidate
5. Chaîne restaurée

### Candidate meurt

1. Shadow détecte (via heartbeat tracker existant)
2. Shadow élit un nouveau Candidate
3. Aucun impact sur le service

### Double mort (Primary + Shadow simultanés)

Cas extrême. Le Candidate ne reçoit plus de signal de personne.
- Après timeout (30s sans nouvelles), le Candidate se promeut Primary
- Envoie HubMigration aux membres
- Élit un nouveau Shadow + Candidate
- Service restauré, mais avec un gap plus long

## Nouveaux payloads GroupPayload

```rust
/// Shadow watchdog ping (shadow → primary, toutes les 3s)
HubPing { group_id: GroupId },

/// Primary response (primary → shadow)
HubPong { group_id: GroupId },

/// State sync (primary → shadow, à chaque changement)
HubShadowSync {
    group_id: GroupId,
    members: Vec<GroupMember>,
    candidate_id: Option<NodeId>,
    config_version: u64,
},

/// Candidate assignment notification
CandidateAssigned { group_id: GroupId },

/// Member reports hub unreachable (member → shadow)
HubUnreachable { group_id: GroupId },
```

## Modules impactés

| Module | Changement |
|--------|-----------|
| `group/types.rs` | Nouveaux payloads + constantes (SHADOW_PING_INTERVAL=3s, PING_TIMEOUT=2s, PING_FAILURE_THRESHOLD=2) |
| `group/hub.rs` | Gestion shadow sync, réponse HubPong, désignation candidate |
| `group/manager.rs` | Rôle shadow (watchdog, promotion), rôle candidate (promotion), envoi HubUnreachable |
| `group/election.rs` | Réutilisé tel quel (elect_hub avec exclusions) |
| `runtime/state.rs` | Timer ping shadow 3s, orchestration des promotions, dispatch nouveaux payloads |
| `runtime/mod.rs` | Nouveaux ProtocolEvents (ShadowPromoted, CandidateAssigned, HubChainRestored) |

## Constantes

```rust
pub const SHADOW_PING_INTERVAL_MS: u64 = 3_000;   // Shadow ping toutes les 3s
pub const SHADOW_PING_TIMEOUT_MS: u64 = 2_000;     // Timeout par ping
pub const SHADOW_PING_FAILURE_THRESHOLD: u32 = 2;   // 2 pings ratés → promotion
pub const HUB_ACK_TIMEOUT_MS: u64 = 3_000;          // Timeout membre avant HubUnreachable
pub const CANDIDATE_ORPHAN_TIMEOUT_MS: u64 = 30_000; // Timeout double-mort
```

## Plan de tests

| # | Test | Vérifie |
|---|------|---------|
| 1 | `shadow_assigned_on_group_create` | Shadow élu à la création du groupe |
| 2 | `shadow_sync_on_member_change` | Primary envoie HubShadowSync quand un membre join/leave |
| 3 | `shadow_ping_pong_cycle` | Ping/pong fonctionne, shadow reste standby |
| 4 | `shadow_promotes_on_primary_death` | 2 pings ratés → shadow devient primary, envoie HubMigration |
| 5 | `candidate_promotes_on_shadow_death` | Candidate devient shadow après détection |
| 6 | `chain_fully_restored_after_promotion` | Après promotion, nouveau candidate élu, chaîne 3 niveaux |
| 7 | `member_hub_unreachable_accelerates_detection` | Alerte membre + 1 ping raté = promotion immédiate |
| 8 | `double_death_candidate_takes_over` | Primary + Shadow meurent → Candidate self-promote après timeout |
| 9 | `hub_migration_updates_all_members` | Après promotion, tous les membres re-routent vers le nouveau hub |
| 10 | `shadow_sync_state_matches_primary` | L'état du shadow est identique au primary après sync |
