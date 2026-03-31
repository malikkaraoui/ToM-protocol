use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use tokio::task::JoinHandle;
use tom_protocol::{AntiSpamConfig, ProtocolEvent, ProtocolRuntime, RuntimeConfig};
use tom_relay::server::{
    AccessConfig, RelayConfig as RelayServerConfig, Server, ServerConfig, SpawnError,
};
use tom_transport::{RelayUrl, TomNode, TomNodeConfig};

const KS: [usize; 3] = [8, 16, 32];
const DEFAULT_NODE_COUNT: usize = 40;
const DEFAULT_TRIALS: usize = 3;
const DEFAULT_BOOTSTRAP_TIMEOUT_SECS: u64 = 20;
const DEFAULT_DELIVERY_TIMEOUT_SECS: u64 = 10;
const DEFAULT_TRIAL_TIMEOUT_SECS: u64 = 60;

struct TestNode {
    id: tom_protocol::NodeId,
    handle: tom_protocol::RuntimeHandle,
    messages: tokio::sync::mpsc::Receiver<tom_protocol::DeliveredMessage>,
}

#[derive(Debug, Clone)]
struct TrialResult {
    k: usize,
    trial_index: usize,
    node_count: usize,
    nodes_bootstrapped: usize,
    median_first_neighbor_ms: Option<f64>,
    worst_first_neighbor_ms: Option<f64>,
    delivery_ms: Option<f64>,
}

impl TrialResult {
    fn trial_success(&self) -> bool {
        self.nodes_bootstrapped == self.node_count && self.delivery_ms.is_some()
    }
}

fn env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<usize>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

fn env_u64(name: &str, default: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .filter(|v| *v > 0)
        .unwrap_or(default)
}

fn env_ks() -> Vec<usize> {
    std::env::var("PEER_PRESENT_KS")
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|part| part.trim().parse::<usize>().ok())
                .filter(|value| *value > 0)
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| KS.to_vec())
}

async fn run_test_relay(peer_present_k: usize) -> Result<(RelayUrl, Server), SpawnError> {
    let config = ServerConfig::<(), ()> {
        relay: Some(RelayServerConfig {
            http_bind_addr: (std::net::Ipv4Addr::LOCALHOST, 0).into(),
            tls: None,
            limits: Default::default(),
            key_cache_capacity: Some(1024),
            peer_present_k: Some(peer_present_k),
            access: AccessConfig::Everyone,
        }),
        quic: None,
        ..Default::default()
    };

    let server = Server::spawn(config).await?;
    let relay_url: RelayUrl = format!("http://{}", server.http_addr().expect("configured"))
        .parse()
        .expect("invalid relay url");

    Ok((relay_url, server))
}

async fn wait_for_first_neighbor(
    mut events: tokio::sync::mpsc::Receiver<ProtocolEvent>,
    started_at: Instant,
) -> Option<Duration> {
    while let Some(event) = events.recv().await {
        if let ProtocolEvent::GossipNeighborUp { .. } = event {
            return Some(started_at.elapsed());
        }
    }
    None
}

async fn spawn_runtime_node(
    relay_url: RelayUrl,
    username: String,
) -> Result<(TestNode, JoinHandle<Option<Duration>>)> {
    let antispam = AntiSpamConfig {
        min_rate: 1000.0,
        ..AntiSpamConfig::default()
    };

    let node = TomNode::bind(
        TomNodeConfig::new()
            .relay_url(relay_url)
            .n0_discovery(false)
            .relay_only(true),
    )
    .await?;

    let id = node.id();
    let started_at = Instant::now();
    let channels = ProtocolRuntime::spawn(
        node,
        RuntimeConfig {
            username,
            enable_dht: false,
            antispam_config: antispam,
            ..RuntimeConfig::default()
        },
    );

    let neighbor_task = tokio::spawn(wait_for_first_neighbor(channels.events, started_at));

    Ok((
        TestNode {
            id,
            handle: channels.handle,
            messages: channels.messages,
        },
        neighbor_task,
    ))
}

async fn run_trial(
    k: usize,
    trial_index: usize,
    node_count: usize,
    bootstrap_timeout: Duration,
    delivery_timeout: Duration,
) -> Result<TrialResult> {
    eprintln!("[k={k} trial={trial_index}] starting relay + {node_count} nodes");
    let (relay_url, relay_server) = run_test_relay(k).await?;
    let mut nodes = Vec::with_capacity(node_count);
    let mut neighbor_tasks = Vec::with_capacity(node_count);

    for idx in 0..node_count {
        let (node, neighbor_task) = spawn_runtime_node(relay_url.clone(), format!("node-{idx}"))
            .await
            .with_context(|| format!("failed to spawn node-{idx}"))?;
        nodes.push(node);
        neighbor_tasks.push(neighbor_task);
        if (idx + 1) % 8 == 0 || idx + 1 == node_count {
            eprintln!("[k={k} trial={trial_index}] spawned {}/{} nodes", idx + 1, node_count);
        }
    }

    eprintln!("[k={k} trial={trial_index}] waiting for first NeighborUp events");
    let deadline = tokio::time::Instant::now() + bootstrap_timeout;
    let mut first_neighbor_times = Vec::with_capacity(node_count);
    let mut nodes_bootstrapped = 0usize;

    for task in neighbor_tasks {
        match tokio::time::timeout_at(deadline, task).await {
            Ok(Ok(Some(duration))) => {
                nodes_bootstrapped += 1;
                first_neighbor_times.push(duration);
            }
            Ok(Ok(None)) => {}
            Ok(Err(_join_err)) => {}
            Err(_) => {}
        }
    }

    first_neighbor_times.sort_unstable();
    let median_first_neighbor_ms = first_neighbor_times
        .get(first_neighbor_times.len().saturating_sub(1) / 2)
        .map(|d| d.as_secs_f64() * 1000.0);
    let worst_first_neighbor_ms = first_neighbor_times
        .last()
        .map(|d| d.as_secs_f64() * 1000.0);

    let sender_handle = nodes
        .last()
        .context("missing sender node")?
        .handle
        .clone();
    let sender_id = nodes.last().context("missing sender id")?.id;
    let receiver = nodes.first_mut().context("missing receiver node")?;
    let payload = format!("k={k};trial={trial_index};from={sender_id}").into_bytes();

    eprintln!("[k={k} trial={trial_index}] probing delivery {} -> {}", sender_id, receiver.id);
    let send_started_at = Instant::now();
    sender_handle
        .send_message(receiver.id, payload.clone())
        .await
        .context("message send failed")?;

    let delivery_ms = match tokio::time::timeout(delivery_timeout, receiver.messages.recv()).await {
        Ok(Some(message)) if message.payload == payload && message.from == sender_id => {
            Some(send_started_at.elapsed().as_secs_f64() * 1000.0)
        }
        Ok(Some(_)) => None,
        Ok(None) => None,
        Err(_) => None,
    };

    for node in nodes {
        node.handle.shutdown().await;
    }
    tokio::time::sleep(Duration::from_millis(200)).await;
    let _ = tokio::time::timeout(Duration::from_secs(5), relay_server.shutdown()).await;
    eprintln!("[k={k} trial={trial_index}] cleanup complete");

    Ok(TrialResult {
        k,
        trial_index,
        node_count,
        nodes_bootstrapped,
        median_first_neighbor_ms,
        worst_first_neighbor_ms,
        delivery_ms,
    })
}

fn format_ms(value: Option<f64>) -> String {
    value
        .map(|v| format!("{v:.1}"))
        .unwrap_or_else(|| "timeout".to_string())
}

fn summarize(results: &[TrialResult]) {
    println!();
    println!("=== Summary by k ===");
    println!("k | trials_ok | node_success | median_first_neighbor_ms | worst_first_neighbor_ms | median_delivery_ms");
    println!("--|-----------|--------------|--------------------------|-------------------------|-------------------");

    for &k in &KS {
        let group: Vec<&TrialResult> = results.iter().filter(|result| result.k == k).collect();
        if group.is_empty() {
            continue;
        }

        let total_nodes: usize = group.iter().map(|result| result.node_count).sum();
        let bootstrapped_nodes: usize = group
            .iter()
            .map(|result| result.nodes_bootstrapped)
            .sum();
        let trial_successes = group.iter().filter(|result| result.trial_success()).count();

        let mut medians: Vec<f64> = group
            .iter()
            .filter_map(|result| result.median_first_neighbor_ms)
            .collect();
        let mut worsts: Vec<f64> = group
            .iter()
            .filter_map(|result| result.worst_first_neighbor_ms)
            .collect();
        let mut deliveries: Vec<f64> = group
            .iter()
            .filter_map(|result| result.delivery_ms)
            .collect();

        medians.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        worsts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        deliveries.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let median_of_medians = medians
            .get(medians.len().saturating_sub(1) / 2)
            .copied();
        let worst_of_worsts = worsts.last().copied();
        let median_delivery = deliveries
            .get(deliveries.len().saturating_sub(1) / 2)
            .copied();

        println!(
            "{k} | {trial_successes}/{} | {:.1}% | {} | {} | {}",
            group.len(),
            (bootstrapped_nodes as f64 / total_nodes as f64) * 100.0,
            format_ms(median_of_medians),
            format_ms(worst_of_worsts),
            format_ms(median_delivery),
        );
    }
}

#[tokio::main(flavor = "multi_thread", worker_threads = 64)]
async fn main() -> Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(
            "tom_protocol=error,tom_transport=error,tom_relay=error,tom_gossip=error",
        )
        .try_init();

    let node_count = env_usize("PEER_PRESENT_NODE_COUNT", DEFAULT_NODE_COUNT);
    let trials = env_usize("PEER_PRESENT_TRIALS", DEFAULT_TRIALS);
    let ks = env_ks();
    let bootstrap_timeout =
        Duration::from_secs(env_u64("PEER_PRESENT_BOOTSTRAP_TIMEOUT_SECS", DEFAULT_BOOTSTRAP_TIMEOUT_SECS));
    let delivery_timeout =
        Duration::from_secs(env_u64("PEER_PRESENT_DELIVERY_TIMEOUT_SECS", DEFAULT_DELIVERY_TIMEOUT_SECS));
    let trial_timeout =
        Duration::from_secs(env_u64("PEER_PRESENT_TRIAL_TIMEOUT_SECS", DEFAULT_TRIAL_TIMEOUT_SECS));

    println!(
        "PeerPresent k matrix: ks={ks:?}, nodes={node_count}, trials={trials}, bootstrap_timeout={}s, delivery_timeout={}s, trial_timeout={}s",
        bootstrap_timeout.as_secs(),
        delivery_timeout.as_secs(),
        trial_timeout.as_secs(),
    );

    let mut results = Vec::new();

    for &k in &ks {
        println!();
        println!("=== k = {k} ===");
        for trial_index in 1..=trials {
            let result = match tokio::time::timeout(
                trial_timeout,
                run_trial(k, trial_index, node_count, bootstrap_timeout, delivery_timeout),
            )
            .await
            {
                Ok(Ok(result)) => result,
                Ok(Err(err)) => return Err(err)
                    .with_context(|| format!("trial failed for k={k}, trial={trial_index}")),
                Err(_) => {
                    eprintln!("[k={k} trial={trial_index}] trial timeout after {}s", trial_timeout.as_secs());
                    TrialResult {
                        k,
                        trial_index,
                        node_count,
                        nodes_bootstrapped: 0,
                        median_first_neighbor_ms: None,
                        worst_first_neighbor_ms: None,
                        delivery_ms: None,
                    }
                }
            };

            println!(
                "trial {}: bootstrapped {}/{} | median_neighbor={} ms | worst_neighbor={} ms | delivery={} ms{}",
                result.trial_index,
                result.nodes_bootstrapped,
                result.node_count,
                format_ms(result.median_first_neighbor_ms),
                format_ms(result.worst_first_neighbor_ms),
                format_ms(result.delivery_ms),
                if result.trial_success() { " | ok" } else { " | partial" },
            );

            results.push(result);
        }
    }

    summarize(&results);
    Ok(())
}