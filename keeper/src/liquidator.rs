use anchor_lang::prelude::Pubkey;
use solana_account_decoder::UiAccountEncoding;
use solana_client::rpc_client::RpcClient;
use solana_client::rpc_config::{RpcAccountInfoConfig, RpcProgramAccountsConfig};
use solana_client::rpc_filter::{Memcmp, RpcFilterType};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::Keypair;
use std::collections::HashMap;
use tracing::{debug, info, warn};

use crate::metrics::Metrics;

/// Cached market state for health factor calculation.
#[derive(Debug, Clone)]
struct CachedMarket {
    pub market_index: u8,
    pub last_oracle_price: u64,
    pub maintenance_margin_ratio: u32,
    pub liquidation_fee_bps: u16,
    pub insurance_fee_bps: u16,
    pub cumulative_funding_rate_long: i128,
    pub cumulative_funding_rate_short: i128,
}

/// Cached user position for health calculation.
#[derive(Debug, Clone)]
struct CachedPosition {
    pub market_index: u8,
    pub is_long: bool,
    pub base_asset_amount: u64,
    pub entry_price: u64,
    pub last_cumulative_funding: i128,
}

/// Cached user account.
#[derive(Debug, Clone)]
struct CachedUser {
    pub address: Pubkey,
    pub authority: Pubkey,
    pub collateral: u64,
    pub positions: Vec<CachedPosition>,
}

const PRICE_PRECISION: u128 = 1_000_000;
const MARGIN_PRECISION: u128 = 1_000_000;
const HEALTH_PRECISION: u64 = 1_000_000;

/// Run a single liquidation cycle: fetch accounts, check health, liquidate.
pub async fn run_liquidation_cycle(
    rpc: &RpcClient,
    program_id: &Pubkey,
    _keypair: &Keypair,
    max_liquidations: usize,
    dry_run: bool,
    metrics: &Metrics,
) -> anyhow::Result<usize> {
    // Fetch all market accounts
    let markets = fetch_markets(rpc, program_id)?;
    if markets.is_empty() {
        debug!("No markets found");
        return Ok(0);
    }

    // Fetch all user accounts
    let users = fetch_user_accounts(rpc, program_id)?;
    metrics.set_positions_monitored(users.len());

    let mut liquidation_count = 0;

    // Sort users by health (worst first) for priority liquidation
    let mut underwater: Vec<(CachedUser, u64)> = Vec::new();

    for user in &users {
        if user.positions.is_empty() || user.positions.iter().all(|p| p.base_asset_amount == 0) {
            continue;
        }

        let health = calculate_health_factor(user, &markets);
        if health < HEALTH_PRECISION {
            underwater.push((user.clone(), health));
        }
    }

    // Sort by health ascending (worst first)
    underwater.sort_by_key(|(_, h)| *h);

    for (user, health) in underwater.iter().take(max_liquidations) {
        if dry_run {
            info!(
                "DRY RUN: Would liquidate {} (health: {:.4})",
                user.authority,
                *health as f64 / HEALTH_PRECISION as f64
            );
        } else {
            info!(
                "Liquidating {} (health: {:.4})",
                user.authority,
                *health as f64 / HEALTH_PRECISION as f64
            );
            // In production: build and submit the liquidation transaction
            // using anchor_client to call the liquidate instruction.
            // For each underwater position, submit a liquidate ix.
            warn!("Transaction submission not yet wired — connect anchor_client for mainnet");
        }
        liquidation_count += 1;
        metrics.record_liquidation();
    }

    Ok(liquidation_count)
}

/// Calculate health factor across all positions.
fn calculate_health_factor(user: &CachedUser, markets: &HashMap<u8, CachedMarket>) -> u64 {
    let mut total_unrealized_pnl: i128 = 0;
    let mut total_margin_required: u128 = 0;

    for pos in &user.positions {
        if pos.base_asset_amount == 0 {
            continue;
        }

        let market = match markets.get(&pos.market_index) {
            Some(m) => m,
            None => continue,
        };

        // Unrealized PnL
        let mark = market.last_oracle_price as i128;
        let entry = pos.entry_price as i128;
        let base = pos.base_asset_amount as i128;

        let price_diff = if pos.is_long {
            mark - entry
        } else {
            entry - mark
        };
        let pnl = base * price_diff / PRICE_PRECISION as i128;
        total_unrealized_pnl += pnl;

        // Margin requirement
        let notional = (pos.base_asset_amount as u128) * (market.last_oracle_price as u128)
            / PRICE_PRECISION;
        let margin =
            notional * (market.maintenance_margin_ratio as u128) / MARGIN_PRECISION;
        total_margin_required += margin;
    }

    if total_margin_required == 0 {
        return u64::MAX;
    }

    let equity = user.collateral as i128 + total_unrealized_pnl;
    if equity <= 0 {
        return 0;
    }

    let health = (equity as u128) * (HEALTH_PRECISION as u128) / total_margin_required;
    health.min(u64::MAX as u128) as u64
}

/// Fetch all Market accounts from the program using getProgramAccounts.
/// Filters by the 8-byte Anchor discriminator for the Market account type.
fn fetch_markets(
    rpc: &RpcClient,
    program_id: &Pubkey,
) -> anyhow::Result<HashMap<u8, CachedMarket>> {
    // Anchor discriminator for Market = sha256("account:Market")[..8]
    let discriminator: [u8; 8] = [
        0xf0, 0xd3, 0x80, 0x68, 0x11, 0x65, 0x02, 0x3f,
    ];

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            discriminator.to_vec(),
        ))]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        ..Default::default()
    };

    let accounts = rpc.get_program_accounts_with_config(program_id, config)?;
    let mut markets = HashMap::new();

    for (_pubkey, account) in accounts {
        let data = &account.data;
        if data.len() < 9 {
            continue;
        }
        // Parse market_index at offset 8 (first field after discriminator)
        let market_index = data[8];
        // In production: fully deserialize with AnchorDeserialize
        // For now, extract key fields at known offsets
        markets.insert(
            market_index,
            CachedMarket {
                market_index,
                last_oracle_price: 0,
                maintenance_margin_ratio: 50_000,
                liquidation_fee_bps: 500,
                insurance_fee_bps: 100,
                cumulative_funding_rate_long: 0,
                cumulative_funding_rate_short: 0,
            },
        );
    }

    Ok(markets)
}

/// Fetch all UserAccount accounts from the program using getProgramAccounts.
fn fetch_user_accounts(
    rpc: &RpcClient,
    program_id: &Pubkey,
) -> anyhow::Result<Vec<CachedUser>> {
    // Anchor discriminator for UserAccount = sha256("account:UserAccount")[..8]
    let discriminator: [u8; 8] = [
        0x21, 0x19, 0x6a, 0x6e, 0xbf, 0x73, 0x04, 0x6a,
    ];

    let config = RpcProgramAccountsConfig {
        filters: Some(vec![RpcFilterType::Memcmp(Memcmp::new_raw_bytes(
            0,
            discriminator.to_vec(),
        ))]),
        account_config: RpcAccountInfoConfig {
            encoding: Some(UiAccountEncoding::Base64),
            commitment: Some(CommitmentConfig::confirmed()),
            ..Default::default()
        },
        ..Default::default()
    };

    let accounts = rpc.get_program_accounts_with_config(program_id, config)?;
    let mut users = Vec::new();

    for (pubkey, account) in accounts {
        let data = &account.data;
        if data.len() < 40 {
            continue;
        }
        // In production: fully deserialize UserAccount with AnchorDeserialize
        // Simplified: extract authority (Pubkey at offset 8) and collateral (u64 at offset 40)
        let mut authority_bytes = [0u8; 32];
        authority_bytes.copy_from_slice(&data[8..40]);
        let authority = Pubkey::new_from_array(authority_bytes);

        users.push(CachedUser {
            address: pubkey,
            authority,
            collateral: 0, // Would parse from data offset
            positions: Vec::new(), // Would parse Position array from data
        });
    }

    Ok(users)
}
