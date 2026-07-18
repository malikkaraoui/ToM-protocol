#!/usr/bin/env python3
"""Matrice des chemins réseau de la flotte — familles v4/v6 et bascules dans le temps.

Pourquoi cet outil (R14, 2026-07-18) : la première « baseline v4/v6 » du chantier a
été tirée d'UN snapshot et concluait « IPv6 majoritaire, l'iPhone est l'exception ».
Trois relevés plus tard, la conclusion s'effondrait : presque chaque chemin change de
famille en une heure, et le système ne converge pas vers le meilleur lien (un chemin
v4 à 9 ms est devenu v6 à 51 ms). Sur un état instable, un snapshot ne prouve rien.

Cet outil interdit cette erreur : il prend N relevés espacés et rapporte les
CHANGEMENTS, pas une photo. C'est le volet outillage du Lot A (observabilité) —
sans toucher au code Rust.

Garde-fous repris de l'orchestrateur :
- chemins absolus / pas de dépendance au PATH ;
- un nœud injoignable est un ÉTAT rapporté, jamais une exception qui tue le run ;
- on lit :9091 (vérité compteurs), pas le collecteur UDP (qui perd des paquets).

Usage :
  python3 scripts/path-matrix.py                    # 3 relevés espacés de 60 s
  python3 scripts/path-matrix.py --n 5 --interval 30
  python3 scripts/path-matrix.py --json /tmp/paths.json
"""

import argparse
import json
import sys
import time
import urllib.request

CURL_TIMEOUT = 6

# Flotte au 18/07 (IP LAN ; le NAS est en DHCP, vérifier si injoignable).
NODES = {
    "mac": "http://127.0.0.1:9091/",
    "iphone": "http://192.168.0.28:9091/",
    "ipad": "http://192.168.0.23:9091/",
    "iphone-laura": "http://192.168.0.49:9091/",
    # NAS : /status n'expose pas paths_by_peer au même format — traité à part si besoin.
}

# node_id (préfixe 8) → nom lisible. Les deux iPhone portent le même champ "node",
# donc le nom NE PEUT PAS servir d'identifiant : on passe par le node_id.
KNOWN = {
    "4acfab2b": "mac",
    "80eb9196": "iphone",
    "01f9bff9": "ipad",
    "11d5bb11": "nas",
    "b76a43d2": "iphone-laura",
}


def family(addr: str) -> str:
    """Famille d'une adresse de chemin : v6 / v4 / relay."""
    if not addr:
        return "?"
    if addr.startswith("["):
        return "v6"
    if addr.startswith("relay") or "http" in addr:
        return "relay"
    return "v4"


def http_json(url):
    try:
        with urllib.request.urlopen(url, timeout=CURL_TIMEOUT) as r:
            return json.loads(r.read().decode())
    except Exception as e:  # noqa: BLE001 — un nœud down est un état, pas un crash
        return {"_erreur": str(e)}


def snapshot():
    """{observateur: {pair: {famille, kind, rtt, addr}}} — pairs down inclus en _erreur."""
    out = {}
    for name, url in NODES.items():
        d = http_json(url)
        if "_erreur" in d:
            out[name] = {"_erreur": d["_erreur"]}
            continue
        paths = {}
        for peer_id, info in (d.get("paths_by_peer") or {}).items():
            addr = info.get("addr", "")
            paths[KNOWN.get(peer_id[:8], peer_id[:8])] = {
                "famille": family(addr),
                "kind": info.get("kind"),
                "rtt": info.get("rtt_ms"),
                "addr": addr,
            }
        out[name] = {
            "build": d.get("app_build"),
            "phase": d.get("phase"),
            "paths": paths,
        }
    return out


def diff(a, b):
    """Changements de famille ou de kind entre deux relevés."""
    changes = []
    for obs, cur in b.items():
        prev = a.get(obs, {})
        if "_erreur" in cur or "_erreur" in prev:
            continue
        for peer, info in cur.get("paths", {}).items():
            old = prev.get("paths", {}).get(peer)
            if old is None:
                changes.append(f"{obs} → {peer} : NOUVEAU chemin {info['famille']} {info['rtt']}ms")
            elif old["famille"] != info["famille"]:
                changes.append(
                    f"{obs} → {peer} : BASCULE {old['famille']} {old['rtt']}ms "
                    f"→ {info['famille']} {info['rtt']}ms"
                    + ("  ⚠️ DÉGRADATION" if (info["rtt"] or 0) > (old["rtt"] or 0) else "")
                )
            elif old["kind"] != info["kind"]:
                changes.append(f"{obs} → {peer} : kind {old['kind']} → {info['kind']}")
        for peer in prev.get("paths", {}):
            if peer not in cur.get("paths", {}):
                changes.append(f"{obs} → {peer} : chemin DISPARU")
    return changes


def asymmetries(snap):
    """Paires A↔B dont les deux sens n'utilisent pas la même famille (ou un seul sens existe).

    Mesuré le 18/07 : ipad→iphone en v4/6ms alors que iphone→ipad est en v6/15ms.
    Chaque sens fait son propre tirage de chemin — c'est le symptôme direct du
    probe à ordre aléatoire (cf. docs/plans/r14-ipv6-first-class.md §1bis).
    """
    out = []
    seen = set()
    for a, da in snap.items():
        if "_erreur" in da:
            continue
        for b, info_ab in da.get("paths", {}).items():
            if b not in snap or "_erreur" in snap.get(b, {"_erreur": 1}):
                continue
            key = tuple(sorted((a, b)))
            if key in seen:
                continue
            seen.add(key)
            info_ba = snap[b].get("paths", {}).get(a)
            if info_ba is None:
                out.append(f"{a} voit {b} ({info_ab['famille']}) mais {b} ne voit PAS {a}")
            elif info_ab["famille"] != info_ba["famille"]:
                out.append(
                    f"{a}→{b} {info_ab['famille']}/{info_ab['rtt']}ms  ≠  "
                    f"{b}→{a} {info_ba['famille']}/{info_ba['rtt']}ms"
                )
    return out


def render(snap):
    lines = []
    for obs, data in snap.items():
        if "_erreur" in data:
            lines.append(f"  {obs:<14} INJOIGNABLE ({data['_erreur'][:60]})")
            continue
        paths = data.get("paths", {})
        desc = ", ".join(
            f"{p}={i['famille']}/{i['rtt']}ms" for p, i in sorted(paths.items())
        )
        lines.append(f"  {obs:<14} build {data.get('build')} {data.get('phase'):<10} {desc}")
    return "\n".join(lines)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=3, help="nombre de relevés (défaut 3)")
    ap.add_argument("--interval", type=int, default=60, help="secondes entre relevés")
    ap.add_argument("--json", help="écrire les relevés bruts dans ce fichier")
    args = ap.parse_args()

    snaps = []
    for i in range(args.n):
        s = snapshot()
        snaps.append(s)
        print(f"\n── Relevé {i + 1}/{args.n} ({time.strftime('%H:%M:%S')}) ──")
        print(render(s))
        if i < args.n - 1:
            time.sleep(args.interval)

    print("\n── Bascules détectées ──")
    total = 0
    for i in range(1, len(snaps)):
        ch = diff(snaps[i - 1], snaps[i])
        total += len(ch)
        for c in ch:
            print(f"  [{i}→{i + 1}] {c}")
    if total == 0:
        print("  aucune — chemins stables sur la fenêtre observée")

    print("\n── Asymétries de sens (dernier relevé) ──")
    asym = asymmetries(snaps[-1])
    for a in asym:
        print(f"  {a}")
    if not asym:
        print("  aucune — les deux sens s'accordent sur la famille")

    # Comptage des familles sur le dernier relevé (photo, à ne PAS sur-interpréter).
    fam = {}
    for data in snaps[-1].values():
        if "_erreur" in data:
            continue
        for info in data.get("paths", {}).values():
            fam[info["famille"]] = fam.get(info["famille"], 0) + 1
    print(f"\n── Familles au dernier relevé : {fam} ──")
    print(f"── {total} bascule(s) sur {args.n} relevés / {(args.n - 1) * args.interval}s ──")

    if args.json:
        with open(args.json, "w") as f:
            json.dump(snaps, f, indent=1)
        print(f"Relevés bruts : {args.json}")

    print("PATHMATRIX_EXIT=0")
    return 0


if __name__ == "__main__":
    sys.exit(main())
