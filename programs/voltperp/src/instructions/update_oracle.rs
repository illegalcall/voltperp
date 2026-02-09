use anchor_lang::prelude::*;

use crate::errors::VoltPerpError;
use crate::state::{ExchangeState, Market};

#[derive(Accounts)]
#[instruction(market_index: u8)]
pub struct UpdateOracle<'info> {
    /// Only the exchange authority can push oracle updates.
    pub authority: Signer<'info>,

    /// Exchange state PDA.
    #[account(
        seeds = [ExchangeState::SEED.as_ref()],
        bump = exchange_state.bump,
        constraint = exchange_state.authority == authority.key() @ VoltPerpError::UnauthorizedAuthority,
        constraint = !exchange_state.paused @ VoltPerpError::ExchangePaused,
    )]
    pub exchange_state: Account<'info, ExchangeState>,

    /// Market PDA to update oracle for.
    #[account(
        mut,
        seeds = [Market::SEED.as_ref(), &[market_index]],
        bump = market.bump,
        constraint = market.market_index == market_index @ VoltPerpError::InvalidMarketIndex,
    )]
    pub market: Account<'info, Market>,
}

pub fn handle_update_oracle(
    ctx: Context<UpdateOracle>,
    _market_index: u8,
    price: u64,
    twap: u64,
    oracle_timestamp: i64,
) -> Result<()> {
    require!(price > 0, VoltPerpError::InvalidOraclePrice);
    require!(twap > 0, VoltPerpError::InvalidOraclePrice);

    let market = &mut ctx.accounts.market;

    // Validate timestamp is newer than last update (or first update).
    require!(
        oracle_timestamp > market.last_oracle_timestamp || market.last_oracle_price == 0,
        VoltPerpError::StaleOraclePrice
    );

    market.last_oracle_price = price;
    market.last_oracle_twap = twap;
    market.last_oracle_timestamp = oracle_timestamp;

    Ok(())
}
