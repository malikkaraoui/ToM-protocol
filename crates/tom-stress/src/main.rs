mod burst;
mod common;
mod events;
mod fanout;
mod ladder;
mod listen;
mod ping;

use clap::{Parser, Subcommand};
use common::parse_node_id;
use std::time::Instant;
use tom_transport::{TomNode, TomNodeConfig};

#[derive(Parser)]
#[command(name = "tom-stress", about = "Stress test for ToM transport layer")]
struct Cli {
    /// Display name for this node.
    #[arg(short, long, default_value = "Node")]
    name: String,

    /// Max message size in bytes.
    #[arg(long, default_value = "1048576")]
    max_message_size: usize,

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
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn".into()),
        )
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let start = Instant::now();

    let config = TomNodeConfig::new().max_message_size(cli.max_message_size);
    let node = TomNode::bind(config).await?;

    eprintln!("tom-stress v{}", env!("CARGO_PKG_VERSION"));
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
    }

    Ok(())
}
