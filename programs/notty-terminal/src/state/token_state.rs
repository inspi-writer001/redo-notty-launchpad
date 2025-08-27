use anchor_lang::prelude::*;

#[account]
#[derive(InitSpace)]
pub struct TokenState {
    pub bump: u8,
    pub migrated: bool,
    pub mint: Pubkey,
    pub initial_price_per_token: u64, // Will be 50 lamports
    pub sol_raised: u64,
    pub tokens_sold: u64,
    pub total_supply: u64,
    pub sol_vault_bump: u8,
    pub start_mcap: u64, // 50 SOL in lamports
    pub target_sol: u64, // 450 SOL in lamports (migration trigger)
    pub raydium_pool: Option<Pubkey>,
    pub migration_timestamp: i64,
    pub creator: Pubkey,
}

impl TokenState {
    pub fn check_migration_ready(&self) -> bool {
        self.sol_raised >= self.target_sol
    }

    pub fn get_progress_percentage(&self) -> u8 {
        let pct = (self.tokens_sold * 100) / self.total_supply;
        std::cmp::min(pct as u8, 100)
    }
}
