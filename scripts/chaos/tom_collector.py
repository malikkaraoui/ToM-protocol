#!/usr/bin/env python3
"""Collecteur central de logs ToM — UDP :9999 → /tmp/tom_collector.log.

RECONSTITUÉ le 2026-07-20 (audit Phase 0) : l'original vivait dans /tmp et a été
PURGÉ par macOS pendant que son process tournait encore — ce fichier versionné
est désormais la source de vérité. Contrat prouvé sur pièces (sorties réelles) :

- bind 0.0.0.0:9999 (UDP) ; chaque datagramme est UNE ligne JSON (schéma Swift
  `appendLog` / Rust `BotContext::log_event`) ;
- sortie : `HH:MM:SS <node:10> peers=N disc=N path=K up=Ns | <detail>`,
  horodatée à la RÉCEPTION (horloge Mac = référentiel unique du banc) ;
- les datagrammes non-JSON sont JETÉS (c'est pour ça que tester le canal avec
  du texte brut « ne reçoit rien » — piège rencontré pendant l'audit) ;
- append en line-buffering (les outils lisent le fichier en quasi-direct).

⚠️ NE PAS lancer en double : vérifier d'abord `lsof -iUDP:9999`.
⚠️ Le log est multi-jours SANS date → les outils filtrent par OFFSET de ligne,
   jamais par heure (piège gravé, runbook §1).
"""
import json
import socket

LOG = "/tmp/tom_collector.log"
ERR = "/tmp/tom_collector_err.log"
PORT = 9999


def fmt(d: dict) -> str:
    node = str(d.get("node", "?"))[:10]
    peers = d.get("number_peers", d.get("taille_reseau"))
    disc = d.get("discovered_peers")
    path = d.get("path")
    up = d.get("uptime_s", 0)
    detail = str(d.get("detail", "")).strip()
    return f"{node:<10} peers={peers} disc={disc} path={path} up={up}s | {detail}"


def main() -> None:
    sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
    sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    sock.bind(("0.0.0.0", PORT))
    out = open(LOG, "a", buffering=1)
    err = open(ERR, "a", buffering=1)
    print(f"collecteur ToM sur UDP :{PORT} → {LOG}")
    while True:
        try:
            data, _addr = sock.recvfrom(65535)
            line = data.decode("utf-8", errors="replace").strip()
            if not line:
                continue
            d = json.loads(line)
            from time import strftime
            out.write(f"{strftime('%H:%M:%S')} {fmt(d)}\n")
        except json.JSONDecodeError:
            err.write(f"non-json: {line[:200]}\n")
        except Exception as e:  # noqa: BLE001 — le collecteur ne meurt jamais
            err.write(f"erreur: {e}\n")


if __name__ == "__main__":
    main()
