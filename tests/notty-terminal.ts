import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NottyTerminal } from "../target/types/notty_terminal";
import admin_file from "./wallets/admin-wallet.json";
import user_1_file from "./wallets/user-1-wallet.json";
import {
  associatedAddress,
  TOKEN_PROGRAM_ID
} from "@coral-xyz/anchor/dist/cjs/utils/token";
import {
  getAssociatedTokenAddressSync,
  getOrCreateAssociatedTokenAccount
} from "@solana/spl-token";
import { publicKey } from "@coral-xyz/anchor/dist/cjs/utils";

let admin_wallet = anchor.web3.Keypair.fromSecretKey(
  new Uint8Array(admin_file)
);
let user_1_wallet = anchor.web3.Keypair.fromSecretKey(
  new Uint8Array(user_1_file)
);

describe("notty-terminal", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.nottyTerminal as Program<NottyTerminal>;

  it.skip("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods
      .initialize({
        listingFeeLamport: new anchor.BN(50_000_000),
        slope: new anchor.BN(1)
      })
      .accounts({
        admin: admin_wallet.publicKey
      })
      .signers([admin_wallet])
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it.skip("should create token and purchase it", async () => {
    try {
      // let tokenMint = anchor.web3.Keypair.generate();
      let tokenMint = {
        publicKey: new anchor.web3.PublicKey(
          "2Y4kJ6DmfQu3ePbfgNtQaFpF1ZYX85FmqvZ156LMpLmb"
        )
      };

      console.log(tokenMint.publicKey.toBase58());
      // Add your test here.
      // const tx = await program.methods
      //   .createToken({
      //     name: "Shinobi Jenks",
      //     tokenSymbol: "SJK",
      //     tokenUri: "https://avatars.githubusercontent.com/u/94226358?v=4",
      //     endMcap: new anchor.BN(460_000_000_000), // 460 SOL (matches your metrics)
      //     startMcap: new anchor.BN(25_000_000_000), // 25 SOL (matches your metrics)
      //     totalSupply: new anchor.BN(1_000_000_000) // 1B tokens (matches your metrics)
      //   })
      //   .signers([user_1_wallet, tokenMint])
      //   .accounts({
      //     creator: user_1_wallet.publicKey,
      //     creatorMint: tokenMint.publicKey,
      //     tokenProgram: TOKEN_PROGRAM_ID
      //   })
      //   .rpc();
      // console.log("Your transaction signature", tx);

      console.log("=== BEFORE PURCHASE ===");

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
        .purchaseToken({
          amount: new anchor.BN(1_000_000_000_000),
          minAmountOut: new anchor.BN(0)
        })
        .signers([user_1_wallet])
        .accounts({
          user: user_1_wallet.publicKey,
          creatorMint: tokenMint.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenVault: token_vault.address
        })
        .rpc();

      console.log("Your transaction signature 2", tx1);
      console.log("=== AFTER PURCHASE ===");
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
      throw error.logs;
    }
  });

  it("should sell tokens", async () => {
    try {
      // let tokenMint = anchor.web3.Keypair.generate();
      let tokenMint = {
        publicKey: new anchor.web3.PublicKey(
          "2Y4kJ6DmfQu3ePbfgNtQaFpF1ZYX85FmqvZ156LMpLmb"
        )
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
          amount: new anchor.BN(1_000_000_000_000),
          minProceeds: new anchor.BN(0)
        })
        .signers([user_1_wallet])
        .accounts({
          user: user_1_wallet.publicKey,
          creatorMint: tokenMint.publicKey,
          tokenProgram: TOKEN_PROGRAM_ID,
          tokenVault: token_vault.address
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
      throw error.logs;
    }
  });

  it.skip("should fetch token state only", async () => {
    const tokenMintAddress = "CBNjiFBXSKZMkKDXeEFnHymwHyqiDqYkvaR8z6BTMbHy";

    let [token_state, _] = anchor.web3.PublicKey.findProgramAddressSync(
      [
        Buffer.from("token_state"),
        new anchor.web3.PublicKey(tokenMintAddress).toBytes()
      ],
      program.programId
    );

    try {
      const tokenStateAccount = await program.account.tokenState.fetch(
        token_state
      );

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
    }
  });
});
