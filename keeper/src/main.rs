use clap::Parser;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::read_keypair_file;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use tracing::{error, info, warn};

mod funding_crank;
mod liquidator;
mod metrics;

use metrics::Metrics;

#[derive(Parser, Debug)]
#[command(name = "voltperp-keeper", about = "Liquidation keeper and funding crank for VoltPerp")]
struct Args {
    /// Solana RPC endpoint URL
    #[arg(long, default_value = "https://api.devnet.solana.com")]
    rpc_url: String,

    /// Path to the keeper's keypair file
    #[arg(long, default_value_t = default_keypair_path())]
    keypair: String,

    /// VoltPerp program ID
    #[arg(long, default_value = "VPERPkD5RqE4rjxHYbRPm74LLNAYxJ6omBMyJEauvXw")]
    program_id: String,

    /// Liquidation polling interval in milliseconds
    #[arg(long, default_value = "2000")]
    liquidation_interval_ms: u64,

    /// Funding crank polling interval in milliseconds
    #[arg(long, default_value = "30000")]
    funding_interval_ms: u64,

    /// Dry run mode — simulate but don't submit transactions
    #[arg(long, default_value = "false")]
    dry_run: bool,

    /// Maximum liquidations per polling cycle
    #[arg(long, default_value = "5")]
    max_liquidations_per_cycle: usize,
}

fn default_keypair_path() -> String {
    dirs::home_dir()
        .map(|p| p.join(".config/solana/id.json").display().to_string())
        .unwrap_or_else(|| "~/.config/solana/id.json".to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let args = Args::parse();

    info!("Starting VoltPerp Keeper");
    info!("RPC: {}", args.rpc_url);
    info!("Program ID: {}", args.program_id);
    info!("Dry run: {}", args.dry_run);

    let program_id = Pubkey::from_str(&args.program_id)?;
    let keypair = read_keypair_file(&args.keypair)
        .map_err(|e| anyhow::anyhow!("Failed to read keypair at {}: {}", args.keypair, e))?;

    let rpc = Arc::new(RpcClient::new_with_commitment(
        args.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    ));

    // Verify connectivity
    let balance = rpc.get_balance(&keypair.pubkey())?;
    info!(
        "Keeper wallet: {} (balance: {} SOL)",
        keypair.pubkey(),
        balance as f64 / 1e9
    );
    if balance < 10_000_000 {
        warn!("Keeper wallet balance is low — may not be able to submit transactions");
    }

    let metrics = Arc::new(Metrics::new());
    let metrics_clone = metrics.clone();

    // Spawn metrics reporter
    let metrics_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            metrics_clone.log_summary();
        }
    });

    // Spawn liquidation loop
    let rpc_liq = rpc.clone();
    let metrics_liq = metrics.clone();
    let liquidation_handle = tokio::spawn(async move {
        let mut interval =
            tokio::time::interval(Duration::from_millis(args.liquidation_interval_ms));
        loop {
            interval.tick().await;
            match liquidator::run_liquidation_cycle(
                &rpc_liq,
                &program_id,
                &keypair,
                args.max_liquidations_per_cycle,
                args.dry_run,
                &metrics_liq,
            )
            .await
            {
                Ok(count) => {
                    if count > 0 {
                        info!("Liquidated {} positions", count);
                    }
                }
                Err(e) => {
                    error!("Liquidation cycle error: {}", e);
                    metrics_liq.record_liquidation_failure();
                }
            }
        }
    });

    // Spawn funding crank loop
    let rpc_fund = rpc.clone();
    let metrics_fund = metrics.clone();
    let funding_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(args.funding_interval_ms));
        loop {
            interval.tick().await;
            match funding_crank::run_funding_cycle(&rpc_fund, &program_id, &metrics_fund).await {
                Ok(settled) => {
                    if settled > 0 {
                        info!("Settled funding for {} markets", settled);
                    }
                }
                Err(e) => {
                    error!("Funding crank error: {}", e);
                    metrics_fund.record_funding_failure();
                }
            }
        }
    });

    info!("All keeper tasks running. Press Ctrl+C to stop.");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    info!("Shutting down keeper...");

    metrics.log_summary();

    liquidation_handle.abort();
    funding_handle.abort();
    metrics_handle.abort();

    info!("Keeper stopped.");
    Ok(())
}
