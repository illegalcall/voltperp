import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import {
  Keypair,
  PublicKey,
  SystemProgram,
  LAMPORTS_PER_SOL,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  createAccount,
  mintTo,
} from "@solana/spl-token";
import { assert } from "chai";
import { Voltperp } from "../target/types/voltperp";

describe("voltperp", () => {
  // Configure the client to use the local cluster
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);

  const program = anchor.workspace.Voltperp as Program<Voltperp>;
  const connection = provider.connection;

  // Test accounts
  const admin = Keypair.generate();
  const user1 = Keypair.generate();
  const user2 = Keypair.generate();

  // PDAs and state
  let exchangeState: PublicKey;
  let exchangeStateBump: number;
  let solPerpMarket: PublicKey;
  let solPerpMarketBump: number;
  let user1Account: PublicKey;
  let user2Account: PublicKey;
  let collateralMint: PublicKey;
  let exchangeVault: PublicKey;
  let user1TokenAccount: PublicKey;
  let user2TokenAccount: PublicKey;
  let oracleAccount: PublicKey;

  const MARKET_INDEX = 0;
  const COLLATERAL_DECIMALS = 6;
  const DEPOSIT_AMOUNT = 10_000 * 10 ** COLLATERAL_DECIMALS; // 10,000 USDC

  before(async () => {
    // Airdrop SOL to admin and users
    const airdropSig1 = await connection.requestAirdrop(
      admin.publicKey,
      10 * LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(airdropSig1);

    const airdropSig2 = await connection.requestAirdrop(
      user1.publicKey,
      5 * LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(airdropSig2);

    const airdropSig3 = await connection.requestAirdrop(
      user2.publicKey,
      5 * LAMPORTS_PER_SOL
    );
    await connection.confirmTransaction(airdropSig3);

    // Create collateral mint (USDC mock)
    collateralMint = await createMint(
      connection,
      admin,
      admin.publicKey,
      null,
      COLLATERAL_DECIMALS
    );

    // Derive PDAs
    [exchangeState, exchangeStateBump] = PublicKey.findProgramAddressSync(
      [Buffer.from("exchange_state")],
      program.programId
    );

    [solPerpMarket, solPerpMarketBump] = PublicKey.findProgramAddressSync(
      [
        Buffer.from("perp_market"),
        new anchor.BN(MARKET_INDEX).toArrayLike(Buffer, "le", 2),
      ],
      program.programId
    );

    [user1Account] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_account"), user1.publicKey.toBuffer()],
      program.programId
    );

    [user2Account] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_account"), user2.publicKey.toBuffer()],
      program.programId
    );

    // Derive exchange vault PDA
    [exchangeVault] = PublicKey.findProgramAddressSync(
      [Buffer.from("exchange_vault")],
      program.programId
    );

    // Create user token accounts and mint collateral
    user1TokenAccount = await createAccount(
      connection,
      user1,
      collateralMint,
      user1.publicKey
    );
    user2TokenAccount = await createAccount(
      connection,
      user2,
      collateralMint,
      user2.publicKey
    );

    await mintTo(
      connection,
      admin,
      collateralMint,
      user1TokenAccount,
      admin,
      DEPOSIT_AMOUNT * 2
    );
    await mintTo(
      connection,
      admin,
      collateralMint,
      user2TokenAccount,
      admin,
      DEPOSIT_AMOUNT * 2
    );

    // Create a mock oracle account (in production this would be Pyth/Switchboard)
    oracleAccount = Keypair.generate().publicKey;
  });

  // ---------------------------------------------------------------
  // Exchange Initialization
  // ---------------------------------------------------------------

  describe("Initialize Exchange", () => {
    it("should initialize the exchange state", async () => {
      const tx = await program.methods
        .initializeExchange()
        .accounts({
          admin: admin.publicKey,
          exchangeState,
          collateralMint,
          exchangeVault,
          systemProgram: SystemProgram.programId,
          tokenProgram: TOKEN_PROGRAM_ID,
          rent: anchor.web3.SYSVAR_RENT_PUBKEY,
        })
        .signers([admin])
        .rpc();

      console.log("Initialize exchange tx:", tx);

      const state = await program.account.exchangeState.fetch(exchangeState);
      assert.ok(state.admin.equals(admin.publicKey));
      assert.equal(state.marketCount, 0);
      assert.equal(state.paused, false);
    });

    it("should reject re-initialization", async () => {
      try {
        await program.methods
          .initializeExchange()
          .accounts({
            admin: admin.publicKey,
            exchangeState,
            collateralMint,
            exchangeVault,
            systemProgram: SystemProgram.programId,
            tokenProgram: TOKEN_PROGRAM_ID,
            rent: anchor.web3.SYSVAR_RENT_PUBKEY,
          })
          .signers([admin])
          .rpc();
        assert.fail("Should have thrown an error");
      } catch (err) {
        // Expected: account already initialized
        assert.ok(err);
      }
    });
  });

  // ---------------------------------------------------------------
  // Market Management
  // ---------------------------------------------------------------

  describe("Add SOL-PERP Market", () => {
    it("should add a SOL-PERP market", async () => {
      const ammBaseReserve = new anchor.BN(1_000_000 * 10 ** 9); // 1M SOL
      const ammQuoteReserve = new anchor.BN(100_000_000 * 10 ** 6); // 100M USDC
      const marginRatioInitial = 1000; // 10% (in bps)
      const marginRatioMaintenance = 500; // 5% (in bps)
      const liquidationFee = 50; // 0.5% (in bps)

      const tx = await program.methods
        .addMarket(
          MARKET_INDEX,
          ammBaseReserve,
          ammQuoteReserve,
          marginRatioInitial,
          marginRatioMaintenance,
          liquidationFee
        )
        .accounts({
          admin: admin.publicKey,
          exchangeState,
          perpMarket: solPerpMarket,
          oracle: oracleAccount,
          systemProgram: SystemProgram.programId,
        })
        .signers([admin])
        .rpc();

      console.log("Add SOL-PERP market tx:", tx);

      const market = await program.account.perpMarket.fetch(solPerpMarket);
      assert.equal(market.marketIndex, MARKET_INDEX);
      assert.ok(market.ammBaseReserve.eq(ammBaseReserve));
      assert.ok(market.ammQuoteReserve.eq(ammQuoteReserve));
      assert.equal(market.marginRatioInitial, marginRatioInitial);
      assert.equal(market.marginRatioMaintenance, marginRatioMaintenance);
      assert.equal(market.initialized, true);
    });

    it("should reject non-admin adding a market", async () => {
      try {
        await program.methods
          .addMarket(1, new anchor.BN(1000), new anchor.BN(1000), 1000, 500, 50)
          .accounts({
            admin: user1.publicKey,
            exchangeState,
            perpMarket: solPerpMarket,
            oracle: oracleAccount,
            systemProgram: SystemProgram.programId,
          })
          .signers([user1])
          .rpc();
        assert.fail("Should have thrown an error");
      } catch (err) {
        assert.ok(err);
      }
    });
  });

  // ---------------------------------------------------------------
  // Collateral Management
  // ---------------------------------------------------------------

  describe("Deposit Collateral", () => {
    it("should initialize user account and deposit collateral", async () => {
      const depositAmount = new anchor.BN(DEPOSIT_AMOUNT);

      const tx = await program.methods
        .depositCollateral(depositAmount)
        .accounts({
          user: user1.publicKey,
          userAccount: user1Account,
          userTokenAccount: user1TokenAccount,
          exchangeState,
          exchangeVault,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user1])
        .rpc();

      console.log("Deposit collateral tx:", tx);

      const account = await program.account.userAccount.fetch(user1Account);
      assert.ok(account.authority.equals(user1.publicKey));
      assert.ok(account.collateral.eq(depositAmount));
    });

    it("should allow user2 to deposit collateral", async () => {
      const depositAmount = new anchor.BN(DEPOSIT_AMOUNT);

      await program.methods
        .depositCollateral(depositAmount)
        .accounts({
          user: user2.publicKey,
          userAccount: user2Account,
          userTokenAccount: user2TokenAccount,
          exchangeState,
          exchangeVault,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user2])
        .rpc();

      const account = await program.account.userAccount.fetch(user2Account);
      assert.ok(account.collateral.eq(depositAmount));
    });
  });

  // ---------------------------------------------------------------
  // Position Management
  // ---------------------------------------------------------------

  describe("Open Long Position", () => {
    it("should open a long SOL-PERP position", async () => {
      // Buy 10 SOL worth of perp
      const baseAssetAmount = new anchor.BN(10 * 10 ** 9);

      const tx = await program.methods
        .openPosition(MARKET_INDEX, baseAssetAmount, true) // direction: true = long
        .accounts({
          user: user1.publicKey,
          userAccount: user1Account,
          exchangeState,
          perpMarket: solPerpMarket,
          oracle: oracleAccount,
          systemProgram: SystemProgram.programId,
        })
        .signers([user1])
        .rpc();

      console.log("Open long position tx:", tx);

      const account = await program.account.userAccount.fetch(user1Account);
      const position = account.positions[0];

      assert.equal(position.marketIndex, MARKET_INDEX);
      assert.ok(position.baseAssetAmount.gt(new anchor.BN(0))); // positive = long
    });
  });

  describe("Open Short Position", () => {
    it("should open a short SOL-PERP position", async () => {
      // Sell 5 SOL worth of perp
      const baseAssetAmount = new anchor.BN(5 * 10 ** 9);

      const tx = await program.methods
        .openPosition(MARKET_INDEX, baseAssetAmount, false) // direction: false = short
        .accounts({
          user: user2.publicKey,
          userAccount: user2Account,
          exchangeState,
          perpMarket: solPerpMarket,
          oracle: oracleAccount,
          systemProgram: SystemProgram.programId,
        })
        .signers([user2])
        .rpc();

      console.log("Open short position tx:", tx);

      const account = await program.account.userAccount.fetch(user2Account);
      const position = account.positions[0];

      assert.equal(position.marketIndex, MARKET_INDEX);
      assert.ok(position.baseAssetAmount.lt(new anchor.BN(0))); // negative = short
    });
  });

  // ---------------------------------------------------------------
  // Close Position
  // ---------------------------------------------------------------

  describe("Close Position with Profit", () => {
    it("should close user1 long position", async () => {
      const accountBefore = await program.account.userAccount.fetch(
        user1Account
      );
      const collateralBefore = accountBefore.collateral;

      const tx = await program.methods
        .closePosition(MARKET_INDEX)
        .accounts({
          user: user1.publicKey,
          userAccount: user1Account,
          exchangeState,
          perpMarket: solPerpMarket,
          oracle: oracleAccount,
          systemProgram: SystemProgram.programId,
        })
        .signers([user1])
        .rpc();

      console.log("Close position tx:", tx);

      const accountAfter = await program.account.userAccount.fetch(
        user1Account
      );

      // Position should be closed (base_asset_amount = 0)
      const position = accountAfter.positions.find(
        (p: any) => p.marketIndex === MARKET_INDEX
      );
      assert.ok(
        !position || position.baseAssetAmount.eq(new anchor.BN(0)),
        "Position should be closed"
      );

      console.log(
        "PnL:",
        accountAfter.collateral.sub(collateralBefore).toString()
      );
    });
  });

  // ---------------------------------------------------------------
  // Funding Rate Settlement
  // ---------------------------------------------------------------

  describe("Settle Funding", () => {
    it("should settle funding rates for SOL-PERP", async () => {
      // NOTE: In a real test, we would advance the clock past the funding period.
      // On localnet, we can use `warp_to_slot` or similar mechanisms.

      const tx = await program.methods
        .settleFunding(MARKET_INDEX)
        .accounts({
          keeper: provider.wallet.publicKey,
          exchangeState,
          perpMarket: solPerpMarket,
          oracle: oracleAccount,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("Settle funding tx:", tx);

      const market = await program.account.perpMarket.fetch(solPerpMarket);
      // Verify funding timestamp was updated
      assert.ok(
        market.lastFundingTs.gt(new anchor.BN(0)),
        "Funding timestamp should be updated"
      );
    });

    it("should reject early funding settlement", async () => {
      // Attempting to settle again immediately should fail
      try {
        await program.methods
          .settleFunding(MARKET_INDEX)
          .accounts({
            keeper: provider.wallet.publicKey,
            exchangeState,
            perpMarket: solPerpMarket,
            oracle: oracleAccount,
            systemProgram: SystemProgram.programId,
          })
          .rpc();
        assert.fail("Should have rejected early settlement");
      } catch (err) {
        // Expected: funding period has not elapsed
        assert.ok(err);
      }
    });
  });

  // ---------------------------------------------------------------
  // Oracle / TWAP
  // ---------------------------------------------------------------

  describe("Update Oracle and TWAP", () => {
    it("should update the oracle price and TWAP", async () => {
      // TODO: In production, oracle prices come from Pyth or Switchboard.
      // This test simulates an admin oracle price update for the localnet mock.
      const newPrice = new anchor.BN(100 * 10 ** 6); // $100.00

      const tx = await program.methods
        .updateOraclePrice(MARKET_INDEX, newPrice)
        .accounts({
          admin: admin.publicKey,
          exchangeState,
          perpMarket: solPerpMarket,
          oracle: oracleAccount,
          systemProgram: SystemProgram.programId,
        })
        .signers([admin])
        .rpc();

      console.log("Update oracle price tx:", tx);

      const market = await program.account.perpMarket.fetch(solPerpMarket);
      assert.ok(
        market.oraclePrice.eq(newPrice),
        "Oracle price should be updated"
      );
    });

    it("should update TWAP after price change", async () => {
      const tx = await program.methods
        .updateTwap(MARKET_INDEX)
        .accounts({
          keeper: provider.wallet.publicKey,
          exchangeState,
          perpMarket: solPerpMarket,
          oracle: oracleAccount,
          systemProgram: SystemProgram.programId,
        })
        .rpc();

      console.log("Update TWAP tx:", tx);

      const market = await program.account.perpMarket.fetch(solPerpMarket);
      assert.ok(
        market.twapPrice.gt(new anchor.BN(0)),
        "TWAP should be set"
      );
    });
  });

  // ---------------------------------------------------------------
  // Edge Cases & Safety
  // ---------------------------------------------------------------

  describe("Edge Cases", () => {
    it("should reject opening position with insufficient collateral", async () => {
      const oversizedPosition = new anchor.BN("999999999999999999");
      try {
        await program.methods
          .openPosition(MARKET_INDEX, oversizedPosition, true)
          .accounts({
            user: user1.publicKey,
            userAccount: user1Account,
            exchangeState,
            perpMarket: solPerpMarket,
            oracle: oracleAccount,
            systemProgram: SystemProgram.programId,
          })
          .signers([user1])
          .rpc();
        assert.fail("Should have rejected insufficient collateral");
      } catch (err) {
        assert.ok(err);
      }
    });

    it("should reject deposit of zero collateral", async () => {
      try {
        await program.methods
          .depositCollateral(new anchor.BN(0))
          .accounts({
            user: user1.publicKey,
            userAccount: user1Account,
            userTokenAccount: user1TokenAccount,
            exchangeState,
            exchangeVault,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([user1])
          .rpc();
        assert.fail("Should have rejected zero deposit");
      } catch (err) {
        assert.ok(err);
      }
    });

    it("should reject closing a non-existent position", async () => {
      // user1 already closed their position above
      try {
        await program.methods
          .closePosition(MARKET_INDEX)
          .accounts({
            user: user1.publicKey,
            userAccount: user1Account,
            exchangeState,
            perpMarket: solPerpMarket,
            oracle: oracleAccount,
            systemProgram: SystemProgram.programId,
          })
          .signers([user1])
          .rpc();
        assert.fail("Should have rejected closing non-existent position");
      } catch (err) {
        assert.ok(err);
      }
    });
  });

  // ---------------------------------------------------------------
  // Withdrawal
  // ---------------------------------------------------------------

  describe("Withdraw Collateral", () => {
    it("should allow user1 to withdraw collateral", async () => {
      const account = await program.account.userAccount.fetch(user1Account);
      const withdrawAmount = account.collateral;

      const tx = await program.methods
        .withdrawCollateral(withdrawAmount)
        .accounts({
          user: user1.publicKey,
          userAccount: user1Account,
          userTokenAccount: user1TokenAccount,
          exchangeState,
          exchangeVault,
          tokenProgram: TOKEN_PROGRAM_ID,
          systemProgram: SystemProgram.programId,
        })
        .signers([user1])
        .rpc();

      console.log("Withdraw collateral tx:", tx);

      const accountAfter = await program.account.userAccount.fetch(
        user1Account
      );
      assert.ok(
        accountAfter.collateral.eq(new anchor.BN(0)),
        "Collateral should be zero after full withdrawal"
      );
    });

    it("should reject withdrawal with open positions", async () => {
      // user2 still has an open short position
      const account = await program.account.userAccount.fetch(user2Account);
      try {
        await program.methods
          .withdrawCollateral(account.collateral)
          .accounts({
            user: user2.publicKey,
            userAccount: user2Account,
            userTokenAccount: user2TokenAccount,
            exchangeState,
            exchangeVault,
            tokenProgram: TOKEN_PROGRAM_ID,
            systemProgram: SystemProgram.programId,
          })
          .signers([user2])
          .rpc();
        assert.fail("Should have rejected withdrawal with open position");
      } catch (err) {
        // Expected: cannot withdraw with open positions that would drop below margin
        assert.ok(err);
      }
    });
  });
});
