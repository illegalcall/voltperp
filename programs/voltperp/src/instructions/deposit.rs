use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

use crate::errors::VoltPerpError;
use crate::events::CollateralDeposited;
use crate::state::{ExchangeState, UserAccount};

#[derive(Accounts)]
pub struct Deposit<'info> {
    /// The user depositing collateral.
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

    /// User account PDA — created on first deposit via `init_if_needed`.
    #[account(
        init_if_needed,
        payer = user,
        space = 8 + UserAccount::INIT_SPACE,
        seeds = [UserAccount::SEED.as_ref(), user.key().as_ref()],
        bump,
    )]
    pub user_account: Account<'info, UserAccount>,

    /// User's token account holding collateral.
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
        constraint = collateral_vault.key() == exchange_state.collateral_vault @ VoltPerpError::InvalidCollateralMint,
    )]
    pub collateral_vault: Box<Account<'info, TokenAccount>>,

    pub collateral_mint: Account<'info, Mint>,
    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
}

pub fn handle_deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    require!(amount > 0, VoltPerpError::ZeroDepositAmount);

    let user_account = &mut ctx.accounts.user_account;

    // Initialize authority on first deposit (when account is freshly created).
    if user_account.authority == Pubkey::default() {
        user_account.authority = ctx.accounts.user.key();
        user_account.bump = ctx.bumps.user_account;
    }

    // Transfer collateral from user to vault.
    let cpi_accounts = Transfer {
        from: ctx.accounts.user_token_account.to_account_info(),
        to: ctx.accounts.collateral_vault.to_account_info(),
        authority: ctx.accounts.user.to_account_info(),
    };
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), cpi_accounts);
    token::transfer(cpi_ctx, amount)?;

    // Update balances.
    user_account.collateral = user_account
        .collateral
        .checked_add(amount)
        .ok_or(VoltPerpError::MathOverflow)?;
    user_account.last_active_slot = Clock::get()?.slot;

    let exchange = &mut ctx.accounts.exchange_state;
    exchange.total_collateral = exchange
        .total_collateral
        .checked_add(amount)
        .ok_or(VoltPerpError::MathOverflow)?;

    emit!(CollateralDeposited {
        user: ctx.accounts.user.key(),
        amount,
        total_collateral: user_account.collateral,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
