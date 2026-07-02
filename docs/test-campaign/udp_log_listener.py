#!/usr/bin/env python3
"""UDP JSON log collector for the ToM real-device test campaign.
Listens for broadcast/unicast JSON lines sent by TomNode iOS/tvOS apps
(udpLogHost/udpLogPort in Settings) and appends them, one per line with
a local receive timestamp, to a log file for later inspection.
"""
import socket
import sys
import datetime
import json

PORT = int(sys.argv[1]) if len(sys.argv) > 1 else 9999
OUT = sys.argv[2] if len(sys.argv) > 2 else "campaign.log"

sock = socket.socket(socket.AF_INET, socket.SOCK_DGRAM)
sock.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
sock.bind(("0.0.0.0", PORT))
print(f"[listener] UDP :{PORT} -> {OUT}", flush=True)

with open(OUT, "a", buffering=1) as f:
    while True:
        data, addr = sock.recvfrom(65535)
        recv_ts = datetime.datetime.now().isoformat(timespec="milliseconds")
        line = data.decode("utf-8", errors="replace").strip()
        try:
            obj = json.loads(line)
            obj["_recv_ts"] = recv_ts
            obj["_recv_from"] = addr[0]
            f.write(json.dumps(obj) + "\n")
        except json.JSONDecodeError:
            f.write(f'{{"_recv_ts":"{recv_ts}","_recv_from":"{addr[0]}","_raw":{json.dumps(line)}}}\n')
