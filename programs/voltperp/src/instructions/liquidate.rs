use anchor_lang::prelude::*;

use crate::errors::VoltPerpError;
use crate::events::PositionLiquidated;
use crate::math::margin::{
    calculate_fee, calculate_unrealized_pnl, is_liquidatable, is_partial_liquidation,
    HEALTH_PRECISION, MARGIN_PRECISION,
};
use crate::math::vamm::{
    get_mark_price, swap_base_for_quote, swap_quote_for_base, PRICE_PRECISION,
};
use crate::state::{ExchangeState, Market, UserAccount, MAX_POSITIONS};

#[derive(Accounts)]
#[instruction(market_index: u8)]
pub struct Liquidate<'info> {
    /// The liquidator / keeper.
    #[account(mut)]
    pub liquidator: Signer<'info>,

    /// The user being liquidated.
    #[account(
        mut,
        seeds = [UserAccount::SEED.as_ref(), user_authority.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    /// CHECK: The authority of the user account. Only used for PDA derivation.
    pub user_authority: UncheckedAccount<'info>,

    /// Exchange state PDA.
    #[account(
        mut,
        seeds = [ExchangeState::SEED.as_ref()],
        bump = exchange_state.bump,
        constraint = !exchange_state.paused @ VoltPerpError::ExchangePaused,
    )]
    pub exchange_state: Account<'info, ExchangeState>,

    /// Market PDA for the position being liquidated.
    #[account(
        mut,
        seeds = [Market::SEED.as_ref(), &[market_index]],
        bump = market.bump,
    )]
    pub market: Account<'info, Market>,
}

pub fn handle_liquidate(ctx: Context<Liquidate>, market_index: u8) -> Result<()> {
    let user_account = &ctx.accounts.user_account;
    let market = &ctx.accounts.market;

    // Prevent self-liquidation.
    require!(
        ctx.accounts.liquidator.key() != user_account.authority,
        VoltPerpError::SelfLiquidation
    );

    // Find the position to liquidate.
    let pos_idx = user_account
        .find_position(market_index)
        .ok_or(VoltPerpError::PositionNotFound)?;

    let position = user_account.positions[pos_idx];
    require!(!position.is_empty(), VoltPerpError::EmptyPosition);

    // Calculate current mark price for the target position's market.
    let mark_price = get_mark_price(market)?;

    // Calculate holistic health factor across ALL user positions.
    let mut total_unrealized_pnl: i128 = 0;
    let mut total_margin_required: u128 = 0;

    for i in 0..MAX_POSITIONS {
        let pos = &user_account.positions[i];
        if pos.is_empty() {
            continue;
        }

        // For the target market, use the already-loaded market account.
        // For other markets, we use remaining_accounts.
        let (pos_mark_price, maintenance_ratio) = if pos.market_index == market_index {
            (mark_price, market.maintenance_margin_ratio)
        } else {
            // Find the market in remaining_accounts.
            let mut found = false;
            let mut p_mark_price = 0u64;
            let mut p_maintenance = 0u32;
            for acct in ctx.remaining_accounts.iter() {
                if acct.owner != &crate::ID {
                    continue;
                }
                let data = acct.try_borrow_data()?;
                if let Ok(m) = Market::try_deserialize(&mut &data[..]) {
                    if m.market_index == pos.market_index {
                        p_mark_price = get_mark_price(&m)?;
                        p_maintenance = m.maintenance_margin_ratio;
                        found = true;
                        break;
                    }
                }
            }
            if !found {
                // If market not provided in remaining_accounts, skip this position.
                // This is conservative: we undercount margin required.
                continue;
            }
            (p_mark_price, p_maintenance)
        };

        let pnl = calculate_unrealized_pnl(pos, pos_mark_price)?;
        total_unrealized_pnl = total_unrealized_pnl
            .checked_add(pnl as i128)
            .ok_or(VoltPerpError::MathOverflow)?;

        let notional = (pos.base_asset_amount as u128)
            .checked_mul(pos_mark_price as u128)
            .ok_or(VoltPerpError::MathOverflow)?
            .checked_div(PRICE_PRECISION)
            .ok_or(VoltPerpError::DivisionByZero)?;

        let margin = notional
            .checked_mul(maintenance_ratio as u128)
            .ok_or(VoltPerpError::MathOverflow)?
            .checked_div(MARGIN_PRECISION as u128)
            .ok_or(VoltPerpError::DivisionByZero)?;

        total_margin_required = total_margin_required
            .checked_add(margin)
            .ok_or(VoltPerpError::MathOverflow)?;
    }

    // Calculate holistic health factor.
    let equity = (user_account.collateral as i128)
        .checked_add(total_unrealized_pnl)
        .ok_or(VoltPerpError::MathOverflow)?;

    let health_factor = if total_margin_required == 0 || equity <= 0 {
        0u64
    } else {
        let h = (equity as u128)
            .checked_mul(HEALTH_PRECISION as u128)
            .ok_or(VoltPerpError::MathOverflow)?
            .checked_div(total_margin_required)
            .ok_or(VoltPerpError::DivisionByZero)?;
        u64::try_from(h).unwrap_or(u64::MAX)
    };

    // Must be liquidatable.
    require!(
        is_liquidatable(health_factor),
        VoltPerpError::NotLiquidatable
    );

    // Determine partial vs full liquidation.
    let is_partial = is_partial_liquidation(health_factor);

    // Calculate the amount to liquidate.
    let liquidation_base = if is_partial {
        // Partial: liquidate 50% of the position.
        position.base_asset_amount / 2
    } else {
        // Full: liquidate entire position.
        position.base_asset_amount
    };

    // Execute the liquidation swap on the vAMM.
    let market = &mut ctx.accounts.market;

    let swap_result = if position.is_long {
        swap_base_for_quote(market, liquidation_base)?
    } else {
        swap_quote_for_base(market, {
            // Scale quote proportionally.
            let proportion = (liquidation_base as u128)
                .checked_mul(PRICE_PRECISION)
                .ok_or(VoltPerpError::MathOverflow)?
                .checked_div(position.base_asset_amount as u128)
                .ok_or(VoltPerpError::DivisionByZero)?;
            let scaled_quote = (position.quote_asset_amount as u128)
                .checked_mul(proportion)
                .ok_or(VoltPerpError::MathOverflow)?
                .checked_div(PRICE_PRECISION)
                .ok_or(VoltPerpError::DivisionByZero)?;
            u64::try_from(scaled_quote).map_err(|_| error!(VoltPerpError::CastOverflow))?
        })?
    };

    // Update vAMM reserves.
    market.base_asset_reserve = swap_result.new_base_reserve;
    market.quote_asset_reserve = swap_result.new_quote_reserve;

    // Calculate liquidation fees.
    let liquidation_notional = (liquidation_base as u128)
        .checked_mul(mark_price as u128)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(PRICE_PRECISION)
        .ok_or(VoltPerpError::DivisionByZero)?;

    let liq_notional_u64 =
        u64::try_from(liquidation_notional).map_err(|_| error!(VoltPerpError::CastOverflow))?;

    let liquidator_reward = calculate_fee(liq_notional_u64, market.liquidation_fee_bps)?;
    let insurance_fee = calculate_fee(liq_notional_u64, market.insurance_fee_bps)?;

    // Update market tracking.
    if position.is_long {
        market.total_long_base = market
            .total_long_base
            .saturating_sub(liquidation_base as u128);
    } else {
        market.total_short_base = market
            .total_short_base
            .saturating_sub(liquidation_base as u128);
    }

    let liq_quote = (position.quote_asset_amount as u128)
        .checked_mul(liquidation_base as u128)
        .ok_or(VoltPerpError::MathOverflow)?
        .checked_div(position.base_asset_amount as u128)
        .ok_or(VoltPerpError::DivisionByZero)?;
    market.open_interest = market.open_interest.saturating_sub(liq_quote);

    // Update user account.
    let user_account = &mut ctx.accounts.user_account;

    // Deduct fees from user collateral.
    let total_fee = liquidator_reward
        .checked_add(insurance_fee)
        .ok_or(VoltPerpError::MathOverflow)?;

    user_account.collateral = user_account.collateral.saturating_sub(total_fee);

    // Apply realized PnL from liquidation.
    let liq_pnl = if position.is_long {
        (swap_result.quote_asset_amount as i64)
            .checked_sub(
                u64::try_from(liq_quote).map_err(|_| error!(VoltPerpError::CastOverflow))? as i64,
            )
            .ok_or(VoltPerpError::MathOverflow)?
    } else {
        (u64::try_from(liq_quote).map_err(|_| error!(VoltPerpError::CastOverflow))? as i64)
            .checked_sub(swap_result.quote_asset_amount as i64)
            .ok_or(VoltPerpError::MathOverflow)?
    };

    if liq_pnl >= 0 {
        user_account.collateral = user_account
            .collateral
            .checked_add(liq_pnl as u64)
            .ok_or(VoltPerpError::MathOverflow)?;
    } else {
        user_account.collateral = user_account
            .collateral
            .saturating_sub(liq_pnl.unsigned_abs());
    }

    // Update or clear position.
    if is_partial {
        let pos = &mut user_account.positions[pos_idx];
        pos.base_asset_amount = pos
            .base_asset_amount
            .checked_sub(liquidation_base)
            .ok_or(VoltPerpError::MathOverflow)?;
        pos.quote_asset_amount = pos
            .quote_asset_amount
            .checked_sub(
                u64::try_from(liq_quote).map_err(|_| error!(VoltPerpError::CastOverflow))?,
            )
            .ok_or(VoltPerpError::MathOverflow)?;
    } else {
        user_account.positions[pos_idx] = Default::default();
        user_account.active_positions = user_account.active_positions.saturating_sub(1);
    }

    let clock = Clock::get()?;
    user_account.last_active_slot = clock.slot;

    emit!(PositionLiquidated {
        user: user_account.authority,
        liquidator: ctx.accounts.liquidator.key(),
        market_index,
        base_asset_amount: liquidation_base,
        liquidation_price: mark_price,
        is_partial,
        liquidator_reward,
        insurance_fee,
        timestamp: clock.unix_timestamp,
    });

    msg!(
        "Liquidated {} of position: market={}, base={}, reward={}",
        if is_partial { "50%" } else { "100%" },
        market_index,
        liquidation_base,
        liquidator_reward,
    );
    Ok(())
}
