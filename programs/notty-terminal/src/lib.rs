pub mod constants;
pub mod error;
pub mod instructions;
pub mod state;

use anchor_lang::prelude::*;

pub use constants::*;
pub use instructions::*;
pub use state::*;

declare_id!("3Jy5qUaaAQMKVUehh4cLncAAYVgf1XELnt1RhNJGe8ZD");

#[program]
pub mod notty_terminal {
    use crate::error::NottyTerminalError;

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

    pub fn migrate_to_raydium(ctx: Context<Launch>, params: LaunchParam) -> Result<()> {
        let init_amount_0 = ctx
            .accounts
            .token_state
            .total_supply
            .checked_sub(ctx.accounts.token_state.tokens_sold)
            .ok_or(NottyTerminalError::InsufficientVaultBalance)?;

        let init_amount_1 = ctx.accounts.token_state.sol_raised;
        let open_time = match params.time {
            Some(value) => value as u64,
            None => Clock::get()?.unix_timestamp as u64,
        };

        ctx.accounts
            .handle_launch(init_amount_0, init_amount_1, open_time)?;
        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct LaunchParam {
    pub time: Option<i64>,
}
