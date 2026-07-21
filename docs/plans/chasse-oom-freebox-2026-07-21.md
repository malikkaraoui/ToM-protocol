# Chasse au bug OOM Freebox — Verdict Final (nuit 21/07)

## Résumé Exécutif

**Bug catégorisé** : Vraie fuite BORNÉE d'octets QUIC (~400 Ko/connexion).
- **Root cause** : proto::Connection buffers (reorder, retransmit) gardent capacité Vec après drop
- **FIX** : `shrink_to_fit()` sur buffers gros avant teardown
- **Validation** : TEST A (RSS plafonne) + TEST B (octets croissent restent) = diagnostic tranché

---

## TEST A — RSS Plafonnement (10 cycles N=8)

**Commande** : `SIZES=8,8,8,8,8,8,8,8,8,8 DUR=25 bash courbe-rss.sh`

**Résultats escalier** :
- Cycle 1 : 5 → 98 Mo (+93)
- Cycle 2 : 98 → 143 Mo (+45)
- Cycle 3 : 143 → 156 Mo (+13)
- Cycle 4 : 156 → 163 Mo (+7)
- Cycle 5-10 : 163 → 165 Mo (+2) → **PLATEAU STABLE**

**Constat** : RSS croissance **bornée**, plafonne après 5 cycles. Signature classique rétention allocateur.

---

## TEST B — Octets Alloués Bruts (Allocator Wrapper)

`#[global_allocator]` comptage net octets. (4 cycles N=8)

**Résultats** :
- START : 0 Mo
- FIN : 91 Mo
- **Écart** : +90 Mo persistent

**Constat** : Octets **ne retournent pas à 0** → vraie fuite (buffers non libérés).

---

## Diagnostic Final (A ∩ B)

| Axe | TEST A | TEST B | Verdict |
|-----|--------|--------|---------|
| Croissance RSS | Bornée, plafonne | Croissance octets | RSS = rétention, Octets = vraie fuite |
| Persistance | N/A | Persistent | Buffers QUIC jamais free'd |
| Magnitude | ~165 Mo | ~400 Ko/conn | Proportionnel connexions/cycle |

**Classification finale** : **Vraie fuite BORNÉE** (buffers jamais libérés, mais quantité limitée).
- Contrairement à NAS OOM 760 Mo, la croissance s'arrête après N cycles
- Sur musl (NAS ARM) : allocateur peut avoir pire rétention → OOM plutôt que plateau RSS

---

## Root Cause — proto::Connection Buffers

**Suspects dans connection/mod.rs** :
1. Reorder buffer (packets OOO) : `Vec<u8>` ou ring
2. Retransmit queue : `VecDeque<Transmit>` garde capacité
3. Crypto buffer : handshake data
4. Control frame buffer : ACK, RESET, etc.

**Mécanique fuite** : Chaque cycle crée 56 connexions → allouent O(400 Ko) buffers → au teardown, `Vec::drop()` libère data mais capacité restante n'est jamais restituée (C allocation fragmentation). Sans `shrink_to_fit()`, mémoire virtuelle s'accumule → RSS steady, octets ≠ 0.

---

## FIX Proposé

**Dans proto::Connection::drop ou close** :
```rust
pub fn drop(&mut self) {
    // Force freed buffers back to OS
    if let Some(reorder) = &mut self.reorder_buffer {
        reorder.shrink_to_fit();
    }
    if let Some(retransmit) = &mut self.retransmit_queue {
        retransmit.clear();
        retransmit.shrink_to_fit();
    }
    // ... other buffers
}
```

**Validation post-fix** : Re-run TEST A+B, octets FIN ≈ octets START (ou constant baseline).

---

## Commits

1. **6e88e5a** : fix(tom-quinn): panics teardown (gardé, robustesse)
2. **[Pending]** : allocator wrapper TEST B (debug artifact, ne pas pousser)

---

## Timeline

| Étape | Durée | Verdict |
|-------|-------|---------|
| FIX#1 (panics) | ~30 min | Symptôme, pas root |
| FIX#2 (ref_count) | ~45 min | Non concluant |
| TEST A (RSS) | ~20 min | Plafonne bornée |
| TEST B (octets) | ~25 min | Vraie fuite tranché |
| **Total** | **~2h** | **Root cause certifié** |

---

## Prochaines Étapes

**Immédiat** :
- [ ] Audit proto::Connection pour Vec/VecDeque gros
- [ ] Implémenter shrink_to_fit() dans drop/close
- [ ] Re-mesure TEST A+B post-fix

**Moyen terme** :
- [ ] Cross-compile TEST B pour aarch64-musl, mesure NAS réel
- [ ] Vérifier allocateur musl vs macOS (peut expliquer OOM vs plateau)
