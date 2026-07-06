mod burst;
mod campaign;
mod common;
mod events;
mod fanout;
mod fleet_probe;
mod ladder;
mod listen;
mod output;
mod ping;
mod responder;
mod scenario_backup;
mod scenario_chaos;
mod scenario_churn;
mod scenario_common;
mod scenario_e2e;
mod scenario_endurance;
mod scenario_failover;
mod scenario_group;
mod scenario_partition;
mod scenario_presence;
mod scenario_presence_attack;
mod scenario_chaos_monkey;
mod scenario_presence_storm;
mod scenario_roles;
mod scenario_runner;

use clap::{Parser, Subcommand};
use common::parse_node_id;
use std::sync::Mutex;
use std::time::Instant;
use tom_transport::{TomNode, TomNodeConfig};
use tracing_subscriber::fmt::writer::MakeWriterExt;

#[derive(Parser)]
#[command(name = "tom-stress", about = "Stress test for ToM transport layer")]
struct Cli {
    /// Display name for this node.
    #[arg(short, long, default_value = "Node")]
    name: String,

    /// Max message size in bytes.
    #[arg(long, default_value = "1048576")]
    max_message_size: usize,

    /// Custom relay URL (overrides TOM_RELAY_URL env var).
    #[arg(long)]
    relay_url: Option<String>,

    /// Disable n0-computer address discovery (Pkarr/DNS).
    /// Use when running your own relay with --relay-url.
    #[arg(long)]
    no_n0_discovery: bool,

    /// Path to persistent identity file (32-byte Ed25519 key).
    /// If the file doesn't exist, a new identity is created.
    #[arg(long)]
    identity: Option<String>,

    /// Directory for persistent state (SQLite — groups, keys, contacts).
    /// If not set, state is ephemeral.
    #[arg(long)]
    data_dir: Option<String>,

    /// Direct socket address of the connect target (ip:port).
    /// Used with --no-n0-discovery for local tests without relay/discovery.
    #[arg(long)]
    target_addr: Option<String>,

    /// Auto-archive output to this directory.
    /// Creates timestamped .jsonl and .log files (never overwrites).
    #[arg(long)]
    output_dir: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Listen mode: echo responder for all test types.
    Listen,

    /// Envelope ping/pong (validates transport layer).
    Ping {
        /// Target node's NodeId (hex).
        #[arg(long)]
        connect: String,
        /// Number of pings.
        #[arg(long, default_value = "20")]
        count: u32,
        /// Delay between pings in ms.
        #[arg(long, default_value = "2000")]
        delay: u64,
        /// Continuous mode (ignore --count).
        #[arg(long)]
        continuous: bool,
        /// Rolling summary interval.
        #[arg(long, default_value = "50")]
        summary_interval: u32,
    },

    /// Send N envelopes as fast as possible (throughput test).
    Burst {
        /// Target node's NodeId (hex).
        #[arg(long)]
        connect: String,
        /// Number of envelopes per burst.
        #[arg(long, default_value = "100")]
        count: u32,
        /// Payload size in bytes.
        #[arg(long, default_value = "1024")]
        payload_size: usize,
        /// Number of burst rounds.
        #[arg(long, default_value = "1")]
        rounds: u32,
        /// Delay between rounds in ms.
        #[arg(long, default_value = "5000")]
        round_delay: u64,
    },

    /// Send messages of increasing sizes, measure latency per size.
    Ladder {
        /// Target node's NodeId (hex).
        #[arg(long)]
        connect: String,
        /// Sizes to test (comma-separated bytes). Default: geometric 1KB→1MB.
        #[arg(long)]
        sizes: Option<String>,
        /// Repetitions per size step.
        #[arg(long, default_value = "5")]
        reps: u32,
        /// Delay between steps in ms.
        #[arg(long, default_value = "1000")]
        delay: u64,
    },

    /// Send to N listeners simultaneously.
    Fanout {
        /// Comma-separated list of target NodeIds (hex).
        #[arg(long, value_delimiter = ',')]
        targets: Vec<String>,
        /// Number of envelopes per target.
        #[arg(long, default_value = "10")]
        count: u32,
        /// Payload size in bytes.
        #[arg(long, default_value = "1024")]
        payload_size: usize,
    },

    /// Protocol scenario: E2E encryption validation.
    E2e,

    /// Protocol scenario: Group lifecycle (create → invite → join → send → leave).
    Group,

    /// Protocol scenario: Backup delivery for offline peers.
    Backup,

    /// Protocol scenario: Failover (shadow chain + hub failure).
    Failover,

    /// Protocol scenario: Roles (scoring pipeline + metrics queries).
    Roles,

    /// Chaos scenario: randomized multi-node test with random delays and message sizes.
    Chaos,

    /// Endurance scenario: 6h soak test with 5 nodes, churn, relay kill, and group test.
    Endurance,

    /// Network scenario: partition (split-brain) + heal verification.
    Partition,

    /// Network scenario: node churn (departure + restart) resilience.
    Churn,

    /// L1-001 scenario: Proof of Presence over live QUIC (relay evidence, challenge round, anti-Sybil gate, entropy seed).
    Presence,

    /// L1-001 STORM: mesh of nodes hammering presence, per-outcome metrics + latency relevés.
    PresenceStorm,

    /// RED TEAM: adversarial presence injection over live QUIC (forge/replay/malformed/flood).
    PresenceAttack,

    /// Chaos Monkey: random aggressive faults (kill/revive/clock-skew) on a live presence fleet.
    ChaosMonkey {
        /// RNG seed (reproducible chaos).
        #[arg(long, default_value = "12648430")]
        seed: u64,
    },

    /// Run all 8 protocol scenarios in sequence (e2e, group, backup, failover, roles, chaos, partition, churn).
    Scenarios,

    /// Full-protocol responder (auto-echo, auto-accept groups, auto-reply).
    Responder,

    /// Fleet probe: join the real network and exercise the API against every
    /// device as it connects (presence challenges + reachability + live relevés).
    FleetProbe {
        /// Seconds between challenge/reachability rounds.
        #[arg(long, default_value = "10")]
        probe_interval: u64,
        /// Seconds between printed reports.
        #[arg(long, default_value = "5")]
        report_interval: u64,
        /// Stop after this many seconds (default: run until Ctrl+C).
        #[arg(long)]
        duration_secs: Option<u64>,
        /// Also send a chat ping to each peer each round (reachability).
        #[arg(long, default_value = "true")]
        reachability: bool,
        /// SIM: presence clock offset in ms (inject skew — anti-NTP test).
        #[arg(long, default_value = "0")]
        clock_offset_ms: i64,
    },

    /// Run a full stress campaign (6 phases) against a remote responder.
    Campaign {
        /// Target responder's NodeId (hex).
        #[arg(long)]
        connect: String,
        /// Total duration for the endurance phase in seconds.
        #[arg(long, default_value = "3600")]
        duration: u64,
        /// Run a single phase only (ping, burst, e2e, group, failover, endurance).
        #[arg(long)]
        phase: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mode_name = match &cli.command {
        Command::Listen => "listen",
        Command::Ping { .. } => "ping",
        Command::Burst { .. } => "burst",
        Command::Ladder { .. } => "ladder",
        Command::Fanout { .. } => "fanout",
        Command::E2e => "e2e",
        Command::Group => "group",
        Command::Backup => "backup",
        Command::Failover => "failover",
        Command::Roles => "roles",
        Command::Chaos => "chaos",
        Command::Presence => "presence",
        Command::PresenceStorm => "presence-storm",
        Command::PresenceAttack => "presence-attack",
        Command::ChaosMonkey { .. } => "chaos-monkey",
        Command::Endurance => "endurance",
        Command::Partition => "partition",
        Command::Churn => "churn",
        Command::Scenarios => "scenarios",
        Command::Responder => "responder",
        Command::FleetProbe { .. } => "fleet-probe",
        Command::Campaign { .. } => "campaign",
    };

    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "warn".into());

    if let Some(ref dir) = cli.output_dir {
        if mode_name != "listen" && mode_name != "responder" {
            let paths = output::resolve_output_paths(
                std::path::Path::new(dir),
                &cli.name,
                mode_name,
            )?;

            output::init_jsonl_writer(&paths.jsonl)?;

            let log_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&paths.log)?;

            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr.and(Mutex::new(log_file)))
                .init();

            eprintln!("Output archiving:");
            eprintln!("  JSONL → {}", paths.jsonl.display());
            eprintln!("  Logs  → {}", paths.log.display());
            eprintln!();
        } else {
            tracing_subscriber::fmt()
                .with_env_filter(env_filter)
                .with_writer(std::io::stderr)
                .init();
        }
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(env_filter)
            .with_writer(std::io::stderr)
            .init();
    }

    eprintln!("tom-stress v{}", env!("CARGO_PKG_VERSION"));

    // ── Protocol scenarios (spawn their own nodes) ───────────────
    match &cli.command {
        Command::E2e | Command::Group | Command::Backup | Command::Failover | Command::Roles | Command::Chaos | Command::Endurance | Command::Partition | Command::Churn | Command::Presence | Command::PresenceStorm | Command::PresenceAttack | Command::ChaosMonkey { .. } => {
            let result = match cli.command {
                Command::E2e => scenario_e2e::run().await?,
                Command::Group => scenario_group::run().await?,
                Command::Backup => scenario_backup::run().await?,
                Command::Failover => scenario_failover::run().await?,
                Command::Roles => scenario_roles::run().await?,
                Command::Chaos => scenario_chaos::run().await?,
                Command::Endurance => scenario_endurance::run().await?,
                Command::Partition => scenario_partition::run().await?,
                Command::Churn => scenario_churn::run().await?,
                Command::Presence => scenario_presence::run().await?,
                Command::PresenceStorm => scenario_presence_storm::run().await?,
                Command::PresenceAttack => scenario_presence_attack::run().await?,
                Command::ChaosMonkey { seed } => scenario_chaos_monkey::run_with_seed(seed).await?,
                _ => unreachable!(),
            };
            result.print_summary();
            result.emit_jsonl();
            if !result.success() {
                std::process::exit(1);
            }
            return Ok(());
        }
        Command::Scenarios => {
            scenario_runner::run().await?;
            return Ok(());
        }
        Command::Responder => {
            responder::run(responder::ResponderConfig {
                name: cli.name.clone(),
                max_message_size: cli.max_message_size,
                relay_url: cli.relay_url.clone(),
                no_n0_discovery: cli.no_n0_discovery,
                identity_path: cli.identity.clone(),
                data_dir: cli.data_dir.clone(),
            })
            .await?;
            return Ok(());
        }
        Command::FleetProbe { probe_interval, report_interval, duration_secs, reachability, clock_offset_ms } => {
            fleet_probe::run(fleet_probe::FleetProbeConfig {
                relay_url: cli.relay_url.clone(),
                duration_secs: *duration_secs,
                probe_interval_secs: *probe_interval,
                report_interval_secs: *report_interval,
                with_reachability: *reachability,
                clock_offset_ms: *clock_offset_ms,
            })
            .await?;
            return Ok(());
        }
        Command::Campaign {
            connect,
            duration,
            phase,
        } => {
            let target = parse_node_id(connect)?;
            campaign::run(campaign::CampaignConfig {
                target,
                target_addr: cli.target_addr.clone(),
                name: cli.name.clone(),
                duration_s: *duration,
                phase: phase.clone(),
                max_message_size: cli.max_message_size,
                relay_url: cli.relay_url.clone(),
                no_n0_discovery: cli.no_n0_discovery,
                data_dir: cli.data_dir.clone(),
            })
            .await?;
            return Ok(());
        }
        _ => {}
    }

    // ── Transport-level tests (shared node) ──────────────────────
    let start = Instant::now();

    let mut config = TomNodeConfig::new().max_message_size(cli.max_message_size);
    if let Some(ref url) = cli.relay_url {
        config = config.relay_url(url.parse()?);
    }
    if cli.no_n0_discovery {
        config = config.n0_discovery(false).local_discovery(true);
    }
    if let Some(ref path) = cli.identity {
        config = config.identity_path(path.into());
    }
    let node = TomNode::bind(config).await?;

    eprintln!("Node ID: {}", node.id());
    eprintln!();

    // Register target address for connectivity.
    // With --target-addr: use direct socket address.
    // Without --target-addr but with relay: register target via relay URL
    // so ConnectionPool can reach the peer through the relay.
    {
        let target_id = match &cli.command {
            Command::Ping { connect, .. }
            | Command::Burst { connect, .. }
            | Command::Ladder { connect, .. } => Some(parse_node_id(connect)?),
            Command::Fanout { targets, .. } => targets.first().map(|s| parse_node_id(s)).transpose()?,
            _ => None,
        };
        if let Some(target) = target_id {
            if let Some(ref addr_str) = cli.target_addr {
                let sock_addr: std::net::SocketAddr = addr_str
                    .parse()
                    .map_err(|e| anyhow::anyhow!("invalid --target-addr '{addr_str}': {e}"))?;
                let endpoint_addr =
                    tom_transport::EndpointAddr::new(*target.as_endpoint_id()).with_ip_addr(sock_addr);
                node.add_peer_addr(endpoint_addr).await;
                eprintln!("Registered target addr: {sock_addr}");
            } else {
                // No direct address — register via relay URL so the connection pool
                // can route through the relay (essential for --no-n0-discovery mode).
                let relays = node.default_relay_urls().await;
                if let Some(url) = relays.first() {
                    let endpoint_addr =
                        tom_transport::EndpointAddr::new(*target.as_endpoint_id()).with_relay_url(url.clone());
                    node.add_peer_addr(endpoint_addr).await;
                    eprintln!("Registered target via relay: {url}");
                }
            }
        }
    }

    match cli.command {
        Command::Listen => {
            listen::run(node, &cli.name, start).await?;
        }

        Command::Presence => unreachable!("handled in the scenario block above"),
        Command::PresenceStorm => unreachable!("handled in the scenario block above"),
        Command::PresenceAttack => unreachable!("handled in the scenario block above"),
        Command::ChaosMonkey { .. } => unreachable!("handled in the scenario block above"),
        Command::FleetProbe { .. } => unreachable!("handled in the dispatch block above"),

        Command::Ping {
            connect,
            count,
            delay,
            continuous,
            summary_interval,
        } => {
            let target = parse_node_id(&connect)?;

            ping::run(
                node,
                ping::PingConfig {
                    target,
                    count,
                    delay_ms: delay,
                    continuous,
                    summary_interval,
                    name: cli.name,
                },
                start,
            )
            .await?;
        }

        Command::Burst {
            connect,
            count,
            payload_size,
            rounds,
            round_delay,
        } => {
            let target = parse_node_id(&connect)?;

            burst::run(
                node,
                burst::BurstConfig {
                    target,
                    count,
                    payload_size,
                    rounds,
                    round_delay_ms: round_delay,
                    name: cli.name,
                },
                start,
            )
            .await?;
        }

        Command::Ladder {
            connect,
            sizes,
            reps,
            delay,
        } => {
            let target = parse_node_id(&connect)?;

            let size_list = if let Some(s) = sizes {
                s.split(',')
                    .map(|v| v.trim().parse::<usize>())
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                ladder::default_sizes(cli.max_message_size)
            };

            ladder::run(
                node,
                ladder::LadderConfig {
                    target,
                    sizes: size_list,
                    reps,
                    delay_ms: delay,
                    name: cli.name,
                },
                start,
            )
            .await?;
        }

        Command::Fanout {
            targets,
            count,
            payload_size,
        } => {
            let target_ids: Vec<_> = targets
                .iter()
                .map(|s| parse_node_id(s))
                .collect::<Result<_, _>>()?;

            fanout::run(
                node,
                fanout::FanoutConfig {
                    targets: target_ids,
                    count,
                    payload_size,
                    name: cli.name,
                },
                start,
            )
            .await?;
        }

        // Already handled above
        Command::E2e | Command::Group | Command::Backup | Command::Failover | Command::Roles
        | Command::Chaos | Command::Endurance | Command::Partition | Command::Churn
        | Command::Scenarios | Command::Responder | Command::Campaign { .. } => {
            unreachable!()
        }
    }

    Ok(())
}
