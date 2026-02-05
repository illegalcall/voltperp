"use client";

import { useState } from "react";
import { useWallet } from "@solana/wallet-adapter-react";
import { useVoltPerp, type PositionData } from "@/hooks/useVoltPerp";
import PositionTable from "@/components/PositionTable";

// Mock PnL history data -- in production this comes from indexer / on-chain events
interface PnlHistoryEntry {
  id: string;
  market: string;
  side: "long" | "short";
  entryPrice: number;
  exitPrice: number;
  size: number;
  pnl: number;
  closedAt: string;
}

const MOCK_PNL_HISTORY: PnlHistoryEntry[] = [
  {
    id: "1",
    market: "SOL-PERP",
    side: "long",
    entryPrice: 142.5,
    exitPrice: 148.2,
    size: 500,
    pnl: 20.0,
    closedAt: "2025-03-19T14:23:00Z",
  },
  {
    id: "2",
    market: "ETH-PERP",
    side: "short",
    entryPrice: 3280.0,
    exitPrice: 3245.0,
    size: 1000,
    pnl: 10.67,
    closedAt: "2025-03-18T09:15:00Z",
  },
  {
    id: "3",
    market: "BTC-PERP",
    side: "long",
    entryPrice: 68200.0,
    exitPrice: 67500.0,
    size: 200,
    pnl: -2.05,
    closedAt: "2025-03-17T21:42:00Z",
  },
  {
    id: "4",
    market: "SOL-PERP",
    side: "short",
    entryPrice: 155.0,
    exitPrice: 152.1,
    size: 750,
    pnl: 14.03,
    closedAt: "2025-03-16T16:30:00Z",
  },
  {
    id: "5",
    market: "ETH-PERP",
    side: "long",
    entryPrice: 3190.0,
    exitPrice: 3150.0,
    size: 500,
    pnl: -6.27,
    closedAt: "2025-03-15T11:00:00Z",
  },
];

type Tab = "positions" | "history";

export default function PortfolioPage() {
  const { connected, publicKey } = useWallet();
  const { collateral, unrealizedPnl, positions } = useVoltPerp();
  const [activeTab, setActiveTab] = useState<Tab>("positions");

  const totalRealizedPnl = MOCK_PNL_HISTORY.reduce(
    (sum, e) => sum + e.pnl,
    0
  );
  const accountValue = collateral + unrealizedPnl;

  if (!connected) {
    return (
      <div className="max-w-[1200px] mx-auto px-4 py-16">
        <div className="text-center">
          <div className="w-16 h-16 rounded-2xl bg-surface-2 border border-surface-4 flex items-center justify-center mx-auto mb-4">
            <svg
              className="w-8 h-8 text-gray-600"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M21 12a2.25 2.25 0 00-2.25-2.25H15a3 3 0 11-6 0H5.25A2.25 2.25 0 003 12m18 0v6a2.25 2.25 0 01-2.25 2.25H5.25A2.25 2.25 0 013 18v-6m18 0V9M3 12V9m18 0a2.25 2.25 0 00-2.25-2.25H5.25A2.25 2.25 0 003 9m18 0V6a2.25 2.25 0 00-2.25-2.25H5.25A2.25 2.25 0 003 6v3"
              />
            </svg>
          </div>
          <h2 className="text-xl font-bold text-white mb-2">
            Connect Your Wallet
          </h2>
          <p className="text-gray-500 text-sm max-w-sm mx-auto">
            Connect a Solana wallet to view your portfolio, open positions, and
            PnL history.
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-[1200px] mx-auto px-4 py-6">
      {/* Page Header */}
      <div className="mb-6">
        <h1 className="text-2xl font-bold text-white mb-1">Portfolio</h1>
        <p className="text-sm text-gray-500 font-mono">
          {publicKey?.toBase58().slice(0, 8)}...
          {publicKey?.toBase58().slice(-6)}
        </p>
      </div>

      {/* Account Overview Cards */}
      <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
        <div className="bg-surface-2 rounded-xl border border-surface-4 p-5">
          <span className="text-xs text-gray-500">Account Value</span>
          <div className="text-2xl font-bold font-mono text-white mt-1">
            ${accountValue.toFixed(2)}
          </div>
        </div>
        <div className="bg-surface-2 rounded-xl border border-surface-4 p-5">
          <span className="text-xs text-gray-500">Collateral</span>
          <div className="text-2xl font-bold font-mono text-white mt-1">
            ${collateral.toFixed(2)}
          </div>
        </div>
        <div className="bg-surface-2 rounded-xl border border-surface-4 p-5">
          <span className="text-xs text-gray-500">Unrealized PnL</span>
          <div
            className={`text-2xl font-bold font-mono mt-1 ${
              unrealizedPnl >= 0 ? "text-long" : "text-short"
            }`}
          >
            {unrealizedPnl >= 0 ? "+" : ""}${unrealizedPnl.toFixed(2)}
          </div>
        </div>
        <div className="bg-surface-2 rounded-xl border border-surface-4 p-5">
          <span className="text-xs text-gray-500">Realized PnL (Total)</span>
          <div
            className={`text-2xl font-bold font-mono mt-1 ${
              totalRealizedPnl >= 0 ? "text-long" : "text-short"
            }`}
          >
            {totalRealizedPnl >= 0 ? "+" : ""}${totalRealizedPnl.toFixed(2)}
          </div>
        </div>
      </div>

      {/* Tabs */}
      <div className="flex items-center gap-1 mb-4 bg-surface-1 rounded-lg p-1 w-fit">
        <button
          onClick={() => setActiveTab("positions")}
          className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
            activeTab === "positions"
              ? "bg-surface-3 text-white"
              : "text-gray-500 hover:text-gray-300"
          }`}
        >
          Open Positions
          {positions.length > 0 && (
            <span className="ml-1.5 bg-surface-4 text-gray-300 text-xs px-1.5 py-0.5 rounded-full">
              {positions.length}
            </span>
          )}
        </button>
        <button
          onClick={() => setActiveTab("history")}
          className={`px-4 py-2 rounded-md text-sm font-medium transition-colors ${
            activeTab === "history"
              ? "bg-surface-3 text-white"
              : "text-gray-500 hover:text-gray-300"
          }`}
        >
          Trade History
        </button>
      </div>

      {/* Content */}
      {activeTab === "positions" ? (
        <PositionTable />
      ) : (
        <div className="bg-surface-2 rounded-xl border border-surface-4 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="text-xs text-gray-500 uppercase tracking-wider border-b border-surface-4">
                  <th className="text-left px-5 py-3 font-medium">Market</th>
                  <th className="text-left px-3 py-3 font-medium">Side</th>
                  <th className="text-right px-3 py-3 font-medium">Size</th>
                  <th className="text-right px-3 py-3 font-medium">
                    Entry Price
                  </th>
                  <th className="text-right px-3 py-3 font-medium">
                    Exit Price
                  </th>
                  <th className="text-right px-3 py-3 font-medium">PnL</th>
                  <th className="text-right px-5 py-3 font-medium">
                    Closed At
                  </th>
                </tr>
              </thead>
              <tbody>
                {MOCK_PNL_HISTORY.map((entry) => (
                  <tr
                    key={entry.id}
                    className="border-b border-surface-3/50 hover:bg-surface-3/30 transition-colors"
                  >
                    <td className="px-5 py-3 font-semibold text-white">
                      {entry.market}
                    </td>
                    <td className="px-3 py-3">
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold ${
                          entry.side === "long"
                            ? "bg-long/15 text-long"
                            : "bg-short/15 text-short"
                        }`}
                      >
                        {entry.side.toUpperCase()}
                      </span>
                    </td>
                    <td className="px-3 py-3 text-right font-mono text-white">
                      ${entry.size.toFixed(2)}
                    </td>
                    <td className="px-3 py-3 text-right font-mono text-gray-300">
                      ${entry.entryPrice.toLocaleString(undefined, {
                        minimumFractionDigits: 2,
                      })}
                    </td>
                    <td className="px-3 py-3 text-right font-mono text-gray-300">
                      ${entry.exitPrice.toLocaleString(undefined, {
                        minimumFractionDigits: 2,
                      })}
                    </td>
                    <td className="px-3 py-3 text-right">
                      <span
                        className={`font-mono font-semibold ${
                          entry.pnl >= 0 ? "text-long" : "text-short"
                        }`}
                      >
                        {entry.pnl >= 0 ? "+" : ""}${entry.pnl.toFixed(2)}
                      </span>
                    </td>
                    <td className="px-5 py-3 text-right text-gray-500 text-xs">
                      {new Date(entry.closedAt).toLocaleDateString(undefined, {
                        month: "short",
                        day: "numeric",
                        hour: "2-digit",
                        minute: "2-digit",
                      })}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>

          {MOCK_PNL_HISTORY.length === 0 && (
            <div className="p-8 text-center text-gray-600 text-sm">
              No trade history yet
            </div>
          )}
        </div>
      )}
    </div>
  );
}
