use anchor_lang::prelude::*;

use crate::errors::VoltPerpError;
use crate::events::PositionClosed;
use crate::math::margin::calculate_fee;
use crate::math::vamm::{get_mark_price, swap_base_for_quote, swap_quote_for_base};
use crate::state::{ExchangeState, Market, UserAccount};

#[derive(Accounts)]
#[instruction(market_index: u8)]
pub struct ClosePosition<'info> {
    /// The trader closing the position.
    #[account(mut)]
    pub user: Signer<'info>,

    /// User account PDA.
    #[account(
        mut,
        seeds = [UserAccount::SEED.as_ref(), user.key().as_ref()],
        bump = user_account.bump,
        constraint = user_account.authority == user.key() @ VoltPerpError::UnauthorizedAuthority,
    )]
    pub user_account: Account<'info, UserAccount>,

    /// Exchange state PDA.
    #[account(
        seeds = [ExchangeState::SEED.as_ref()],
        bump = exchange_state.bump,
        constraint = !exchange_state.paused @ VoltPerpError::ExchangePaused,
    )]
    pub exchange_state: Account<'info, ExchangeState>,

    /// Market PDA.
    #[account(
        mut,
        seeds = [Market::SEED.as_ref(), &[market_index]],
        bump = market.bump,
    )]
    pub market: Account<'info, Market>,
}

pub fn handle_close_position(ctx: Context<ClosePosition>, market_index: u8) -> Result<()> {
    let user_account = &ctx.accounts.user_account;

    // Find position.
    let pos_idx = user_account
        .find_position(market_index)
        .ok_or(VoltPerpError::PositionNotFound)?;

    let position = user_account.positions[pos_idx];
    require!(!position.is_empty(), VoltPerpError::EmptyPosition);

    let market = &ctx.accounts.market;
    let _mark_price = get_mark_price(market)?;

    // Close: reverse the swap.
    let market = &mut ctx.accounts.market;
    let swap_result = if position.is_long {
        // Long close: sell base back to get quote.
        swap_base_for_quote(market, position.base_asset_amount)?
    } else {
        // Short close: buy back base with quote.
        swap_quote_for_base(market, position.quote_asset_amount)?
    };

    // Update vAMM reserves.
    market.base_asset_reserve = swap_result.new_base_reserve;
    market.quote_asset_reserve = swap_result.new_quote_reserve;

    // Calculate PnL.
    let realized_pnl = if position.is_long {
        // Long PnL = exit_quote - entry_quote.
        (swap_result.quote_asset_amount as i64)
            .checked_sub(position.quote_asset_amount as i64)
            .ok_or(VoltPerpError::MathOverflow)?
    } else {
        // Short PnL = entry_quote - exit_quote.
        (position.quote_asset_amount as i64)
            .checked_sub(swap_result.quote_asset_amount as i64)
            .ok_or(VoltPerpError::MathOverflow)?
    };

    // Calculate close fee on the notional.
    let fee = calculate_fee(swap_result.quote_asset_amount, market.taker_fee_bps)?;

    // Update market open interest.
    if position.is_long {
        market.total_long_base = market
            .total_long_base
            .saturating_sub(position.base_asset_amount as u128);
    } else {
        market.total_short_base = market
            .total_short_base
            .saturating_sub(position.base_asset_amount as u128);
    }
    market.open_interest = market
        .open_interest
        .saturating_sub(position.quote_asset_amount as u128);

    // Update user account.
    let user_account = &mut ctx.accounts.user_account;

    // Apply PnL to collateral.
    if realized_pnl >= 0 {
        user_account.collateral = user_account
            .collateral
            .checked_add(realized_pnl as u64)
            .ok_or(VoltPerpError::MathOverflow)?;
    } else {
        let loss = realized_pnl.unsigned_abs();
        user_account.collateral = user_account.collateral.saturating_sub(loss);
    }

    // Deduct fee.
    user_account.collateral = user_account.collateral.saturating_sub(fee);
    user_account.total_fees_paid = user_account
        .total_fees_paid
        .checked_add(fee)
        .ok_or(VoltPerpError::MathOverflow)?;

    // Store realized PnL on position before clearing.
    user_account.positions[pos_idx].realized_pnl = user_account.positions[pos_idx]
        .realized_pnl
        .checked_add(realized_pnl)
        .ok_or(VoltPerpError::MathOverflow)?;

    // Clear position.
    user_account.positions[pos_idx] = Default::default();
    user_account.active_positions = user_account.active_positions.saturating_sub(1);

    let clock = Clock::get()?;
    user_account.last_active_slot = clock.slot;

    emit!(PositionClosed {
        user: ctx.accounts.user.key(),
        market_index,
        base_asset_amount: position.base_asset_amount,
        quote_asset_amount: swap_result.quote_asset_amount,
        realized_pnl,
        fee,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Closed position: market={}, pnl={}, fee={}",
        market_index,
        realized_pnl,
        fee,
    );
    Ok(())
}
