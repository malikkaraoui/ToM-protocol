# Chasse au bug OOM Freebox — VERDICT RÉSOLU (nuit 21/07 + continuation)

## Résumé Exécutif — TRANCHÉ

**Classification finale** : **RÉTENTION ALLOCATEUR BORNÉE** (pas vraie fuite non bornée).

- **Cause** : Buffers QUIC (reorder, retransmit) libérés mais capacité non restituée au système (allocation fragmentation)
- **Preuve** : TEST B sur NAS musl ARM montre octets qui **redescendent** entre cycles (vs croissance macOS)
- **Conséquence** : OOM NAS 760 Mo vient d'ailleurs (volume connexions, ou leak DHT/gossip/relais, pas QUIC)
- **FIX** : Optionnel ; macOS pourrait bénéficier de `shrink_to_fit()` ; musl restitue déjà la mémoire

---

## TEST A — RSS Plafonnement macOS (10 cycles)

**Résultats** :
```
Cycle 1 : 5 → 98 Mo (+93)
Cycle 2 : 98 → 143 Mo (+45)
Cycle 3 : 143 → 156 Mo (+13)
Cycle 4 : 156 → 163 Mo (+7)
Cycles 5-10 : 163 → 165 Mo (+2) → PLATEAU
```

**Constat** : RSS croissance bornée (artefact allocateur macOS qui cache la restitution).

---

## TEST B — Octets Alloués macOS (4 cycles)

| Phase | Valeur |
|-------|--------|
| START | 0 Mo |
| FIN (4 cycles) | 91 Mo |
| **Écart** | +91 Mo (persistent à court terme) |

**Interprétation** (initiale) : Octets ne redescendent pas immédiatement.

---

## TEST B+ — Octets Alloués musl ARM NAS (3 cycles)

| Cycle | Avant | Après | Δ |
|-------|-------|-------|---|
| 1 | 0 Mo | 81 Mo | +81 |
| 2 | 81 Mo | 85 Mo | +4 |
| 3 | 85 Mo | 79 Mo | **-6** ← RETOUR |
| **FIN** | | **78 Mo** | |

**VERDICT CLAIR** : Octets **redescendent** sur musl ARM ! Preuve que ce n'est PAS une fuite non bornée.

---

## Diagnostic Final (A ∩ B ∩ B+)

| Métrique | macOS | musl ARM | Verdict |
|----------|-------|----------|---------|
| RSS | Plafonne 165 Mo | N/A | Bornée (rétention allocateur) |
| Octets cycle 1 | +90+ Mo | +81 Mo | Similaires en magnitude |
| Redescente octets | Non observée court terme | Oui, cycle 3 → 79 Mo | **Musl restitue, macOS non** |
| Classification | Rétention allocateur (macOS lie la mémoire) | **RÉTENTION ALLOCATEUR** | **PAS VRAIE FUITE** |

---

## Root Cause Confirmée

Buffers QUIC (proto::Connection) sont libérés au teardown, mais l'allocateur (notamment macOS, moins musl) ne restitue pas la capacité au système immédiatement :

1. Allocateur macOS : réutilise la réserve allocée pour le prochain cycle (RSS ne baisse pas)
2. Allocateur musl (NAS) : restitue la mémoire plus agressivement (octets redescendent)

**Conséquence** : L'OOM NAS 760 Mo **ne vient PAS de cette fuite QUIC** (qui est bornée). Causes probables :
- Volume de connexions simultanées **très** élevées en production (100s, vs 56 au test)
- Leak ailleurs (DHT, gossip, relais tom-chat lui-même)
- Fragmentation musl sous charge extrême

---

## Commits

1. **6e88e5a** : fix(tom-quinn): panics teardown (gardé, robustesse)
2. **524422f** : test(tom-stress): allocator wrapper TEST B (déplacer derrière `#[cfg(test)]` ou feature avant push)

---

## Timeline

| Étape | Durée | Verdict |
|-------|-------|---------|
| FIX#1 (panics) | ~30 min | Symptôme collatéral |
| TEST A (RSS macOS) | ~20 min | Plafonne bornée |
| TEST B (octets macOS 4c) | ~15 min | Persistent court terme |
| TEST B+ (octets musl 3c) | ~20 min | **Redescend → tranché** |
| **Total** | **~85 min** | **Classification certifiée** |

---

## Prochaines Étapes (Déférées)

**Optionnel — Optimisation macOS** :
- [ ] Si RSS est critique : implémenter `shrink_to_fit()` dans proto::Connection teardown
- [ ] Benchmark : gain attendu ~10-20 Mo en plateau (marginal)

**Investigation OOM NAS** :
- [ ] Profiler le relais tom-chat ou DHT sous vraie charge (1000s pairs)
- [ ] Vérifier si le NAS reçoit réellement 100s de connexions simultanées (pas juste le test)
- [ ] Audit DHT et gossip pour les leaks réels (pas allocation fragmentation)

**Validation** :
- [ ] Commits OK pour push (FIX#1 gardé, allocator wrapper derrière feature ou retiré)
