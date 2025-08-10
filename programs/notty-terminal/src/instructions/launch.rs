use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Token},
    token_interface::{Mint, TokenAccount, TokenInterface},
};
use raydium_cpmm_cpi::{
    cpi,
    program::RaydiumCpmm,
    states::{AmmConfig, OBSERVATION_SEED, POOL_LP_MINT_SEED, POOL_SEED, POOL_VAULT_SEED},
};

use crate::{error::NottyTerminalError, TokenState};

#[derive(Accounts)]
#[instruction(param: LaunchParam)]
pub struct Launch<'info> {
    pub cp_swap_program: Program<'info, RaydiumCpmm>,
    /// Address paying to create the pool. Can be anyone

    #[account(
        constraint = token_state.creator.key() == creator.key() @NottyTerminalError::WrongCreator
   )]
    pub creator: SystemAccount<'info>,

    #[account(mut)]
    pub signer: Signer<'info>,

    /// Which config the pool belongs to.
    pub amm_config: Box<Account<'info, AmmConfig>>,

    /// CHECK: pool vault and lp mint authority
    #[account(
        seeds = [
            raydium_cpmm_cpi::AUTH_SEED.as_bytes(),
        ],
        seeds::program = cp_swap_program.key(),
        bump,
    )]
    pub authority: UncheckedAccount<'info>,

    /// CHECK: Initialize an account to store the pool state, init by cp-swap
    #[account(
        mut,
        seeds = [
            POOL_SEED.as_bytes(),
            amm_config.key().as_ref(),
            token_0_mint.key().as_ref(),
            token_1_mint.key().as_ref(),
        ],
        seeds::program = cp_swap_program.key(),
        bump,
    )]
    pub pool_state: UncheckedAccount<'info>,

    /// Token_0 mint, the key must smaller then token_1 mint.
    #[account(
        constraint = token_0_mint.key() < token_1_mint.key(),
        mint::token_program = token_0_program,
    )]
    pub token_0_mint: Box<InterfaceAccount<'info, Mint>>,

    /// Token_1 mint, the key must grater then token_0 mint.
    #[account(
        mint::token_program = token_1_program,
    )]
    pub token_1_mint: Box<InterfaceAccount<'info, Mint>>,

    /// CHECK: pool lp mint, init by cp-swap
    #[account(
        mut,
        seeds = [
            POOL_LP_MINT_SEED.as_bytes(),
            pool_state.key().as_ref(),
        ],
        seeds::program = cp_swap_program.key(),
        bump,
    )]
    pub lp_mint: UncheckedAccount<'info>,

    /// payer token0 account
    #[account(
        mut,
        token::mint = token_0_mint,
        token::authority = creator,
    )]
    pub creator_token_0: Box<InterfaceAccount<'info, TokenAccount>>,

    /// creator token1 account
    #[account(
        mut,
        token::mint = token_1_mint,
        token::authority = creator,
    )]
    pub creator_token_1: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: creator lp ATA token account, init by cp-swap
    #[account(mut)]
    pub creator_lp_token: UncheckedAccount<'info>,

    // using tokenMint because token order could be rotated from client
    #[account(
        mut,
        seeds = [
            b"token_state", param.token_mint.key().as_ref()
        ],
        bump = token_state.bump,
        constraint = param.token_mint.key() == token_state.mint.key() @NottyTerminalError::WrongMint
    )]
    pub token_state: Box<Account<'info, TokenState>>,

    /// CHECK: Token_0 vault for the pool, init by cp-swap
    #[account(
        mut,
        seeds = [
            POOL_VAULT_SEED.as_bytes(),
            pool_state.key().as_ref(),
            token_0_mint.key().as_ref()
        ],
        seeds::program = cp_swap_program.key(),
        bump,
    )]
    pub token_0_vault: UncheckedAccount<'info>,

    /// CHECK: Token_1 vault for the pool, init by cp-swap
    #[account(
        mut,
        seeds = [
            POOL_VAULT_SEED.as_bytes(),
            pool_state.key().as_ref(),
            token_1_mint.key().as_ref()
        ],
        seeds::program = cp_swap_program.key(),
        bump,
    )]
    pub token_1_vault: UncheckedAccount<'info>,

    /// create pool fee account
    #[account(
        mut,
        address= raydium_cpmm_cpi::create_pool_fee_reveiver::id(),
    )]
    pub create_pool_fee: Box<InterfaceAccount<'info, TokenAccount>>,

    /// CHECK: an account to store oracle observations, init by cp-swap
    #[account(
        mut,
        seeds = [
            OBSERVATION_SEED.as_bytes(),
            pool_state.key().as_ref(),
        ],
        seeds::program = cp_swap_program.key(),
        bump,
    )]
    pub observation_state: UncheckedAccount<'info>,

    #[account(
        init_if_needed,
        payer = signer,
        associated_token::mint = wsol_mint,
        associated_token::authority = token_state)]
    pub vault_wsol_account: Box<InterfaceAccount<'info, TokenAccount>>,

    pub wsol_mint: Box<InterfaceAccount<'info, Mint>>,

    #[account(
        mut,
        constraint = token_vault.owner == token_state.key(),
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [b"sol_vault", token_vault.key().as_ref()],
        bump = token_state.sol_vault_bump,
    )]
    pub sol_vault: SystemAccount<'info>,

    /// Program to create mint account and mint tokens
    pub token_program: Program<'info, Token>,
    /// Spl token program or token program 2022
    pub token_0_program: Interface<'info, TokenInterface>,
    /// Spl token program or token program 2022
    pub token_1_program: Interface<'info, TokenInterface>,
    /// Program to create an ATA for receiving position NFT
    pub associated_token_program: Program<'info, AssociatedToken>,
    /// To create a new program account
    pub system_program: Program<'info, System>,
    /// Sysvar for program account
    pub rent: Sysvar<'info, Rent>,
}

impl<'info> Launch<'info> {
    pub fn handle_launch(
        &mut self,
        init_amount_0: u64,
        init_amount_1: u64,
        open_time: u64,
    ) -> Result<()> {
        require!(
            !self.token_state.migrated,
            NottyTerminalError::AlreadyMigrated
        );

        // require!(
        //     self.token_state.sol_raised == self.token_state.target_sol,
        //     NottyTerminalError::TargetNotReached
        // );

        // Step 1: Wrap SOL
        self.wrap_sol()?;
        // Step 2: Transfer tokens to creator accounts for Raydium
        self.prepare_liquidity(init_amount_0, init_amount_1)?;

        let cpi_accounts = cpi::accounts::Initialize {
            creator: self.creator.to_account_info(),
            amm_config: self.amm_config.to_account_info(),
            authority: self.authority.to_account_info(),
            pool_state: self.pool_state.to_account_info(),
            token_0_mint: self.token_0_mint.to_account_info(),
            token_1_mint: self.token_1_mint.to_account_info(),
            lp_mint: self.lp_mint.to_account_info(),
            creator_token_0: self.creator_token_0.to_account_info(),
            creator_token_1: self.creator_token_1.to_account_info(),
            creator_lp_token: self.creator_lp_token.to_account_info(),
            token_0_vault: self.token_0_vault.to_account_info(),
            token_1_vault: self.token_1_vault.to_account_info(),
            create_pool_fee: self.create_pool_fee.to_account_info(),
            observation_state: self.observation_state.to_account_info(),
            token_program: self.token_program.to_account_info(),
            token_0_program: self.token_0_program.to_account_info(),
            token_1_program: self.token_1_program.to_account_info(),
            associated_token_program: self.associated_token_program.to_account_info(),
            system_program: self.system_program.to_account_info(),
            rent: self.rent.to_account_info(),
        };
        let cpi_context = CpiContext::new(self.cp_swap_program.to_account_info(), cpi_accounts);
        cpi::initialize(cpi_context, init_amount_0, init_amount_1, open_time)?;
        self.token_state.migrated = true;
        self.token_state.migration_timestamp = Clock::get()?.unix_timestamp;

        Ok(())
    }

    pub fn prepare_liquidity(&mut self, init_amount_0: u64, init_amount_1: u64) -> Result<()> {
        let token_state_seeds: &[&[&[u8]]] = &[&[
            b"token_state",
            self.token_state.mint.as_ref(),
            &[self.token_state.bump],
        ]];

        // Determine which token is our custom token
        let is_custom_token_first = self.token_0_mint.key() == self.token_state.mint;

        if is_custom_token_first {
            // Token 0 is custom token, Token 1 is WSOL

            // Transfer custom tokens from token_vault to creator_token_0
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    token::Transfer {
                        from: self.token_vault.to_account_info(),
                        to: self.creator_token_0.to_account_info(),
                        authority: self.token_state.to_account_info(),
                    },
                    token_state_seeds,
                ),
                init_amount_0,
            )?;

            // Transfer WSOL from vault_wsol_account to creator_token_1
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    token::Transfer {
                        from: self.vault_wsol_account.to_account_info(),
                        to: self.creator_token_1.to_account_info(),
                        authority: self.token_state.to_account_info(),
                    },
                    token_state_seeds,
                ),
                init_amount_1,
            )?;
        } else {
            // Token 0 is WSOL, Token 1 is custom token

            // Transfer WSOL from vault_wsol_account to creator_token_0
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    token::Transfer {
                        from: self.vault_wsol_account.to_account_info(),
                        to: self.creator_token_0.to_account_info(),
                        authority: self.token_state.to_account_info(),
                    },
                    token_state_seeds,
                ),
                init_amount_0,
            )?;

            // Transfer custom tokens from token_vault to creator_token_1
            token::transfer(
                CpiContext::new_with_signer(
                    self.token_program.to_account_info(),
                    token::Transfer {
                        from: self.token_vault.to_account_info(),
                        to: self.creator_token_1.to_account_info(),
                        authority: self.token_state.to_account_info(),
                    },
                    token_state_seeds,
                ),
                init_amount_1,
            )?;
        }

        msg!(
            "Liquidity prepared: {} token0, {} token1",
            init_amount_0,
            init_amount_1
        );
        Ok(())
    }

    pub fn wrap_sol(&mut self) -> Result<()> {
        // transfer sol to token account

        // let creator_mint = &self.token_state.mint.key();
        // let signer_seeds: &[&[&[u8]]] = &[&[
        //     b"token_state",
        //     creator_mint.as_ref(),
        //     &[self.token_state.bump],
        // ]];

        let token_vault = self.token_vault.key();

        let sol_vault_seeds: &[&[&[u8]]] = &[&[
            b"sol_vault",
            token_vault.as_ref(), // Use token_vault.key() to match CreateToken!
            &[self.token_state.sol_vault_bump],
        ]];

        let cpi_context = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            Transfer {
                from: self.sol_vault.to_account_info(),
                to: self.vault_wsol_account.to_account_info(),
            },
            sol_vault_seeds,
        );

        let amount = self.sol_vault.to_account_info().lamports();
        transfer(cpi_context, amount)?;

        // Sync the native token to reflect the new SOL balance as wSOL
        let cpi_accounts = token::SyncNative {
            account: self.vault_wsol_account.to_account_info(),
        };
        let cpi_program = self.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::sync_native(cpi_ctx)?;
        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct LaunchParam {
    pub token_mint: Pubkey,
    pub time: Option<i64>,
}
