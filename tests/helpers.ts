// import * as anchor from "@coral-xyz/anchor";

import { PublicKey } from "@solana/web3.js";

// const RAYDIUM_CPMM_PROGRAM_ID = new anchor.web3.PublicKey(
//   "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C"
// );
// const SOL_MINT = new anchor.web3.PublicKey(
//   "So11111111111111111111111111111111111111112"
// );

// // You'll need to get these from Raydium's deployed configs
// // These are example addresses - replace with actual ones for your environment
// const AMM_CONFIG = new anchor.web3.PublicKey(
//   "D8wAxwpH2aKaEGBKfeGdnQbCc2s54NrRvTDXCK98VAeT"
// );
// const CREATE_POOL_FEE_RECEIVER = new anchor.web3.PublicKey(
//   "7YttLkHDoNj9wyDur5pM1ejNaAvT9X4eqaYcHQqtj2G5"
// );

// Helper functions similar to Raydium's utils
async function getAuthAddress(
  programId: PublicKey
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("amm_authority")],
    programId
  );
}

async function getPoolAddress(
  configId: PublicKey,
  token0: PublicKey,
  token1: PublicKey,
  programId: PublicKey
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddressSync(
    [
      Buffer.from("pool"),
      configId.toBytes(),
      token0.toBytes(),
      token1.toBytes()
    ],
    programId
  );
}

async function getPoolLpMintAddress(
  poolId: PublicKey,
  programId: PublicKey
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("pool_lp_mint"), poolId.toBytes()],
    programId
  );
}

async function getPoolVaultAddress(
  poolId: PublicKey,
  tokenMint: PublicKey,
  programId: PublicKey
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("pool_vault"), poolId.toBytes(), tokenMint.toBytes()],
    programId
  );
}

async function getObservationAddress(
  poolId: PublicKey,
  programId: PublicKey
): Promise<[PublicKey, number]> {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("observation"), poolId.toBytes()],
    programId
  );
}

export {
  getAuthAddress,
  getObservationAddress,
  getPoolAddress,
  getPoolLpMintAddress,
  getPoolVaultAddress
};
