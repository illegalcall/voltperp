"use client";

import Link from "next/link";

export default function Home() {
  return (
    <div className="flex flex-col items-center justify-center min-h-[calc(100vh-8rem)] px-4">
      {/* Hero */}
      <div className="text-center max-w-2xl mx-auto">
        <div className="mb-6">
          <span className="inline-flex items-center gap-2 px-3 py-1 rounded-full bg-surface-2 border border-surface-4 text-xs text-gray-400">
            <span className="w-1.5 h-1.5 rounded-full bg-volt-400 animate-pulse" />
            Live on Solana Devnet
          </span>
        </div>

        <h1 className="text-5xl sm:text-6xl font-bold mb-4 leading-tight">
          Trade Perps on{" "}
          <span className="bg-gradient-to-r from-volt-400 to-accent bg-clip-text text-transparent">
            Solana
          </span>
        </h1>

        <p className="text-lg text-gray-400 mb-8 max-w-lg mx-auto leading-relaxed">
          Up to 10x leverage on SOL, ETH, and BTC perpetual futures.
          Non-custodial. Lightning fast. Powered by Solana.
        </p>

        <div className="flex flex-col sm:flex-row gap-3 justify-center">
          <Link
            href="/trade"
            className="inline-flex items-center justify-center gap-2 px-8 py-3.5 rounded-xl bg-gradient-to-r from-volt-500 to-volt-600 text-white font-semibold text-base hover:from-volt-400 hover:to-volt-500 transition-all shadow-lg shadow-volt-600/20"
          >
            Start Trading
            <svg
              className="w-4 h-4"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M13 7l5 5m0 0l-5 5m5-5H6"
              />
            </svg>
          </Link>
          <Link
            href="/portfolio"
            className="inline-flex items-center justify-center px-8 py-3.5 rounded-xl bg-surface-2 border border-surface-4 text-gray-300 font-semibold text-base hover:bg-surface-3 transition-all"
          >
            View Portfolio
          </Link>
        </div>
      </div>

      {/* Stats */}
      <div className="mt-20 grid grid-cols-3 gap-8 sm:gap-16 text-center">
        <div>
          <div className="text-2xl sm:text-3xl font-bold text-white font-mono">
            10x
          </div>
          <div className="text-xs text-gray-500 mt-1">Max Leverage</div>
        </div>
        <div>
          <div className="text-2xl sm:text-3xl font-bold text-white font-mono">
            3
          </div>
          <div className="text-xs text-gray-500 mt-1">Markets</div>
        </div>
        <div>
          <div className="text-2xl sm:text-3xl font-bold text-white font-mono">
            ~400ms
          </div>
          <div className="text-xs text-gray-500 mt-1">Settlement</div>
        </div>
      </div>

      {/* Markets Preview */}
      <div className="mt-16 w-full max-w-lg">
        <div className="bg-surface-2 rounded-xl border border-surface-4 overflow-hidden">
          <div className="px-5 py-3 border-b border-surface-4">
            <span className="text-xs font-semibold text-gray-500 uppercase tracking-wider">
              Available Markets
            </span>
          </div>
          {[
            { symbol: "SOL-PERP", name: "Solana", color: "text-volt-400" },
            { symbol: "ETH-PERP", name: "Ethereum", color: "text-blue-400" },
            { symbol: "BTC-PERP", name: "Bitcoin", color: "text-orange-400" },
          ].map((m) => (
            <Link
              key={m.symbol}
              href={`/trade?market=${m.symbol}`}
              className="flex items-center justify-between px-5 py-3.5 hover:bg-surface-3 transition-colors border-b border-surface-3/50 last:border-b-0"
            >
              <div className="flex items-center gap-3">
                <div
                  className={`w-8 h-8 rounded-full bg-surface-3 flex items-center justify-center text-xs font-bold ${m.color}`}
                >
                  {m.name[0]}
                </div>
                <div>
                  <div className="font-semibold text-white text-sm">
                    {m.symbol}
                  </div>
                  <div className="text-xs text-gray-500">{m.name}</div>
                </div>
              </div>
              <div className="text-xs text-gray-500">Up to 10x</div>
            </Link>
          ))}
        </div>
      </div>
    </div>
  );
}
