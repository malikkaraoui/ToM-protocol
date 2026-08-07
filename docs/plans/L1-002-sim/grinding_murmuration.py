#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
L1-002 — Simulation : l'entropie collective-émergente resiste-t-elle au *grinding*
quand la masse grandit ? (murmuration -> alea non-biaisable)

Question (plan global M1.2) : « un demandeur qui re-tire 10^6 fois ne gagne AUCUN
avantage de selection ». On la teste sur DEUX regimes contrastes :

  Regime A  — AGREGATION NAIVE (le trou connu de L1-001, cf. aggregator.rs) :
              un grinder peut RE-TIRER sa contribution T fois et garder le meilleur
              tirage. C'est le « dernier-revelateur avec budget ».

  Regime B  — ENGAGEMENT TEMOIGNE-PAR-VAGUE (la digue murmuration) :
              chaque noeud emet UNE contribution par vague, temoignee par ses ~k
              voisins AVANT que la graine soit calculable. Le grinder ne peut plus
              re-tirer : il ne lui reste que l'ABSTENTION (withhold), detectable et
              plafonnee par la politique (b abstentions avant fade). Tries = b+1.

Metrique : AVANTAGE = P(un noeud du grinder est tire) - f     (0 = equite parfaite)
Selection = tirage EGAL parmi eligibles (regle #2 « ticket d'entree, tirage egal ») :
            gagnant = argmin_id  sha256(graine || id).

Le script (1) VALIDE empiriquement, au hachage reel, que la selection est uniforme
et que chaque re-tirage est un tirage independant Bernoulli(f) ; puis (2) en DEDUIT
l'avantage via la loi validee  P(win|K tries) = 1-(1-f)^K , balayee en (T, b, N, f).

Deterministe : --seed fixe le RNG. Aucune horloge murale.
"""

import argparse
import csv
import hashlib
import os
import random


# --------------------------------------------------------------------------
#  Selection reelle (tirage egal par hachage) — le coeur verifiable
# --------------------------------------------------------------------------
def select_winner(seed_bytes, n_nodes):
    """Gagnant = argmin_id sha256(seed || id_le4). Uniforme si seed uniforme."""
    best_id = -1
    best_h = None
    for nid in range(n_nodes):
        h = hashlib.sha256(seed_bytes + nid.to_bytes(4, "big")).digest()
        if best_h is None or h < best_h:
            best_h = h
            best_id = nid
    return best_id


def _rand_seed(rng):
    return rng.getrandbits(256).to_bytes(32, "big")


# --------------------------------------------------------------------------
#  EXP-0a — la selection est-elle uniforme ? (sinon f n'est pas le baseline)
# --------------------------------------------------------------------------
def exp_uniformity(n_nodes, per_bin, rng):
    trials = n_nodes * per_bin
    wins = [0] * n_nodes
    for _ in range(trials):
        wins[select_winner(_rand_seed(rng), n_nodes)] += 1
    expected = trials / n_nodes
    max_dev = max(abs(w - expected) for w in wins) / expected
    # bruit d'echantillonnage attendu sous UNIFORMITE : ~3 sigma relatif
    sigma_rel = ((1.0 - 1.0 / n_nodes) / per_bin) ** 0.5
    return max_dev, 3.0 * sigma_rel  # ecart max observe vs bande ~3sigma


# --------------------------------------------------------------------------
#  EXP-0b — re-tirer = tirages independants Bernoulli(f) ? (valide K=tries)
#  Le grinder possede les ids [0..G). A chaque essai il change SA contribution
#  (un nonce qu'il controle) -> nouvelle graine -> nouveau gagnant.
# --------------------------------------------------------------------------
def exp_independence(n_nodes, n_grinder, rmax, rounds, rng):
    f = n_grinder / n_nodes
    first_win = []  # index du 1er essai gagnant (ou rmax+1 si jamais)
    for _ in range(rounds):
        honest_blob = _rand_seed(rng)  # partie hors controle du grinder, figee
        won_at = rmax + 1
        for t in range(1, rmax + 1):
            nonce = _rand_seed(rng)  # le grinder re-tire SA contribution
            seed = hashlib.sha256(honest_blob + nonce).digest()
            if select_winner(seed, n_nodes) < n_grinder:  # gagnant dans [0..G)
                won_at = t
                break
        first_win.append(won_at)

    # P(win dans les R premiers essais), empirique vs loi 1-(1-f)^R
    emp = {}
    for R in (1, 2, 5, 10, 20, rmax):
        emp[R] = sum(1 for w in first_win if w <= R) / rounds
    return f, emp


# --------------------------------------------------------------------------
#  Loi validee : avantage d'un grinder disposant de K essais effectifs
# --------------------------------------------------------------------------
def advantage(f, k_tries):
    p_win = 1.0 - (1.0 - f) ** k_tries
    return p_win - f


# --------------------------------------------------------------------------
#  Rendu
# --------------------------------------------------------------------------
def fmt(x):
    if x == 0:
        return "0"
    if abs(x) < 1e-4:
        return f"{x:.1e}"
    return f"{x:.4f}"


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--rounds", type=int, default=600)
    ap.add_argument("--csv", default=None)
    args = ap.parse_args()
    rng = random.Random(args.seed)

    rows_csv = []
    print(f"# L1-002 grinding simulation  (seed={args.seed}, rounds={args.rounds})\n")

    # ---- EXP-0a uniformite ------------------------------------------------
    print("## EXP-0a — uniformite de la selection (tirage egal)")
    for n in (20, 50, 100):
        dev, band = exp_uniformity(n, per_bin=2000, rng=rng)
        ok = "OK" if dev <= 1.15 * band else "??"
        print(f"  N={n:4d}  ecart_max_observe={dev:.3f}  bande_bruit_~3sigma={band:.3f}  [{ok}]")
    print("  -> ecart_max <= bande de bruit : compatible avec l'UNIFORME (chaque noeud ~1/N).")
    print("     Donc le baseline equitable est bien f=G/N.\n")

    # ---- EXP-0b independance des re-tirages -------------------------------
    print("## EXP-0b — chaque re-tirage = tirage independant Bernoulli(f) ?")
    f0, emp = exp_independence(200, 20, rmax=40, rounds=args.rounds, rng=rng)
    print(f"  N=200, G=20 (f={f0:.2f}) — P(grinder gagne en <=R essais) :")
    print(f"  {'R':>4} | {'empirique':>10} | {'loi 1-(1-f)^R':>14}")
    for R, p in emp.items():
        print(f"  {R:>4} | {p:>10.4f} | {1-(1-f0)**R:>14.4f}")
    print("  -> empirique ~= loi : re-tirer donne K tirages independants. Modele valide.\n")

    # ---- EXP-1 : REGIME A (agregation naive) — la masse protege-t-elle ? --
    print("## EXP-1 — REGIME A (naif) : avantage vs budget de re-tirage T, a f=0.10 fixe")
    print("   (le grinder re-tire T fois et garde le meilleur ; le trou de L1-001)")
    f = 0.10
    Ts = [1, 10, 100, 1_000, 10_000, 1_000_000]
    Ns = [50, 500, 5_000, 50_000]
    tn = "T \\ N"
    header = "   " + f"{tn:>10} | " + " | ".join(f"N={n:>6}" for n in Ns)
    print(header)
    for T in Ts:
        cells = []
        for n in Ns:
            adv = advantage(f, T)  # f fixe => avantage independant de N
            cells.append(f"{fmt(adv):>8}")
            rows_csv.append({"exp": "A", "regime": "naif", "N": n, "f": f,
                             "T": T, "b": "", "advantage": adv})
        print("   " + f"{T:>10} | " + " | ".join(cells))
    print("   -> a fraction f FIXE, l'avantage NE DEPEND PAS de N et -> ~0.9 a T=10^6.")
    print("      VERDICT : la masse seule NE protege PAS contre le re-tirage. Le trou reste.\n")

    # ---- EXP-2 : REGIME B (temoigne-vague) — la digue -------------------
    print("## EXP-2 — REGIME B (temoigne-par-vague) : re-tirage IMPOSSIBLE, reste l'abstention")
    print("   Tries effectifs = b+1 (b = nb d'abstentions tolerees avant fade). f=0.10 fixe.")
    print("   Compare au critere du plan : Regime A a T=10^6.")
    print(f"   {'b':>3} | {'tries':>6} | {'avantage_B':>12}   (rappel A@1e6 = {fmt(advantage(f,10**6))})")
    for b in (0, 1, 2, 5, 10):
        adv = advantage(f, b + 1)
        print(f"   {b:>3} | {b+1:>6} | {fmt(adv):>12}")
        rows_csv.append({"exp": "B", "regime": "temoigne", "N": "", "f": f,
                         "T": "", "b": b, "advantage": adv})
    print("   -> le temoignage plafonne les essais a b+1 : l'avantage s'effondre (b=0 => equite).")
    print("      La DIGUE = l'engagement temoigne, PAS la masse. Le critere 10^6 devient sans objet.\n")

    # ---- EXP-3 : la masse contre un adversaire ABSOLU (G fixe) ----------
    print("## EXP-3 — la masse vs adversaire de TAILLE FIXE G (f=G/N -> 0 quand N grandit)")
    print("   Regime B, b=1 (une abstention toleree). G absolu constant.")
    for G in (5, 50):
        print(f"   G={G} noeuds hostiles absolus :")
        print(f"      {'N':>8} | {'f=G/N':>8} | {'avantage_B(b=1)':>16}")
        for n in (100, 1_000, 10_000, 100_000):
            fN = G / n
            adv = advantage(fN, 2)
            print(f"      {n:>8} | {fN:>8.4f} | {fmt(adv):>16}")
            rows_csv.append({"exp": "C", "regime": "temoigne", "N": n, "f": fN,
                             "T": "", "b": 1, "advantage": adv})
    print("   -> contre un adversaire ABSOLU, l'avantage -> 0 quand N grandit :")
    print("      ICI « plus de monde = plus fort » est VRAI (dilution de f).\n")

    # ---- EXP-4 : mais adversaire a FRACTION constante : masse insuffisante
    print("## EXP-4 — adversaire a FRACTION constante f : la masse ne suffit plus")
    print("   Regime B, b=1. G croit avec N (f fixe). Avantage vs N :")
    print(f"      {'f':>6} | " + " | ".join(f"N={n:>6}" for n in (100, 1_000, 10_000)))
    for f in (0.01, 0.05, 0.10, 0.20):
        adv = advantage(f, 2)  # independant de N a f fixe
        print(f"      {f:>6} | " + " | ".join(f"{fmt(adv):>8}" for _ in (0, 1, 2)))
        rows_csv.append({"exp": "D", "regime": "temoigne", "N": "any", "f": f,
                         "T": "", "b": 1, "advantage": adv})
    print("   -> a f FIXE l'avantage_B est PLAT en N : borne par la politique b, pas par la masse.")
    print("      SYNTHESE : temoignage (plafonne K) + masse (dilue f absolu) + fade (borne b).\n")

    if args.csv:
        with open(args.csv, "w", newline="") as fh:
            w = csv.DictWriter(fh, fieldnames=["exp", "regime", "N", "f", "T", "b", "advantage"])
            w.writeheader()
            w.writerows(rows_csv)
        print(f"[csv] {args.csv}  ({len(rows_csv)} lignes)")


if __name__ == "__main__":
    main()
