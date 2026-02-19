#!/usr/bin/env python3
"""Analyze tom-stress JSONL output files.

Usage:
    python3 analyze-stress.py results/wifi-lan_*/*.jsonl
    python3 analyze-stress.py results/*/ping.jsonl   # compare pings across scenarios
"""

import json
import sys
import statistics
from pathlib import Path
from collections import defaultdict


def load_events(path: str) -> list[dict]:
    events = []
    with open(path) as f:
        for line in f:
            line = line.strip()
            if line:
                try:
                    events.append(json.loads(line))
                except json.JSONDecodeError:
                    pass
    return events


def analyze_pings(events: list[dict], label: str):
    pings = [e for e in events if e.get("event") == "ping"]
    if not pings:
        return

    rtts = [p["rtt_ms"] for p in pings]
    summaries = [e for e in events if e.get("event") == "summary"]
    path_changes = [e for e in events if e.get("event") == "path_change"]

    print(f"\n  {'─' * 50}")
    print(f"  PING — {label}")
    print(f"  {'─' * 50}")
    print(f"  Total pings:    {len(pings)}")
    print(f"  RTT min:        {min(rtts):.1f} ms")
    print(f"  RTT max:        {max(rtts):.1f} ms")
    print(f"  RTT avg:        {statistics.mean(rtts):.1f} ms")
    if len(rtts) > 1:
        print(f"  RTT median:     {statistics.median(rtts):.1f} ms")
        print(f"  RTT stddev:     {statistics.stdev(rtts):.1f} ms")
        print(f"  RTT p95:        {sorted(rtts)[int(len(rtts) * 0.95)]:.1f} ms")

    if summaries:
        s = summaries[-1]
        total = s.get("total_pings", 0)
        ok = s.get("successful", 0)
        fail = s.get("failed", 0)
        reconn = s.get("reconnections", 0)
        print(f"  Success rate:   {ok}/{total} ({100*ok/total:.0f}%)" if total else "")
        print(f"  Reconnections:  {reconn}")

    if path_changes:
        direct = sum(1 for p in path_changes if p.get("kind") == "direct")
        relay = sum(1 for p in path_changes if p.get("kind") == "relay")
        print(f"  Path changes:   {len(path_changes)} ({direct} direct, {relay} relay)")


def analyze_burst(events: list[dict], label: str):
    results = [e for e in events if e.get("event") == "burst_result"]
    if not results:
        return

    print(f"\n  {'─' * 50}")
    print(f"  BURST — {label}")
    print(f"  {'─' * 50}")

    for r in results:
        sent = r.get("messages_sent", 0)
        acked = r.get("messages_acked", 0)
        lost = r.get("lost", 0)
        size = r.get("payload_size", 0)
        mps = r.get("messages_per_sec", 0)
        bps = r.get("bytes_per_sec", 0)
        elapsed = r.get("elapsed_ms", 0)

        print(f"  Round {r.get('round', '?')}:")
        print(f"    Sent/Acked:   {sent}/{acked} (lost: {lost})")
        print(f"    Payload:      {size} bytes")
        print(f"    Throughput:   {mps:.0f} msg/s, {bps/1024:.1f} KB/s")
        print(f"    Duration:     {elapsed:.0f} ms")
        print(f"    RTT:          {r.get('rtt_min_ms',0):.1f} / {r.get('rtt_avg_ms',0):.1f} / {r.get('rtt_max_ms',0):.1f} ms (min/avg/max)")


def analyze_ladder(events: list[dict], label: str):
    results = [e for e in events if e.get("event") == "ladder_result"]
    if not results:
        return

    print(f"\n  {'─' * 50}")
    print(f"  LADDER — {label}")
    print(f"  {'─' * 50}")
    print(f"  {'Size':>10}  {'OK':>4}  {'Fail':>4}  {'RTT min':>10}  {'RTT avg':>10}  {'RTT max':>10}")

    for r in results:
        size = r.get("size", 0)
        ok = r.get("successful", 0)
        fail = r.get("failed", 0)
        rmin = r.get("rtt_min_ms") or 0
        ravg = r.get("rtt_avg_ms") or 0
        rmax = r.get("rtt_max_ms") or 0

        if size >= 1024:
            size_str = f"{size // 1024}KB"
        else:
            size_str = f"{size}B"

        print(f"  {size_str:>10}  {ok:>4}  {fail:>4}  {rmin:>9.1f}ms  {ravg:>9.1f}ms  {rmax:>9.1f}ms")


def analyze_fanout(events: list[dict], label: str):
    results = [e for e in events if e.get("event") == "fanout_result"]
    if not results:
        return

    print(f"\n  {'─' * 50}")
    print(f"  FANOUT — {label}")
    print(f"  {'─' * 50}")

    for r in results:
        print(f"  Targets:       {r.get('target_count', '?')}")
        print(f"  Sent/Delivered: {r.get('total_sent', 0)}/{r.get('total_delivered', 0)}")
        print(f"  Failed:        {r.get('total_failed', 0)}")
        print(f"  Avg RTT:       {r.get('avg_rtt_ms', 0):.1f} ms")
        print(f"  Max RTT:       {r.get('max_rtt_ms', 0):.1f} ms")


def main():
    if len(sys.argv) < 2:
        print("Usage: analyze-stress.py <file1.jsonl> [file2.jsonl] ...")
        sys.exit(1)

    files = sys.argv[1:]
    print(f"╔══════════════════════════════════════════════════════╗")
    print(f"║          tom-stress results analysis                ║")
    print(f"╚══════════════════════════════════════════════════════╝")
    print(f"  Files: {len(files)}")

    for path in sorted(files):
        p = Path(path)
        label = f"{p.parent.name}/{p.name}"
        events = load_events(path)

        if not events:
            print(f"\n  (empty: {label})")
            continue

        analyze_pings(events, label)
        analyze_burst(events, label)
        analyze_ladder(events, label)
        analyze_fanout(events, label)

    print(f"\n  {'═' * 50}")
    print(f"  Done.")


if __name__ == "__main__":
    main()
