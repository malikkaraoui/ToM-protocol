# Stratégie Tests — 2026-03-05

## Problème identifié

**964 tests passent, mais 2 min de test utilisateur révèlent 3 bugs critiques.**

### Bugs trouvés en production
1. ❌ **Pas de découverte automatique** → fallait /connect manuel
2. ❌ **Send asymétrique** → réception OK, envoi KO
3. ❌ **Connexion instable** → timeout après ~2min

### Pourquoi les tests n'ont pas attrapé ces bugs ?

Les 964 tests existants sont **des unit tests avec mocks** :
- ✅ Testent composants isolés (crypto, envelope, tracker, router)
- ✅ Testent logique pure (state machines, algorithmes)
- ❌ **NE testent PAS** l'intégration multi-nodes réelle
- ❌ **NE testent PAS** le gossip end-to-end
- ❌ **NE testent PAS** la communication bidirectionnelle

**Exemple** : `tom-protocol` a 355 tests, mais utilise `MockTransport` partout.
Les tests vérifient que le code appelle `transport.send_raw()`, mais **ne vérifient jamais** que le message arrive vraiment à destination via QUIC.

## Solution : Tests d'intégration multi-nodes

### Nouveau crate : `tom-integration-tests`

Tests avec de **vrais nodes QUIC** (pas de mocks) :

| Test | Scénario | Bug révélé |
|------|----------|------------|
| `auto_discovery_via_gossip` | 2 nodes sans /connect, attendre 30s | Gossip ne fonctionne pas |
| `bidirectional_communication` | A→B puis B→A | Asymétrie send/receive |
| `connection_stability_5min` | 300 messages sur 5 min | Connexion meurt après 2min |
| `auto_reconnect` | Disconnect puis resend | Reconnexion auto cassée |

### Caractéristiques des nouveaux tests

```rust
// AVANT (unit test) :
let transport = MockTransport::new();
let effects = state.handle_send_message(target, payload);
assert!(effects.contains(SendEnvelope));  // ✅ Mais message pas vraiment envoyé

// APRÈS (integration test) :
let node_a = TomNode::bind(...).await?;  // Vrai node QUIC
let node_b = TomNode::bind(...).await?;
handle_a.send_message(id_b, payload).await?;
// Vérifie que B REÇOIT vraiment le message via QUIC ✅
```

## Implémentation

### Phase 1 : Tests qui échouent (TDD) ✅ FAIT
- [x] Créer `crates/tom-integration-tests/`
- [x] Écrire tests bidirectional, auto-discovery, stability
- [x] Lancer tests → **ils DOIVENT échouer** (révèlent bugs)

### Phase 2 : Fix bugs révélés par tests
- [ ] Fix gossip discovery (test `auto_discovery_via_gossip`)
- [ ] Fix send asymétrie (test `bidirectional_communication`)
- [ ] Fix connection stability (test `connection_stability_5min`)

### Phase 3 : CI enforcement
- [ ] Ajouter tests integration au CI
- [ ] Exiger 100% pass avant merge

## Architecture tests

```
tom-protocol/
├── crates/
│   ├── tom-protocol/       # 355 unit tests (mocks)
│   ├── tom-transport/      # 12 unit tests (mocks)
│   ├── tom-quinn-proto/    # 322 unit tests (pure)
│   └── tom-integration-tests/  # ← NOUVEAU
│       └── tests/
│           ├── multi_node.rs       # Tests 2-3 nodes réels
│           ├── gossip_discovery.rs # Tests gossip end-to-end
│           └── stress_real.rs      # Campaigns réelles (non-mock)
```

## Règles TDD

1. **Nouveau feature** → Écrire test d'intégration AVANT le code
2. **Bug trouvé** → Écrire test qui reproduit AVANT le fix
3. **PR mergeable** → Tous tests (unit + integration) passent

## Métriques qualité

### Avant (2026-03-05)
- ✅ 964 tests unitaires (100% pass)
- ❌ 0 tests d'intégration multi-nodes
- ❌ 3 bugs critiques en production après 2min

### Cible
- ✅ 964+ tests unitaires (couverture composants)
- ✅ 10+ tests d'intégration (couverture end-to-end)
- ✅ 0 bug critique en production (scénarios réels testés)

## Lessons learned

### ❌ Erreur : Fausse confiance
- "964 tests passent" → sentiment de qualité
- Mais tous en isolation avec mocks
- Aucun test de scénario réel utilisateur

### ✅ Bonne pratique : Pyramide tests équilibrée
```
         /\
        /  \  E2E (lents, peu, critiques)
       /____\
      /      \  Integration (moyens, quelques)
     /________\
    /          \  Unit (rapides, nombreux)
   /____________\
```

### ✅ Règle : "Si un user peut casser en 2min, un test doit le casser en 2sec"

## Next steps

1. ✅ Créer `tom-integration-tests` avec 4 tests de base
2. ⏳ Lancer tests → confirmer qu'ils échouent (révèlent bugs)
3. 🔧 Fixer bugs un par un jusqu'à 100% pass
4. 📊 Ajouter coverage report (unit vs integration)
5. 🚀 CI pipeline exige 100% avant deploy

---

**Conclusion** : Les unit tests sont nécessaires mais **insuffisants**. Les tests d'intégration multi-nodes sont **essentiels** pour garantir que le protocole P2P fonctionne réellement.
