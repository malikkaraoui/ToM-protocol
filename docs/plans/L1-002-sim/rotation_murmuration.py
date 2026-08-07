#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
L1-002 — Simulation ETENDUE : la ROTATION du role dans le temps.

La 1re sim (grinding_murmuration.py) mesurait UN tirage. Elle ignorait que le role
TOURNE round apres round. Ce fichier ajoute la dynamique temporelle, cad les trois
menaces que le tirage-unique masque :

  1. FEEDBACK (boucle de retroaction) — LE danger vicieux de la rotation.
     Si tenir le role au round w donne un levier sur la graine de w+1 (le tenant
     ancre/co-signe l'etat vivant suivant), alors gagner peut s'AUTO-RENFORCER
     -> capture progressive. On modelise ce levier HONNETEMENT : le tenant adverse
     obtient K_fb essais de re-tirage sur la graine du round suivant (pouvoir de
     grinding confere par le role). rho in [0,1] regle ce pouvoir ; rho=0 = aucun
     (equite), rho=1 = controle quasi total. Le QUORUM q-of-n est precisement ce
     qui MAINTIENT rho bas (le tenant seul ne controle pas la graine).
     NB : correction vs 1er proto d'agent, ou "feedback = hash(base||winner)" laissait
     la graine uniforme -> aucun auto-renforcement. Ici le levier = des ESSAIS, seul
     mecanisme qui cree reellement une boucle.

  2. RUNS CONSECUTIFS — tenir le role R fois d'AFFILEE (censure durable) est bien
     pire que R fois reparties. On mesure la distribution + le max des runs.

  3. COOLDOWN (tour de garde, regle #2) — apres un gain, le gagnant est ineligible
     C rounds. Casse-t-il les runs ? Casse-t-il la capture collective par feedback ?

Selection = tirage EGAL : gagnant = argmin_id sha256(graine || id) parmi les eligibles.
Hachage sha256 REEL. Deterministe (--seed). Aucune horloge murale.

Metrique cle : PHI_T = fraction du temps ou l'adversaire tient le role sur T rounds.
  PHI_T ~ f            => equite (contraction, sain)
  PHI_T >> f  et croit => capture (expansion, DANGER)
"""

import argparse
import hashlib
import random
import statistics


# --------------------------------------------------------------------------
def select_winner(seed_bytes, eligible_ids):
    """Gagnant = argmin_id sha256(seed || id) parmi les eligibles (tirage egal)."""
    best_id = -1
    best_h = None
    for nid in eligible_ids:
        h = hashlib.sha256(seed_bytes + nid.to_bytes(4, "big")).digest()
        if best_h is None or h < best_h:
            best_h = h
            best_id = nid
    return best_id


GRIND_MAX = 30  # essais de re-tirage max qu'un pouvoir de role plein (rho=1) confere


def k_feedback(rho):
    """Pouvoir de grinding confere par le role au round suivant, en nb d'essais."""
    return 1 + round(rho * GRIND_MAX)


# --------------------------------------------------------------------------
def simulate(n, n_adv, rounds, cooldown_c, rho, feedback_on, seed_rng):
    """Un run de T rounds. Retourne les metriques temporelles."""
    rng = random.Random(seed_rng)
    honest_base = rng.getrandbits(256).to_bytes(32, "big")  # bruit honnete du run
    cooldown = [0] * n
    prev_adv = False           # le gagnant du round precedent etait-il adverse ?
    adv_win_series = []        # 1 si adv gagne au round w, sinon 0

    for w in range(rounds):
        eligible = [i for i in range(n) if cooldown[i] == 0]
        if not eligible:                       # petit reseau sature de cooldowns
            eligible = list(range(n))          # filet : tout le monde re-eligible

        # Pouvoir de grinding : seulement si feedback actif ET tenant precedent adverse
        tries = k_feedback(rho) if (feedback_on and prev_adv) else 1

        # Le tenant re-tire la graine jusqu'a faire gagner un adversaire eligible
        winner = None
        for t in range(tries):
            s = hashlib.sha256(honest_base + w.to_bytes(4, "big") + t.to_bytes(4, "big")).digest()
            cand = select_winner(s, eligible)
            if winner is None:
                winner = cand                  # 1er tirage = defaut si aucun adv trouve
            if cand < n_adv:                   # un adversaire gagne -> le grinder s'arrete la
                winner = cand
                break

        is_adv = winner < n_adv
        adv_win_series.append(1 if is_adv else 0)

        # cooldown : decremente, puis met le gagnant au repos C rounds
        for i in range(n):
            if cooldown[i] > 0:
                cooldown[i] -= 1
        if cooldown_c > 0:
            cooldown[winner] = cooldown_c

        prev_adv = is_adv

    return metrics(adv_win_series, n_adv / n, rounds)


def metrics(series, f, rounds):
    adv_wins = sum(series)
    phi_t = adv_wins / rounds

    # runs consecutifs d'adversaire
    runs, cur = [], 0
    for x in series:
        if x:
            cur += 1
        elif cur:
            runs.append(cur)
            cur = 0
    if cur:
        runs.append(cur)
    r_max = max(runs) if runs else 0

    # derive : PHI mesure sur 1er tiers vs dernier tiers (expansion si ca monte)
    third = max(1, rounds // 3)
    phi_early = sum(series[:third]) / third
    phi_late = sum(series[-third:]) / third

    return {"phi_t": phi_t, "r_max": r_max, "phi_early": phi_early,
            "phi_late": phi_late, "f": f}


def run_regime(name, n, n_adv, rounds, cooldown_c, rho, feedback_on, trials, seed_base):
    res = [simulate(n, n_adv, rounds, cooldown_c, rho, feedback_on, seed_base + t)
           for t in range(trials)]
    agg = {
        "phi_t": statistics.mean(r["phi_t"] for r in res),
        "r_max": statistics.mean(r["r_max"] for r in res),
        "r_max_p95": sorted(r["r_max"] for r in res)[int(0.95 * (len(res) - 1))],
        "phi_early": statistics.mean(r["phi_early"] for r in res),
        "phi_late": statistics.mean(r["phi_late"] for r in res),
        "f": n_adv / n,
    }
    verdict = "EQUITE" if agg["phi_t"] <= 1.6 * agg["f"] else (
        "CAPTURE" if agg["phi_late"] >= agg["phi_early"] * 1.2 or agg["phi_t"] > 3 * agg["f"]
        else "biais")
    print(f"  {name:<46} PHI_T={agg['phi_t']:.3f} (f={agg['f']:.2f})  "
          f"R_max={agg['r_max']:.0f} (p95={agg['r_max_p95']:.0f})  "
          f"early->late={agg['phi_early']:.2f}->{agg['phi_late']:.2f}  [{verdict}]")
    return agg


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--rounds", type=int, default=400)
    ap.add_argument("--trials", type=int, default=20)
    ap.add_argument("-n", type=int, default=50)
    ap.add_argument("-f", type=float, default=0.10)
    args = ap.parse_args()
    n, n_adv, T, tr = args.n, max(1, round(args.f * args.n)), args.rounds, args.trials
    sb = args.seed

    print(f"# L1-002 ROTATION simulation  (N={n}, f={n_adv/n:.2f} [{n_adv} adv], "
          f"T={T}, trials={tr}, seed={sb}, GRIND_MAX={GRIND_MAX})\n")
    print("PHI_T = fraction du temps ou l'adversaire TIENT le role.  Equite <=> PHI_T ~ f.\n")

    print("## Regimes (feedback = pouvoir de grinding confere par le role)")
    run_regime("B  feedback OFF, cooldown 0  (reference)", n, n_adv, T, 0, 0.0, False, tr, sb)
    run_regime("C  feedback OFF, cooldown C=2", n, n_adv, T, 2, 0.0, False, tr, sb + 1000)
    run_regime("D  feedback rho=0.3, cooldown 0", n, n_adv, T, 0, 0.3, True, tr, sb + 2000)
    run_regime("E  feedback rho=0.9, cooldown 0  (danger)", n, n_adv, T, 0, 0.9, True, tr, sb + 3000)
    run_regime("F  feedback rho=0.9, cooldown C=2", n, n_adv, T, 2, 0.9, True, tr, sb + 4000)
    run_regime("G  feedback rho=0.9, cooldown C=n_adv+1", n, n_adv, T, n_adv + 1, 0.9, True, tr, sb + 5000)

    print("\n## Balayage rho (cooldown 0) — ou bascule-t-on de l'equite a la capture ?")
    print(f"  {'rho':>5} | {'K_fb':>5} | {'PHI_T':>7} | {'R_max':>6} | verdict")
    for rho in (0.0, 0.2, 0.4, 0.6, 0.8, 1.0):
        a = run_regime(f"   rho={rho:.1f}", n, n_adv, T, 0, rho, True, tr, sb + 6000 + int(rho * 10))

    print("\n## Balayage cooldown (feedback rho=0.9 FORT) — le cooldown sauve-t-il ?")
    print(f"  viabilite petit reseau : C < 1/(2f) = {1/(2*(n_adv/n)):.1f}")
    for c in (0, 2, 4, n_adv, n_adv + 1, 2 * n_adv):
        run_regime(f"   C={c}", n, n_adv, T, c, 0.9, True, tr, sb + 7000 + c)


if __name__ == "__main__":
    main()
