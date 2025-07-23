use anchor_lang::prelude::*;

use crate::GlobalState;

#[derive(Accounts)]
pub struct InitializeGlobalState<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        init_if_needed,
        payer = admin,
        seeds = [b"global_state"],
        space = 8 + GlobalState::INIT_SPACE,
        bump 
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [b"vault"],
        bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
}


impl<'info> InitializeGlobalState<'info> {

    pub fn handle_initialize (&mut self, args: InitializeArgs, bumps: &InitializeGlobalStateBumps) -> Result<()> {

        self.global_state.set_inner(GlobalState {
             admin: self.admin.key(), vault: self.vault.key(), vault_bump: bumps.vault, bump: bumps.global_state, listing_fee_lamport: args.listing_fee_lamport, slope: args.slope, start_mcap: args.start_mcap, end_mcap: args.end_mcap, total_tokens_created: 0, total_fees_collected: 0, total_supply: args.total_supply
             });

        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, PartialEq)]
pub struct InitializeArgs {
    pub listing_fee_lamport: u64,
    pub slope: u64,
    pub start_mcap: u64,
    pub end_mcap: u64, 
    pub total_supply: u64,
}