mod burst;
mod campaign;
mod common;
mod events;
mod fanout;
mod ladder;
mod listen;
mod output;
mod ping;
mod responder;
mod scenario_backup;
mod scenario_chaos;
mod scenario_common;
mod scenario_e2e;
mod scenario_failover;
mod scenario_group;
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

    /// Run all 6 protocol scenarios in sequence (e2e, group, backup, failover, roles, chaos).
    Scenarios,

    /// Full-protocol responder (auto-echo, auto-accept groups, auto-reply).
    Responder,

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
        Command::Scenarios => "scenarios",
        Command::Responder => "responder",
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
        Command::E2e | Command::Group | Command::Backup | Command::Failover | Command::Roles | Command::Chaos => {
            let result = match cli.command {
                Command::E2e => scenario_e2e::run().await?,
                Command::Group => scenario_group::run().await?,
                Command::Backup => scenario_backup::run().await?,
                Command::Failover => scenario_failover::run().await?,
                Command::Roles => scenario_roles::run().await?,
                Command::Chaos => scenario_chaos::run().await?,
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
                name: cli.name.clone(),
                duration_s: *duration,
                phase: phase.clone(),
                max_message_size: cli.max_message_size,
                relay_url: cli.relay_url.clone(),
                no_n0_discovery: cli.no_n0_discovery,
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
        config = config.n0_discovery(false);
    }
    let node = TomNode::bind(config).await?;

    eprintln!("Node ID: {}", node.id());
    eprintln!();

    match cli.command {
        Command::Listen => {
            listen::run(node, &cli.name, start).await?;
        }

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
        | Command::Chaos | Command::Scenarios | Command::Responder | Command::Campaign { .. } => {
            unreachable!()
        }
    }

    Ok(())
}
