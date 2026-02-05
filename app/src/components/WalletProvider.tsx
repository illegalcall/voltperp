"use client";

import { FC, ReactNode, useMemo } from "react";
import {
  ConnectionProvider,
  WalletProvider as SolanaWalletProvider,
} from "@solana/wallet-adapter-react";
import { WalletModalProvider } from "@solana/wallet-adapter-react-ui";
import {
  PhantomWalletAdapter,
  SolflareWalletAdapter,
  BackpackWalletAdapter,
} from "@solana/wallet-adapter-wallets";
import { RPC_ENDPOINT, COMMITMENT } from "@/lib/constants";

import "@solana/wallet-adapter-react-ui/styles.css";

interface Props {
  children: ReactNode;
}

const WalletProvider: FC<Props> = ({ children }) => {
  const endpoint = useMemo(() => RPC_ENDPOINT, []);

  const wallets = useMemo(
    () => [
      new PhantomWalletAdapter(),
      new SolflareWalletAdapter(),
      new BackpackWalletAdapter(),
    ],
    []
  );

  return (
    <ConnectionProvider
      endpoint={endpoint}
      config={{ commitment: COMMITMENT }}
    >
      <SolanaWalletProvider wallets={wallets} autoConnect>
        <WalletModalProvider>{children}</WalletModalProvider>
      </SolanaWalletProvider>
    </ConnectionProvider>
  );
};

export default WalletProvider;
