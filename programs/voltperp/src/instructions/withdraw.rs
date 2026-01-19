use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::errors::VoltPerpError;
use crate::events::CollateralWithdrawn;
use crate::math::margin::MARGIN_PRECISION;
use crate::math::vamm;
use crate::state::{ExchangeState, Market, UserAccount, MAX_POSITIONS};

#[derive(Accounts)]
pub struct Withdraw<'info> {
    /// The user withdrawing collateral.
    #[account(mut)]
    pub user: Signer<'info>,

    /// Exchange state PDA.
    #[account(
        mut,
        seeds = [ExchangeState::SEED.as_ref()],
        bump = exchange_state.bump,
        constraint = !exchange_state.paused @ VoltPerpError::ExchangePaused,
    )]
    pub exchange_state: Account<'info, ExchangeState>,

    /// User account PDA.
    #[account(
        mut,
        seeds = [UserAccount::SEED.as_ref(), user.key().as_ref()],
        bump = user_account.bump,
        constraint = user_account.authority == user.key() @ VoltPerpError::UnauthorizedAuthority,
    )]
    pub user_account: Account<'info, UserAccount>,

    /// User's token account to receive collateral.
    #[account(
        mut,
        constraint = user_token_account.mint == exchange_state.collateral_mint @ VoltPerpError::InvalidCollateralMint,
        constraint = user_token_account.owner == user.key() @ VoltPerpError::InvalidTokenAccountOwner,
    )]
    pub user_token_account: Box<Account<'info, TokenAccount>>,

    /// Collateral vault owned by exchange PDA.
    #[account(
        mut,
        constraint = collateral_vault.mint == exchange_state.collateral_mint @ VoltPerpError::InvalidCollateralMint,
    )]
    pub collateral_vault: Box<Account<'info, TokenAccount>>,

    pub collateral_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
}

pub fn handle_withdraw(ctx: Context<Withdraw>, amount: u64) -> Result<()> {
    require!(amount > 0, VoltPerpError::ZeroWithdrawAmount);

    let user_account = &ctx.accounts.user_account;
    require!(
        user_account.collateral >= amount,
        VoltPerpError::InsufficientCollateral
    );

    // Check that withdrawal won't violate margin requirements for open positions.
    let remaining_collateral = user_account
        .collateral
        .checked_sub(amount)
        .ok_or(VoltPerpError::MathOverflow)?;

    // If user has any open positions, verify margin after withdrawal.
    if user_account.active_positions > 0 {
        let mut total_initial_margin_required: u128 = 0;

        let remaining_accounts = &ctx.remaining_accounts;
        for i in 0..MAX_POSITIONS {
            let pos = &user_account.positions[i];
            if pos.is_empty() {
                continue;
            }

            // Remaining accounts should contain Market accounts for each active position.
            let market_idx = pos.market_index as usize;
            require!(
                market_idx < remaining_accounts.len(),
                VoltPerpError::InvalidMarketIndex
            );

            let market_ai = &remaining_accounts[market_idx];
            let market_data = market_ai.try_borrow_data()?;
            let market =
                Market::try_deserialize(&mut &market_data[..]).map_err(|_| VoltPerpError::InvalidMarketIndex)?;

            let mark_price = vamm::get_mark_price(&market)?;

            // Notional = base * mark_price / PRICE_PRECISION
            let notional = (pos.base_asset_amount as u128)
                .checked_mul(mark_price as u128)
                .ok_or(VoltPerpError::MathOverflow)?
                .checked_div(vamm::PRICE_PRECISION)
                .ok_or(VoltPerpError::DivisionByZero)?;

            let margin_req = notional
                .checked_mul(market.initial_margin_ratio as u128)
                .ok_or(VoltPerpError::MathOverflow)?
                .checked_div(MARGIN_PRECISION as u128)
                .ok_or(VoltPerpError::DivisionByZero)?;

            total_initial_margin_required = total_initial_margin_required
                .checked_add(margin_req)
                .ok_or(VoltPerpError::MathOverflow)?;
        }

        let equity = (remaining_collateral as i128)
            .checked_add(user_account.total_unrealized_pnl as i128)
            .ok_or(VoltPerpError::MathOverflow)?;

        require!(
            equity >= 0 && (equity as u128) >= total_initial_margin_required,
            VoltPerpError::WithdrawalViolatesMargin
        );
    }

    // Transfer collateral from vault to user via PDA signer.
    let exchange_bump = ctx.accounts.exchange_state.bump;
    let signer_seeds: &[&[&[u8]]] = &[&[ExchangeState::SEED.as_ref(), &[exchange_bump]]];

    let cpi_accounts = Transfer {
        from: ctx.accounts.collateral_vault.to_account_info(),
        to: ctx.accounts.user_token_account.to_account_info(),
        authority: ctx.accounts.exchange_state.to_account_info(),
    };
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        cpi_accounts,
        signer_seeds,
    );
    token::transfer(cpi_ctx, amount)?;

    // Update balances.
    let user_account = &mut ctx.accounts.user_account;
    user_account.collateral = remaining_collateral;
    user_account.last_active_slot = Clock::get()?.slot;

    let exchange = &mut ctx.accounts.exchange_state;
    exchange.total_collateral = exchange
        .total_collateral
        .checked_sub(amount)
        .ok_or(VoltPerpError::MathOverflow)?;

    emit!(CollateralWithdrawn {
        user: ctx.accounts.user.key(),
        amount,
        remaining_collateral: user_account.collateral,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
