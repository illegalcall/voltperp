"use client";

import { FC, useState, useCallback } from "react";
import { useWallet } from "@solana/wallet-adapter-react";
import { useVoltPerp } from "@/hooks/useVoltPerp";
import { MARKETS, type MarketConfig } from "@/lib/constants";

interface TradeFormProps {
  selectedMarket: string;
  markPrice: number | null;
}

const TradeForm: FC<TradeFormProps> = ({ selectedMarket, markPrice }) => {
  const { connected } = useWallet();
  const { openPosition, collateral, loading, error } = useVoltPerp();

  const [side, setSide] = useState<"long" | "short">("long");
  const [size, setSize] = useState<string>("");
  const [leverage, setLeverage] = useState<number>(1);

  const market = MARKETS[selectedMarket];
  const maxLeverage = market?.maxLeverage ?? 10;

  const sizeNum = parseFloat(size) || 0;
  const notionalValue = sizeNum * leverage;
  const requiredMargin = sizeNum > 0 ? sizeNum : 0;
  const estimatedEntry = markPrice ?? 0;
  const estimatedFee = notionalValue * (market?.takerFee ?? 0.001);

  const estimatedLiqPrice = useCallback(() => {
    if (!markPrice || sizeNum <= 0) return null;
    const maintenanceMargin = market?.maintenanceMarginRatio ?? 0.05;
    if (side === "long") {
      return markPrice * (1 - (1 / leverage) + maintenanceMargin);
    } else {
      return markPrice * (1 + (1 / leverage) - maintenanceMargin);
    }
  }, [markPrice, sizeNum, leverage, side, market]);

  const handleSubmit = async () => {
    if (!connected || sizeNum <= 0 || !market) return;
    try {
      await openPosition(
        selectedMarket,
        side,
        sizeNum,
        leverage,
        market.oracle
      );
      setSize("");
      setLeverage(1);
    } catch (e) {
      console.error("Failed to open position:", e);
    }
  };

  const isValid = connected && sizeNum > 0 && requiredMargin <= collateral;

  return (
    <div className="bg-surface-2 rounded-xl p-5 border border-surface-4">
      <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
        Place Order
      </h3>

      {/* Side Toggle */}
      <div className="grid grid-cols-2 gap-1 bg-surface-1 rounded-lg p-1 mb-4">
        <button
          onClick={() => setSide("long")}
          className={`py-2.5 rounded-md text-sm font-semibold transition-all ${
            side === "long"
              ? "bg-long text-white shadow-lg shadow-long/20"
              : "text-gray-400 hover:text-gray-200"
          }`}
        >
          Long
        </button>
        <button
          onClick={() => setSide("short")}
          className={`py-2.5 rounded-md text-sm font-semibold transition-all ${
            side === "short"
              ? "bg-short text-white shadow-lg shadow-short/20"
              : "text-gray-400 hover:text-gray-200"
          }`}
        >
          Short
        </button>
      </div>

      {/* Size Input */}
      <div className="mb-4">
        <label className="block text-xs text-gray-500 mb-1.5">
          Size (USDC)
        </label>
        <div className="relative">
          <input
            type="number"
            value={size}
            onChange={(e) => setSize(e.target.value)}
            placeholder="0.00"
            min="0"
            step="0.01"
            className="w-full bg-surface-1 border border-surface-4 rounded-lg px-3 py-2.5 text-white placeholder-gray-600 focus:outline-none focus:border-accent focus:ring-1 focus:ring-accent/30 font-mono text-sm"
          />
          <span className="absolute right-3 top-1/2 -translate-y-1/2 text-xs text-gray-500">
            USDC
          </span>
        </div>
        {collateral > 0 && (
          <div className="flex gap-2 mt-2">
            {[25, 50, 75, 100].map((pct) => (
              <button
                key={pct}
                onClick={() =>
                  setSize(((collateral * pct) / 100).toFixed(2))
                }
                className="flex-1 text-xs py-1 rounded bg-surface-3 text-gray-400 hover:text-white hover:bg-surface-4 transition-colors"
              >
                {pct}%
              </button>
            ))}
          </div>
        )}
      </div>

      {/* Leverage Slider */}
      <div className="mb-5">
        <div className="flex justify-between items-center mb-1.5">
          <label className="text-xs text-gray-500">Leverage</label>
          <span className="text-sm font-semibold text-white font-mono">
            {leverage}x
          </span>
        </div>
        <input
          type="range"
          min="1"
          max={maxLeverage}
          step="1"
          value={leverage}
          onChange={(e) => setLeverage(parseInt(e.target.value))}
          className="w-full h-1.5 rounded-full appearance-none cursor-pointer bg-surface-4 accent-accent"
        />
        <div className="flex justify-between text-[10px] text-gray-600 mt-1">
          <span>1x</span>
          <span>{Math.floor(maxLeverage / 2)}x</span>
          <span>{maxLeverage}x</span>
        </div>
      </div>

      {/* Order Summary */}
      <div className="bg-surface-1 rounded-lg p-3 mb-4 space-y-2 text-xs">
        <div className="flex justify-between">
          <span className="text-gray-500">Est. Entry Price</span>
          <span className="text-white font-mono">
            {estimatedEntry > 0
              ? `$${estimatedEntry.toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 2,
                })}`
              : "--"}
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-500">Notional Value</span>
          <span className="text-white font-mono">
            ${notionalValue.toFixed(2)}
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-500">Required Margin</span>
          <span className="text-white font-mono">
            ${requiredMargin.toFixed(2)}
          </span>
        </div>
        <div className="flex justify-between">
          <span className="text-gray-500">Est. Liq. Price</span>
          <span className="text-white font-mono">
            {estimatedLiqPrice()
              ? `$${estimatedLiqPrice()!.toLocaleString(undefined, {
                  minimumFractionDigits: 2,
                  maximumFractionDigits: 2,
                })}`
              : "--"}
          </span>
        </div>
        <div className="flex justify-between border-t border-surface-3 pt-2">
          <span className="text-gray-500">Est. Fee</span>
          <span className="text-gray-400 font-mono">
            ${estimatedFee.toFixed(4)}
          </span>
        </div>
      </div>

      {/* Error */}
      {error && (
        <div className="text-short text-xs mb-3 bg-short/10 rounded-lg px-3 py-2">
          {error}
        </div>
      )}

      {/* Submit */}
      {!connected ? (
        <p className="text-center text-xs text-gray-500 py-2">
          Connect wallet to trade
        </p>
      ) : (
        <button
          onClick={handleSubmit}
          disabled={!isValid || loading}
          className={`w-full py-3 rounded-lg font-semibold text-sm transition-all disabled:opacity-40 disabled:cursor-not-allowed ${
            side === "long"
              ? "bg-long hover:bg-long/90 text-white shadow-lg shadow-long/20"
              : "bg-short hover:bg-short/90 text-white shadow-lg shadow-short/20"
          }`}
        >
          {loading
            ? "Submitting..."
            : `${side === "long" ? "Long" : "Short"} ${selectedMarket}`}
        </button>
      )}
    </div>
  );
};

export default TradeForm;
