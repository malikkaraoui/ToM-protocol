#!/usr/bin/env python3
"""
Analyze nat-test JSON output files.

Usage:
  python3 analyze-results.py results/03-car-continuous.jsonl
  python3 analyze-results.py results/*.jsonl
"""

import json
import sys
import statistics
from pathlib import Path


def analyze_file(filepath: str) -> dict:
    """Parse a .jsonl file and extract metrics."""
    events = []
    with open(filepath) as f:
        for line in f:
            line = line.strip()
            if not line:
                continue
            try:
                events.append(json.loads(line))
            except json.JSONDecodeError:
                pass

    pings = [e for e in events if e.get("event") == "ping"]
    path_changes = [e for e in events if e.get("event") == "path_change"]
    hole_punches = [e for e in events if e.get("event") == "hole_punch"]
    disconnects = [e for e in events if e.get("event") == "disconnected"]
    reconnects = [e for e in events if e.get("event") == "reconnected"]
    summaries = [e for e in events if e.get("event") == "summary"]

    direct_pings = [p for p in pings if p.get("via") == "DIRECT"]
    relay_pings = [p for p in pings if p.get("via") == "RELAY"]

    direct_rtts = [p["rtt_ms"] for p in direct_pings]
    relay_rtts = [p["rtt_ms"] for p in relay_pings]
    all_rtts = [p["rtt_ms"] for p in pings]

    result = {
        "file": Path(filepath).name,
        "total_pings": len(pings),
        "direct_pings": len(direct_pings),
        "relay_pings": len(relay_pings),
        "direct_pct": (len(direct_pings) / len(pings) * 100) if pings else 0,
        "path_changes": len(path_changes),
        "hole_punches": len(hole_punches),
        "disconnections": len(disconnects),
        "reconnections": len(reconnects),
    }

    # RTT stats
    if all_rtts:
        result["rtt_mean"] = statistics.mean(all_rtts)
        result["rtt_median"] = statistics.median(all_rtts)
        result["rtt_p95"] = sorted(all_rtts)[int(len(all_rtts) * 0.95)]
        result["rtt_min"] = min(all_rtts)
        result["rtt_max"] = max(all_rtts)

    if direct_rtts:
        result["direct_rtt_mean"] = statistics.mean(direct_rtts)
        result["direct_rtt_p95"] = sorted(direct_rtts)[int(len(direct_rtts) * 0.95)]

    if relay_rtts:
        result["relay_rtt_mean"] = statistics.mean(relay_rtts)

    # Hole punch timing
    if hole_punches:
        result["hole_punch_time_s"] = hole_punches[0].get("time_to_direct_s", 0)

    # Reconnection stats
    if reconnects:
        recon_times = [r["reconnect_time_ms"] for r in reconnects]
        result["reconnect_mean_ms"] = statistics.mean(recon_times)
        result["reconnect_max_ms"] = max(recon_times)

    # Duration
    if pings:
        result["duration_s"] = pings[-1].get("elapsed_s", 0)

    # Use final summary if available (more accurate)
    if summaries:
        s = summaries[-1]
        result["total_disconnected_s"] = s.get("total_disconnected_s", 0)

    return result


def print_single(r: dict):
    """Print detailed analysis for a single file."""
    print(f"\n{'=' * 60}")
    print(f"  {r['file']}")
    print(f"{'=' * 60}")
    print()

    dur = r.get("duration_s", 0)
    print(f"  Duration:        {dur:.0f}s ({dur/60:.1f} min)")
    print(f"  Total pings:     {r['total_pings']}")
    print(f"  Direct:          {r['direct_pings']} ({r['direct_pct']:.0f}%)")
    print(f"  Relay:           {r['relay_pings']}")
    print()

    if "rtt_mean" in r:
        print(f"  RTT (all):       mean={r['rtt_mean']:.1f}ms  "
              f"median={r['rtt_median']:.1f}ms  "
              f"p95={r['rtt_p95']:.1f}ms  "
              f"min={r['rtt_min']:.1f}ms  max={r['rtt_max']:.1f}ms")

    if "direct_rtt_mean" in r:
        print(f"  RTT (direct):    mean={r['direct_rtt_mean']:.1f}ms  "
              f"p95={r['direct_rtt_p95']:.1f}ms")

    if "relay_rtt_mean" in r:
        print(f"  RTT (relay):     mean={r['relay_rtt_mean']:.1f}ms")

    print()

    if "hole_punch_time_s" in r:
        print(f"  Hole punch:      {r['hole_punch_time_s']:.2f}s")

    print(f"  Path changes:    {r['path_changes']}")
    print(f"  Disconnections:  {r['disconnections']}")
    print(f"  Reconnections:   {r['reconnections']}")

    if "reconnect_mean_ms" in r:
        print(f"  Reconnect time:  mean={r['reconnect_mean_ms']:.0f}ms  "
              f"max={r['reconnect_max_ms']:.0f}ms")

    if "total_disconnected_s" in r:
        print(f"  Total downtime:  {r['total_disconnected_s']:.1f}s")

    print()


def print_comparison(results: list[dict]):
    """Print comparison table across multiple files."""
    print(f"\n{'=' * 100}")
    print("  COMPARISON TABLE")
    print(f"{'=' * 100}")
    print()

    # Header
    hdr = f"{'Scenario':<30} {'Pings':>6} {'Direct%':>8} "
    hdr += f"{'RTT ms':>8} {'Punch s':>8} "
    hdr += f"{'Discon':>7} {'Recon':>7} {'Down s':>8}"
    print(hdr)
    print("-" * 100)

    for r in results:
        name = r["file"].replace(".jsonl", "")[:29]
        rtt = f"{r.get('direct_rtt_mean', r.get('rtt_mean', 0)):.1f}"
        punch = f"{r.get('hole_punch_time_s', 0):.2f}" if "hole_punch_time_s" in r else "-"
        down = f"{r.get('total_disconnected_s', 0):.1f}" if "total_disconnected_s" in r else "-"

        row = f"{name:<30} {r['total_pings']:>6} {r['direct_pct']:>7.0f}% "
        row += f"{rtt:>8} {punch:>8} "
        row += f"{r['disconnections']:>7} {r['reconnections']:>7} {down:>8}"
        print(row)

    print()

    # Aggregate
    all_direct = [r["direct_pct"] for r in results]
    print(f"  Average direct: {statistics.mean(all_direct):.0f}%  "
          f"Min: {min(all_direct):.0f}%  Max: {max(all_direct):.0f}%")

    total_discon = sum(r["disconnections"] for r in results)
    total_recon = sum(r["reconnections"] for r in results)
    print(f"  Total disconnections: {total_discon}  reconnections: {total_recon}")
    print()


def main():
    if len(sys.argv) < 2:
        print("Usage: python3 analyze-results.py <file.jsonl> [file2.jsonl ...]")
        sys.exit(1)

    files = sys.argv[1:]
    results = []

    for f in files:
        try:
            results.append(analyze_file(f))
        except FileNotFoundError:
            print(f"  [SKIP] {f}: not found")
        except Exception as e:
            print(f"  [ERROR] {f}: {e}")

    if not results:
        print("No valid results to analyze.")
        sys.exit(1)

    for r in results:
        print_single(r)

    if len(results) > 1:
        print_comparison(results)


if __name__ == "__main__":
    main()
