"use client";

import { FC } from "react";
import { useVoltPerp, type PositionData } from "@/hooks/useVoltPerp";
import { MARKETS } from "@/lib/constants";

interface PositionTableProps {
  onClosePosition?: (market: string) => void;
}

const PositionTable: FC<PositionTableProps> = ({ onClosePosition }) => {
  const { positions, closePosition, loading, connected } = useVoltPerp();

  const handleClose = async (pos: PositionData) => {
    const market = MARKETS[pos.market];
    if (!market) return;
    try {
      await closePosition(pos.market, market.oracle);
      onClosePosition?.(pos.market);
    } catch (e) {
      console.error("Failed to close position:", e);
    }
  };

  const formatPrice = (bn: any): string => {
    const val = typeof bn === "number" ? bn : bn.toNumber() / 1e6;
    return val.toLocaleString(undefined, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 2,
    });
  };

  const formatSize = (bn: any): string => {
    const val = typeof bn === "number" ? bn : bn.toNumber() / 1e6;
    return val.toLocaleString(undefined, {
      minimumFractionDigits: 2,
      maximumFractionDigits: 4,
    });
  };

  const formatPnl = (bn: any): { text: string; isPositive: boolean } => {
    const val = typeof bn === "number" ? bn : bn.toNumber() / 1e6;
    return {
      text: `${val >= 0 ? "+" : ""}$${val.toLocaleString(undefined, {
        minimumFractionDigits: 2,
        maximumFractionDigits: 2,
      })}`,
      isPositive: val >= 0,
    };
  };

  if (!connected) {
    return (
      <div className="bg-surface-2 rounded-xl border border-surface-4 p-8">
        <p className="text-center text-gray-500 text-sm">
          Connect your wallet to view positions
        </p>
      </div>
    );
  }

  return (
    <div className="bg-surface-2 rounded-xl border border-surface-4 overflow-hidden">
      <div className="px-5 py-3 border-b border-surface-4">
        <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider">
          Open Positions
          {positions.length > 0 && (
            <span className="ml-2 text-white bg-surface-4 rounded-full px-2 py-0.5 text-xs">
              {positions.length}
            </span>
          )}
        </h3>
      </div>

      {positions.length === 0 ? (
        <div className="p-8">
          <p className="text-center text-gray-600 text-sm">
            No open positions
          </p>
        </div>
      ) : (
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
                  Mark Price
                </th>
                <th className="text-right px-3 py-3 font-medium">PnL</th>
                <th className="text-right px-3 py-3 font-medium">
                  Liq. Price
                </th>
                <th className="text-right px-5 py-3 font-medium">Action</th>
              </tr>
            </thead>
            <tbody>
              {positions.map((pos, idx) => {
                const pnl = formatPnl(pos.unrealizedPnl);
                return (
                  <tr
                    key={`${pos.market}-${idx}`}
                    className="border-b border-surface-3/50 hover:bg-surface-3/30 transition-colors"
                  >
                    <td className="px-5 py-3">
                      <span className="font-semibold text-white">
                        {pos.market}
                      </span>
                    </td>
                    <td className="px-3 py-3">
                      <span
                        className={`inline-flex items-center px-2 py-0.5 rounded text-xs font-semibold ${
                          pos.side === "long"
                            ? "bg-long/15 text-long"
                            : "bg-short/15 text-short"
                        }`}
                      >
                        {pos.side === "long" ? "LONG" : "SHORT"}
                      </span>
                    </td>
                    <td className="px-3 py-3 text-right font-mono text-white">
                      ${formatSize(pos.size)}
                    </td>
                    <td className="px-3 py-3 text-right font-mono text-gray-300">
                      ${formatPrice(pos.entryPrice)}
                    </td>
                    <td className="px-3 py-3 text-right font-mono text-gray-300">
                      --
                    </td>
                    <td className="px-3 py-3 text-right">
                      <span
                        className={`font-mono font-semibold ${
                          pnl.isPositive ? "text-long" : "text-short"
                        }`}
                      >
                        {pnl.text}
                      </span>
                    </td>
                    <td className="px-3 py-3 text-right font-mono text-gray-400">
                      ${formatPrice(pos.liquidationPrice)}
                    </td>
                    <td className="px-5 py-3 text-right">
                      <button
                        onClick={() => handleClose(pos)}
                        disabled={loading}
                        className="text-xs px-3 py-1.5 rounded-md bg-surface-4 text-gray-300 hover:bg-short/20 hover:text-short transition-colors disabled:opacity-40"
                      >
                        Close
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

export default PositionTable;
