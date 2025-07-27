pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("Brqm9dR2GiZkuy9FDo2ToHwQoAykgng8LhsZ2jwpYFoE");

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

    pub fn purchase_token(ctx: Context<TokenInteraction>, args: PurchaseTokenArgs) -> Result<()> {
        ctx.accounts.handle_purchase(args)?;
        Ok(())
    }

    pub fn sell_token(ctx: Context<TokenInteraction>, args: SellTokenArgs) -> Result<()> {
        ctx.accounts.handle_sell(args)?;
        Ok(())
    }
}
