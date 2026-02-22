/// Common types and helpers for protocol-level scenarios.
use serde::Serialize;
use std::time::{Duration, Instant};

/// Result of a scenario step.
#[derive(Debug, Clone, Serialize)]
pub struct StepResult {
    pub step: String,
    pub ok: bool,
    pub elapsed_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Result of a full scenario run.
#[derive(Debug, Serialize)]
pub struct ScenarioResult {
    pub scenario: String,
    pub steps: Vec<StepResult>,
    pub total_ms: f64,
    pub passed: usize,
    pub failed: usize,
}

impl ScenarioResult {
    pub fn new(scenario: &str) -> Self {
        Self {
            scenario: scenario.into(),
            steps: Vec::new(),
            total_ms: 0.0,
            passed: 0,
            failed: 0,
        }
    }

    pub fn add(&mut self, step: StepResult) {
        if step.ok {
            self.passed += 1;
        } else {
            self.failed += 1;
        }
        self.steps.push(step);
    }

    pub fn finalize(&mut self, start: Instant) {
        self.total_ms = start.elapsed().as_secs_f64() * 1000.0;
    }

    pub fn success(&self) -> bool {
        self.failed == 0
    }

    pub fn print_summary(&self) {
        let icon = if self.success() { "PASS" } else { "FAIL" };
        eprintln!("\n[{icon}] Scenario: {} ({:.1}ms)", self.scenario, self.total_ms);
        eprintln!("  {} passed, {} failed", self.passed, self.failed);
        for step in &self.steps {
            let mark = if step.ok { " ok" } else { "FAIL" };
            eprint!("  [{mark}] {} ({:.1}ms)", step.step, step.elapsed_ms);
            if let Some(detail) = &step.detail {
                eprint!(" — {detail}");
            }
            eprintln!();
        }
    }

    pub fn emit_jsonl(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            println!("{json}");
        }
    }
}

/// Run a timed async step.
pub async fn timed_step_async<F, Fut>(name: &str, f: F) -> StepResult
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let start = Instant::now();
    match f().await {
        Ok(detail) => StepResult {
            step: name.into(),
            ok: true,
            elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
            detail: if detail.is_empty() { None } else { Some(detail) },
        },
        Err(detail) => StepResult {
            step: name.into(),
            ok: false,
            elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
            detail: Some(detail),
        },
    }
}

/// Receive with timeout — returns the received item or an error message.
pub async fn recv_timeout<T>(
    rx: &mut tokio::sync::mpsc::Receiver<T>,
    timeout: Duration,
) -> Result<T, String> {
    tokio::time::timeout(timeout, rx.recv())
        .await
        .map_err(|_| "timeout".to_string())?
        .ok_or_else(|| "channel closed".to_string())
}
