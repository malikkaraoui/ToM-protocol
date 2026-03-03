mod freebox;
mod nat;
mod token;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "tom-gateway", about = "Auto-configure Freebox NAT for ToM relay")]
struct Cli {
    /// Freebox API base URL (auto-discovered if omitted).
    #[arg(long)]
    freebox_url: Option<String>,

    /// Token file path (default: ~/.tom/freebox_token.json).
    #[arg(long)]
    token_file: Option<String>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Authenticate with the Freebox (requires LCD button press).
    Auth {
        /// Application name shown on the Freebox LCD.
        #[arg(long, default_value = "ToM Gateway")]
        app_name: String,
    },

    /// Create NAT port forwarding rules for the relay (UDP + TCP).
    Setup {
        /// Relay port.
        #[arg(long, default_value = "3340")]
        port: u16,

        /// LAN IP of the relay host. Auto-detected if omitted.
        #[arg(long)]
        lan_ip: Option<String>,

        /// Comment for the NAT rules.
        #[arg(long, default_value = "ToM relay")]
        comment: String,

        /// Delete conflicting rules and recreate them.
        #[arg(long)]
        force: bool,
    },

    /// Show NAT rules, public IP, and relay status.
    Status {
        /// Relay port to check.
        #[arg(long, default_value = "3340")]
        port: u16,
    },

    /// List LAN devices (useful to find the NAS IP).
    Lan,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "tom_gateway=info".parse().unwrap()),
        )
        .init();

    let cli = Cli::parse();

    let token_path = match &cli.token_file {
        Some(p) => std::path::PathBuf::from(p),
        None => token::default_token_path()?,
    };

    match cli.command {
        Command::Auth { app_name } => {
            let (base_url, api_base) =
                freebox::discover(cli.freebox_url.as_deref()).await?;
            let (app_id, app_token) =
                freebox::authorize(&base_url, &api_base, &app_name).await?;

            token::save(
                &token_path,
                &token::StoredToken {
                    app_id,
                    app_token,
                    freebox_url: base_url,
                },
            )?;
        }

        Command::Setup {
            port,
            lan_ip,
            comment,
            force,
        } => {
            let stored = token::load(&token_path)?;
            let (base_url, api_base) =
                freebox::discover(Some(&stored.freebox_url)).await?;
            let client =
                freebox::open_session(&base_url, &api_base, &stored.app_id, &stored.app_token)
                    .await?;

            let detected_ip =
                nat::detect_nas_ip(&client, lan_ip.as_deref()).await?;

            println!("Configuration NAT pour {}:{}...", detected_ip, port);
            nat::setup(&client, port, &detected_ip, &comment, force).await?;
        }

        Command::Status { port } => {
            let stored = token::load(&token_path)?;
            let (base_url, api_base) =
                freebox::discover(Some(&stored.freebox_url)).await?;
            let client =
                freebox::open_session(&base_url, &api_base, &stored.app_id, &stored.app_token)
                    .await?;

            nat::status(&client, port).await?;
        }

        Command::Lan => {
            let stored = token::load(&token_path)?;
            let (base_url, api_base) =
                freebox::discover(Some(&stored.freebox_url)).await?;
            let client =
                freebox::open_session(&base_url, &api_base, &stored.app_id, &stored.app_token)
                    .await?;

            let hosts = client.lan_browser().await?;
            println!("Devices LAN ({} total):", hosts.len());
            nat::print_lan_hosts(&hosts);
        }
    }

    Ok(())
}
