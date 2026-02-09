use anchor_lang::prelude::*;

use crate::errors::VoltPerpError;
use crate::events::MarketAdded;
use crate::state::{ExchangeState, Market};

/// Maximum number of markets the exchange supports.
pub const MAX_MARKETS: u8 = 16;

#[derive(Accounts)]
#[instruction(market_index: u8)]
pub struct AddMarket<'info> {
    /// Exchange authority -- must match `exchange_state.authority`.
    #[account(
        mut,
        constraint = authority.key() == exchange_state.authority @ VoltPerpError::UnauthorizedAuthority,
    )]
    pub authority: Signer<'info>,

    /// Exchange state PDA.
    #[account(
        mut,
        seeds = [ExchangeState::SEED.as_ref()],
        bump = exchange_state.bump,
        constraint = !exchange_state.paused @ VoltPerpError::ExchangePaused,
    )]
    pub exchange_state: Account<'info, ExchangeState>,

    /// The new market PDA -- derived from `b"market"` + `[market_index]`.
    #[account(
        init,
        payer = authority,
        space = 8 + Market::INIT_SPACE,
        seeds = [Market::SEED.as_ref(), &[market_index]],
        bump,
    )]
    pub market: Account<'info, Market>,

    pub system_program: Program<'info, System>,
}

#[allow(clippy::too_many_arguments)]
pub fn handle_add_market(
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
    let exchange = &mut ctx.accounts.exchange_state;

    require!(
        exchange.num_markets < MAX_MARKETS,
        VoltPerpError::MaxMarketsReached
    );
    require!(
        market_index == exchange.num_markets,
        VoltPerpError::InvalidMarketIndex
    );
    require!(base_asset_reserve > 0, VoltPerpError::InvalidVammParams);
    require!(quote_asset_reserve > 0, VoltPerpError::InvalidVammParams);
    require!(peg_multiplier > 0, VoltPerpError::InvalidVammParams);
    require!(funding_period > 0, VoltPerpError::InvalidFundingPeriod);
    require!(symbol[0] != 0, VoltPerpError::EmptyMarketSymbol);

    // Calculate sqrt_k using integer square root approximation.
    let k = base_asset_reserve
        .checked_mul(quote_asset_reserve)
        .ok_or(VoltPerpError::MathOverflow)?;
    let sqrt_k = integer_sqrt(k);

    let clock = Clock::get()?;

    let market = &mut ctx.accounts.market;
    market.market_index = market_index;
    market.oracle_feed = oracle_feed;
    market.symbol = symbol;
    market.base_asset_reserve = base_asset_reserve;
    market.quote_asset_reserve = quote_asset_reserve;
    market.sqrt_k = sqrt_k;
    market.peg_multiplier = peg_multiplier;
    market.total_long_base = 0;
    market.total_short_base = 0;
    market.open_interest = 0;
    market.cumulative_funding_rate_long = 0;
    market.cumulative_funding_rate_short = 0;
    market.last_funding_timestamp = clock.unix_timestamp;
    market.funding_period = funding_period;
    market.taker_fee_bps = taker_fee_bps;
    market.max_leverage = max_leverage;
    market.maintenance_margin_ratio = maintenance_margin_ratio;
    market.initial_margin_ratio = initial_margin_ratio;
    market.liquidation_fee_bps = liquidation_fee_bps;
    market.insurance_fee_bps = insurance_fee_bps;
    market.last_oracle_price = 0;
    market.last_oracle_twap = 0;
    market.last_oracle_timestamp = 0;
    market.max_oracle_staleness = max_oracle_staleness;
    market.bump = ctx.bumps.market;

    exchange.num_markets = exchange
        .num_markets
        .checked_add(1)
        .ok_or(VoltPerpError::MathOverflow)?;

    emit!(MarketAdded {
        market_index,
        symbol,
        oracle_feed,
        base_asset_reserve,
        quote_asset_reserve,
        peg_multiplier,
        timestamp: clock.unix_timestamp,
    });

    msg!("Market {} added", market_index);
    Ok(())
}

/// Integer square root via Newton's method.
fn integer_sqrt(n: u128) -> u128 {
    if n == 0 {
        return 0;
    }
    let mut x = n;
    let mut y = (x + 1) / 2;
    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }
    x
}
