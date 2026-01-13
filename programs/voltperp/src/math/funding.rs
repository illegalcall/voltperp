use crate::errors::VoltPerpError;
use anchor_lang::prelude::*;

/// Funding rate precision (1e12)
pub const FUNDING_PRECISION: i128 = 1_000_000_000_000;

/// Maximum funding rate cap: ±0.1% per period = ±0.001
pub const MAX_FUNDING_RATE: i128 = 1_000_000_000; // 0.001 * 1e12

/// Calculate the funding rate based on mark price vs index (oracle) price:
/// funding_rate = (mark_price - index_price) * FUNDING_PRECISION / index_price
/// Capped at ±MAX_FUNDING_RATE (±0.1%)
pub fn calculate_funding_rate(mark_price: u64, index_price: u64) -> Result<i128> {
    require!(index_price > 0, VoltPerpError::InvalidOraclePrice);

    let mark = mark_price as i128;
    let index = index_price as i128;

    let price_diff = mark.checked_sub(index).ok_or(VoltPerpError::MathOverflow)?;

    let funding_rate = price_diff
        .checked_mul(FUNDING_PRECISION)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(index)
        .ok_or(VoltPerpError::DivisionByZero)?;

    // Cap funding rate at ±0.1%
    let capped = funding_rate.max(-MAX_FUNDING_RATE).min(MAX_FUNDING_RATE);

    Ok(capped)
}

/// Calculate the funding payment for a position:
/// For longs: payment = base_amount * (cumulative_rate - last_rate) / FUNDING_PRECISION
/// For shorts: payment = -base_amount * (cumulative_rate - last_rate) / FUNDING_PRECISION
/// Positive = user pays, Negative = user receives
pub fn calculate_funding_payment(
    base_asset_amount: u64,
    is_long: bool,
    cumulative_funding_rate: i128,
    last_cumulative_funding: i128,
) -> Result<i64> {
    let rate_diff = cumulative_funding_rate
        .checked_sub(last_cumulative_funding)
        .ok_or(VoltPerpError::MathOverflow)?;

    let payment = (base_asset_amount as i128)
        .checked_mul(rate_diff)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(FUNDING_PRECISION)
        .ok_or(VoltPerpError::DivisionByZero)?;

    // Longs pay positive funding when mark > index
    // Shorts receive positive funding when mark > index
    let signed_payment = if is_long { payment } else { -payment };

    i64::try_from(signed_payment).map_err(|_| error!(VoltPerpError::CastOverflow))
}

/// Check if funding period has elapsed
pub fn is_funding_period_elapsed(
    last_funding_timestamp: i64,
    funding_period: i64,
    current_timestamp: i64,
) -> bool {
    current_timestamp >= last_funding_timestamp.saturating_add(funding_period)
}
