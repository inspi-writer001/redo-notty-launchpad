use anchor_lang::prelude::*;

#[derive(InitSpace)]
#[account]
pub struct GlobalState {
    pub admin: Pubkey,
    pub vault: Pubkey,
    pub vault_bump: u8,
    pub bump: u8,
    pub listing_fee_lamport: u64,
    pub slope: u64,
    pub start_mcap: u64,
    pub end_mcap: u64,
    pub total_tokens_created: u64,
    pub total_fees_collected: u64,
    pub total_supply: u64,
}
