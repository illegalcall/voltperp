use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct Market {
    /// Index of this market (0, 1, 2, ...)
    pub market_index: u8,
    /// Oracle price feed address
    pub oracle_feed: Pubkey,
    /// Market symbol, e.g. b"SOL-PERP\0\0\0\0"
    pub symbol: [u8; 12],
    /// vAMM base asset reserve
    pub base_asset_reserve: u128,
    /// vAMM quote asset reserve
    pub quote_asset_reserve: u128,
    /// sqrt(k) = sqrt(base * quote) for invariant tracking
    pub sqrt_k: u128,
    /// Peg multiplier (price scaling factor, precision 1e6)
    pub peg_multiplier: u128,
    /// Total long base asset amount across all positions
    pub total_long_base: u128,
    /// Total short base asset amount across all positions
    pub total_short_base: u128,
    /// Open interest in quote asset terms
    pub open_interest: u128,
    /// Cumulative funding rate for longs (precision 1e12)
    pub cumulative_funding_rate_long: i128,
    /// Cumulative funding rate for shorts (precision 1e12)
    pub cumulative_funding_rate_short: i128,
    /// Last time funding was settled (unix timestamp)
    pub last_funding_timestamp: i64,
    /// Funding period in seconds (e.g. 3600 for hourly)
    pub funding_period: i64,
    /// Taker fee in basis points (e.g. 10 = 0.10%)
    pub taker_fee_bps: u16,
    /// Maximum allowed leverage (e.g. 10 = 10x)
    pub max_leverage: u8,
    /// Maintenance margin ratio (precision 1e6, e.g. 50000 = 5%)
    pub maintenance_margin_ratio: u32,
    /// Initial margin ratio (precision 1e6, e.g. 100000 = 10%)
    pub initial_margin_ratio: u32,
    /// Liquidation fee in basis points paid to keeper
    pub liquidation_fee_bps: u16,
    /// Insurance fee in basis points taken during liquidation
    pub insurance_fee_bps: u16,
    /// Last recorded oracle price (precision 1e6)
    pub last_oracle_price: u64,
    /// Last recorded oracle TWAP (precision 1e6)
    pub last_oracle_twap: u64,
    /// Maximum oracle staleness in seconds
    pub max_oracle_staleness: u32,
    /// PDA bump seed
    pub bump: u8,
}

impl Market {
    pub const SEED: &'static [u8] = b"market";
}
