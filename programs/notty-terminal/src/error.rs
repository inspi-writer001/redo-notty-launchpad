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
