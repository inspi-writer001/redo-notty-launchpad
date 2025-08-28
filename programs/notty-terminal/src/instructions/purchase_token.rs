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

use std::cmp::min;

pub const INITIAL_MCAP_SOL: u64 = 50;
pub const MIGRATION_MCAP_SOL: u64 = 450;
pub const TOTAL_SUPPLY: u64 = 1_000_000_000; // 1B tokens
pub const MIGRATION_THRESHOLD_PCT: u64 = 86; // Migration at 86% sold

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
        constraint = platform_sol_vault.key() == global_state.vault.key() @NottyTerminalError::WrongVault,
        seeds = [b"vault"],
        bump = global_state.vault_bump
    )]
    pub platform_sol_vault: SystemAccount<'info>,

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
        require!(
            !self.token_state.migrated,
            NottyTerminalError::AlreadyGraduated
        );

        let amount = args.amount;

        // Calculate base cost without fees
        let base_cost_lamports = self.get_current_token_price(amount)?;

        // Calculate trading fee (1.5%)
        let trading_fee = base_cost_lamports
            .checked_mul(self.global_state.trading_fee_bps as u64)
            .and_then(|f| f.checked_div(10000))
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        // Total cost = base + fee
        let total_cost_lamports = base_cost_lamports
            .checked_add(trading_fee)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        // Check slippage against total cost
        require!(
            total_cost_lamports <= args.max_sol_cost,
            NottyTerminalError::SlippageExceeded
        );

        // Transfer total cost from buyer
        let transfer_cpi_account = Transfer {
            from: self.user.to_account_info(),
            to: self.sol_vault.to_account_info(),
        };

        transfer(
            CpiContext::new(self.system_program.to_account_info(), transfer_cpi_account),
            total_cost_lamports,
        )?;

        // Transfer trading fee to platform vault
        let token_vault = self.token_vault.key();
        let sol_vault_seeds: &[&[&[u8]]] = &[&[
            b"sol_vault",
            token_vault.as_ref(),
            &[self.token_state.sol_vault_bump],
        ]];

        let fee_transfer = Transfer {
            from: self.sol_vault.to_account_info(),
            to: self.platform_sol_vault.to_account_info(), // Platform vault
        };

        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                fee_transfer,
                sol_vault_seeds,
            ),
            trading_fee,
        )?;

        // Transfer tokens to buyer
        let creator_mint = self.creator_mint.key();
        let signer_seeds: &[&[&[u8]]] = &[&[
            b"token_state",
            creator_mint.as_ref(),
            &[self.token_state.bump],
        ]];

        token_interface::transfer_checked(
            CpiContext::new_with_signer(
                self.token_program.to_account_info(),
                token_interface::TransferChecked {
                    authority: self.token_state.to_account_info(),
                    from: self.token_vault.to_account_info(),
                    mint: self.creator_mint.to_account_info(),
                    to: self.user_ata.to_account_info(),
                },
                signer_seeds,
            ),
            amount,
            self.creator_mint.decimals,
        )?;

        // Update state (only track base amount for bonding curve)
        self.token_state.tokens_sold = self
            .token_state
            .tokens_sold
            .checked_add(amount)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        self.token_state.sol_raised = self
            .token_state
            .sol_raised
            .checked_add(base_cost_lamports) // Only base cost counted toward migration
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        // Update global metrics
        self.global_state.total_fees_collected = self
            .global_state
            .total_fees_collected
            .checked_add(trading_fee)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        self.global_state.total_trading_volume = self
            .global_state
            .total_trading_volume
            .checked_add(total_cost_lamports)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        emit!(PurchasedToken {
            amount_purchased: amount,
            base_cost: base_cost_lamports,
            trading_fee,
            total_cost: total_cost_lamports,
            current_price: self.get_current_token_price(1_000_000_000)?,
            migrated: self.token_state.migrated,
            mint: self.token_state.mint,
            sol_raised: self.token_state.sol_raised,
            tokens_sold: self.token_state.tokens_sold,
            total_supply: self.token_state.total_supply,
            buyer: self.user.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }

    pub fn handle_sell(&mut self, args: SellTokenArgs) -> Result<()> {
        require!(
            !self.token_state.migrated,
            NottyTerminalError::AlreadyGraduated
        );

        let amount = args.amount;

        // Validate seller has enough tokens
        require!(
            self.user_ata.amount >= amount,
            NottyTerminalError::InsufficientTokenBalance
        );

        // Calculate base sell proceeds
        let base_proceeds = self.get_current_sell_price(amount)?;

        // Calculate trading fee (1.5% of proceeds)
        let trading_fee = base_proceeds
            .checked_mul(self.global_state.trading_fee_bps as u64)
            .and_then(|f| f.checked_div(10000))
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        // Net proceeds after fee
        let net_proceeds = base_proceeds
            .checked_sub(trading_fee)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        // Check slippage
        require!(
            net_proceeds >= args.min_proceeds,
            NottyTerminalError::SlippageExceeded
        );

        // Check vault has enough SOL
        let sol_vault_balance = self.sol_vault.lamports();
        require!(
            sol_vault_balance >= base_proceeds,
            NottyTerminalError::InsufficientVaultBalance
        );

        // Transfer tokens from seller to vault
        token_interface::transfer_checked(
            CpiContext::new(
                self.token_program.to_account_info(),
                token_interface::TransferChecked {
                    authority: self.user.to_account_info(),
                    from: self.user_ata.to_account_info(),
                    mint: self.creator_mint.to_account_info(),
                    to: self.token_vault.to_account_info(),
                },
            ),
            amount,
            self.creator_mint.decimals,
        )?;

        let token_vault = self.token_vault.key();
        let sol_vault_seeds: &[&[&[u8]]] = &[&[
            b"sol_vault",
            token_vault.as_ref(),
            &[self.token_state.sol_vault_bump],
        ]];

        // Transfer fee to platform vault
        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.sol_vault.to_account_info(),
                    to: self.platform_sol_vault.to_account_info(),
                },
                sol_vault_seeds,
            ),
            trading_fee,
        )?;

        // Transfer net proceeds to seller
        transfer(
            CpiContext::new_with_signer(
                self.system_program.to_account_info(),
                Transfer {
                    from: self.sol_vault.to_account_info(),
                    to: self.user.to_account_info(),
                },
                sol_vault_seeds,
            ),
            net_proceeds,
        )?;

        // Update state
        self.token_state.tokens_sold = self
            .token_state
            .tokens_sold
            .checked_sub(amount)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        self.token_state.sol_raised = self
            .token_state
            .sol_raised
            .checked_sub(base_proceeds)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        // Update global metrics
        self.global_state.total_fees_collected = self
            .global_state
            .total_fees_collected
            .checked_add(trading_fee)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        self.global_state.total_trading_volume = self
            .global_state
            .total_trading_volume
            .checked_add(base_proceeds)
            .ok_or(NottyTerminalError::NumericalOverflow)?;

        emit!(SoldToken {
            amount_sold: amount,
            base_proceeds,
            trading_fee,
            net_proceeds,
            current_price: self.get_current_token_price(1_000_000_000)?,
            migrated: self.token_state.migrated,
            mint: self.token_state.mint,
            sol_raised: self.token_state.sol_raised,
            tokens_sold: self.token_state.tokens_sold,
            total_supply: self.token_state.total_supply,
            seller: self.user.key(),
            timestamp: Clock::get()?.unix_timestamp,
        });

        Ok(())
    }

    pub fn calculate_current_market_cap(&self) -> Result<u64> {
        const BASE_PRICE_PER_MILLION: u64 = 50;
        const MAX_PRICE_PER_MILLION: u64 = 450;
        const PRICE_RANGE: u64 = MAX_PRICE_PER_MILLION - BASE_PRICE_PER_MILLION;

        let total_base_units = TOTAL_SUPPLY * 1_000_000_000;
        let migration_base_units = (total_base_units / 100) * MIGRATION_THRESHOLD_PCT;

        let progress = (self.token_state.tokens_sold * 1000) / migration_base_units;
        let capped_progress = min(progress, 1000);
        let sqrt_progress = integer_sqrt(capped_progress)?;

        let price_per_million = BASE_PRICE_PER_MILLION + (PRICE_RANGE * sqrt_progress / 31);
        let total_millions = total_base_units / 1_000_000;
        let market_cap_lamports = price_per_million * total_millions;

        Ok(market_cap_lamports)
    }

    pub fn get_current_sell_price(&self, amount_base_units: u64) -> Result<u64> {
        const BASE_PRICE_PER_MILLION: u64 = 50;
        const MAX_PRICE_PER_MILLION: u64 = 450;
        const PRICE_RANGE: u64 = MAX_PRICE_PER_MILLION - BASE_PRICE_PER_MILLION;

        let new_tokens_sold = self
            .token_state
            .tokens_sold
            .checked_sub(amount_base_units)
            .ok_or(NottyTerminalError::InsufficientTokensSold)?;

        let total_base_units = TOTAL_SUPPLY * 1_000_000_000;
        // Fix overflow here too
        let migration_base_units = (total_base_units / 100) * MIGRATION_THRESHOLD_PCT;

        let new_progress = (new_tokens_sold * 1000) / migration_base_units;
        let new_sqrt = integer_sqrt(min(new_progress, 1000))?;
        let new_price = BASE_PRICE_PER_MILLION + (PRICE_RANGE * new_sqrt / 31);

        let current_progress = (self.token_state.tokens_sold * 1000) / migration_base_units;
        let current_sqrt = integer_sqrt(min(current_progress, 1000))?;
        let current_price = BASE_PRICE_PER_MILLION + (PRICE_RANGE * current_sqrt / 31);

        let avg_price_per_million = (current_price + new_price) / 2;
        let sell_proceeds = (amount_base_units / 1_000_000) * avg_price_per_million;

        Ok(sell_proceeds)
    }

    pub fn get_current_token_price(&self, amount_base_units: u64) -> Result<u64> {
        const BASE_PRICE_PER_MILLION: u64 = 50;
        const MAX_PRICE_PER_MILLION: u64 = 450;
        const PRICE_RANGE: u64 = MAX_PRICE_PER_MILLION - BASE_PRICE_PER_MILLION;

        // Avoid overflow by dividing first
        let total_base_units = TOTAL_SUPPLY * 1_000_000_000;
        // Instead of: total_base_units * 86 / 100
        // Do: (total_base_units / 100) * 86
        let migration_base_units = (total_base_units / 100) * MIGRATION_THRESHOLD_PCT;

        let progress = (self.token_state.tokens_sold * 1000) / migration_base_units;
        let capped_progress = min(progress, 1000);
        let sqrt_progress = integer_sqrt(capped_progress)?;

        let price_per_million = BASE_PRICE_PER_MILLION + (PRICE_RANGE * sqrt_progress / 31);
        let total_cost = (amount_base_units / 1_000_000) * price_per_million;

        Ok(total_cost)
    }
}

pub fn integer_sqrt(n: u64) -> Result<u64> {
    if n == 0 {
        return Ok(0);
    }
    if n == 1 {
        return Ok(1);
    }

    let mut x = n;
    let mut y = (x + 1) / 2;

    while y < x {
        x = y;
        y = (x + n / x) / 2;
    }

    Ok(x)
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct PurchaseTokenArgs {
    pub amount: u64,
    pub max_sol_cost: u64,
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone)]
pub struct SellTokenArgs {
    pub amount: u64,       // Amount of tokens to sell (in base units)
    pub min_proceeds: u64, // Minimum SOL to receive (slippage protection)
}

#[event]
pub struct PurchasedToken {
    pub base_cost: u64,
    pub trading_fee: u64,
    pub total_cost: u64,
    pub mint: Pubkey,
    pub amount_purchased: u64,
    pub migrated: bool,
    pub total_supply: u64,
    pub tokens_sold: u64,
    pub sol_raised: u64,
    pub current_price: u64,
    pub buyer: Pubkey,
    pub timestamp: i64,
}

#[event]
pub struct SoldToken {
    pub mint: Pubkey,
    pub base_proceeds: u64,
    pub trading_fee: u64,
    pub net_proceeds: u64,
    pub amount_sold: u64,
    pub migrated: bool,
    pub total_supply: u64,
    pub tokens_sold: u64,
    pub sol_raised: u64,
    pub current_price: u64,
    pub seller: Pubkey,
    pub timestamp: i64,
}
