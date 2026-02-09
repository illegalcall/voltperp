use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct ExchangeState {
    /// The authority that can manage the exchange
    pub authority: Pubkey,
    /// Insurance fund vault token account
    pub insurance_fund_vault: Pubkey,
    /// Collateral vault token account
    pub collateral_vault: Pubkey,
    /// Collateral token mint (e.g. USDC)
    pub collateral_mint: Pubkey,
    /// Number of active markets
    pub num_markets: u8,
    /// Total collateral deposited across all users
    pub total_collateral: u64,
    /// Whether the exchange is paused
    pub paused: bool,
    /// PDA bump seed
    pub bump: u8,
}

impl ExchangeState {
    pub const SEED: &'static [u8] = b"exchange_state";
}
