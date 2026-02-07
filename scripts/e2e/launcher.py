#!/usr/bin/env python3
"""
ToM Protocol E2E Test Launcher

Orchestrates multi-browser E2E tests for the ToM Protocol.
Handles:
- Signaling server lifecycle
- Demo app startup
- Parallel browser sessions
- Test execution and reporting

Usage:
    python launcher.py                    # Run all tests
    python launcher.py --test group       # Run specific test
    python launcher.py --browsers 3       # Run with 3 browser instances
    python launcher.py --headed           # Run in headed mode (visible)
"""

import argparse
import subprocess
import sys
import time
import signal
import os
import json
from pathlib import Path
from datetime import datetime
from typing import Optional

# Project paths
SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent
SIGNALING_DIR = PROJECT_ROOT / "tools" / "signaling-server"
DEMO_DIR = PROJECT_ROOT / "apps" / "demo"
REPORTS_DIR = SCRIPT_DIR / "reports"

# Process handles
processes: list[subprocess.Popen] = []


def log(message: str, level: str = "INFO") -> None:
    """Log with timestamp."""
    timestamp = datetime.now().strftime("%H:%M:%S")
    print(f"[{timestamp}] [{level}] {message}")


def cleanup(signum=None, frame=None) -> None:
    """Clean up all spawned processes."""
    log("Cleaning up processes...", "WARN")
    for proc in processes:
        try:
            proc.terminate()
            proc.wait(timeout=5)
        except Exception:
            proc.kill()
    if signum:
        sys.exit(1)


# Register signal handlers
signal.signal(signal.SIGINT, cleanup)
signal.signal(signal.SIGTERM, cleanup)


def check_dependencies() -> bool:
    """Verify required dependencies are available."""
    log("Checking dependencies...")

    # Check pnpm
    try:
        subprocess.run(["pnpm", "--version"], capture_output=True, check=True)
    except (subprocess.CalledProcessError, FileNotFoundError):
        log("pnpm not found. Please install pnpm.", "ERROR")
        return False

    # Check Playwright
    try:
        result = subprocess.run(
            ["pnpm", "exec", "playwright", "--version"],
            capture_output=True,
            cwd=SCRIPT_DIR,
            check=True
        )
        log(f"Playwright version: {result.stdout.decode().strip()}")
    except (subprocess.CalledProcessError, FileNotFoundError):
        log("Playwright not found. Run: pnpm install", "ERROR")
        return False

    return True


def start_signaling_server(port: int = 3000) -> Optional[subprocess.Popen]:
    """Start the signaling server."""
    log(f"Starting signaling server on port {port}...")

    # Build if needed
    dist_dir = SIGNALING_DIR / "dist"
    if not dist_dir.exists():
        log("Building signaling server...")
        subprocess.run(
            ["pnpm", "build"],
            cwd=SIGNALING_DIR,
            check=True,
            capture_output=True
        )

    # Start server
    proc = subprocess.Popen(
        ["node", "dist/cli.js", "--port", str(port)],
        cwd=SIGNALING_DIR,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE
    )
    processes.append(proc)

    # Wait for server to be ready
    time.sleep(2)
    if proc.poll() is not None:
        log("Signaling server failed to start", "ERROR")
        return None

    log(f"Signaling server running (PID: {proc.pid})")
    return proc


def start_demo_server(port: int = 5173) -> Optional[subprocess.Popen]:
    """Start the demo dev server."""
    log(f"Starting demo server on port {port}...")

    proc = subprocess.Popen(
        ["pnpm", "dev", "--port", str(port)],
        cwd=DEMO_DIR,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        env={**os.environ, "FORCE_COLOR": "0"}
    )
    processes.append(proc)

    # Wait for Vite to be ready
    log("Waiting for Vite dev server...")
    time.sleep(5)

    if proc.poll() is not None:
        log("Demo server failed to start", "ERROR")
        return None

    log(f"Demo server running (PID: {proc.pid})")
    return proc


def run_tests(
    test_pattern: Optional[str] = None,
    browsers: int = 3,
    headed: bool = False,
    project: Optional[str] = None
) -> int:
    """Run Playwright tests."""
    log("Running E2E tests...")

    cmd = ["pnpm", "exec", "playwright", "test"]

    # Add test pattern if specified
    if test_pattern:
        cmd.append(f"*{test_pattern}*")

    # Browser mode
    if headed:
        cmd.append("--headed")

    # Specific browser project
    if project:
        cmd.extend(["--project", project])

    # Environment for multi-user count
    env = {
        **os.environ,
        "TOM_TEST_USERS": str(browsers),
        "FORCE_COLOR": "1"
    }

    log(f"Command: {' '.join(cmd)}")

    result = subprocess.run(
        cmd,
        cwd=SCRIPT_DIR,
        env=env
    )

    return result.returncode


def generate_report() -> None:
    """Generate and display test report summary."""
    results_file = REPORTS_DIR / "results.json"

    if not results_file.exists():
        log("No results file found", "WARN")
        return

    with open(results_file) as f:
        results = json.load(f)

    # Summary statistics
    suites = results.get("suites", [])
    total_tests = 0
    passed = 0
    failed = 0
    skipped = 0

    def count_specs(suite: dict) -> None:
        nonlocal total_tests, passed, failed, skipped
        for spec in suite.get("specs", []):
            for test in spec.get("tests", []):
                total_tests += 1
                status = test.get("status")
                if status == "passed":
                    passed += 1
                elif status == "failed":
                    failed += 1
                elif status == "skipped":
                    skipped += 1
        for child in suite.get("suites", []):
            count_specs(child)

    for suite in suites:
        count_specs(suite)

    log("=" * 50)
    log("TEST RESULTS SUMMARY")
    log("=" * 50)
    log(f"Total:   {total_tests}")
    log(f"Passed:  {passed} ✅")
    log(f"Failed:  {failed} ❌")
    log(f"Skipped: {skipped} ⏭️")
    log("=" * 50)

    if failed > 0:
        log(f"HTML Report: {REPORTS_DIR / 'html' / 'index.html'}", "WARN")


def main() -> int:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="ToM Protocol E2E Test Launcher"
    )
    parser.add_argument(
        "--test", "-t",
        help="Run specific test (pattern match)"
    )
    parser.add_argument(
        "--browsers", "-b",
        type=int,
        default=3,
        help="Number of browser instances (default: 3)"
    )
    parser.add_argument(
        "--headed",
        action="store_true",
        help="Run browsers in headed mode"
    )
    parser.add_argument(
        "--project", "-p",
        choices=["chromium", "webkit", "firefox", "mobile-chrome", "mobile-safari"],
        help="Run on specific browser"
    )
    parser.add_argument(
        "--no-server",
        action="store_true",
        help="Skip server startup (use existing)"
    )
    parser.add_argument(
        "--report-only",
        action="store_true",
        help="Only show last test report"
    )

    args = parser.parse_args()

    # Report only mode
    if args.report_only:
        generate_report()
        return 0

    # Check dependencies
    if not check_dependencies():
        return 1

    # Ensure reports directory exists
    REPORTS_DIR.mkdir(parents=True, exist_ok=True)

    try:
        # Start servers if needed
        if not args.no_server:
            signaling = start_signaling_server()
            if not signaling:
                return 1

            # Note: Demo server started by Playwright webServer config
            # but we can also start it manually for more control

        # Run tests
        exit_code = run_tests(
            test_pattern=args.test,
            browsers=args.browsers,
            headed=args.headed,
            project=args.project
        )

        # Generate report
        generate_report()

        return exit_code

    finally:
        cleanup()


if __name__ == "__main__":
    sys.exit(main())
