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
    pub fn handle_purchase(&mut self, args: PurchaseTokenArgs) -> Result<()> {
        let amount = args.amount;
        let cost_lamports = self.get_current_token_price(amount)?;

        // (2) Check slippage
        require!(
            amount >= args.min_amount_out,
            NottyTerminalError::SlippageExceeded
        );

        // (3) Transfer SOL from buyer to sol_vault
        let transfer_cpi_account = Transfer {
            from: self.buyer.to_account_info(),
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
            to: self.buyer_ata.to_account_info(),
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

    pub fn calculate_current_market_cap(&self) -> Result<u64> {
        let current_price = self.get_current_token_price(1); // base_price + slope * tokens_sold
        let cap_lamports = current_price?
            .checked_mul(self.token_state.total_supply)
            .ok_or_else(|| NottyTerminalError::NumericalOverflow)
            .unwrap();
        Ok(cap_lamports)
    }

    pub fn get_current_token_price(&self, amount: u64) -> Result<u64> {
        let base_price = self.token_state.initial_price_per_token;
        let slope = self.global_state.slope;
        let tokens_sold = self.token_state.tokens_sold;
        let n = amount;

        // (1) base_price * n
        let first = base_price
            .checked_mul(n)
            .and_then(|result| result.checked_div(1_000_000_000)) // Divide by 10^9
            .ok_or(PriceCalculationError::LinearCostOverflow)?;

        // (2) n² / 2
        let n_squared = {
            let n_u128 = n as u128;
            let n_squared_u128 = n_u128 * n_u128;

            if n_squared_u128 > u64::MAX as u128 {
                // For very large n, we can still calculate safely in u128
                // since we'll be dividing by 2 and then by 10^9 later
                n_squared_u128
            } else {
                n_squared_u128
            }
        };

        let second = n_squared
            .checked_div(2)
            .ok_or(PriceCalculationError::QuadraticDivisionOverflow)? as u64;

        // (3) tokens_sold * n
        let third = tokens_sold
            .checked_mul(n)
            .ok_or(PriceCalculationError::SlopeSupplyOverflow)?;

        // (4) third + second
        let inner = third
            .checked_add(second)
            .ok_or(PriceCalculationError::QuadraticSlopeOverflow)?;

        // (5) slope * inner / DECIMALS_FACTOR using u128 to prevent overflow
        const DECIMALS_FACTOR: u128 = 1_000_000_000; // 10^9 for 9 decimals

        let slope_part = {
            let slope_u128 = slope as u128;
            let inner_u128 = inner as u128;
            let result_u128 = slope_u128 * inner_u128 / DECIMALS_FACTOR;

            if result_u128 > u64::MAX as u128 {
                return Err(PriceCalculationError::QuadraticSlopeOverflow.into());
            }
            result_u128 as u64
        };

        // (6) first + slope_part
        let total_cost_in_lamports = first
            .checked_add(slope_part)
            .ok_or(PriceCalculationError::FinalSumOverflow)?;

        Ok(total_cost_in_lamports)
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct PurchaseTokenArgs {
    pub amount: u64,
    pub min_amount_out: u64,
}

// token price = base_price + slope × S

// where S = current token supply (tokens sold)
// base_price = price when supply = 0 (starting price)
// slope = how much price increases per token sold

// cost for n tokens = base_price * n + slope * (current_supply * n + n²/2)

// this becomes integral because user is purchasing multiple tokens at a go and needs to calculate integral
// this is gotten from

// Total Cost = ∫[S₀ to S₀+n] (base_price + slope × S) dS
//           V
// Total Cost = [base_price × S + slope × S²/2] evaluated from S₀ to (S₀+n)
//           V
// Total Cost = base_price × n + slope × (S₀×n + n²/2)

// Market Cap = current_price × TOTAL_SUPPLY
