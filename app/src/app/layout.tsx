import type { Metadata } from "next";
import { Inter } from "next/font/google";
import dynamic from "next/dynamic";
import WalletProvider from "@/components/WalletProvider";
import "./globals.css";

const inter = Inter({ subsets: ["latin"] });

// Dynamic import for wallet button to avoid SSR hydration issues
const WalletMultiButtonDynamic = dynamic(
  async () =>
    (await import("@solana/wallet-adapter-react-ui")).WalletMultiButton,
  { ssr: false }
);

export const metadata: Metadata = {
  title: "VoltPerp - Perpetual Futures on Solana",
  description:
    "Trade perpetual futures with up to 10x leverage on Solana. Fast, decentralized, non-custodial.",
  icons: {
    icon: "/favicon.ico",
  },
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en" className="dark">
      <body
        className={`${inter.className} bg-surface-0 text-white antialiased min-h-screen`}
      >
        <WalletProvider>
          <div className="flex flex-col min-h-screen">
            {/* Navbar */}
            <header className="sticky top-0 z-50 border-b border-surface-3 bg-surface-0/80 backdrop-blur-xl">
              <div className="max-w-[1440px] mx-auto px-4 h-14 flex items-center justify-between">
                <div className="flex items-center gap-6">
                  <a href="/" className="flex items-center gap-2">
                    <span className="text-xl font-bold bg-gradient-to-r from-volt-400 to-accent bg-clip-text text-transparent">
                      VoltPerp
                    </span>
                  </a>
                  <nav className="hidden md:flex items-center gap-1">
                    <a
                      href="/trade"
                      className="px-3 py-1.5 rounded-md text-sm text-gray-400 hover:text-white hover:bg-surface-2 transition-colors"
                    >
                      Trade
                    </a>
                    <a
                      href="/portfolio"
                      className="px-3 py-1.5 rounded-md text-sm text-gray-400 hover:text-white hover:bg-surface-2 transition-colors"
                    >
                      Portfolio
                    </a>
                  </nav>
                </div>

                <div className="flex items-center gap-3">
                  <div className="hidden sm:flex items-center gap-1.5 px-2.5 py-1 rounded-full bg-surface-2 border border-surface-4">
                    <div className="w-1.5 h-1.5 rounded-full bg-volt-400 animate-pulse" />
                    <span className="text-xs text-gray-400">Devnet</span>
                  </div>
                  <WalletMultiButtonDynamic />
                </div>
              </div>
            </header>

            {/* Main content */}
            <main className="flex-1">{children}</main>

            {/* Footer */}
            <footer className="border-t border-surface-3 py-4">
              <div className="max-w-[1440px] mx-auto px-4 flex items-center justify-between text-xs text-gray-600">
                <span>VoltPerp Protocol v0.1.0</span>
                <div className="flex gap-4">
                  <a href="#" className="hover:text-gray-400 transition-colors">
                    Docs
                  </a>
                  <a href="#" className="hover:text-gray-400 transition-colors">
                    GitHub
                  </a>
                  <a href="#" className="hover:text-gray-400 transition-colors">
                    Discord
                  </a>
                </div>
              </div>
            </footer>
          </div>
        </WalletProvider>
      </body>
    </html>
  );
}
