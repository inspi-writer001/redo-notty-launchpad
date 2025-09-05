import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NottyTerminal } from "../target/types/notty_terminal";
import admin_file from "./wallets/admin-wallet.json";
import user_1_file from "./wallets/user-1-wallet.json";
import {
  associatedAddress,
  TOKEN_PROGRAM_ID,
} from "@coral-xyz/anchor/dist/cjs/utils/token";
import {
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount,
} from "@solana/spl-token";
import { publicKey } from "@coral-xyz/anchor/dist/cjs/utils";
import { SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import { SYSTEM_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/native/system";

let admin_wallet = anchor.web3.Keypair.fromSecretKey(
  new Uint8Array(admin_file)
);
let user_1_wallet = anchor.web3.Keypair.fromSecretKey(
  new Uint8Array(user_1_file)
);

const WSOL_MINT = new anchor.web3.PublicKey(
  "So11111111111111111111111111111111111111112"
);

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
const RAYDIUM_CPMM_PROGRAM_ID = new anchor.web3.PublicKey(
  "CPMDWBwJDtYax9qW7AyRuVC19Cc4L4Vcy4n2BHAbHkCW"
);
const AMM_CONFIG_25BPS = new anchor.web3.PublicKey(
  "9zSzfkYy6awexsHvmggeH36pfVUdDGyCcwmjT3AQPBj6"
);
const CREATE_POOL_FEE_RECEIVER = new anchor.web3.PublicKey(
  "G11FKBRaAkHAKuLCgLM6K6NUc9rTjPAznRCjZifrTQe2"
);

const POOL_SEED = "pool";
const POOL_LP_MINT_SEED = "pool_lp_mint";
const POOL_VAULT_SEED = "pool_vault";
const OBSERVATION_SEED = "observation";
const AUTH_SEED = "vault_and_lp_mint_auth_seed";

describe("notty-terminal", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.nottyTerminal as Program<NottyTerminal>;
  let tokenMint;

  it.skip("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods
      .initialize({
        listingFeeLamport: new anchor.BN(50_000_000),
        tradingFeeBps: 150,
        migrationFeeLamport: new anchor.BN(150_000_000),
      })
      .accounts({
        admin: admin_wallet.publicKey,
      })
      .signers([admin_wallet])
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it.only("should create token and purchase it", async () => {
    try {
      // tokenMint = anchor.web3.Keypair.generate();
      tokenMint = {
        publicKey: new anchor.web3.PublicKey(
          "976uSFjh3w2ENsCFHj2v1iE8kNzG3DTGAiTs9jh3kQ5k"
        ),
      };

      console.log(tokenMint.publicKey.toBase58());
      // Add your test here.
      // const tx = await program.methods
      //   .createToken({
      //     name: "Shinobi Jenks",
      //     tokenSymbol: "SJK",
      //     tokenUri: "https://avatars.githubusercontent.com/u/94226358?v=4",
      //     targetSol: new anchor.BN(450_000_000_000), // 450 SOL (matches your metrics)
      //     startMcap: new anchor.BN(50_000_000_000), // 50 SOL (matches your metrics)
      //     totalSupply: new anchor.BN(1_000_000_000), // 1B tokens (matches your metrics)
      //   })
      //   .signers([user_1_wallet, tokenMint])
      //   .accounts({
      //     creator: user_1_wallet.publicKey,
      //     creatorMint: tokenMint.publicKey,
      //     tokenProgram: TOKEN_PROGRAM_ID,
      //   })
      //   .rpc();
      // console.log("Your transaction signature", tx);

      console.log("=== BEFORE PURCHASE ===");

      let [token_state, _] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("token_state"), tokenMint.publicKey.toBytes()],
        program.programId
      );

      const beforeState = await program.account.tokenState.fetch(token_state);

      console.log(beforeState);
      console.log(
        "SOL Raised before:",
        Number(beforeState.solRaised) / 1_000_000_000
      );

      let token_vault = await getOrCreateAssociatedTokenAccount(
        anchor.getProvider().connection,
        user_1_wallet,
        tokenMint.publicKey,
        token_state,
        true
      );

      // let platformSolVault = anchor.web3.PublicKey.findProgramAddressSync(
      //   [Buffer.from("vault")],
      //   program.programId
      // )[0];

      // const tx1 = await program.methods
      //   .purchaseToken({
      //     amount: new anchor.BN(30_000_000_000_000),
      //     maxSolCost: new anchor.BN(2)
      //       .pow(new anchor.BN(64))
      //       .sub(new anchor.BN(1)), // 2 ^ 64 - 1 ( for u64 )
      //   })
      //   .signers([user_1_wallet])
      //   .accounts({
      //     user: user_1_wallet.publicKey,
      //     creatorMint: tokenMint.publicKey,
      //     tokenProgram: TOKEN_PROGRAM_ID,
      //     tokenVault: token_vault.address,
      //   })
      //   .rpc();

      const tx2 = await program.methods
        .purchaseToken({
          amount: new anchor.BN(10_000_000_000_000),
          maxSolCost: new anchor.BN(2)
            .pow(new anchor.BN(64))
            .sub(new anchor.BN(1)), // 2 ^ 64 - 1 ( for u64 )
        })
        .signers([user_1_wallet])
        .accounts({
          user: user_1_wallet.publicKey,
          creatorMint: tokenMint.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenVault: token_vault.address,
        })
        .rpc();

      // console.log("Your transaction signature 2", tx1);
      console.log("Your transaction signature 3", tx2);
      console.log("=== AFTER PURCHASE ===");
      const afterState = await program.account.tokenState.fetch(token_state);

      console.log(afterState);
      console.log(
        "SOL Raised after:",
        Number(afterState.solRaised) / 1_000_000_000
      );
      console.log(
        "Tokens Sold:",
        Number(afterState.tokensSold) / 1_000_000_000
      );
    } catch (error) {
      console.log(error);
      if (error.logs) {
        throw Error(error.logs);
      } else {
        throw Error(error);
      }
    }
  });

  it.skip("should sell tokens", async () => {
    try {
      // let tokenMint = anchor.web3.Keypair.generate();
      let tokenMint = {
        publicKey: new anchor.web3.PublicKey(
          "5yyWspyRd8uVQ7LoQWh2W6AcjtbMr6RgfPLt9o88h1jn"
        ),
      };

      console.log(tokenMint.publicKey.toBase58());

      console.log("=== BEFORE SALES ===");

      let [token_state, _] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("token_state"), tokenMint.publicKey.toBytes()],
        program.programId
      );

      const beforeState = await program.account.tokenState.fetch(token_state);
      console.log(
        "SOL Raised before:",
        Number(beforeState.solRaised) / 1_000_000_000
      );

      let token_vault = await getOrCreateAssociatedTokenAccount(
        anchor.getProvider().connection,
        user_1_wallet,
        tokenMint.publicKey,
        token_state,
        true
      );

      const tx1 = await program.methods
        .sellToken({
          amount: new anchor.BN(1_000_000_000_0),
          minProceeds: new anchor.BN(0),
        })
        .signers([user_1_wallet])
        .accounts({
          user: user_1_wallet.publicKey,
          creatorMint: tokenMint.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenVault: token_vault.address,
        })
        .rpc();

      console.log("Your transaction signature 2", tx1);
      console.log("=== AFTER SALES ===");
      const afterState = await program.account.tokenState.fetch(token_state);
      console.log(
        "SOL Raised after:",
        Number(afterState.solRaised) / 1_000_000_000
      );
      console.log(
        "Tokens Sold:",
        Number(afterState.tokensSold) / 1_000_000_000
      );
    } catch (error) {
      console.log(error);
      if (error.logs) {
        throw Error(error.logs);
      } else {
        throw Error(error);
      }
    }
  });

  // it("should launch token to raydium following their pattern", async () => {
  //   try {
  //     console.log("=== PREPARING RAYDIUM LAUNCH ===");

  //     // Get current token state
  //     const currentState = await program.account.tokenState.fetch(tokenState);
  //     console.log("Current migration status:", currentState.migrated);
  //     console.log(
  //       "SOL raised:",
  //       Number(currentState.solRaised) / 1_000_000_000
  //     );

  //     // Determine token order (token_0 must be < token_1)
  //     const token0 =
  //       tokenMint.publicKey.toBuffer().compare(SOL_MINT.toBuffer()) < 0
  //         ? tokenMint.publicKey
  //         : SOL_MINT;
  //     const token1 =
  //       tokenMint.publicKey.toBuffer().compare(SOL_MINT.toBuffer()) < 0
  //         ? SOL_MINT
  //         : tokenMint.publicKey;

  //     console.log("Token 0 (smaller):", token0.toBase58());
  //     console.log("Token 1 (larger):", token1.toBase58());

  //     // Derive all necessary addresses following Raydium's pattern
  //     const [authority] = await getAuthAddress(RAYDIUM_CPMM_PROGRAM_ID);
  //     const [poolAddress] = await getPoolAddress(
  //       AMM_CONFIG,
  //       token0,
  //       token1,
  //       RAYDIUM_CPMM_PROGRAM_ID
  //     );
  //     const [lpMintAddress] = await getPoolLpMintAddress(
  //       poolAddress,
  //       RAYDIUM_CPMM_PROGRAM_ID
  //     );
  //     const [vault0] = await getPoolVaultAddress(
  //       poolAddress,
  //       token0,
  //       RAYDIUM_CPMM_PROGRAM_ID
  //     );
  //     const [vault1] = await getPoolVaultAddress(
  //       poolAddress,
  //       token1,
  //       RAYDIUM_CPMM_PROGRAM_ID
  //     );
  //     const [observationAddress] = await getObservationAddress(
  //       poolAddress,
  //       RAYDIUM_CPMM_PROGRAM_ID
  //     );

  //     // Get creator token accounts
  //     const creatorToken0 = getAssociatedTokenAddressSync(
  //       token0,
  //       user_1_wallet.publicKey,
  //       false,
  //       token0.equals(tokenMint.publicKey) ? TOKEN_PROGRAM_ID : TOKEN_PROGRAM_ID
  //     );

  //     const creatorToken1 = getAssociatedTokenAddressSync(
  //       token1,
  //       user_1_wallet.publicKey,
  //       false,
  //       token1.equals(tokenMint.publicKey) ? TOKEN_PROGRAM_ID : TOKEN_PROGRAM_ID
  //     );

  //     // Creator LP token account (will be created by Raydium)
  //     const [creatorLpToken] = await PublicKey.findProgramAddressSync(
  //       [
  //         user_1_wallet.publicKey.toBuffer(),
  //         TOKEN_PROGRAM_ID.toBuffer(),
  //         lpMintAddress.toBuffer()
  //       ],
  //       ASSOCIATED_TOKEN_PROGRAM_ID
  //     );

  //     console.log("=== DERIVED ADDRESSES ===");
  //     console.log("Pool:", poolAddress.toBase58());
  //     console.log("LP Mint:", lpMintAddress.toBase58());
  //     console.log("Token 0 Vault:", vault0.toBase58());
  //     console.log("Token 1 Vault:", vault1.toBase58());

  //     // Launch amounts - adjust these based on your tokenomics
  //     const initAmount0 = token0.equals(tokenMint.publicKey)
  //       ? new BN(100_000_000_000_000) // Token amount (with decimals)
  //       : new BN(50_000_000_000); // SOL amount in lamports

  //     const initAmount1 = token1.equals(tokenMint.publicKey)
  //       ? new BN(100_000_000_000_000) // Token amount (with decimals)
  //       : new BN(50_000_000_000); // SOL amount in lamports

  //     console.log("=== LAUNCHING TO RAYDIUM ===");
  //     console.log("Init Amount 0:", initAmount0.toString());
  //     console.log("Init Amount 1:", initAmount1.toString());

  //     const launchTx = await program.methods
  //       .migrateToRaydium(
  //         null // open_time - current timestamp
  //       )
  //       .accounts({
  //         cpSwapProgram: RAYDIUM_CPMM_PROGRAM_ID,
  //         creator: user_1_wallet.publicKey,
  //         ammConfig: AMM_CONFIG_25BPS,
  //         authority: authority,
  //         poolState: poolAddress,
  //         token0Mint: token0,
  //         token1Mint: token1,
  //         lpMint: lpMintAddress,
  //         creatorToken0: creatorToken0,
  //         creatorToken1: creatorToken1,
  //         creatorLpToken: creatorLpToken,
  //         tokenState: tokenState,
  //         token0Vault: vault0,
  //         token1Vault: vault1,
  //         createPoolFee: CREATE_POOL_FEE_RECEIVER,
  //         observationState: observationAddress,
  //         tokenProgram: TOKEN_PROGRAM_ID,
  //         token0Program: TOKEN_PROGRAM_ID,
  //         token1Program: TOKEN_PROGRAM_ID,
  //         associatedTokenProgram: ASSOCIATED_TOKEN_PROGRAM_ID,
  //         systemProgram: SystemProgram.programId,
  //         rent: SYSVAR_RENT_PUBKEY
  //       })

  //       .signers([user_1_wallet])
  //       .rpc();

  //     console.log("🚀 Launch transaction signature:", launchTx);

  //     // Verify migration status
  //     const finalState = await program.account.tokenState.fetch(tokenState);
  //     console.log("=== POST-LAUNCH STATE ===");
  //     console.log("✅ Migration completed:", finalState.migrated);
  //     console.log(
  //       "⏰ Migration timestamp:",
  //       new Date(finalState.migrationTimestamp * 1000).toISOString()
  //     );

  //     // Get pool info
  //     const poolAccountInfo = await anchor
  //       .getProvider()
  //       .connection.getAccountInfo(poolAddress);
  //     if (poolAccountInfo) {
  //       console.log("✅ Pool created successfully");
  //       console.log("Pool data length:", poolAccountInfo.data.length);
  //     }
  //   } catch (error) {
  //     console.log("❌ Launch Failed:", error);
  //     if (error.logs) {
  //       console.log("Error logs:", error.logs);
  //     }
  //     throw error;
  //   }
  // });

  it.skip("should fetch token state only", async () => {
    const tokenMintAddress = "4XozFuD6kdZDqEG6PoxnASkYd1Hw5WcPwWXycd9hjnew";

    let [token_state, _] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("token_state"),
        new anchor.web3.PublicKey(tokenMintAddress).toBytes(),
      ],
      program.programId
    );

    try {
      const tokenStateAccount = await program.account.tokenState.fetch(
        token_state
      );

      console.log(tokenStateAccount);

      console.log("=== QUICK TOKEN STATE CHECK ===");
      console.log("✅ Token State exists!");
      console.log(
        "📊 Tokens Sold:",
        Number(tokenStateAccount.tokensSold) / 1_000_000_000,
        "tokens"
      );
      console.log(
        "💰 SOL Raised:",
        Number(tokenStateAccount.solRaised) / 1_000_000_000,
        "SOL"
      );
      console.log(
        "💎 Total Supply:",
        Number(tokenStateAccount.totalSupply).toLocaleString(),
        "tokens"
      );
      console.log(
        "🏷️  Initial Price:",
        tokenStateAccount.initialPricePerToken.toString(),
        "lamports per token"
      );
    } catch (error) {
      console.log("❌ Token State not found or error:", error.message);
      if (error.logs) {
        throw Error(error.logs);
      } else {
        throw Error(error);
      }
    }
  });

  it.skip("should launch token to raydium: ", async () => {
    try {
      // Determine token order (token_0 must be < token_1)
      let [token_state, _] = anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("token_state"),
          new anchor.web3.PublicKey(tokenMint.publicKey).toBytes(),
        ],
        program.programId
      );

      let token_vault = await getOrCreateAssociatedTokenAccount(
        anchor.getProvider().connection,
        user_1_wallet,
        tokenMint.publicKey,
        token_state,
        true
      );
      let wsol_token_vault = await getOrCreateAssociatedTokenAccount(
        anchor.getProvider().connection,
        user_1_wallet,
        WSOL_MINT,
        token_state,
        true
      );
      const token0Mint =
        tokenMint.publicKey.toBuffer().compare(WSOL_MINT.toBuffer()) < 0
          ? tokenMint.publicKey
          : WSOL_MINT;
      const token1Mint =
        tokenMint.publicKey.toBuffer().compare(WSOL_MINT.toBuffer()) < 0
          ? WSOL_MINT
          : tokenMint.publicKey;

      const [authority] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("amm_authority")],
        RAYDIUM_CPMM_PROGRAM_ID
      );

      const [poolState] = anchor.web3.PublicKey.findProgramAddressSync(
        [
          Buffer.from("pool"),
          AMM_CONFIG_25BPS.toBytes(),
          token0Mint.toBytes(),
          token1Mint.toBytes(),
        ],
        RAYDIUM_CPMM_PROGRAM_ID
      );

      const [lpMint] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("pool_lp_mint"), poolState.toBytes()],
        RAYDIUM_CPMM_PROGRAM_ID
      );

      const [token0Vault] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("pool_vault"), poolState.toBytes(), token0Mint.toBytes()],
        RAYDIUM_CPMM_PROGRAM_ID
      );

      const [token1Vault] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("pool_vault"), poolState.toBytes(), token1Mint.toBytes()],
        RAYDIUM_CPMM_PROGRAM_ID
      );

      const [observationState] = anchor.web3.PublicKey.findProgramAddressSync(
        [Buffer.from("observation"), poolState.toBytes()],
        RAYDIUM_CPMM_PROGRAM_ID
      );

      const creatorToken0 = getAssociatedTokenAddressSync(
        token0Mint,
        user_1_wallet.publicKey,
        false,
        TOKEN_PROGRAM_ID
      );
      const creatorToken1 = getAssociatedTokenAddressSync(
        token1Mint,
        user_1_wallet.publicKey,
        false,
        TOKEN_PROGRAM_ID
      );
      const creatorLpToken = getAssociatedTokenAddressSync(
        lpMint,
        user_1_wallet.publicKey,
        false,
        TOKEN_PROGRAM_ID
      );

      const tx = await program.methods
        .migrateToRaydium({ tokenMint: tokenMint.publicKey, time: null })
        .accounts({
          ammConfig: AMM_CONFIG_25BPS,
          creator: user_1_wallet.publicKey,
          signer: user_1_wallet.publicKey,
          creatorLpToken: creatorLpToken,
          creatorToken0: creatorToken0,
          creatorToken1: creatorToken1,
          token0Mint,
          token1Mint,
          token0Program: TOKEN_PROGRAM_ID,
          token1Program: TOKEN_PROGRAM_ID,
          tokenVault: token_vault.address,
          wsolMint: WSOL_MINT,

          // @ts-ignore
          tokenState: token_state,
        })
        .signers([user_1_wallet])
        .rpc();
      console.log("Your transaction signature: ", tx);
    } catch (error) {
      console.log("❌Launch Failed:", error);
      console.log(error.logs);
      if (error.logs) {
        throw Error(error.logs);
      } else {
        throw Error(error);
      }
    }
  });
});
