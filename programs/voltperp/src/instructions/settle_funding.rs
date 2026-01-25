use anchor_lang::prelude::*;

use crate::errors::VoltPerpError;
use crate::events::{FundingAppliedToUser, FundingSettled};
use crate::math::funding::{
    calculate_funding_payment, calculate_funding_rate, is_funding_period_elapsed,
};
use crate::math::vamm::get_mark_price;
use crate::state::{ExchangeState, Market, UserAccount};

#[derive(Accounts)]
#[instruction(market_index: u8)]
pub struct SettleFunding<'info> {
    /// The keeper / cranker who triggers funding settlement.
    #[account(mut)]
    pub keeper: Signer<'info>,

    /// Exchange state PDA.
    #[account(
        seeds = [ExchangeState::SEED.as_ref()],
        bump = exchange_state.bump,
        constraint = !exchange_state.paused @ VoltPerpError::ExchangePaused,
    )]
    pub exchange_state: Account<'info, ExchangeState>,

    /// Market PDA to settle funding on.
    #[account(
        mut,
        seeds = [Market::SEED.as_ref(), &[market_index]],
        bump = market.bump,
    )]
    pub market: Account<'info, Market>,

    /// Optional: user account to apply funding to.
    /// If provided with a valid position, funding is also applied to this user.
    #[account(
        mut,
        seeds = [UserAccount::SEED.as_ref(), user_authority.key().as_ref()],
        bump = user_account.bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    /// CHECK: Authority of the user account, used only for PDA derivation.
    pub user_authority: UncheckedAccount<'info>,
}

pub fn handle_settle_funding(ctx: Context<SettleFunding>, market_index: u8) -> Result<()> {
    let market = &ctx.accounts.market;
    let clock = Clock::get()?;

    // Check if funding period has elapsed.
    require!(
        is_funding_period_elapsed(
            market.last_funding_timestamp,
            market.funding_period,
            clock.unix_timestamp,
        ),
        VoltPerpError::FundingPeriodNotElapsed
    );

    // Calculate mark price from vAMM.
    let mark_price = get_mark_price(market)?;

    // Use last oracle price as index price. If not set, use mark price.
    let effective_index = if market.last_oracle_price == 0 {
        mark_price
    } else {
        market.last_oracle_price
    };

    // Calculate funding rate.
    let funding_rate = calculate_funding_rate(mark_price, effective_index)?;

    // Update market cumulative funding rates.
    let market = &mut ctx.accounts.market;

    market.cumulative_funding_rate_long = market
        .cumulative_funding_rate_long
        .checked_add(funding_rate)
        .ok_or(VoltPerpError::MathOverflow)?;
    market.cumulative_funding_rate_short = market
        .cumulative_funding_rate_short
        .checked_add(funding_rate)
        .ok_or(VoltPerpError::MathOverflow)?;

    market.last_funding_timestamp = clock.unix_timestamp;
    market.last_oracle_price = effective_index;

    emit!(FundingSettled {
        market_index,
        funding_rate_long: market.cumulative_funding_rate_long,
        funding_rate_short: market.cumulative_funding_rate_short,
        mark_price,
        oracle_price: effective_index,
        timestamp: clock.unix_timestamp,
    });

    // Apply funding to the provided user account if they have a position in this market.
    let user_account = &mut ctx.accounts.user_account;
    if let Some(pos_idx) = user_account.find_position(market_index) {
        let pos = &user_account.positions[pos_idx];
        if !pos.is_empty() {
            let cumulative_rate = if pos.is_long {
                market.cumulative_funding_rate_long
            } else {
                market.cumulative_funding_rate_short
            };

            let funding_payment = calculate_funding_payment(
                pos.base_asset_amount,
                pos.is_long,
                cumulative_rate,
                pos.last_cumulative_funding,
            )?;

            // Apply funding payment: positive = user pays, negative = user receives.
            if funding_payment > 0 {
                user_account.collateral = user_account
                    .collateral
                    .saturating_sub(funding_payment as u64);
            } else {
                user_account.collateral = user_account
                    .collateral
                    .checked_add(funding_payment.unsigned_abs())
                    .ok_or(VoltPerpError::MathOverflow)?;
            }

            // Update the position's last cumulative funding.
            user_account.positions[pos_idx].last_cumulative_funding = cumulative_rate;

            emit!(FundingAppliedToUser {
                user: user_account.authority,
                market_index,
                funding_payment,
                timestamp: clock.unix_timestamp,
            });

            msg!("Funding applied to user: payment={}", funding_payment,);
        }
    }

    msg!(
        "Funding settled for market {}: rate={}",
        market_index,
        funding_rate,
    );
    Ok(())
}
