use anchor_lang::prelude::*;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{Mint, TokenAccount, TokenInterface},
};

use crate::{GlobalState, TokenState};

#[derive(Accounts)]
pub struct PurchaseToken<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        mut,
        constraint = creator_mint.mint_authority == Some(token_state.key()).into(),
        constraint = creator_mint.freeze_authority == Some(token_state.key()).into(),
        constraint = creator_mint.key() == token_state.mint.key()
    )]
    pub creator_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = creator_mint,
        associated_token::authority = buyer
    )]
    pub buyer_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint =  token_vault.mint == creator_mint.key(),
        constraint = token_vault.owner == token_state.key(),
    )]
    pub token_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        seeds = [
            b"token_state", creator_mint.key().as_ref()
        ],
        bump = token_state.bump
    )]
    pub token_state: Account<'info, TokenState>,

    #[account(
        mut,
        seeds = [b"sol_vault", token_vault.key().as_ref()],
        bump,
    )]
    pub sol_vault: SystemAccount<'info>,

    #[account(
        mut,
        seeds = [b"global_state"],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,

    pub token_program: Interface<'info, TokenInterface>,
    pub system_program: Program<'info, System>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

impl<'info> PurchaseToken<'info> {
    pub fn handle_purchase(
        &mut self,
        args: PurchaseTokenArgs,
        bumps: PurchaseTokenBumps,
    ) -> Result<()> {
        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, PartialEq)]
pub struct PurchaseTokenArgs {
    pub amount: u64,
    pub min_amount_out: u64,
}
