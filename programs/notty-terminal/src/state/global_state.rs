use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct GlobalState {
    pub admin: Pubkey,
    pub vault: Pubkey,
    pub vault_bump: u8,
    pub bump: u8,
    pub listing_fee_lamport: u64,   // 0.05 SOL token creation fee
    pub trading_fee_bps: u16,       // 150 = 1.5% (basis points)
    pub migration_fee_lamport: u64, // 0.15 SOL for Raydium migration             // For bonding curve (if still needed)
    pub total_tokens_created: u64,
    pub total_fees_collected: u64,
    pub total_trading_volume: u64, // Track platform volume
    pub total_migrations: u64,     // Track successful migrations
}
