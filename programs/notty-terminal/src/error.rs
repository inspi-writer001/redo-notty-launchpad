use anchor_lang::prelude::*;

#[error_code]
pub enum NottyTerminalError {
    #[msg("Not enough SOL to buy tokens")]
    InsufficientFunds,
    #[msg("Vault doesn't have enough SOL to refund")]
    VaultInsufficientSol,
    #[msg("Numerical overflow occurred")]
    NumericalOverflow,
    #[msg("Liquidity has already been migrated")]
    AlreadyMigrated,
    #[msg("Vault hasn't reached the migration threshold")]
    TargetNotReached,
    #[msg("Token amount exceeds available supply")]
    ExceedsSupply,
    #[msg("Slippage tolerance exceeded")]
    SlippageExceeded,
    #[msg("Invalid amount specified")]
    InvalidAmount,
    #[msg("All tokens have been sold")]
    SoldOut,
    #[msg("Insufficient tokens sold to support this sale")]
    InsufficientTokensSold,
    #[msg("Bonding curve has already graduated")]
    AlreadyGraduated,
    #[msg("Bonding curve has not graduated yet")]
    NotGraduated,
    #[msg("Only admin can perform this action")]
    UnauthorizedAdmin,
    #[msg("Fee vault doesn't have enough balance")]
    InsufficientFeeVaultBalance,
    #[msg("User rovided wrong vault account")]
    WrongVault,
}

#[error_code]
pub enum PriceCalculationError {
    #[msg("Overflow in supply calculation: total_supply - tokens_sold")]
    SupplyOverflow,
    #[msg("Overflow in slope calculation: slope * current_supply")]
    SlopeSupplyOverflow,
    #[msg("Overflow in price_per_token calculation: base_price + (slope * current_supply)")]
    PricePerTokenOverflow,
    #[msg("Overflow in linear_cost calculation: n * price_per_token")]
    LinearCostOverflow,
    #[msg("Overflow in n_squared calculation: n * n")]
    NSquaredOverflow,
    #[msg("Overflow in quadratic calculation: slope * n²")]
    QuadraticSlopeOverflow,
    #[msg("Overflow in quadratic division: (slope * n²) / 2")]
    QuadraticDivisionOverflow,
    #[msg("Overflow in final sum: linear_cost + quadratic_adjustment")]
    FinalSumOverflow,
}
