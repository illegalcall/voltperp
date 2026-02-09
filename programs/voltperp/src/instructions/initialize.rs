use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

use crate::events::ExchangeInitialized;
use crate::state::ExchangeState;

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// The authority that will manage the exchange.
    #[account(mut)]
    pub authority: Signer<'info>,

    /// Exchange state PDA — created once.
    #[account(
        init,
        payer = authority,
        space = 8 + ExchangeState::INIT_SPACE,
        seeds = [ExchangeState::SEED.as_ref()],
        bump,
    )]
    pub exchange_state: Box<Account<'info, ExchangeState>>,

    /// Collateral token mint (e.g. USDC).
    pub collateral_mint: Box<Account<'info, Mint>>,

    /// Collateral vault — holds all user deposits.
    #[account(
        init,
        payer = authority,
        token::mint = collateral_mint,
        token::authority = exchange_state,
    )]
    pub collateral_vault: Box<Account<'info, TokenAccount>>,

    /// Insurance fund vault — receives liquidation fees.
    #[account(
        init,
        payer = authority,
        token::mint = collateral_mint,
        token::authority = exchange_state,
    )]
    pub insurance_fund_vault: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>,
}

pub fn handle_initialize(ctx: Context<Initialize>) -> Result<()> {
    let exchange = &mut ctx.accounts.exchange_state;

    exchange.authority = ctx.accounts.authority.key();
    exchange.insurance_fund_vault = ctx.accounts.insurance_fund_vault.key();
    exchange.collateral_vault = ctx.accounts.collateral_vault.key();
    exchange.collateral_mint = ctx.accounts.collateral_mint.key();
    exchange.num_markets = 0;
    exchange.total_collateral = 0;
    exchange.paused = false;
    exchange.bump = ctx.bumps.exchange_state;

    emit!(ExchangeInitialized {
        authority: exchange.authority,
        collateral_mint: exchange.collateral_mint,
        timestamp: Clock::get()?.unix_timestamp,
    });

    Ok(())
}
