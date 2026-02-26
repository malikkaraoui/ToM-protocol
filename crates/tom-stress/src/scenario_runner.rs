/// Scenario runner — executes all protocol scenarios in sequence and produces
/// an aggregated pass/fail report.
///
/// Scenarios: e2e → group → backup → failover → roles → chaos
use std::time::Instant;

use serde::Serialize;

use crate::scenario_common::ScenarioResult;
use crate::{
    scenario_backup, scenario_chaos, scenario_e2e, scenario_failover, scenario_group,
    scenario_roles,
};

#[derive(Serialize)]
struct RunnerSummary {
    event: &'static str,
    scenarios: Vec<ScenarioLine>,
    total_passed: usize,
    total_failed: usize,
    total_elapsed_s: f64,
    overall_status: &'static str,
}

#[derive(Serialize)]
struct ScenarioLine {
    scenario: String,
    status: &'static str,
    passed: usize,
    failed: usize,
    elapsed_ms: f64,
}

pub async fn run() -> anyhow::Result<()> {
    let runner_start = Instant::now();

    eprintln!("╔══════════════════════════════════════════╗");
    eprintln!("║       SCENARIO RUNNER (6 scenarios)      ║");
    eprintln!("╚══════════════════════════════════════════╝\n");

    let scenarios: Vec<(&str, _)> = vec![
        ("e2e", run_scenario("e2e", scenario_e2e::run()).await),
        ("group", run_scenario("group", scenario_group::run()).await),
        ("backup", run_scenario("backup", scenario_backup::run()).await),
        ("failover", run_scenario("failover", scenario_failover::run()).await),
        ("roles", run_scenario("roles", scenario_roles::run()).await),
        ("chaos", run_scenario("chaos", scenario_chaos::run()).await),
    ];

    let mut lines = Vec::new();
    let mut total_passed = 0usize;
    let mut total_failed = 0usize;
    let mut any_failure = false;

    for (name, result) in &scenarios {
        match result {
            Ok(r) => {
                r.print_summary();
                total_passed += r.passed;
                total_failed += r.failed;
                if !r.success() {
                    any_failure = true;
                }
                lines.push(ScenarioLine {
                    scenario: name.to_string(),
                    status: if r.success() { "PASS" } else { "FAIL" },
                    passed: r.passed,
                    failed: r.failed,
                    elapsed_ms: r.total_ms,
                });
            }
            Err(e) => {
                eprintln!("\n[FAIL] Scenario {name}: {e}");
                any_failure = true;
                total_failed += 1;
                lines.push(ScenarioLine {
                    scenario: name.to_string(),
                    status: "ERROR",
                    passed: 0,
                    failed: 1,
                    elapsed_ms: 0.0,
                });
            }
        }
    }

    let overall = if any_failure { "FAIL" } else { "PASS" };
    let elapsed_s = runner_start.elapsed().as_secs_f64();

    let summary = RunnerSummary {
        event: "scenario_runner_summary",
        scenarios: lines,
        total_passed,
        total_failed,
        total_elapsed_s: elapsed_s,
        overall_status: overall,
    };

    // Emit JSONL
    if let Ok(json) = serde_json::to_string(&summary) {
        println!("{json}");
    }

    // Print summary table
    eprintln!("\n╔══════════════════════════════════════════╗");
    eprintln!("║          SCENARIO RUNNER SUMMARY         ║");
    eprintln!("╠══════════════════════════════════════════╣");
    for s in &summary.scenarios {
        let icon = match s.status {
            "PASS" => " OK ",
            _ => "FAIL",
        };
        eprintln!(
            "║ [{icon}] {:<12} {}/{} steps ({:.1}ms)",
            s.scenario,
            s.passed,
            s.passed + s.failed,
            s.elapsed_ms,
        );
    }
    eprintln!("╠══════════════════════════════════════════╣");
    eprintln!(
        "║ Total: {total_passed} passed, {total_failed} failed | {elapsed_s:.1}s | [{overall}]",
    );
    eprintln!("╚══════════════════════════════════════════╝");

    if any_failure {
        std::process::exit(1);
    }

    Ok(())
}

async fn run_scenario(
    name: &str,
    future: impl std::future::Future<Output = anyhow::Result<ScenarioResult>>,
) -> Result<ScenarioResult, anyhow::Error> {
    eprintln!("\n───────────────────────────────────────────");
    eprintln!("  Running scenario: {name}");
    eprintln!("───────────────────────────────────────────");
    future.await
}
