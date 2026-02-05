"use client";

import { useCallback, useEffect, useState } from "react";
import { useConnection, useWallet } from "@solana/wallet-adapter-react";
import { PublicKey, SystemProgram, SYSVAR_RENT_PUBKEY } from "@solana/web3.js";
import {
  Program,
  AnchorProvider,
  BN,
  type IdlAccounts,
} from "@coral-xyz/anchor";
import {
  TOKEN_PROGRAM_ID,
  ASSOCIATED_TOKEN_PROGRAM_ID,
  getAssociatedTokenAddress,
} from "@solana/spl-token";
import { VOLTPERP_PROGRAM_ID, USDC_MINT, COMMITMENT } from "@/lib/constants";

// ---------- IDL type placeholders ----------
// In production these come from anchor's generated IDL.
// We define minimal shapes so the hook compiles standalone.

interface VoltPerpIDL {
  version: string;
  name: string;
  instructions: unknown[];
  accounts: unknown[];
}

interface UserAccount {
  authority: PublicKey;
  collateral: BN;
  positions: PositionData[];
  bump: number;
}

export interface PositionData {
  market: string;
  side: "long" | "short";
  size: BN;
  entryPrice: BN;
  leverage: number;
  timestamp: BN;
  unrealizedPnl: BN;
  liquidationPrice: BN;
}

// ---------- hook ----------

export function useVoltPerp() {
  const { connection } = useConnection();
  const wallet = useWallet();
  const { publicKey, signTransaction, signAllTransactions } = wallet;

  const [program, setProgram] = useState<Program | null>(null);
  const [userAccount, setUserAccount] = useState<UserAccount | null>(null);
  const [positions, setPositions] = useState<PositionData[]>([]);
  const [collateral, setCollateral] = useState<number>(0);
  const [unrealizedPnl, setUnrealizedPnl] = useState<number>(0);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // ---------- provider / program bootstrap ----------

  useEffect(() => {
    if (!publicKey || !signTransaction || !signAllTransactions) {
      setProgram(null);
      return;
    }

    const provider = new AnchorProvider(
      connection,
      {
        publicKey,
        signTransaction,
        signAllTransactions,
      } as any,
      { commitment: COMMITMENT }
    );

    // In production, import the generated IDL JSON.
    // For now we create the program with a minimal IDL stub so the rest of
    // the hook signatures are correct.
    const idl: VoltPerpIDL = {
      version: "0.1.0",
      name: "voltperp",
      instructions: [],
      accounts: [],
    };

    const prog = new Program(idl as any, VOLTPERP_PROGRAM_ID, provider);
    setProgram(prog);
  }, [connection, publicKey, signTransaction, signAllTransactions]);

  // ---------- PDA helpers ----------

  const getUserAccountPDA = useCallback(async () => {
    if (!publicKey) throw new Error("Wallet not connected");
    const [pda] = PublicKey.findProgramAddressSync(
      [Buffer.from("user_account"), publicKey.toBuffer()],
      VOLTPERP_PROGRAM_ID
    );
    return pda;
  }, [publicKey]);

  const getVaultPDA = useCallback(async () => {
    const [pda] = PublicKey.findProgramAddressSync(
      [Buffer.from("vault")],
      VOLTPERP_PROGRAM_ID
    );
    return pda;
  }, []);

  // ---------- fetch account data ----------

  const fetchUserAccount = useCallback(async () => {
    if (!program || !publicKey) return;
    try {
      const userPda = await getUserAccountPDA();
      const account = await (program.account as any).userAccount.fetch(
        userPda
      );
      setUserAccount(account as UserAccount);
      setCollateral(account.collateral.toNumber() / 1e6); // USDC 6 decimals
      setPositions(account.positions ?? []);

      const totalPnl = (account.positions ?? []).reduce(
        (sum: number, p: PositionData) => sum + p.unrealizedPnl.toNumber(),
        0
      );
      setUnrealizedPnl(totalPnl / 1e6);
    } catch (e: any) {
      if (e?.message?.includes("Account does not exist")) {
        setUserAccount(null);
        setCollateral(0);
        setPositions([]);
        setUnrealizedPnl(0);
      } else {
        console.error("Failed to fetch user account:", e);
        setError(e.message);
      }
    }
  }, [program, publicKey, getUserAccountPDA]);

  useEffect(() => {
    fetchUserAccount();
    const id = setInterval(fetchUserAccount, 10_000);
    return () => clearInterval(id);
  }, [fetchUserAccount]);

  // ---------- instructions ----------

  const initializeAccount = useCallback(async () => {
    if (!program || !publicKey) throw new Error("Wallet not connected");
    setLoading(true);
    setError(null);
    try {
      const userPda = await getUserAccountPDA();
      const tx = await (program.methods as any)
        .initializeUser()
        .accounts({
          userAccount: userPda,
          authority: publicKey,
          systemProgram: SystemProgram.programId,
        })
        .rpc();
      await connection.confirmTransaction(tx, COMMITMENT);
      await fetchUserAccount();
      return tx;
    } catch (e: any) {
      setError(e.message);
      throw e;
    } finally {
      setLoading(false);
    }
  }, [program, publicKey, connection, getUserAccountPDA, fetchUserAccount]);

  const deposit = useCallback(
    async (amountUsdc: number) => {
      if (!program || !publicKey) throw new Error("Wallet not connected");
      setLoading(true);
      setError(null);
      try {
        const userPda = await getUserAccountPDA();
        const vaultPda = await getVaultPDA();
        const userAta = await getAssociatedTokenAddress(USDC_MINT, publicKey);
        const vaultAta = await getAssociatedTokenAddress(
          USDC_MINT,
          vaultPda,
          true
        );

        const amount = new BN(Math.floor(amountUsdc * 1e6));

        const tx = await (program.methods as any)
          .deposit(amount)
          .accounts({
            userAccount: userPda,
            authority: publicKey,
            userTokenAccount: userAta,
            vaultTokenAccount: vaultAta,
            vault: vaultPda,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();

        await connection.confirmTransaction(tx, COMMITMENT);
        await fetchUserAccount();
        return tx;
      } catch (e: any) {
        setError(e.message);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [
      program,
      publicKey,
      connection,
      getUserAccountPDA,
      getVaultPDA,
      fetchUserAccount,
    ]
  );

  const withdraw = useCallback(
    async (amountUsdc: number) => {
      if (!program || !publicKey) throw new Error("Wallet not connected");
      setLoading(true);
      setError(null);
      try {
        const userPda = await getUserAccountPDA();
        const vaultPda = await getVaultPDA();
        const userAta = await getAssociatedTokenAddress(USDC_MINT, publicKey);
        const vaultAta = await getAssociatedTokenAddress(
          USDC_MINT,
          vaultPda,
          true
        );

        const amount = new BN(Math.floor(amountUsdc * 1e6));

        const tx = await (program.methods as any)
          .withdraw(amount)
          .accounts({
            userAccount: userPda,
            authority: publicKey,
            userTokenAccount: userAta,
            vaultTokenAccount: vaultAta,
            vault: vaultPda,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();

        await connection.confirmTransaction(tx, COMMITMENT);
        await fetchUserAccount();
        return tx;
      } catch (e: any) {
        setError(e.message);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [
      program,
      publicKey,
      connection,
      getUserAccountPDA,
      getVaultPDA,
      fetchUserAccount,
    ]
  );

  const openPosition = useCallback(
    async (
      market: string,
      side: "long" | "short",
      size: number,
      leverage: number,
      oracle: PublicKey
    ) => {
      if (!program || !publicKey) throw new Error("Wallet not connected");
      setLoading(true);
      setError(null);
      try {
        const userPda = await getUserAccountPDA();
        const vaultPda = await getVaultPDA();

        const sizeAmount = new BN(Math.floor(size * 1e6));
        const sideArg = side === "long" ? { long: {} } : { short: {} };

        const tx = await (program.methods as any)
          .openPosition(market, sideArg, sizeAmount, leverage)
          .accounts({
            userAccount: userPda,
            authority: publicKey,
            vault: vaultPda,
            oracle,
            systemProgram: SystemProgram.programId,
          })
          .rpc();

        await connection.confirmTransaction(tx, COMMITMENT);
        await fetchUserAccount();
        return tx;
      } catch (e: any) {
        setError(e.message);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [
      program,
      publicKey,
      connection,
      getUserAccountPDA,
      getVaultPDA,
      fetchUserAccount,
    ]
  );

  const closePosition = useCallback(
    async (market: string, oracle: PublicKey) => {
      if (!program || !publicKey) throw new Error("Wallet not connected");
      setLoading(true);
      setError(null);
      try {
        const userPda = await getUserAccountPDA();
        const vaultPda = await getVaultPDA();
        const userAta = await getAssociatedTokenAddress(USDC_MINT, publicKey);
        const vaultAta = await getAssociatedTokenAddress(
          USDC_MINT,
          vaultPda,
          true
        );

        const tx = await (program.methods as any)
          .closePosition(market)
          .accounts({
            userAccount: userPda,
            authority: publicKey,
            vault: vaultPda,
            userTokenAccount: userAta,
            vaultTokenAccount: vaultAta,
            oracle,
            tokenProgram: TOKEN_PROGRAM_ID,
          })
          .rpc();

        await connection.confirmTransaction(tx, COMMITMENT);
        await fetchUserAccount();
        return tx;
      } catch (e: any) {
        setError(e.message);
        throw e;
      } finally {
        setLoading(false);
      }
    },
    [
      program,
      publicKey,
      connection,
      getUserAccountPDA,
      getVaultPDA,
      fetchUserAccount,
    ]
  );

  return {
    program,
    connected: !!publicKey,
    publicKey,
    userAccount,
    positions,
    collateral,
    unrealizedPnl,
    loading,
    error,
    initializeAccount,
    deposit,
    withdraw,
    openPosition,
    closePosition,
    refreshAccount: fetchUserAccount,
  };
}
