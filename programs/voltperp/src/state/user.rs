use anchor_lang::prelude::*;

/// Represents a single perpetual futures position
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, Default, InitSpace)]
pub struct Position {
    /// Market index this position belongs to
    pub market_index: u8,
    /// True = long, False = short
    pub is_long: bool,
    /// Base asset amount (position size in base units, precision 1e9)
    pub base_asset_amount: u64,
    /// Quote asset amount (notional value at entry, precision 1e6)
    pub quote_asset_amount: u64,
    /// Entry price (precision 1e6)
    pub entry_price: u64,
    /// Last cumulative funding rate applied (precision 1e12)
    pub last_cumulative_funding: i128,
    /// Realized PnL accumulated (precision 1e6, signed)
    pub realized_pnl: i64,
}

impl Position {
    pub fn is_empty(&self) -> bool {
        self.base_asset_amount == 0
    }
}

/// Maximum number of concurrent positions per user
pub const MAX_POSITIONS: usize = 4;

#[account]
#[derive(InitSpace)]
pub struct UserAccount {
    /// User's wallet authority
    pub authority: Pubkey,
    /// Deposited collateral balance (precision 1e6)
    pub collateral: u64,
    /// Total unrealized PnL across all positions (precision 1e6, signed)
    pub total_unrealized_pnl: i64,
    /// Total fees paid lifetime (precision 1e6)
    pub total_fees_paid: u64,
    /// Array of positions (fixed size = MAX_POSITIONS)
    pub positions: [Position; MAX_POSITIONS],
    /// Number of active (non-empty) positions
    pub active_positions: u8,
    /// Last slot the user was active
    pub last_active_slot: u64,
    /// PDA bump seed
    pub bump: u8,
}

impl UserAccount {
    pub const SEED: &'static [u8] = b"user_account";

    /// Find the first empty position slot
    pub fn find_empty_slot(&self) -> Option<usize> {
        self.positions.iter().position(|p| p.is_empty())
    }

    /// Find position by market index
    pub fn find_position(&self, market_index: u8) -> Option<usize> {
        self.positions
            .iter()
            .position(|p| !p.is_empty() && p.market_index == market_index)
    }
}
