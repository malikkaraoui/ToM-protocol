#!/usr/bin/env python3
"""Orchestrateur étage F — banc de test chaos sur la VRAIE flotte.

V1 (18/07) : scénarios #1 sanity, #2 broadcast, #3 message lourd, #7 kill, #8 retour.
Pilotage : control NAS :9300 (/send /sendall /metrics), status apps :9091 (/).
Réf : docs/plans/banc-test-chaos.md §5-§7 (invariants I1-I8, garde-fous 17-18/07).

Garde-fous gravés :
- chemins absolus (PATH se corrompt en session longue) ;
- verdict par invariant, jamais « ça a l'air d'aller » ;
- compteurs :9091/:9300 = vérité (le collecteur UDP perd des paquets) ;
- fenêtre témoin AVANT trafic pour mesurer le bruit ambiant (bot NAS ping en fond) ;
- marqueur ORCH_EXIT en dernière ligne (notifications background menteuses).

Usage : python3 scripts/chaos/orchestrator.py [--seed N] [--scenarios 1,2,3,7,8]
"""

import argparse
import json
import random
import subprocess
import sys
import time
import urllib.request

CURL_TIMEOUT = 6

# ── Flotte (IP LAN 18/07 — le NAS est en DHCP, vérifier si injoignable) ──────
NAS_CONTROL = "http://192.168.0.83:9300"
NODES = {
    "nas": {"status": "http://192.168.0.83:8085/status", "kind": "nas"},
    "mac": {"status": "http://127.0.0.1:9091/", "kind": "app"},
    "iphone": {"status": "http://192.168.0.28:9091/", "kind": "app"},
    "ipad": {"status": "http://192.168.0.23:9091/", "kind": "app"},
    # iPhone de Laura, ajouté à la flotte le 18/07 soir (build 125).
    # ⚠️ Son champ "node" vaut aussi "iPhone" — dans le collecteur les deux
    # iPhone sont indiscernables par le nom ; distinguer par node_id
    # (Malik 80eb9196…, Laura b76a43d2…) ou par IP.
    "iphone-laura": {"status": "http://192.168.0.49:9091/", "kind": "app"},
    # ATV retirée de la flotte 137 (20/07 soir, remplacée par iPhone Laura —
    # cf handoff/PROMPT-REPRISE-ROLES §2 ; sondée injoignable sur .76:9091).
    # Réintégrer ici si elle revient :
    # "atv": {"status": "http://192.168.0.76:9091/", "kind": "app"},
}


def http_json(url):
    """GET url → dict, None si injoignable (jamais d'exception qui tue le run)."""
    try:
        with urllib.request.urlopen(url, timeout=CURL_TIMEOUT) as r:
            return json.loads(r.read().decode())
    except Exception as e:  # noqa: BLE001 — un nœud down est un ÉTAT, pas un crash
        return {"_erreur": str(e)}


def counters(name):
    """Compteurs normalisés d'un nœud : {envoyes, recus, echoues} ou None si down."""
    node = NODES[name]
    d = http_json(node["status"])
    if "_erreur" in d:
        return None
    if node["kind"] == "nas":
        # status :8085 n'a pas les compteurs — control /metrics oui
        m = http_json(f"{NAS_CONTROL}/metrics")
        if "_erreur" in m:
            return None
        return {"envoyes": m["envoyes"], "recus": m["recus"], "echoues": m["echoues"]}
    return {
        "envoyes": d.get("messages_envoyes", 0),
        "recus": d.get("messages_recus", 0),
        "echoues": d.get("messages_echoues", 0),
    }


def node_id(name):
    d = http_json(NODES[name]["status"])
    return d.get("node_id")


def snapshot():
    return {n: counters(n) for n in NODES}


def ambient_window(seconds):
    """Fenêtre témoin : mesure le bruit de fond (bot NAS, heartbeats applicatifs)."""
    before = snapshot()
    time.sleep(seconds)
    after = snapshot()
    noise = {}
    for n in NODES:
        if before[n] and after[n]:
            noise[n] = {k: after[n][k] - before[n][k] for k in before[n]}
        else:
            noise[n] = None
    return noise


class Report:
    def __init__(self, seed):
        self.seed = seed
        self.results = []  # [{scenario, invariant, verdict, preuve}]

    def add(self, scenario, invariant, ok, preuve):
        self.results.append(
            {"scenario": scenario, "invariant": invariant,
             "verdict": "PASS" if ok else "FAIL", "preuve": preuve}
        )
        print(f"  [{'PASS' if ok else 'FAIL'}] {scenario} · {invariant} — {preuve}")

    def failed(self):
        return [r for r in self.results if r["verdict"] == "FAIL"]


def wait_settle(seconds=5):
    time.sleep(seconds)


# ── Scénarios ────────────────────────────────────────────────────────────────

def scenario_1_sanity(rep, target="iphone", count=20, size=1024):
    """#1 Sanity unicast NAS→cible. Inv I1 (tout livré), I2 (pas de fantôme)."""
    print(f"\n▶ #1 sanity unicast NAS→{target} ({count}×{size}o)")
    tid = node_id(target)
    if not tid:
        rep.add("#1", "précondition", False, f"{target} injoignable")
        return
    noise = ambient_window(8)
    base = snapshot()
    r = http_json(f"{NAS_CONTROL}/send?to={tid}&size={size}&count={count}")
    if not r.get("ok"):
        rep.add("#1", "I1", False, f"/send a échoué : {r}")
        return
    wait_settle(8)
    cur = snapshot()
    d_recv = cur[target]["recus"] - base[target]["recus"]
    d_fail = cur[target]["echoues"] - base[target]["echoues"]
    amb = noise[target]["recus"] if noise[target] else "?"
    rep.add("#1", "I1", d_recv >= count,
            f"{target} recus +{d_recv}/{count} (ambiant témoin 8s: +{amb})")
    rep.add("#1", "I2", d_recv <= count + max(amb if isinstance(amb, int) else 0, 0) + 2,
            f"pas plus de reçus que d'envoyés+ambiant (delta {d_recv}, envoyés {count})")
    rep.add("#1", "I5", d_fail == 0, f"{target} echoues delta {d_fail}")


def scenario_2_broadcast(rep, count=5, size=1024):
    """#2 Broadcast /sendall. Inv I1 sur chaque pair connecté du NAS."""
    print(f"\n▶ #2 broadcast /sendall ({count}×{size}o)")
    base = snapshot()
    r = http_json(f"{NAS_CONTROL}/sendall?size={size}&count={count}")
    if not r.get("ok"):
        rep.add("#2", "I1", False, f"/sendall a échoué : {r}")
        return
    npeers = r.get("pairs", 0)
    print(f"  NAS a envoyé {r.get('envoyes')} messages à {npeers} pairs")
    wait_settle(8)
    cur = snapshot()
    for n in NODES:
        if n == "nas" or base[n] is None:
            continue
        d = cur[n]["recus"] - base[n]["recus"]
        # Un pair peut être connu du NAS sans être dans NODES (iPad 107, etc.) —
        # on ne vérifie que les nœuds qu'on observe.
        rep.add("#2", "I1", d >= count, f"{n} recus +{d}/{count}")


def scenario_3_heavy(rep, target="mac", sizes=(1048576, 2097152, 4194304, 8388608)):
    """#3 Message lourd, ladder géométrique 1→8 Mo. Inv I1 + pas d'échec (plafond 1 MiB?
    la doc DoS dit plafond wire 1 MiB post-fix — le ladder DOIT rapporter la vérité)."""
    print(f"\n▶ #3 message lourd NAS→{target} ladder {[s // 1048576 for s in sizes]} Mo")
    tid = node_id(target)
    if not tid:
        rep.add("#3", "précondition", False, f"{target} injoignable")
        return
    for size in sizes:
        base = snapshot()
        r = http_json(f"{NAS_CONTROL}/send?to={tid}&size={size}&count=1")
        wait_settle(10)
        cur = snapshot()
        d = cur[target]["recus"] - base[target]["recus"]
        mb = size / 1048576
        if r.get("ok"):
            rep.add("#3", "I1", d >= 1, f"{mb:g} Mo : recus +{d} (send ok)")
        else:
            # Refus émetteur = comportement anti-DoS documenté, pas une perte silencieuse
            rep.add("#3", "I1", True,
                    f"{mb:g} Mo : REFUSÉ à l'émission ({r.get('erreur', r)}) — plafond assumé")


COLLECTOR = "/tmp/tom_collector.log"


def collector_lines():
    """Nombre de lignes actuel du collecteur (offset de fenêtre)."""
    r = subprocess.run(["/usr/bin/grep", "-c", "", COLLECTOR],
                       capture_output=True, text=True, check=False)
    try:
        return int(r.stdout.strip())
    except ValueError:
        return 0


def collector_since(offset, node_label, needle):
    """Lignes du collecteur APRÈS l'offset, pour un nœud, contenant needle.
    ⚠️ Jamais de grep par heure : le log est multi-jours sans date (piège 18/07)."""
    out = subprocess.run(
        ["/usr/bin/awk", f"NR>{offset} && /{node_label} / && /{needle}/", COLLECTOR],
        capture_output=True, text=True, check=False)
    return [l for l in out.stdout.splitlines() if l.strip()]


def seq_counts(lines, size_label):
    """Compte par seq (#n) les lignes 'MSG from … 📦 <size_label> #n'."""
    counts = {}
    for l in lines:
        if size_label not in l or "#" not in l:
            continue
        seq = l.rsplit("#", 1)[-1].strip()
        if seq.isdigit():
            counts[int(seq)] = counts.get(int(seq), 0) + 1
    return counts


def scenario_7_kill(rep, victim="mac"):
    """#7 Kill brutal du récepteur en plein trafic + #8 retour.
    Mesure par SEQ AU COLLECTEUR (les compteurs :9091 ne survivent pas au
    SIGKILL — throttle de persistance : faux FAIL du 18/07). Deux vagues à
    tailles DISTINCTES (1 Ko pré-kill / 2 Ko pendant absence) pour lever
    l'ambiguïté des seq identiques."""
    print(f"\n▶ #7 kill brutal {victim} en plein trafic + #8 retour")
    tid = node_id(victim)
    if not tid:
        rep.add("#7", "précondition", False, f"{victim} injoignable")
        return
    label = {"mac": "Mac"}.get(victim, victim)
    off = collector_lines()
    # Vague 1 (1 Ko ×10) : livraison à l'instance VIVANTE (2 s de grâce)
    http_json(f"{NAS_CONTROL}/send?to={tid}&size=1024&count=10")
    time.sleep(2)
    subprocess.run(["/usr/bin/pkill", "-9", "-f", "TomNode-macOS.app"], check=False)
    t_kill = time.time()
    print(f"  SIGKILL à t0 ; vague 2 (2 Ko ×20) pendant l'absence")
    http_json(f"{NAS_CONTROL}/send?to={tid}&size=2048&count=20")
    time.sleep(10)
    # #8 retour
    # ⚠️ Chemin = celui du process OBSERVÉ en live (ps + app_build :9091), PAS .build/xcode/
    # (audit Phase 0 20/07 : deux bundles à md5 différents coexistaient — relancer l'autre
    # aurait ressuscité un binaire non vérifié, piège du « vieux 51 »).
    subprocess.run(
        ["/usr/bin/open",
         "/Users/malik/Documents/tom-protocol/apps/tom-node-tvos/build/Build/Products/Debug/TomNode-macOS.app"],
        check=False)
    t_back = None
    for _ in range(60):
        if counters(victim) is not None:
            t_back = time.time()
            break
        time.sleep(1)
    if t_back is None:
        rep.add("#8", "retour", False, "le nœud n'est jamais revenu (60 s)")
        return
    rep.add("#8", "retour", True, f"nœud up {t_back - t_kill:.1f} s après kill (10 s d'absence voulue)")
    # I1 : vague 1 livrée AVANT le kill ; vague 2 rejouée par backup APRÈS
    deadline = time.time() + 90
    v1 = v2 = {}
    while time.time() < deadline:
        lines = collector_since(off, label, "MSG from")
        v1 = seq_counts(lines, "1 Ko")
        v2 = seq_counts(lines, "2 Ko")
        if len(v1) >= 10 and len(v2) >= 20:
            break
        time.sleep(3)
    rep.add("#7", "I1-v1", len(v1) >= 10,
            f"vague pré-kill : {len(v1)}/10 seq vus au collecteur")
    rep.add("#7", "I1-v2", len(v2) >= 20,
            f"vague absence : {len(v2)}/20 seq rejoués par backup")
    dups = {s: c for s, c in {**v1, **v2}.items() if c > 1}
    rep.add("#7", "I8-dédup", not dups,
            f"doublons de livraison : {dups if dups else 'aucun'}")
    # I3 : le pair est re-vu par le NAS
    peers = http_json(f"{NAS_CONTROL}/peers")
    ok = tid[:8] in json.dumps(peers)
    rep.add("#7", "I3", ok, f"NAS re-voit {victim} ({tid[:8]}) : {ok}")


# ── Main ─────────────────────────────────────────────────────────────────────

def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--seed", type=int, default=42)
    ap.add_argument("--scenarios", default="1,2,3,7")
    args = ap.parse_args()
    random.seed(args.seed)

    print(f"Orchestrateur étage F — seed {args.seed} — {time.strftime('%F %T')}")
    print("Flotte observée :", ", ".join(NODES))

    # Préconditions : tous les nœuds observés répondent
    up = {n: counters(n) is not None for n in NODES}
    print("État initial :", up)

    rep = Report(args.seed)
    wanted = set(args.scenarios.split(","))
    if "1" in wanted:
        scenario_1_sanity(rep)
    if "2" in wanted:
        scenario_2_broadcast(rep)
    if "3" in wanted:
        scenario_3_heavy(rep)
    if "7" in wanted:
        scenario_7_kill(rep)

    print("\n── Rapport ──")
    for r in rep.results:
        print(f"{r['verdict']:4} {r['scenario']:3} {r['invariant']:12} {r['preuve']}")
    out = f"/tmp/chaos_report_{int(time.time())}.json"
    with open(out, "w") as f:
        json.dump({"seed": args.seed, "results": rep.results}, f, indent=2, ensure_ascii=False)
    print(f"Rapport JSON : {out}")
    nfail = len(rep.failed())
    print(f"ORCH_EXIT={1 if nfail else 0} ({nfail} FAIL)")
    sys.exit(1 if nfail else 0)


if __name__ == "__main__":
    main()
