use anchor_lang::prelude::*;

pub use crate::{error::NottyTerminalError, GlobalState};

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

    pub fn handle_initialize(&mut self, args: InitializeArgs, bumps: &InitializeGlobalStateBumps) -> Result<()> {
        
        // Validate fee parameters
        require!(
            args.trading_fee_bps <= 1000,  // Max 10% trading fee
            NottyTerminalError::InvalidTradingFee
        );
        
        require!(
            args.migration_fee_lamport <= 1_000_000_000, // Max 1 SOL migration fee
            NottyTerminalError::InvalidMigrationFee
        );
        
        self.global_state.set_inner(GlobalState {
            admin: self.admin.key(),
            vault: self.vault.key(),
            vault_bump: bumps.vault,
            bump: bumps.global_state,
            listing_fee_lamport: args.listing_fee_lamport,
            trading_fee_bps: args.trading_fee_bps,
            migration_fee_lamport: args.migration_fee_lamport,
            total_tokens_created: 0,
            total_fees_collected: 0,
            total_trading_volume: 0,
            total_migrations: 0,
        });

        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]

pub struct InitializeArgs {
    pub listing_fee_lamport: u64,      // 50_000_000 (0.05 SOL)
    pub trading_fee_bps: u16,          // 150 (1.5%)
    pub migration_fee_lamport: u64,    // 150_000_000 (0.15 SOL)
            
}