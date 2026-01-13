use crate::errors::VoltPerpError;
use crate::math::vamm::PRICE_PRECISION;
use crate::state::{Market, Position, UserAccount, MAX_POSITIONS};
use anchor_lang::prelude::*;

/// Margin ratio precision (1e6)
pub const MARGIN_PRECISION: u64 = 1_000_000;

/// Health factor precision (1e6). Health = 1_000_000 means exactly at maintenance margin.
pub const HEALTH_PRECISION: u64 = 1_000_000;

/// Partial liquidation threshold: health factor > 50% of maintenance
pub const PARTIAL_LIQUIDATION_THRESHOLD: u64 = 500_000; // 50% in health precision

/// Calculate the unrealized PnL for a single position given the current mark price.
/// For long: pnl = base_amount * (mark_price - entry_price) / PRICE_PRECISION
/// For short: pnl = base_amount * (entry_price - mark_price) / PRICE_PRECISION
pub fn calculate_unrealized_pnl(position: &Position, mark_price: u64) -> Result<i64> {
    if position.is_empty() {
        return Ok(0);
    }

    let mark = mark_price as i128;
    let entry = position.entry_price as i128;
    let base = position.base_asset_amount as i128;

    let price_diff = if position.is_long {
        mark.checked_sub(entry).ok_or(VoltPerpError::MathOverflow)?
    } else {
        entry.checked_sub(mark).ok_or(VoltPerpError::MathOverflow)?
    };

    let pnl = base
        .checked_mul(price_diff)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(PRICE_PRECISION as i128)
        .ok_or(VoltPerpError::DivisionByZero)?;

    i64::try_from(pnl).map_err(|_| error!(VoltPerpError::CastOverflow))
}

/// Calculate the health factor for a user account.
/// health = (collateral + total_unrealized_pnl) / total_margin_requirement
/// Returns value in HEALTH_PRECISION (1e6). health < HEALTH_PRECISION means liquidatable.
pub fn calculate_health_factor(
    user: &UserAccount,
    markets: &[Market],
    mark_prices: &[u64],
) -> Result<u64> {
    let mut total_margin_required: u128 = 0;
    let mut total_unrealized_pnl: i128 = 0;

    for i in 0..MAX_POSITIONS {
        let pos = &user.positions[i];
        if pos.is_empty() {
            continue;
        }

        // Find matching market and price
        let market_idx = pos.market_index as usize;
        if market_idx >= markets.len() || market_idx >= mark_prices.len() {
            continue;
        }

        let market = &markets[market_idx];
        let mark_price = mark_prices[market_idx];

        // Calculate unrealized PnL
        let pnl = calculate_unrealized_pnl(pos, mark_price)?;
        total_unrealized_pnl = total_unrealized_pnl
            .checked_add(pnl as i128)
            .ok_or(VoltPerpError::MathOverflow)?;

        // Calculate notional value at current price
        let notional = (pos.base_asset_amount as u128)
            .checked_mul(mark_price as u128)
            .ok_or(VoltPerpError::MathOverflow)?
            .checked_div(PRICE_PRECISION)
            .ok_or(VoltPerpError::DivisionByZero)?;

        // Margin required = notional * maintenance_margin_ratio / MARGIN_PRECISION
        let margin = notional
            .checked_mul(market.maintenance_margin_ratio as u128)
            .ok_or(VoltPerpError::MathOverflow)?
            .checked_div(MARGIN_PRECISION as u128)
            .ok_or(VoltPerpError::DivisionByZero)?;

        total_margin_required = total_margin_required
            .checked_add(margin)
            .ok_or(VoltPerpError::MathOverflow)?;
    }

    // If no margin required, user is perfectly healthy
    if total_margin_required == 0 {
        return Ok(u64::MAX);
    }

    // Effective equity = collateral + unrealized PnL
    let collateral_i128 = user.collateral as i128;
    let equity = collateral_i128
        .checked_add(total_unrealized_pnl)
        .ok_or(VoltPerpError::MathOverflow)?;

    // If equity is negative or zero, health is 0
    if equity <= 0 {
        return Ok(0);
    }

    let equity_u128 = equity as u128;

    // health_factor = equity * HEALTH_PRECISION / total_margin_required
    let health = equity_u128
        .checked_mul(HEALTH_PRECISION as u128)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(total_margin_required)
        .ok_or(VoltPerpError::DivisionByZero)?;

    Ok(u64::try_from(health).unwrap_or(u64::MAX))
}

/// Check if a position is liquidatable (health_factor < HEALTH_PRECISION)
pub fn is_liquidatable(health_factor: u64) -> bool {
    health_factor < HEALTH_PRECISION
}

/// Determine if partial or full liquidation:
/// If health > PARTIAL_LIQUIDATION_THRESHOLD => partial (liquidate 50% of position)
/// Otherwise => full liquidation
pub fn is_partial_liquidation(health_factor: u64) -> bool {
    health_factor >= PARTIAL_LIQUIDATION_THRESHOLD
}

/// Check initial margin requirement when opening a position.
/// Required margin = notional / max_leverage
/// Returns true if user has sufficient margin.
pub fn check_initial_margin(
    collateral: u64,
    total_unrealized_pnl: i64,
    notional_value: u64,
    initial_margin_ratio: u32,
) -> Result<bool> {
    let equity = (collateral as i128)
        .checked_add(total_unrealized_pnl as i128)
        .ok_or(VoltPerpError::MathOverflow)?;

    if equity <= 0 {
        return Ok(false);
    }

    // Required margin = notional * initial_margin_ratio / MARGIN_PRECISION
    let required = (notional_value as u128)
        .checked_mul(initial_margin_ratio as u128)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(MARGIN_PRECISION as u128)
        .ok_or(VoltPerpError::DivisionByZero)?;

    Ok(equity as u128 >= required)
}

/// Calculate the fee for a trade: notional * fee_bps / 10_000
pub fn calculate_fee(notional_value: u64, fee_bps: u16) -> Result<u64> {
    let fee = (notional_value as u128)
        .checked_mul(fee_bps as u128)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(10_000)
        .ok_or(VoltPerpError::DivisionByZero)?;

    u64::try_from(fee).map_err(|_| error!(VoltPerpError::CastOverflow))
}
