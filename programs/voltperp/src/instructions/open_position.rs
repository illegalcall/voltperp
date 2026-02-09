use anchor_lang::prelude::*;

use crate::errors::VoltPerpError;
use crate::events::PositionOpened;
use crate::math::margin::{calculate_fee, check_initial_margin};
use crate::math::vamm::{
    calculate_entry_price, swap_base_for_quote, swap_quote_for_base, PRICE_PRECISION,
};
use crate::state::{ExchangeState, Market, UserAccount};

#[derive(Accounts)]
#[instruction(market_index: u8)]
pub struct OpenPosition<'info> {
    /// The trader opening the position.
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

    /// Market PDA for the target market.
    #[account(
        mut,
        seeds = [Market::SEED.as_ref(), &[market_index]],
        bump = market.bump,
    )]
    pub market: Account<'info, Market>,
}

pub fn handle_open_position(
    ctx: Context<OpenPosition>,
    market_index: u8,
    quote_amount: u64,
    is_long: bool,
    leverage: u8,
    limit_price: u64,
) -> Result<()> {
    let market = &ctx.accounts.market;
    let user_account = &ctx.accounts.user_account;

    require!(quote_amount > 0, VoltPerpError::ZeroPositionSize);
    require!(
        leverage > 0 && leverage <= market.max_leverage,
        VoltPerpError::MaxLeverageExceeded
    );

    // Check if user already has a position in this market.
    let existing_idx = user_account.find_position(market_index);
    if let Some(idx) = existing_idx {
        let pos = &user_account.positions[idx];
        // Don't allow opening in opposite direction.
        require!(
            pos.is_long == is_long,
            VoltPerpError::OppositeDirectionNotAllowed
        );
    }

    // Calculate leveraged notional value.
    let notional_value = (quote_amount as u128)
        .checked_mul(leverage as u128)
        .ok_or(VoltPerpError::MathOverflow)?;

    let notional_u64 =
        u64::try_from(notional_value).map_err(|_| error!(VoltPerpError::CastOverflow))?;

    // Calculate fee.
    let fee = calculate_fee(notional_u64, market.taker_fee_bps)?;

    // Check initial margin requirement.
    let has_margin = check_initial_margin(
        user_account.collateral,
        user_account.total_unrealized_pnl,
        notional_u64,
        market.initial_margin_ratio,
    )?;
    require!(has_margin, VoltPerpError::InitialMarginNotMet);

    // Execute the swap on the vAMM.
    let market = &mut ctx.accounts.market;
    let swap_result = if is_long {
        // Long: buy base by selling quote.
        swap_quote_for_base(market, notional_u64)?
    } else {
        // Short: sell base to get quote.
        // Calculate base amount from the notional at current mark price.
        let mark_price = market
            .quote_asset_reserve
            .checked_mul(market.peg_multiplier)
            .ok_or(VoltPerpError::MathOverflow)?
            .checked_div(market.base_asset_reserve)
            .ok_or(VoltPerpError::DivisionByZero)?;

        require!(mark_price > 0, VoltPerpError::InvalidPrice);

        let base_amount = (notional_u64 as u128)
            .checked_mul(PRICE_PRECISION)
            .ok_or(VoltPerpError::MathOverflow)?
            .checked_div(mark_price)
            .ok_or(VoltPerpError::DivisionByZero)?;

        let base_u64 =
            u64::try_from(base_amount).map_err(|_| error!(VoltPerpError::CastOverflow))?;
        swap_base_for_quote(market, base_u64)?
    };

    // Update vAMM reserves.
    market.base_asset_reserve = swap_result.new_base_reserve;
    market.quote_asset_reserve = swap_result.new_quote_reserve;

    // Update market open interest.
    if is_long {
        market.total_long_base = market
            .total_long_base
            .checked_add(swap_result.base_asset_amount as u128)
            .ok_or(VoltPerpError::MathOverflow)?;
    } else {
        market.total_short_base = market
            .total_short_base
            .checked_add(swap_result.base_asset_amount as u128)
            .ok_or(VoltPerpError::MathOverflow)?;
    }
    market.open_interest = market
        .open_interest
        .checked_add(notional_value)
        .ok_or(VoltPerpError::MathOverflow)?;

    // Calculate entry price.
    let entry_price =
        calculate_entry_price(swap_result.quote_asset_amount, swap_result.base_asset_amount)?;

    // Slippage protection: for longs, limit_price is max_entry_price;
    // for shorts, limit_price is min_entry_price.
    if is_long {
        require!(
            entry_price <= limit_price,
            VoltPerpError::SlippageExceeded
        );
    } else {
        require!(
            entry_price >= limit_price,
            VoltPerpError::SlippageExceeded
        );
    }

    // Get the cumulative funding rate for this position direction.
    let last_cumulative_funding = if is_long {
        market.cumulative_funding_rate_long
    } else {
        market.cumulative_funding_rate_short
    };

    // Update user account.
    let user_account = &mut ctx.accounts.user_account;

    // Find or create position slot.
    let slot_idx = if let Some(idx) = existing_idx {
        idx
    } else {
        user_account
            .find_empty_slot()
            .ok_or(VoltPerpError::NoAvailablePositionSlot)?
    };

    let pos = &mut user_account.positions[slot_idx];

    if existing_idx.is_some() {
        // Add to existing position (weighted average entry price).
        let old_notional = (pos.base_asset_amount as u128)
            .checked_mul(pos.entry_price as u128)
            .ok_or(VoltPerpError::MathOverflow)?;
        let new_notional = (swap_result.base_asset_amount as u128)
            .checked_mul(entry_price as u128)
            .ok_or(VoltPerpError::MathOverflow)?;

        let total_base = (pos.base_asset_amount as u128)
            .checked_add(swap_result.base_asset_amount as u128)
            .ok_or(VoltPerpError::MathOverflow)?;

        let avg_entry = old_notional
            .checked_add(new_notional)
            .ok_or(VoltPerpError::MathOverflow)?
            .checked_div(total_base)
            .ok_or(VoltPerpError::DivisionByZero)?;

        pos.base_asset_amount =
            u64::try_from(total_base).map_err(|_| error!(VoltPerpError::CastOverflow))?;
        pos.quote_asset_amount = pos
            .quote_asset_amount
            .checked_add(swap_result.quote_asset_amount)
            .ok_or(VoltPerpError::MathOverflow)?;
        pos.entry_price =
            u64::try_from(avg_entry).map_err(|_| error!(VoltPerpError::CastOverflow))?;
        pos.last_cumulative_funding = last_cumulative_funding;
    } else {
        // New position.
        pos.market_index = market_index;
        pos.is_long = is_long;
        pos.base_asset_amount = swap_result.base_asset_amount;
        pos.quote_asset_amount = swap_result.quote_asset_amount;
        pos.entry_price = entry_price;
        pos.last_cumulative_funding = last_cumulative_funding;
        pos.realized_pnl = 0;

        user_account.active_positions = user_account
            .active_positions
            .checked_add(1)
            .ok_or(VoltPerpError::MathOverflow)?;
    }

    // Deduct fee from collateral.
    user_account.collateral = user_account
        .collateral
        .checked_sub(fee)
        .ok_or(VoltPerpError::InsufficientCollateral)?;
    user_account.total_fees_paid = user_account
        .total_fees_paid
        .checked_add(fee)
        .ok_or(VoltPerpError::MathOverflow)?;

    let clock = Clock::get()?;
    user_account.last_active_slot = clock.slot;

    emit!(PositionOpened {
        user: ctx.accounts.user.key(),
        market_index,
        is_long,
        base_asset_amount: swap_result.base_asset_amount,
        quote_asset_amount: swap_result.quote_asset_amount,
        entry_price,
        leverage,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Opened {} position: {} base @ {} entry",
        if is_long { "long" } else { "short" },
        swap_result.base_asset_amount,
        entry_price,
    );
    Ok(())
}
