use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct TokenState {
    pub bump: u8,
    pub sol_vault_bump: u8,
    pub mint: Pubkey,
    pub initial_price_per_token: u64,
    pub migrated: bool,
    pub total_supply: u64,
    pub tokens_sold: u64,
    pub sol_raised: u64,
    pub start_mcap: u64,
    pub end_mcap: u64,

    pub raydium_pool: Option<Pubkey>,
    pub migration_timestamp: i64,
}
