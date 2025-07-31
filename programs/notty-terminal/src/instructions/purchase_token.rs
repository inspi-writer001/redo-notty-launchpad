use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, Mint, TokenAccount, TokenInterface},
};
use raydium_cpmm_cpi::{
    cpi::{accounts::Initialize, initialize},
    program::RaydiumCpmm,
    states::{AmmConfig, ObservationState, PoolState},
};

use crate::{
    error::{NottyTerminalError, PriceCalculationError},
    GlobalState, TokenState,
};

pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");

// Raydium addresses
// #[cfg(feature = "devnet")]
// pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey = pubkey!("CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW");
// #[cfg(not(feature = "devnet"))]
// pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey = pubkey!("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");

// #[cfg(feature = "devnet")]
// pub const AMM_CONFIG_25BPS: Pubkey = pubkey!("9zSzfkYy6awexsHvmggeH36pfVUdDGyCcwmjT3AQPBj6");
// #[cfg(not(feature = "devnet"))]
// pub const AMM_CONFIG_25BPS: Pubkey = pubkey!("D4FPEruKEHrG5TenZ2mpDGEfu1iUvTiqBxvpU8HLBvC2");

// #[cfg(feature = "devnet")]
// pub const CREATE_POOL_FEE_RECEIVER: Pubkey =
//     pubkey!("G11FKBRaAkHAKuLCgLM6K6NUc9rTjPAznRCjZifrTQe2");
// #[cfg(not(feature = "devnet"))]
// pub const CREATE_POOL_FEE_RECEIVER: Pubkey =
//     pubkey!("DNXgeM9EiiaAbaWvwjHj9fQQLAX5ZsfHyvmYUNRAdNC8");

//  Simple hardcoded devnet values
pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey = pubkey!("CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW");
pub const AMM_CONFIG_25BPS: Pubkey = pubkey!("9zSzfkYy6awexsHvmggeH36pfVUdDGyCcwmjT3AQPBj6");
pub const CREATE_POOL_FEE_RECEIVER: Pubkey =
    pubkey!("G11FKBRaAkHAKuLCgLM6K6NUc9rTjPAznRCjZifrTQe2");

pub const POOL_SEED: &str = "pool";
pub const POOL_LP_MINT_SEED: &str = "pool_lp_mint";
pub const POOL_VAULT_SEED: &str = "pool_vault";
pub const OBSERVATION_SEED: &str = "observation";
pub const AUTH_SEED: &str = "vault_and_lp_mint_auth_seed";

#[derive(Accounts)]
pub struct TokenInteraction<'info> {
    #[account(mut)]
    pub user: Signer<'info>,

    #[account(
        mut,
        constraint = creator_mint.mint_authority == Some(token_state.key()).into(),
        constraint = creator_mint.freeze_authority == Some(token_state.key()).into(),
        constraint = creator_mint.key() == token_state.mint.key()
    )]
    pub creator_mint: Box<InterfaceAccount<'info, Mint>>, // ✅ BOXED

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = creator_mint,
        associated_token::authority = user
    )]
    pub user_ata: Box<InterfaceAccount<'info, TokenAccount>>, // ✅ BOXED

    #[account(
        mut,
        constraint =  token_vault.mint == creator_mint.key(),
        constraint = token_vault.owner == token_state.key(),
    )]
    pub token_vault: Box<InterfaceAccount<'info, TokenAccount>>, // ✅ BOXED

    #[account(
        mut,
        seeds = [
            b"token_state", creator_mint.key().as_ref()
        ],
        bump = token_state.bump
    )]
    pub token_state: Box<Account<'info, TokenState>>, // ✅ BOXED

    #[account(
        mut,
        seeds = [b"sol_vault", token_vault.key().as_ref()],
        bump,
    )]
    pub sol_vault: SystemAccount<'info>, // ⚪ Small - no boxing needed

    #[account(
        mut,
        seeds = [b"global_state"],
        bump = global_state.bump
    )]
    pub global_state: Box<Account<'info, GlobalState>>, // ✅ BOXED

    pub cp_swap_program: Program<'info, RaydiumCpmm>, // ⚪ Small - no boxing needed

    #[account(
        address = AMM_CONFIG_25BPS @ NottyTerminalError::InvalidAmmConfig
    )]
    pub amm_config: Box<Account<'info, AmmConfig>>, // ✅ BOXED (Raydium state can be large)

    /// CHECK: pool vault and lp mint authority
    #[account(
        seeds = [
            AUTH_SEED.as_bytes(),
        ],
        bump,
        seeds::program = cp_swap_program.key(),
    )]
    pub authority: UncheckedAccount<'info>, // ⚪ Small - no boxing needed

    /// CHECK: Initialize an account to store the pool state (only used if migration triggers)
    #[account(
        mut,
        seeds = [
            POOL_SEED.as_bytes(),
            amm_config.key().as_ref(),
            creator_mint.key().as_ref(),  // token_0_mint (must be smaller than WSOL)
            WSOL_MINT.as_ref(),           // token_1_mint
        ],
        bump,
        seeds::program = cp_swap_program.key(),
    )]
    pub pool_state: UncheckedAccount<'info>, // ⚪ Small - no boxing needed

    /// CHECK: Pool LP mint (only created if migration triggers)
    #[account(
        mut,
        seeds = [
            POOL_LP_MINT_SEED.as_bytes(),
            pool_state.key().as_ref(),
        ],
        bump,
        seeds::program = cp_swap_program.key(),
    )]
    pub lp_mint: UncheckedAccount<'info>, // ⚪ Small - no boxing needed

    /// Creator's token_0 account (your token)
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = creator_mint,
        associated_token::authority = user,
    )]
    pub creator_token_0: Box<InterfaceAccount<'info, TokenAccount>>, // ✅ BOXED

    pub wsol_mint: Box<InterfaceAccount<'info, Mint>>, // ✅ BOXED

    /// Creator's token_1 account (WSOL)
    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = wsol_mint,
        associated_token::authority = user,
    )]
    pub creator_token_1: Box<InterfaceAccount<'info, TokenAccount>>, // ✅ BOXED

    /// CHECK: Creator LP token account (only created if migration triggers)
    #[account(
        mut,
        seeds = [creator_lp_token.key().as_ref()],
        bump,
        seeds::program = associated_token_program.key(),
    )]
    pub creator_lp_token: UncheckedAccount<'info>, // ⚪ Small - no boxing needed
    // /// CHECK: Creator LP token - will be created by Raydium
    // #[account(mut)] // Raydium will derive this as ATA
    // pub creator_lp_token: UncheckedAccount<'info>,
    /// CHECK: Token_0 vault for the pool (only created if migration triggers)
    #[account(
        mut,
        seeds = [
            POOL_VAULT_SEED.as_bytes(),
            pool_state.key().as_ref(),
            creator_mint.key().as_ref()
        ],
        bump,
        seeds::program = cp_swap_program.key(),
    )]
    pub token_0_vault: UncheckedAccount<'info>, // ⚪ Small - no boxing needed

    /// CHECK: Token_1 vault for the pool (only created if migration triggers)
    #[account(
        mut,
        seeds = [
            POOL_VAULT_SEED.as_bytes(),
            pool_state.key().as_ref(),
            WSOL_MINT.as_ref()
        ],
        bump,
        seeds::program = cp_swap_program.key(),
    )]
    pub token_1_vault: UncheckedAccount<'info>, // ⚪ Small - no boxing needed

    #[account(
        mut,
        address = CREATE_POOL_FEE_RECEIVER @ NottyTerminalError::InvalidFeeReceiver
    )]
    pub create_pool_fee: Box<InterfaceAccount<'info, TokenAccount>>, // ✅ BOXED

    /// CHECK: Observation state for oracle data (only created if migration triggers)
    #[account(
        mut,
        seeds = [
            OBSERVATION_SEED.as_bytes(),
            pool_state.key().as_ref(),
        ],
        bump,
        seeds::program = cp_swap_program.key(),
    )]
    pub observation_state: UncheckedAccount<'info>, // ⚪ Small - no boxing needed

    pub token_program: Interface<'info, TokenInterface>, // ⚪ Small - no boxing needed
    pub system_program: Program<'info, System>,          // ⚪ Small - no boxing needed
    pub associated_token_program: Program<'info, AssociatedToken>, // ⚪ Small - no boxing needed
    pub rent: Sysvar<'info, Rent>,                       // ⚪ Small - no boxing needed
}

impl<'info> TokenInteraction<'info> {
    pub fn handle_purchase(&mut self, args: PurchaseTokenArgs) -> Result<()> {
        // check that token hasn't migrated
        require!(
            !self.token_state.migrated,
            NottyTerminalError::AlreadyGraduated
        );

        let amount = args.amount;
        let cost_lamports = self.get_current_token_price(amount)?;

        // (2) Check slippage
        require!(
            amount >= args.min_amount_out,
            NottyTerminalError::SlippageExceeded
        );

        // (3) Transfer SOL from buyer to sol_vault
        let transfer_cpi_account = Transfer {
            from: self.user.to_account_info(),
            to: self.sol_vault.to_account_info(),
        };

        let cpi_context =
            CpiContext::new(self.system_program.to_account_info(), transfer_cpi_account);

        transfer(cpi_context, cost_lamports)?;

        // (4) Transfer tokens to buyer's associated token account
        let creator_mint = self.creator_mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"token_state",
            creator_mint.as_ref(),
            &[self.token_state.bump],
        ]];

        let cpi_accounts_tf_spl = token_interface::TransferChecked {
            authority: self.token_state.to_account_info(),
            from: self.token_vault.to_account_info(),
            mint: self.creator_mint.to_account_info(),
            to: self.user_ata.to_account_info(),
        };

        let cpi_ctx_tf_spl = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts_tf_spl,
            signer_seeds,
        );

        token_interface::transfer_checked(cpi_ctx_tf_spl, amount, self.creator_mint.decimals)?;

        self.token_state.tokens_sold = self
            .token_state
            .tokens_sold
            .checked_add(amount)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        self.token_state.sol_raised = self
            .token_state
            .sol_raised
            .checked_add(cost_lamports)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        Ok(())
    }

    pub fn handle_sell(&mut self, args: SellTokenArgs) -> Result<()> {
        // check that token hasn't migrated
        require!(
            !self.token_state.migrated,
            NottyTerminalError::AlreadyGraduated
        );

        let amount = args.amount;

        // (1) Validate seller has enough tokens
        require!(
            self.user_ata.amount >= amount,
            NottyTerminalError::InsufficientTokenBalance
        );

        // (2) Validate there are enough tokens sold to sell back
        require!(
            self.token_state.tokens_sold >= amount,
            NottyTerminalError::InsufficientTokensSold
        );

        // (3) Calculate sell price (typically lower than buy price)
        let sell_proceeds = self.get_current_sell_price(amount)?;

        // (4) Check minimum proceeds (slippage protection)
        require!(
            sell_proceeds >= args.min_proceeds,
            NottyTerminalError::SlippageExceeded
        );

        // (5) Check sol_vault has enough SOL
        let sol_vault_balance = self.sol_vault.lamports();
        require!(
            sol_vault_balance >= sell_proceeds,
            NottyTerminalError::InsufficientVaultBalance
        );

        // (6) Transfer tokens from seller to vault
        let cpi_accounts_token = token_interface::TransferChecked {
            authority: self.user.to_account_info(),
            from: self.user_ata.to_account_info(),
            mint: self.creator_mint.to_account_info(),
            to: self.token_vault.to_account_info(),
        };

        let cpi_ctx_token =
            CpiContext::new(self.token_program.to_account_info(), cpi_accounts_token);

        token_interface::transfer_checked(cpi_ctx_token, amount, self.creator_mint.decimals)?;

        // (7) Transfer SOL from vault to seller
        let token_vault = self.token_vault.key();

        let signer_seeds: &[&[&[u8]]] = &[&[
            b"sol_vault",
            token_vault.as_ref(),
            &[self.token_state.sol_vault_bump],
        ]];

        let cpi_transfer_accounts = Transfer {
            from: self.sol_vault.to_account_info(),
            to: self.user.to_account_info(),
        };

        let cpi_ctx_transfer = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            cpi_transfer_accounts,
            signer_seeds,
        );

        transfer(cpi_ctx_transfer, sell_proceeds)?;

        // (8) Update state
        self.token_state.tokens_sold = self
            .token_state
            .tokens_sold
            .checked_sub(amount)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        self.token_state.sol_raised = self
            .token_state
            .sol_raised
            .checked_sub(sell_proceeds)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        msg!("Tokens sold: {} for {} lamports", amount, sell_proceeds);

        Ok(())
    }

    pub fn calculate_current_market_cap(&self) -> Result<u64> {
        let current_price = self.get_current_token_price(1); // base_price + slope * tokens_sold
        let cap_lamports = current_price?
            .checked_mul(self.token_state.total_supply)
            .ok_or_else(|| NottyTerminalError::NumericalOverflow)
            .unwrap();
        Ok(cap_lamports)
    }

    pub fn get_current_sell_price(&self, amount: u64) -> Result<u64> {
        // For linear bonding curve selling, we calculate the price as if we're "un-buying"
        // The sell price should be based on the price range that would be covered when selling

        let base_price_per_token = self.token_state.initial_price_per_token;
        let slope_per_token = self.global_state.slope;
        let tokens_sold = self.token_state.tokens_sold;

        // Convert to token units
        let amount_in_tokens = amount / 1_000_000_000;
        let tokens_sold_in_tokens = tokens_sold / 1_000_000_000;

        // When selling, we're reducing the tokens_sold count
        // So we calculate based on the price range from (tokens_sold - amount) to tokens_sold
        let new_tokens_sold = tokens_sold_in_tokens
            .checked_sub(amount_in_tokens)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        // Average price during the sell = price at midpoint
        let midpoint_tokens_sold = new_tokens_sold + (amount_in_tokens / 2);
        let sell_price_per_token = base_price_per_token + (slope_per_token * midpoint_tokens_sold);

        // Apply sell fee (e.g., 95% of buy price to prevent arbitrage)
        let sell_fee_basis_points = 500; // 5% fee (9500 basis points = 95%)
        let sell_proceeds = sell_price_per_token
            .checked_mul(amount_in_tokens)
            .and_then(|total| total.checked_mul(10000 - sell_fee_basis_points))
            .and_then(|total| total.checked_div(10000))
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        Ok(sell_proceeds)
    }

    pub fn get_current_token_price(&self, amount: u64) -> Result<u64> {
        let base_price_per_token = self.token_state.initial_price_per_token; // 25 lamports per token
        let slope_per_token = self.global_state.slope; // Linear increase per token sold
        let tokens_sold_in_tokens = self.token_state.tokens_sold / 1_000_000_000; // Convert to token units

        // Linear pricing: current_price = base + (slope × tokens_already_sold)
        let current_price_per_token =
            base_price_per_token + (slope_per_token * tokens_sold_in_tokens);

        // Cost = current_price × amount_buying (in token units)
        let amount_in_tokens = amount / 1_000_000_000;
        let total_cost = current_price_per_token * amount_in_tokens;

        Ok(total_cost)
    }

    fn create_wsol_and_deposit(&mut self, amount: u64) -> Result<()> {
        // Transfer SOL to WSOL token account
        let cpi_accounts = Transfer {
            from: self.user.to_account_info(),
            to: self.creator_token_1.to_account_info(),
        };

        let cpi_context = CpiContext::new(self.system_program.to_account_info(), cpi_accounts);

        transfer(cpi_context, amount)?;

        // Sync native to update the WSOL token account balance
        let sync_accounts = anchor_spl::token_interface::SyncNative {
            account: self.creator_token_1.to_account_info(),
        };

        let sync_context = CpiContext::new(self.token_program.to_account_info(), sync_accounts);

        anchor_spl::token_interface::sync_native(sync_context)?;

        Ok(())
    }

    fn prepare_migration_assets(&mut self, token_amount: u64, sol_amount: u64) -> Result<()> {
        // Transfer remaining tokens from vault to user (who will provide to pool)
        let creator_mint = self.creator_mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"token_state",
            creator_mint.as_ref(),
            &[self.token_state.bump],
        ]];

        let cpi_accounts_token = token_interface::TransferChecked {
            authority: self.token_state.to_account_info(),
            from: self.token_vault.to_account_info(),
            mint: self.creator_mint.to_account_info(),
            to: self.creator_token_0.to_account_info(),
        };

        let cpi_ctx_token = CpiContext::new_with_signer(
            self.token_program.to_account_info(),
            cpi_accounts_token,
            signer_seeds,
        );

        token_interface::transfer_checked(cpi_ctx_token, token_amount, self.creator_mint.decimals)?;

        // Transfer SOL for WSOL conversion
        let token_vault_key = self.token_vault.key();
        let sol_signer_seeds: &[&[&[u8]]] = &[&[
            b"sol_vault",
            token_vault_key.as_ref(),
            &[self.token_state.sol_vault_bump],
        ]];

        let cpi_transfer_accounts = Transfer {
            from: self.sol_vault.to_account_info(),
            to: self.user.to_account_info(),
        };

        let cpi_ctx_transfer = CpiContext::new_with_signer(
            self.system_program.to_account_info(),
            cpi_transfer_accounts,
            sol_signer_seeds,
        );

        transfer(cpi_ctx_transfer, sol_amount)?;

        // Convert SOL to WSOL
        self.create_wsol_and_deposit(sol_amount)?;

        Ok(())
    }

    fn handle_migration(&mut self) -> Result<()> {
        msg!("🚀 Triggering automatic migration to Raydium...");

        // Verify token ordering constraint
        require!(
            self.creator_mint.key() < WSOL_MINT,
            NottyTerminalError::InvalidTokenOrdering
        );

        // Calculate migration amounts
        let remaining_tokens = self.token_vault.amount;
        let sol_liquidity = self.sol_vault.lamports();
        let pool_creation_fee = 150_000_000; // 0.15 SOL
        let available_sol = sol_liquidity.saturating_sub(pool_creation_fee);

        // Transfer assets for migration
        self.prepare_migration_assets(remaining_tokens, available_sol)?;

        // Create Raydium pool via CPI
        self.create_raydium_pool(remaining_tokens, available_sol)?;

        // Update state
        self.token_state.migrated = true;
        self.token_state.raydium_pool = Some(self.pool_state.key());
        self.token_state.migration_timestamp = Clock::get()?.unix_timestamp;

        emit!(MigrationCompletedEvent {
            token_mint: self.creator_mint.key(),
            completing_buyer: self.user.key(),
            pool_address: self.pool_state.key(),
            tokens_migrated: remaining_tokens,
            sol_migrated: available_sol,
            timestamp: self.token_state.migration_timestamp,
        });

        msg!(
            "✅ Migration successful! Raydium pool: {}",
            self.pool_state.key()
        );

        Ok(())
    }

    fn create_raydium_pool(&mut self, token_amount: u64, sol_amount: u64) -> Result<()> {
        // Create all Raydium accounts first (pool_state, lp_mint, vaults, observation_state, creator_lp_token)
        // self.initialize_raydium_accounts()?;

        // Now call Raydium initialize with all accounts created
        let initialize_accounts = Initialize {
            creator: self.user.to_account_info(),
            amm_config: self.amm_config.to_account_info(),
            authority: self.authority.to_account_info(),
            pool_state: self.pool_state.to_account_info(),
            token_0_mint: self.creator_mint.to_account_info(),
            token_1_mint: self.wsol_mint.to_account_info(),
            lp_mint: self.lp_mint.to_account_info(),
            creator_token_0: self.creator_token_0.to_account_info(),
            creator_token_1: self.creator_token_1.to_account_info(),
            creator_lp_token: self.creator_lp_token.to_account_info(),
            token_0_vault: self.token_0_vault.to_account_info(),
            token_1_vault: self.token_1_vault.to_account_info(),
            create_pool_fee: self.create_pool_fee.to_account_info(),
            observation_state: self.observation_state.to_account_info(),
            token_program: self.token_program.to_account_info(),
            token_0_program: self.token_program.to_account_info(),
            token_1_program: self.token_program.to_account_info(),
            associated_token_program: self.associated_token_program.to_account_info(),
            system_program: self.system_program.to_account_info(),
            rent: self.rent.to_account_info(),
        };

        let cpi_context =
            CpiContext::new(self.cp_swap_program.to_account_info(), initialize_accounts);

        initialize(
            cpi_context,
            token_amount,
            sol_amount,
            Clock::get()?.unix_timestamp as u64, // Open immediately
        )?;

        Ok(())
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct PurchaseTokenArgs {
    pub amount: u64,
    pub min_amount_out: u64,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct SellTokenArgs {
    pub amount: u64,       // Amount of tokens to sell (in base units)
    pub min_proceeds: u64, // Minimum SOL to receive (slippage protection)
}

#[event]
pub struct MigrationCompletedEvent {
    pub token_mint: Pubkey,
    pub completing_buyer: Pubkey,
    pub pool_address: Pubkey,
    pub tokens_migrated: u64,
    pub sol_migrated: u64,
    pub timestamp: i64,
}
// token price = base_price + slope × S (Linear)

// where S = current token supply (tokens sold)
// base_price = price when supply = 0 (starting price)
// slope = how much price increases per token sold

// cost for n tokens = base_price * n + slope * (current_supply * n + n²/2) (Quadratic)

// this becomes integral because user is purchasing multiple tokens at a go and needs to calculate integral
// this is gotten from

// Total Cost = ∫[S₀ to S₀+n] (base_price + slope × S) dS
//           V
// Total Cost = [base_price × S + slope × S²/2] evaluated from S₀ to (S₀+n)
//           V
// Total Cost = base_price × n + slope × (S₀×n + n²/2)

// Market Cap = current_price × TOTAL_SUPPLY

// pub fn get_current_token_price_quad(&self, amount: u64) -> Result<u64> {
//         let base_price = self.token_state.initial_price_per_token;
//         let slope = self.global_state.slope;
//         let tokens_sold = self.token_state.tokens_sold;
//         let n = amount;

//         // (1) base_price * n
//         let first = base_price
//             .checked_mul(n)
//             .and_then(|result| result.checked_div(1_000_000_000)) // Divide by 10^9
//             .ok_or(PriceCalculationError::LinearCostOverflow)?;

//         // (2) n² / 2
//         let n_squared = {
//             let n_u128 = n as u128;
//             let n_squared_u128 = n_u128 * n_u128;

//             if n_squared_u128 > u64::MAX as u128 {
//                 // For very large n, we can still calculate safely in u128
//                 // since we'll be dividing by 2 and then by 10^9 later
//                 n_squared_u128
//             } else {
//                 n_squared_u128
//             }
//         };

//         let second = n_squared
//             .checked_div(2)
//             .ok_or(PriceCalculationError::QuadraticDivisionOverflow)? as u64;

//         // (3) tokens_sold * n
//         let third = tokens_sold
//             .checked_mul(n)
//             .ok_or(PriceCalculationError::SlopeSupplyOverflow)?;

//         // (4) third + second
//         let inner = third
//             .checked_add(second)
//             .ok_or(PriceCalculationError::QuadraticSlopeOverflow)?;

//         // (5) slope * inner / DECIMALS_FACTOR using u128 to prevent overflow
//         const DECIMALS_FACTOR: u128 = 1_000_000_000; // 10^9 for 9 decimals

//         let slope_part = {
//             let slope_u128 = slope as u128;
//             let inner_u128 = inner as u128;
//             let result_u128 = slope_u128 * inner_u128 / DECIMALS_FACTOR;

//             if result_u128 > u64::MAX as u128 {
//                 return Err(PriceCalculationError::QuadraticSlopeOverflow.into());
//             }
//             result_u128 as u64
//         };

//         // (6) first + slope_part
//         let total_cost_in_lamports = first
//             .checked_add(slope_part)
//             .ok_or(PriceCalculationError::FinalSumOverflow)?;

//         Ok(total_cost_in_lamports)
//     }
