import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { NottyTerminal } from "../target/types/notty_terminal";
import admin_file from "./wallets/admin-wallet.json";
import { TOKEN_PROGRAM_ID } from "@coral-xyz/anchor/dist/cjs/utils/token";

let admin_wallet = anchor.web3.Keypair.fromSecretKey(
  new Uint8Array(admin_file)
);

describe("notty-terminal", () => {
  // Configure the client to use the local cluster.
  anchor.setProvider(anchor.AnchorProvider.env());

  const program = anchor.workspace.nottyTerminal as Program<NottyTerminal>;

  it("Is initialized!", async () => {
    // Add your test here.
    const tx = await program.methods
      .initialize({
        endMcap: new anchor.BN(20_000),
        listingFeeLamport: new anchor.BN(500_000_00),
        slope: new anchor.BN(1_000_000_0),
        startMcap: new anchor.BN(1000),
        totalSupply: new anchor.BN(10_000_000)
      })
      .accounts({
        admin: admin_wallet.publicKey
      })
      .signers([admin_wallet])
      .rpc();
    console.log("Your transaction signature", tx);
  });

  it("should create token", async () => {
    let tokenMint = new anchor.web3.Keypair();
    // Add your test here.
    const tx = await program.methods
      .createToken({
        name: "Shinobi Jenks",
        tokenSymbol: "SJK",
        tokenUri: "https://avatars.githubusercontent.com/u/94226358?v=4"
      })
      .signers([admin_wallet, tokenMint])
      .accounts({
        creator: admin_wallet.publicKey,
        creatorMint: tokenMint.publicKey,
        tokenProgram: TOKEN_PROGRAM_ID
      })
      .rpc();
    console.log("Your transaction signature", tx);
  });
});
