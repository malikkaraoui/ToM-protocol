#!/usr/bin/env python3
"""Capstone Phase C — « la vie de l'organisme » (banc-roles-sous-charge.md §3).

Séquence 15 min sur la flotte réelle, charge de fond continue, événements
orchestrés, 3 verdicts globaux pré-enregistrés (V1 livraison, V2 preuves par
mécanisme, V3 pas-de-fantôme). Rôles humains : extinction device à t+4 min,
rallumage à t+10 min (annonces vocales `say` + détection réelle par compteurs).

Écarts au design §3, actés le 21/07 (V1 du capstone) :
- t+2 : kill de l'APP MAC (pas « hub de groupe » — aucune route groupe au
  control :9300 ; R6-failover reste un scénario Phase B dédié).
- R7-F : `online_count` PoP n'est pas exposé au :9091 (LOCKED #6) — le verdict
  « pas de fantôme » s'appuie sur `pairs_connectes` (aucun id hors flotte
  figée + acteurs du run) et `taille_reseau`.

Usage :
  python3 scripts/chaos/capstone.py                 # run complet (~17 min)
  python3 scripts/chaos/capstone.py --dry-run       # briques seules (~3 min),
                                                    # sans kill/extinction/humain
"""

import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from orchestrator import (  # noqa: E402
    NAS_CONTROL, NODES, Report, collector_lines, collector_since,
    counters, http_json, node_id, seq_counts,
)

REPO = "/Users/malik/Documents/tom-protocol"
TOM_STRESS = f"{REPO}/target/debug/tom-stress"
MAC_APP = f"{REPO}/apps/tom-node-tvos/build/Build/Products/Debug/TomNode-macOS.app"
DEVICE_OFF = "ipad"          # device éteint par l'humain à t+4
SPAM_TARGET = "mac"          # cible du spam t+8 (vivant à cet instant)
KILL_VICTIM = "mac"

# Tailles étiquetées pour lever l'ambiguïté au collecteur (pattern #7) :
SIZE_FOND = 1024             # « 1 Ko » — charge de fond
SIZE_KILL_WAVE = 2048        # « 2 Ko » — vague pendant l'absence du Mac killé
SIZE_OFF_WAVE = 3072         # « 3 Ko » — messages vers le device éteint (R2)


def say(msg):
    """Annonce vocale macOS (non bloquante) + print — les ordres humains.
    Volume forcé à 75 % avant chaque annonce (run 1 : Mac muet → créneau
    d'extinction raté ; l'annonce inaudible invalide l'oracle, pas le réseau)."""
    print(f"\n📢 {msg}", flush=True)
    subprocess.run(["/usr/bin/osascript", "-e", "set volume output volume 75"],
                   check=False)
    subprocess.Popen(["/usr/bin/say", "-v", "Thomas", msg])


def status_full(name):
    d = http_json(NODES[name]["status"])
    return None if "_erreur" in d else d


class LoadStats:
    """Envois de fond horodatés (fenêtres V1) + envois vers le device éteint."""

    def __init__(self):
        self.lock = threading.Lock()
        self.sends = []          # (t_rel, target, count, ok)
        self.off_sends = 0       # messages envoyés vers DEVICE_OFF pendant l'absence

    def add(self, t_rel, target, count, ok):
        with self.lock:
            self.sends.append((t_rel, target, count, ok))

    def add_off(self, n):
        with self.lock:
            self.off_sends += n

    def sent_window(self, t0_rel, t1_rel, targets):
        with self.lock:
            return sum(c for (t, tg, c, ok) in self.sends
                       if ok and t0_rel <= t < t1_rel and tg in targets)


def background_load(stop, stats, targets_ids, t_start, off_state):
    """Toutes les 10 s : NAS /send 5×1 Ko vers chaque cible vivante attendue.
    Vers DEVICE_OFF pendant son absence : 3 Ko (étiquette R2), comptés à part."""
    while not stop.is_set():
        for name, tid in targets_ids.items():
            if tid is None:
                continue
            t_rel = time.time() - t_start
            if name == DEVICE_OFF and off_state["off"]:
                r = http_json(f"{NAS_CONTROL}/send?to={tid}&size={SIZE_OFF_WAVE}&count=5")
                if r.get("ok"):
                    stats.add_off(5)
            else:
                r = http_json(f"{NAS_CONTROL}/send?to={tid}&size={SIZE_FOND}&count=5")
                stats.add(t_rel, name, 5, bool(r.get("ok")))
        stop.wait(10)


def releves_thread(stop, releves, t_start):
    """Toutes les 30 s : compteurs + pairs_connectes de chaque nœud (V1+V3)."""
    while not stop.is_set():
        t_rel = time.time() - t_start
        snap = {}
        for n in NODES:
            st = status_full(n)
            c = counters(n)
            snap[n] = {
                "counters": c,
                "pairs": (st or {}).get("pairs_connectes", []),
                "taille_reseau": (st or {}).get("taille_reseau"),
            }
        releves.append((t_rel, snap))
        stop.wait(30)


def wait_state(name, want_up, timeout_s):
    """Attend que `name` soit up (want_up) ou down, retourne le t de bascule."""
    t0 = time.time()
    while time.time() - t0 < timeout_s:
        up = counters(name) is not None
        if up == want_up:
            return time.time()
        time.sleep(2)
    return None


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--dry-run", action="store_true")
    ap.add_argument("--seed", type=int, default=42)
    args = ap.parse_args()

    print(f"Capstone Phase C — {time.strftime('%F %T')} — "
          f"{'DRY-RUN (briques)' if args.dry_run else 'RUN COMPLET 15 min'}")

    # ── Préconditions : figer la flotte vivante ──
    alive = {n: counters(n) is not None for n in NODES}
    print("Flotte :", alive)
    vivants = [n for n, up in alive.items() if up]
    requis = {"nas", "mac", "iphone", "ipad"}
    if not requis.issubset(set(vivants)):
        print(f"ABORT : flotte incomplète (requis {requis}, up {vivants})")
        print("ORCH_EXIT=1 (précondition)")
        sys.exit(1)
    ids = {n: node_id(n) for n in vivants if n != "nas"}
    flotte_ids = {node_id(n) for n in vivants if node_id(n)}
    flotte_ids.add(status_full("nas")["node_id"])
    print("IDs figés :", {n: (i or "?")[:8] for n, i in ids.items()})

    rep = Report(args.seed)
    stats = LoadStats()
    releves = []
    off_state = {"off": False}
    stop = threading.Event()
    t_start = time.time()

    th_load = threading.Thread(
        target=background_load, args=(stop, stats, ids, t_start, off_state), daemon=True)
    th_rel = threading.Thread(
        target=releves_thread, args=(stop, releves, t_start), daemon=True)

    # ── Baseline pré-enregistrée (60 s de charge seule) ──
    base_snap = {n: counters(n) for n in vivants}
    th_load.start()
    th_rel.start()
    base_len = 30 if args.dry_run else 60
    print(f"\n▶ Baseline {base_len} s (charge de fond seule)…")
    time.sleep(base_len)
    base_after = {n: counters(n) for n in vivants}
    base_sent = stats.sent_window(0, base_len, set(ids))
    base_recv = sum(
        (base_after[n]["recus"] - base_snap[n]["recus"])
        for n in ids if base_after.get(n) and base_snap.get(n))
    base_rate = (base_recv / base_sent) if base_sent else 0.0
    # Cible = baseline réelle − 10 pts, plancher 70 % (run 1 : plancher 85 %
    # au-dessus de la baseline réelle 76 % = oracle infaillible à échouer).
    cible_v1 = max(0.70, round(base_rate - 0.10, 2))
    print(f"  Baseline : {base_recv}/{base_sent} livrés ({base_rate:.0%}) "
          f"→ CIBLE V1 PRÉ-ENREGISTRÉE : ≥ {cible_v1:.0%} par fenêtre 60 s")
    t0 = time.time()

    def t_rel():
        return time.time() - t0

    inconnu = {"proc": None, "id": None, "verdict": None, "out": ""}
    spam = {"offert": 0, "recv_delta": None, "victime_ok": None}
    kill_seq_off = None
    off_events = {"t_off": None, "t_back": None, "recv_at_back": None}

    if args.dry_run:
        # Briques seules : burst court (le spam compte-t-il ?) + relevés.
        print("\n▶ [dry] burst 20×512 → mac (les enveloppes comptent-elles ?)")
        mac_before = counters("mac")["recus"]
        env = dict(os.environ, TOM_RELAY_URL="http://192.168.0.83:3340")
        subprocess.run(
            [TOM_STRESS, "burst", "--connect", ids["mac"], "--count", "20",
             "--payload-size", "512"],
            env=env, capture_output=True, text=True, timeout=90)
        time.sleep(5)
        mac_delta = counters("mac")["recus"] - mac_before
        rep.add("dry", "burst-compte", mac_delta >= 10,
                f"mac recus +{mac_delta}/20 après burst (si ~0 : les enveloppes "
                f"stress-burst ne comptent pas → adapter l'oracle spam)")
        time.sleep(10)
        stop.set()
        rep.add("dry", "relevés", len(releves) >= 1, f"{len(releves)} relevés pris")
        fin_rapport(rep, releves, args)
        return

    # ══ SÉQUENCE (t0 = fin de baseline) ══
    say("Capstone démarré. Prochaine action humaine dans 4 minutes.")

    # t+120 : kill Mac + vague 2 Ko pendant l'absence + relance t+150
    while t_rel() < 120:
        time.sleep(1)
    print(f"\n▶ t+{t_rel():.0f}s : SIGKILL {KILL_VICTIM} + vague 2 Ko pendant l'absence")
    kill_seq_off = collector_lines()
    subprocess.run(["/usr/bin/pkill", "-9", "-f", "TomNode-macOS.app"], check=False)
    http_json(f"{NAS_CONTROL}/send?to={ids['mac']}&size={SIZE_KILL_WAVE}&count=10")
    while t_rel() < 150:
        time.sleep(1)
    subprocess.run(["/usr/bin/open", MAC_APP], check=False)
    t_back_mac = wait_state("mac", True, 60)
    rep.add("capstone", "kill-retour",
            t_back_mac is not None, "mac revenu ≤ 60 s" if t_back_mac else "mac PAS revenu")

    # t+240 : ordre humain — extinction du device
    while t_rel() < 235:
        time.sleep(1)
    say(f"Éteins le {DEVICE_OFF} maintenant. Mode avion ou extinction complète.")
    t_off = wait_state(DEVICE_OFF, False, 90)
    if t_off:
        off_state["off"] = True
        off_events["t_off"] = t_off - t0
        print(f"  {DEVICE_OFF} éteint détecté à t+{t_off - t0:.0f}s — "
              f"les envois continuent en 3 Ko (backup R2)")
    rep.add("capstone", "extinction-détectée", t_off is not None,
            f"{DEVICE_OFF} down à t+{(t_off - t0):.0f}s" if t_off else "extinction NON détectée en 90 s")

    # t+360 : l'inconnu rejoint par le rendez-vous RÉEL
    while t_rel() < 360:
        time.sleep(1)
    print(f"\n▶ t+{t_rel():.0f}s : l'inconnu rejoint (rendez-vous réel, probing off)")
    inconnu["proc"] = subprocess.Popen(
        [TOM_STRESS, "r4", "--production-rendezvous",
         "--timeout-secs", "240", "--linger-secs", "120"],
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True)

    # t+480 : le spam (headless → Mac vivant)
    while t_rel() < 480:
        time.sleep(1)
    print(f"\n▶ t+{t_rel():.0f}s : spam headless → {SPAM_TARGET} (3×500×512o)")
    mac_before = counters(SPAM_TARGET)
    iph_before = counters("iphone")
    env = dict(os.environ, TOM_RELAY_URL="http://192.168.0.83:3340")
    spam_proc = subprocess.Popen(
        [TOM_STRESS, "burst", "--connect", ids[SPAM_TARGET], "--count", "500",
         "--payload-size", "512", "--rounds", "3", "--round-delay", "2000"],
        env=env, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    spam["offert"] = 1500

    # t+600 : ordre humain — rallumage (seulement si l'extinction a eu lieu :
    # run 1 = faux-PASS « retour » sur un device jamais éteint)
    while t_rel() < 595:
        time.sleep(1)
    if off_events["t_off"] is not None:
        say(f"Rallume le {DEVICE_OFF} maintenant, et rouvre l'application.")
        t_back = wait_state(DEVICE_OFF, True, 180)
        if t_back:
            off_state["off"] = False
            off_events["t_back"] = t_back - t0
            off_events["recv_at_back"] = counters(DEVICE_OFF)["recus"]
        rep.add("capstone", "retour-détecté", t_back is not None,
                f"{DEVICE_OFF} up à t+{(t_back - t0):.0f}s" if t_back else "retour NON détecté en 180 s")
    else:
        rep.add("capstone", "retour-détecté", False,
                "SKIP : extinction jamais détectée, pas de retour à mesurer")

    # Fin du spam : la mesure vient des COMPTEURS, le process peut être tué
    # (les apps ne renvoient pas d'échos stress-burst → le collect du burst
    # attendrait son plein timeout : crash TimeoutExpired du run 1).
    try:
        spam_proc.wait(timeout=30)
    except subprocess.TimeoutExpired:
        spam_proc.kill()
        print("  (burst tué après la fenêtre de mesure — attendu, pas d'échos)")
    time.sleep(5)
    mac_after = counters(SPAM_TARGET)
    iph_after = counters("iphone")
    if mac_before and mac_after:
        spam["recv_delta"] = mac_after["recus"] - mac_before["recus"]
    if iph_before and iph_after:
        # La victime témoin continue de recevoir son fond (~5/10 s pendant ~2,5 min)
        spam["victime_ok"] = (iph_after["recus"] - iph_before["recus"]) >= 30

    # t+900 : fin de charge, drain
    while t_rel() < 900:
        time.sleep(1)
    print(f"\n▶ t+900s : fin de charge, drain 90 s")
    stop.set()
    time.sleep(90)

    # ── V2 : preuves par mécanisme ──
    lines_mac = collector_since(kill_seq_off, "Mac", "MSG from")
    v_kill = seq_counts(lines_mac, "2 Ko")
    rep.add("V2", "kill-backup-rejoue", len(v_kill) >= 10,
            f"vague 2 Ko pendant absence Mac : {len(v_kill)}/10 seq revus au collecteur")

    if off_events["t_back"] and off_events["t_off"]:
        # Livraison différée : le device éteint reçoit les 3 Ko backupés à son retour
        time.sleep(30)
        recv_final = counters(DEVICE_OFF)
        differes = (recv_final["recus"] - off_events["recv_at_back"]) if recv_final else 0
        attendu = stats.off_sends
        rep.add("V2", "extinction-livraison-différée",
                attendu > 0 and (differes + (off_events["recv_at_back"] or 0)) >= 0 and differes >= max(1, int(attendu * 0.5)),
                f"{DEVICE_OFF} : {differes} reçus post-retour (3 Ko envoyés pendant absence : {attendu})")
    else:
        rep.add("V2", "extinction-livraison-différée", False, "extinction/retour non détectés")

    out, _ = inconnu["proc"].communicate(timeout=180)
    inconnu["out"] = out or ""
    m = re.search(r"Node: ([0-9a-f]{64})", inconnu["out"])
    inconnu["id"] = m.group(1) if m else None
    src_ok = "R4.source-rendez-vous : DiscoveryTiming" in inconnu["out"] and "PASS" in inconnu["out"]
    seen = False
    if inconnu["id"]:
        for (_t, snap) in releves:
            for n, s in snap.items():
                if any(p.startswith(inconnu["id"][:8]) for p in (s["pairs"] or [])):
                    seen = True
    rep.add("V2", "inconnu-source-rendezvous", src_ok,
            "stdout inconnu : source=rendez-vous PASS" if src_ok else "PASS absent du stdout inconnu")
    rep.add("V2", "inconnu-vu-par-la-flotte", seen,
            f"id {inconnu['id'][:8] if inconnu['id'] else '?'} vu dans pairs_connectes d'un vivant : {seen}")

    throttled = spam["recv_delta"] is not None and spam["recv_delta"] < spam["offert"] // 2
    rep.add("V2", "spam-freiné", throttled,
            f"{SPAM_TARGET} recus +{spam['recv_delta']}/{spam['offert']} offerts (throttle attendu < 50 %)")
    rep.add("V2", "spam-victimes-intactes", bool(spam["victime_ok"]),
            f"iphone continue de recevoir son fond pendant le spam : {spam['victime_ok']}")

    # ── V1 : livraison de fond par fenêtre 60 s ──
    v1_ok, v1_detail = verdict_v1(stats, releves, ids, cible_v1, off_events)
    rep.add("V1", "livraison-fond-continue", v1_ok, v1_detail)

    # ── V3 : pas de fantôme parmi les connectés ──
    acteurs = set(flotte_ids)
    if inconnu["id"]:
        acteurs.add(inconnu["id"])
    intrus = set()
    for (_t, snap) in releves:
        for n, s in snap.items():
            for p in (s["pairs"] or []):
                if not any(a.startswith(p[:8]) or p.startswith(a[:8]) for a in acteurs):
                    intrus.add(f"{n}:{p[:8]}")
    # Le spammeur headless est un acteur légitime du run (id non capturé) :
    # tolérance 1 id inconnu au plus, documentée.
    rep.add("V3", "pas-de-fantôme-connecté", len(intrus) <= 1,
            f"ids hors flotte+acteurs vus connectés : {sorted(intrus) or 'aucun'} (≤1 toléré = spammeur)")

    fin_rapport(rep, releves, args)


def verdict_v1(stats, releves, ids, cible, off_events):
    """Par fenêtre 60 s : Σ delta recus vivants / Σ envoyés de fond ≥ cible.
    Fenêtres d'un nœud mort (kill 120-150, extinction) : nœud exclu de SA fenêtre."""
    if len(releves) < 4:
        return False, f"relevés insuffisants ({len(releves)})"
    pires = []
    # Fenêtres de 60 s reconstruites depuis les relevés 30 s (pas de fenêtre baseline)
    by_min = {}
    for (t, snap) in releves:
        by_min.setdefault(int(t // 60), []).append((t, snap))
    for w, pts in sorted(by_min.items()):
        if w < 0 or len(pts) < 2:
            continue
        (t_a, a), (t_b, b) = pts[0], pts[-1]
        recv = sent = 0
        for n in ids:
            ca, cb = a.get(n, {}).get("counters"), b.get(n, {}).get("counters")
            if not ca or not cb or cb["recus"] < ca["recus"]:
                continue  # nœud mort/reset sur la fenêtre → exclu (kill, extinction)
            recv += cb["recus"] - ca["recus"]
            sent += stats.sent_window(t_a, t_b, {n})
        if sent >= 10:
            rate = recv / sent
            pires.append((w, rate))
    if not pires:
        return False, "aucune fenêtre mesurable"
    worst = min(pires, key=lambda x: x[1])
    ok = worst[1] >= cible
    return ok, (f"pire fenêtre : min {worst[0]} à {worst[1]:.0%} "
                f"(cible {cible:.0%}, {len(pires)} fenêtres)")


def fin_rapport(rep, releves, args):
    print("\n── Rapport capstone ──")
    for r in rep.results:
        print(f"{r['verdict']:4} {r['scenario']:9} {r['invariant']:28} {r['preuve']}")
    out = f"/tmp/capstone_report_{int(time.time())}.json"
    with open(out, "w") as f:
        json.dump({"seed": args.seed, "dry_run": args.dry_run,
                   "results": rep.results, "releves": len(releves)}, f,
                  indent=2, ensure_ascii=False)
    print(f"Rapport JSON : {out}")
    nfail = len(rep.failed())
    print(f"ORCH_EXIT={1 if nfail else 0} ({nfail} FAIL)")
    sys.exit(1 if nfail else 0)


if __name__ == "__main__":
    main()
