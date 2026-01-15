use anchor_lang::prelude::*;

#[error_code]
pub enum VoltPerpError {
    // Authority / Permission errors (6000-6009)
    #[msg("Unauthorized: signer is not the exchange authority")]
    UnauthorizedAuthority,
    #[msg("Exchange is currently paused")]
    ExchangePaused,
    #[msg("Invalid program authority")]
    InvalidAuthority,

    // Market errors (6010-6019)
    #[msg("Maximum number of markets reached")]
    MaxMarketsReached,
    #[msg("Market index out of bounds")]
    InvalidMarketIndex,
    #[msg("Market is not active")]
    MarketNotActive,
    #[msg("Invalid oracle feed address")]
    InvalidOracleFeed,
    #[msg("Oracle price is stale")]
    StaleOraclePrice,
    #[msg("Oracle price is non-positive")]
    InvalidOraclePrice,
    #[msg("Invalid vAMM reserve parameters")]
    InvalidVammParams,
    #[msg("Market symbol cannot be empty")]
    EmptyMarketSymbol,

    // Position errors (6020-6029)
    #[msg("No available position slot")]
    NoAvailablePositionSlot,
    #[msg("Position not found for this market")]
    PositionNotFound,
    #[msg("Position is already empty")]
    EmptyPosition,
    #[msg("Position size is zero")]
    ZeroPositionSize,
    #[msg("Maximum leverage exceeded")]
    MaxLeverageExceeded,
    #[msg("Position exceeds open interest limits")]
    OpenInterestLimitExceeded,
    #[msg("Cannot open position in opposite direction; close first")]
    OppositeDirectionNotAllowed,

    // Margin / Collateral errors (6030-6039)
    #[msg("Insufficient collateral for this operation")]
    InsufficientCollateral,
    #[msg("Deposit amount must be greater than zero")]
    ZeroDepositAmount,
    #[msg("Withdrawal amount must be greater than zero")]
    ZeroWithdrawAmount,
    #[msg("Withdrawal would violate margin requirements")]
    WithdrawalViolatesMargin,
    #[msg("Initial margin requirement not met")]
    InitialMarginNotMet,
    #[msg("Below maintenance margin")]
    BelowMaintenanceMargin,

    // Liquidation errors (6040-6049)
    #[msg("Position is not liquidatable")]
    NotLiquidatable,
    #[msg("Liquidator cannot liquidate own position")]
    SelfLiquidation,
    #[msg("Partial liquidation amount too large")]
    PartialLiquidationTooLarge,
    #[msg("Insurance fund insufficient for socialized loss")]
    InsufficientInsuranceFund,

    // Funding errors (6050-6059)
    #[msg("Funding period has not elapsed")]
    FundingPeriodNotElapsed,
    #[msg("Invalid funding period")]
    InvalidFundingPeriod,
    #[msg("Funding rate exceeds maximum cap")]
    FundingRateExceedsCap,

    // Math / Overflow errors (6060-6069)
    #[msg("Math overflow in calculation")]
    MathOverflow,
    #[msg("Division by zero")]
    DivisionByZero,
    #[msg("Cast overflow: value does not fit target type")]
    CastOverflow,
    #[msg("Invalid price: must be greater than zero")]
    InvalidPrice,
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,

    // Token / SPL errors (6070-6079)
    #[msg("Invalid collateral mint")]
    InvalidCollateralMint,
    #[msg("Token transfer failed")]
    TokenTransferFailed,
    #[msg("Invalid token account owner")]
    InvalidTokenAccountOwner,
}
