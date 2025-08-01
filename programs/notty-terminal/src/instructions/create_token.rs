use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    metadata::{
        create_metadata_accounts_v3, mpl_token_metadata::types::DataV2, CreateMetadataAccountsV3,
        Metadata, MetadataAccount,
    },
    token_interface::{mint_to, Mint, MintTo, TokenAccount, TokenInterface},
};

use crate::{error::NottyTerminalError, GlobalState, TokenState};

#[derive(Accounts)]
#[instruction(args: CreateTokenArgs)]
pub struct CreateToken<'info> {
    #[account(mut)]
    pub creator: Signer<'info>,

    #[account(
        init,
        seeds = [
            b"token_state", creator_mint.key().as_ref()
        ],
        bump,
        space = 8 + TokenState::INIT_SPACE,
        payer = creator
    )]
    pub token_state: Account<'info, TokenState>,

    #[account(
        init,
        mint::authority = token_state,
        mint::decimals = 9,
        mint::token_program = token_program,
        mint::freeze_authority = token_state,
        payer = creator
    )]
    pub creator_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        associated_token::mint = creator_mint,
        associated_token::authority = token_state,
        payer = creator
    )]
    pub token_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"sol_vault", token_vault.key().as_ref()],
        bump,
    )]
    pub sol_vault: SystemAccount<'info>,

    #[account(
        init_if_needed,
        associated_token::authority = creator,
        associated_token::token_program = token_program,
        associated_token::mint = creator_mint,
        payer = creator
    )]
    pub creator_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"global_state"],
        bump = global_state.bump
    )]
    pub global_state: Account<'info, GlobalState>,

    #[account(
        mut,
        seeds = [
            b"metadata",
            token_metadata_program.key().as_ref(),
            creator_mint.key().as_ref(),
        ],
        bump,
        seeds::program = token_metadata_program.key()
    )]
    /// CHECK MEtaplex cre4ates this account
    pub metadata_account: UncheckedAccount<'info>,

    #[account(
        mut,
        constraint = vault.key() == global_state.vault.key() @NottyTerminalError::WrongVault,
        seeds = [b"vault"],
        bump = global_state.vault_bump
    )]
    pub vault: SystemAccount<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub token_metadata_program: Program<'info, Metadata>,
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> CreateToken<'info> {
    pub fn handle_create_token(
        &mut self,
        args: CreateTokenArgs,
        bumps: &CreateTokenBumps,
    ) -> Result<()> {
        // pay token creation fee
        let cpi_transfer_accounts = Transfer {
            from: self.creator.to_account_info(),
            to: self.vault.to_account_info(),
        };

        let amount_to_transfer = self
            .global_state
            .listing_fee_lamport
            .checked_div(2)
            .ok_or_else(|| error!(NottyTerminalError::NumericalOverflow))
            .unwrap();

        transfer(
            CpiContext::new(self.system_program.to_account_info(), cpi_transfer_accounts),
            amount_to_transfer as u64,
        )?;

        // create token

        let create_metadata_accounts = CreateMetadataAccountsV3 {
            metadata: self.metadata_account.to_account_info(),
            mint: self.creator_mint.to_account_info(),
            mint_authority: self.token_state.to_account_info(),
            payer: self.creator.to_account_info(),
            rent: self.rent.to_account_info(),
            system_program: self.system_program.to_account_info(),
            update_authority: self.token_state.to_account_info(),
        };

        let datav2 = DataV2 {
            name: args.name.clone(),
            symbol: args.token_symbol.clone(),
            uri: args.token_uri.clone(),
            seller_fee_basis_points: 0,
            creators: None,
            collection: None,
            uses: None,
        };

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"token_state",
            &self.creator_mint.key().to_bytes(),
            &[bumps.token_state],
        ]];

        create_metadata_accounts_v3(
            CpiContext::new_with_signer(
                self.token_metadata_program.to_account_info(),
                create_metadata_accounts,
                signer_seeds,
            ),
            datav2,
            false,
            false,
            None,
        )?;

        // transfer rent exempt amount
        let rent_exempt: u64 =
            Rent::get()?.minimum_balance(self.sol_vault.to_account_info().data_len());

        let cpi_rent_exempt_accounts = Transfer {
            from: self.creator.to_account_info(),
            to: self.sol_vault.to_account_info(),
        };

        transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                cpi_rent_exempt_accounts,
            ),
            rent_exempt,
        )?;

        // transfer other half of amount to sol vault
        let cpi_accounts_transfer_to_sol_vault = Transfer {
            from: self.creator.to_account_info(),
            to: self.sol_vault.to_account_info(),
        };

        transfer(
            CpiContext::new(
                self.system_program.to_account_info(),
                cpi_accounts_transfer_to_sol_vault,
            ),
            amount_to_transfer,
        )?;

        let mint_to_accounts = MintTo {
            authority: self.token_state.to_account_info(),
            mint: self.creator_mint.to_account_info(),
            to: self.token_vault.to_account_info(),
        };

        let cpi_context = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            mint_to_accounts,
            signer_seeds,
        );

        mint_to(
            cpi_context,
            args.total_supply
                .checked_mul(1_000_000_000)
                .ok_or(NottyTerminalError::NumericalOverflow)?, // using 9 decimals
        )?;

        // Calculate price per base unit directly
        let total_supply_base_units = args
            .total_supply
            .checked_mul(1_000_000_000)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        let initial_price_per_token = args
            .start_mcap
            .checked_div(total_supply_base_units)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        // set Token state
        self.token_state.set_inner(TokenState {
            bump: bumps.token_state,
            migrated: false,
            mint: self.creator_mint.key(),
            initial_price_per_token,
            sol_raised: amount_to_transfer,
            tokens_sold: 0,
            total_supply: args.total_supply,
            sol_vault_bump: bumps.sol_vault,
            start_mcap: args.start_mcap,
            end_mcap: args.end_mcap,
            raydium_pool: None,
            migration_timestamp: 0,
            creator: self.creator.key(),
        });

        emit!(TokenCreated {
            migrated: false,
            mint: self.creator_mint.key(),
            initial_price_per_token,
            sol_raised: amount_to_transfer,
            tokens_sold: 0,
            total_supply: args.total_supply,
            start_mcap: args.start_mcap,
            end_mcap: args.end_mcap,
            raydium_pool: None,
            migration_timestamp: 0,
            creator: self.creator.key(),
        });

        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct CreateTokenArgs {
    pub name: String,
    pub token_symbol: String,
    pub token_uri: String,
    pub total_supply: u64, // Token-specific total supply
    pub start_mcap: u64,   // Starting market cap in lamports
    pub end_mcap: u64,     // Ending market cap in lamports
}

#[event]
pub struct TokenCreated {
    pub mint: Pubkey,
    pub initial_price_per_token: u64,
    pub migrated: bool,
    pub total_supply: u64,
    pub tokens_sold: u64,
    pub sol_raised: u64,
    pub start_mcap: u64,
    pub end_mcap: u64,
    pub creator: Pubkey,
    pub raydium_pool: Option<Pubkey>,
    pub migration_timestamp: i64,
}
