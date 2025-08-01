use anchor_lang::prelude::*;

use raydium_cpmm_cpi::{
    cpi::{accounts::Initialize, initialize},
    program::RaydiumCpmm,
    states::{AmmConfig, ObservationState, PoolState},
};

pub const RAYDIUM_CPMM_PROGRAM_ID: Pubkey = pubkey!("CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW");
pub const AMM_CONFIG_25BPS: Pubkey = pubkey!("9zSzfkYy6awexsHvmggeH36pfVUdDGyCcwmjT3AQPBj6");
pub const CREATE_POOL_FEE_RECEIVER: Pubkey =
    pubkey!("G11FKBRaAkHAKuLCgLM6K6NUc9rTjPAznRCjZifrTQe2");

pub const POOL_SEED: &str = "pool";
pub const POOL_LP_MINT_SEED: &str = "pool_lp_mint";
pub const POOL_VAULT_SEED: &str = "pool_vault";
pub const OBSERVATION_SEED: &str = "observation";
pub const AUTH_SEED: &str = "vault_and_lp_mint_auth_seed";

pub const WSOL_MINT: Pubkey = pubkey!("So11111111111111111111111111111111111111112");
