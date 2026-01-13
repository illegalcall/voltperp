use crate::errors::VoltPerpError;
use crate::state::Market;
use anchor_lang::prelude::*;

/// Precision constants
pub const PRICE_PRECISION: u128 = 1_000_000; // 1e6
pub const BASE_PRECISION: u128 = 1_000_000_000; // 1e9
pub const PEG_PRECISION: u128 = 1_000_000; // 1e6

/// Calculate mark price from vAMM reserves:
/// mark_price = (quote_reserve * peg_multiplier) / base_reserve
pub fn get_mark_price(market: &Market) -> Result<u64> {
    require!(market.base_asset_reserve > 0, VoltPerpError::DivisionByZero);

    let numerator = (market.quote_asset_reserve as u128)
        .checked_mul(market.peg_multiplier)
        .ok_or(VoltPerpError::MathOverflow)?;

    let price = numerator
        .checked_div(market.base_asset_reserve as u128)
        .ok_or(VoltPerpError::DivisionByZero)?;

    // Price should fit in u64 for our precision
    u64::try_from(price).map_err(|_| error!(VoltPerpError::CastOverflow))
}

/// Result of a vAMM swap operation
pub struct SwapResult {
    /// Amount of base asset received/sold
    pub base_asset_amount: u64,
    /// Amount of quote asset spent/received
    pub quote_asset_amount: u64,
    /// New base reserve after swap
    pub new_base_reserve: u128,
    /// New quote reserve after swap
    pub new_quote_reserve: u128,
}

/// Swap quote for base (opening a long position or closing a short):
/// User adds quote_amount to the pool, receives base tokens.
/// Uses constant product: base_new = k / (quote + quote_amount)
/// base_received = base_old - base_new
pub fn swap_quote_for_base(market: &Market, quote_amount: u64) -> Result<SwapResult> {
    require!(quote_amount > 0, VoltPerpError::ZeroPositionSize);

    let k = (market.base_asset_reserve)
        .checked_mul(market.quote_asset_reserve)
        .ok_or(VoltPerpError::MathOverflow)?;

    let scaled_quote_amount = (quote_amount as u128)
        .checked_mul(PEG_PRECISION)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(market.peg_multiplier)
        .ok_or(VoltPerpError::DivisionByZero)?;

    let new_quote_reserve = market
        .quote_asset_reserve
        .checked_add(scaled_quote_amount)
        .ok_or(VoltPerpError::MathOverflow)?;

    require!(new_quote_reserve > 0, VoltPerpError::DivisionByZero);

    let new_base_reserve = k
        .checked_div(new_quote_reserve)
        .ok_or(VoltPerpError::DivisionByZero)?;

    let base_received = market
        .base_asset_reserve
        .checked_sub(new_base_reserve)
        .ok_or(VoltPerpError::MathOverflow)?;

    let base_asset_amount =
        u64::try_from(base_received).map_err(|_| error!(VoltPerpError::CastOverflow))?;

    Ok(SwapResult {
        base_asset_amount,
        quote_asset_amount: quote_amount,
        new_base_reserve,
        new_quote_reserve,
    })
}

/// Swap base for quote (closing a long position or opening a short):
/// User adds base_amount to the pool, receives quote tokens.
/// Uses constant product: quote_new = k / (base + base_amount)
/// quote_received = quote_old - quote_new
pub fn swap_base_for_quote(market: &Market, base_amount: u64) -> Result<SwapResult> {
    require!(base_amount > 0, VoltPerpError::ZeroPositionSize);

    let k = (market.base_asset_reserve)
        .checked_mul(market.quote_asset_reserve)
        .ok_or(VoltPerpError::MathOverflow)?;

    let new_base_reserve = market
        .base_asset_reserve
        .checked_add(base_amount as u128)
        .ok_or(VoltPerpError::MathOverflow)?;

    require!(new_base_reserve > 0, VoltPerpError::DivisionByZero);

    let new_quote_reserve = k
        .checked_div(new_base_reserve)
        .ok_or(VoltPerpError::DivisionByZero)?;

    let quote_released = market
        .quote_asset_reserve
        .checked_sub(new_quote_reserve)
        .ok_or(VoltPerpError::MathOverflow)?;

    let quote_scaled = quote_released
        .checked_mul(market.peg_multiplier)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(PEG_PRECISION)
        .ok_or(VoltPerpError::DivisionByZero)?;

    let quote_asset_amount =
        u64::try_from(quote_scaled).map_err(|_| error!(VoltPerpError::CastOverflow))?;

    Ok(SwapResult {
        base_asset_amount: base_amount,
        quote_asset_amount,
        new_base_reserve,
        new_quote_reserve,
    })
}

/// Calculate the entry price for a position given base and quote amounts
/// entry_price = quote_asset_amount * PRICE_PRECISION / base_asset_amount
pub fn calculate_entry_price(quote_amount: u64, base_amount: u64) -> Result<u64> {
    require!(base_amount > 0, VoltPerpError::DivisionByZero);

    let price = (quote_amount as u128)
        .checked_mul(PRICE_PRECISION)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(base_amount as u128)
        .ok_or(VoltPerpError::DivisionByZero)?;

    u64::try_from(price).map_err(|_| error!(VoltPerpError::CastOverflow))
}
