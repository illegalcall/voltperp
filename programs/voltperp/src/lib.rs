use anchor_lang::prelude::*;

pub mod errors;
pub mod events;
pub mod instructions;
pub mod math;
pub mod state;

use instructions::*;

declare_id!("VPERPkD5RqE4rjxHYbRPm74LLNAYxJ6omBMyJEauvXw");

#[program]
pub mod voltperp {
    use super::*;

    /// Initialize the exchange: creates exchange state PDA, collateral vault,
    /// and insurance fund vault.
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        instructions::initialize::handle_initialize(ctx)
    }

    /// Add a new perpetual market with vAMM parameters.
    #[allow(clippy::too_many_arguments)]
    pub fn add_market(
        ctx: Context<AddMarket>,
        market_index: u8,
        oracle_feed: Pubkey,
        symbol: [u8; 12],
        base_asset_reserve: u128,
        quote_asset_reserve: u128,
        peg_multiplier: u128,
        funding_period: i64,
        taker_fee_bps: u16,
        max_leverage: u8,
        maintenance_margin_ratio: u32,
        initial_margin_ratio: u32,
        liquidation_fee_bps: u16,
        insurance_fee_bps: u16,
        max_oracle_staleness: u32,
    ) -> Result<()> {
        instructions::add_market::handle_add_market(
            ctx,
            market_index,
            oracle_feed,
            symbol,
            base_asset_reserve,
            quote_asset_reserve,
            peg_multiplier,
            funding_period,
            taker_fee_bps,
            max_leverage,
            maintenance_margin_ratio,
            initial_margin_ratio,
            liquidation_fee_bps,
            insurance_fee_bps,
            max_oracle_staleness,
        )
    }

    /// Deposit USDC collateral into the exchange.
    /// Creates the user account on first deposit via `init_if_needed`.
    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handle_deposit(ctx, amount)
    }

    /// Withdraw USDC collateral from the exchange.
    /// Validates margin requirements if user has open positions.
    pub fn withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
        instructions::withdraw::handle_withdraw(ctx, amount)
    }

    /// Open a long or short perpetual position with leverage.
    pub fn open_position(
        ctx: Context<OpenPosition>,
        market_index: u8,
        quote_amount: u64,
        is_long: bool,
        leverage: u8,
    ) -> Result<()> {
        instructions::open_position::handle_open_position(
            ctx,
            market_index,
            quote_amount,
            is_long,
            leverage,
        )
    }

    /// Close an existing position and settle PnL.
    pub fn close_position(ctx: Context<ClosePosition>, market_index: u8) -> Result<()> {
        instructions::close_position::handle_close_position(ctx, market_index)
    }

    /// Liquidate an under-collateralized position.
    /// Multi-tiered: partial (50%) if health > 50% of maintenance, full otherwise.
    pub fn liquidate(ctx: Context<Liquidate>, market_index: u8) -> Result<()> {
        instructions::liquidate::handle_liquidate(ctx, market_index)
    }

    /// Settle funding rate for a market and optionally apply to a user.
    /// Anyone can crank this after the funding period has elapsed.
    pub fn settle_funding(ctx: Context<SettleFunding>, market_index: u8) -> Result<()> {
        instructions::settle_funding::handle_settle_funding(ctx, market_index)
    }

    /// Update the oracle price for a market. Authority-only.
    pub fn update_oracle(
        ctx: Context<UpdateOracle>,
        market_index: u8,
        price: u64,
        twap: u64,
    ) -> Result<()> {
        instructions::update_oracle::handle_update_oracle(ctx, market_index, price, twap)
    }
}
