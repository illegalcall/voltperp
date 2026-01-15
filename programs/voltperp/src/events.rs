use anchor_lang::prelude::*;

#[event]
pub struct ExchangeInitialized {
    pub authority: Pubkey,
    pub collateral_mint: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct MarketAdded {
    pub market_index: u8,
    pub symbol: [u8; 12],
    pub oracle_feed: Pubkey,
    pub base_asset_reserve: u128,
    pub quote_asset_reserve: u128,
    pub peg_multiplier: u128,
    pub timestamp: i64,
}

#[event]
pub struct CollateralDeposited {
    pub user: Pubkey,
    pub amount: u64,
    pub total_collateral: u64,
    pub timestamp: i64,
}

#[event]
pub struct CollateralWithdrawn {
    pub user: Pubkey,
    pub amount: u64,
    pub remaining_collateral: u64,
    pub timestamp: i64,
}

#[event]
pub struct PositionOpened {
    pub user: Pubkey,
    pub market_index: u8,
    pub is_long: bool,
    pub base_asset_amount: u64,
    pub quote_asset_amount: u64,
    pub entry_price: u64,
    pub leverage: u8,
    pub timestamp: i64,
}

#[event]
pub struct PositionClosed {
    pub user: Pubkey,
    pub market_index: u8,
    pub base_asset_amount: u64,
    pub quote_asset_amount: u64,
    pub realized_pnl: i64,
    pub fee: u64,
    pub timestamp: i64,
}

#[event]
pub struct PositionLiquidated {
    pub user: Pubkey,
    pub liquidator: Pubkey,
    pub market_index: u8,
    pub base_asset_amount: u64,
    pub liquidation_price: u64,
    pub is_partial: bool,
    pub liquidator_reward: u64,
    pub insurance_fee: u64,
    pub timestamp: i64,
}

#[event]
pub struct FundingSettled {
    pub market_index: u8,
    pub funding_rate_long: i128,
    pub funding_rate_short: i128,
    pub mark_price: u64,
    pub oracle_price: u64,
    pub timestamp: i64,
}

#[event]
pub struct FundingAppliedToUser {
    pub user: Pubkey,
    pub market_index: u8,
    pub funding_payment: i64,
    pub timestamp: i64,
}
