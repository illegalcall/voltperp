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

#[cfg(test)]
mod tests {
    use super::*;

    fn test_market() -> Market {
        Market {
            market_index: 0,
            oracle_feed: Pubkey::default(),
            symbol: *b"SOL-PERP\0\0\0\0",
            base_asset_reserve: 1_000_000_000_000,
            quote_asset_reserve: 100_000_000_000,
            sqrt_k: 0,
            peg_multiplier: PEG_PRECISION as u128,
            total_long_base: 0,
            total_short_base: 0,
            open_interest: 0,
            cumulative_funding_rate_long: 0,
            cumulative_funding_rate_short: 0,
            last_funding_timestamp: 0,
            funding_period: 3600,
            taker_fee_bps: 10,
            max_leverage: 10,
            maintenance_margin_ratio: 50_000,
            initial_margin_ratio: 100_000,
            liquidation_fee_bps: 500,
            insurance_fee_bps: 100,
            last_oracle_price: 100_000_000,
            last_oracle_twap: 100_000_000,
            last_oracle_timestamp: 0,
            max_oracle_staleness: 30,
            bump: 0,
        }
    }

    #[test]
    fn test_mark_price_positive() {
        let market = test_market();
        let price = get_mark_price(&market).unwrap();
        assert!(price > 0);
    }

    #[test]
    fn test_long_swap_reduces_base() {
        let market = test_market();
        let result = swap_quote_for_base(&market, 1_000_000).unwrap();
        assert!(result.base_asset_amount > 0);
        assert!(result.new_base_reserve < market.base_asset_reserve);
    }

    #[test]
    fn test_short_swap_reduces_quote() {
        let market = test_market();
        let result = swap_base_for_quote(&market, 1_000_000).unwrap();
        assert!(result.quote_asset_amount > 0);
        assert!(result.new_quote_reserve < market.quote_asset_reserve);
    }

    #[test]
    fn test_entry_price_calc() {
        let price = calculate_entry_price(100_000_000, 1_000_000).unwrap();
        assert_eq!(price, 100_000_000);
    }

    #[test]
    fn test_constant_product_preserved() {
        let market = test_market();
        let k_before = market.base_asset_reserve * market.quote_asset_reserve;
        let result = swap_quote_for_base(&market, 1_000_000).unwrap();
        let k_after = result.new_base_reserve * result.new_quote_reserve;
        let diff = k_before.abs_diff(k_after);
        assert!(diff < k_before / 1_000_000);
    }
}
