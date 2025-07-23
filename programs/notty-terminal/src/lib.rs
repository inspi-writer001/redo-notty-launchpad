pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("HfQ4uSk1G3VuSNMCDWj2sJ7e4SVkdXbzuvGtfmzhmK2L");

#[program]
pub mod notty_terminal {
    use super::*;

    pub fn initialize(ctx: Context<InitializeGlobalState>, args: InitializeArgs) -> Result<()> {
        ctx.accounts.handle_initialize(args, &ctx.bumps)?;
        Ok(())
    }

    pub fn create_token(ctx: Context<CreateToken>, args: CreateTokenArgs) -> Result<()> {
        ctx.accounts.handle_create_token(args, &ctx.bumps)?;
        Ok(())
    }
}
