use anchor_lang::{
    prelude::*,
    system_program::{transfer, Transfer},
};
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{self, Mint, TokenAccount, TokenInterface},
};

use crate::{
    error::{NottyTerminalError, PriceCalculationError},
    GlobalState, TokenState,
};

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
    pub creator_mint: InterfaceAccount<'info, Mint>,

    #[account(
        init_if_needed,
        payer = user,
        associated_token::mint = creator_mint,
        associated_token::authority = user
    )]
    pub user_ata: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
        constraint =  token_vault.mint == creator_mint.key(),
        constraint = token_vault.owner == token_state.key(),
    )]
    pub token_vault: InterfaceAccount<'info, TokenAccount>,

    #[account(
        mut,
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

        emit!(PurchasedToken {
            amount_purchased: amount,
            current_price: self.get_current_token_price(1_000_000_000)?,
            migrated: self.token_state.migrated,
            mint: self.token_state.mint,
            sol_raised: self.token_state.sol_raised,
            tokens_sold: self.token_state.tokens_sold,
            total_supply: self.token_state.total_supply,
            cost: cost_lamports,
            buyer: self.user.key(),
            timestamp: Clock::get()?.unix_timestamp
        });

        Ok(())
    }

    pub fn handle_sell(&mut self, args: SellTokenArgs) -> Result<()> {
        // check that token hasn't migrated
        require!(
            !self.token_state.migrated,
            NottyTerminalError::AlreadyGraduated
        );

        require!(
            self.token_state.sol_raised < self.token_state.target_sol,
            NottyTerminalError::AwaitingGraduation
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

        emit!(SoldToken {
            amount_sold: amount,
            cost: sell_proceeds,
            current_price: self.get_current_token_price(1_000_000_000)?,
            migrated: self.token_state.migrated,
            mint: self.token_state.mint,
            sol_raised: self.token_state.sol_raised,
            tokens_sold: self.token_state.tokens_sold,
            total_supply: self.token_state.total_supply,
            seller: self.user.key(),
            timestamp: Clock::get()?.unix_timestamp
        });

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
pub struct PurchasedToken {
    pub mint: Pubkey,
    pub amount_purchased: u64,
    pub migrated: bool,
    pub total_supply: u64,
    pub tokens_sold: u64,
    pub sol_raised: u64,
    pub cost: u64,
    pub current_price: u64,
    pub buyer: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct SoldToken {
    pub mint: Pubkey,
    pub cost: u64,
    pub amount_sold: u64,
    pub migrated: bool,
    pub total_supply: u64,
    pub tokens_sold: u64,
    pub sol_raised: u64,
    pub current_price: u64,
    pub seller: Pubkey,
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
