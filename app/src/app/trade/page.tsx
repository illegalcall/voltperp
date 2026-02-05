"use client";

import { useState, useEffect, useRef, useCallback, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import { useWallet } from "@solana/wallet-adapter-react";
import TradeForm from "@/components/TradeForm";
import PositionTable from "@/components/PositionTable";
import { useVoltPerp } from "@/hooks/useVoltPerp";
import { MARKETS, DEFAULT_MARKET } from "@/lib/constants";

function TradeContent() {
  const searchParams = useSearchParams();
  const initialMarket = searchParams.get("market") || DEFAULT_MARKET;

  const [selectedMarket, setSelectedMarket] = useState(initialMarket);
  const [markPrice, setMarkPrice] = useState<number | null>(null);
  const [marketSelectorOpen, setMarketSelectorOpen] = useState(false);

  const chartContainerRef = useRef<HTMLDivElement>(null);
  const chartRef = useRef<any>(null);

  const { connected } = useWallet();
  const { collateral, unrealizedPnl, deposit, withdraw, initializeAccount, userAccount, loading } = useVoltPerp();

  const [depositAmount, setDepositAmount] = useState("");
  const [withdrawAmount, setWithdrawAmount] = useState("");
  const [showDepositModal, setShowDepositModal] = useState(false);

  // Simulated mark price -- in production, this comes from the oracle
  useEffect(() => {
    const prices: Record<string, number> = {
      "SOL-PERP": 148.32,
      "ETH-PERP": 3245.67,
      "BTC-PERP": 67892.45,
    };
    setMarkPrice(prices[selectedMarket] ?? null);

    // Simulate small price movements
    const interval = setInterval(() => {
      setMarkPrice((prev) => {
        if (!prev) return prev;
        const delta = (Math.random() - 0.5) * prev * 0.001;
        return parseFloat((prev + delta).toFixed(2));
      });
    }, 2000);
    return () => clearInterval(interval);
  }, [selectedMarket]);

  // Initialize lightweight-charts
  useEffect(() => {
    if (!chartContainerRef.current) return;

    let chart: any;

    const initChart = async () => {
      const { createChart, ColorType } = await import("lightweight-charts");

      if (!chartContainerRef.current) return;

      chart = createChart(chartContainerRef.current, {
        layout: {
          background: { type: ColorType.Solid, color: "#111118" },
          textColor: "#9ca3af",
        },
        grid: {
          vertLines: { color: "#1a1a24" },
          horzLines: { color: "#1a1a24" },
        },
        crosshair: {
          vertLine: { color: "#8b5cf6", width: 1, style: 2 },
          horzLine: { color: "#8b5cf6", width: 1, style: 2 },
        },
        timeScale: {
          borderColor: "#232330",
          timeVisible: true,
        },
        rightPriceScale: {
          borderColor: "#232330",
        },
        width: chartContainerRef.current.clientWidth,
        height: 400,
      });

      const candleSeries = chart.addCandlestickSeries({
        upColor: "#22c55e",
        downColor: "#ef4444",
        borderDownColor: "#ef4444",
        borderUpColor: "#22c55e",
        wickDownColor: "#ef4444",
        wickUpColor: "#22c55e",
      });

      // Generate sample candlestick data
      const basePrice =
        selectedMarket === "SOL-PERP"
          ? 148
          : selectedMarket === "ETH-PERP"
          ? 3245
          : 67892;

      const data = [];
      const now = Math.floor(Date.now() / 1000);
      for (let i = 200; i >= 0; i--) {
        const time = now - i * 300; // 5-min candles
        const volatility = basePrice * 0.003;
        const open =
          basePrice + (Math.random() - 0.5) * volatility * 10;
        const close = open + (Math.random() - 0.5) * volatility * 4;
        const high = Math.max(open, close) + Math.random() * volatility;
        const low = Math.min(open, close) - Math.random() * volatility;
        data.push({
          time: time as any,
          open: parseFloat(open.toFixed(2)),
          high: parseFloat(high.toFixed(2)),
          low: parseFloat(low.toFixed(2)),
          close: parseFloat(close.toFixed(2)),
        });
      }

      candleSeries.setData(data);
      chart.timeScale().fitContent();
      chartRef.current = chart;
    };

    initChart();

    const handleResize = () => {
      if (chart && chartContainerRef.current) {
        chart.applyOptions({
          width: chartContainerRef.current.clientWidth,
        });
      }
    };
    window.addEventListener("resize", handleResize);

    return () => {
      window.removeEventListener("resize", handleResize);
      if (chart) chart.remove();
    };
  }, [selectedMarket]);

  const handleDeposit = async () => {
    const amt = parseFloat(depositAmount);
    if (isNaN(amt) || amt <= 0) return;
    try {
      if (!userAccount) {
        await initializeAccount();
      }
      await deposit(amt);
      setDepositAmount("");
      setShowDepositModal(false);
    } catch (e) {
      console.error("Deposit failed:", e);
    }
  };

  const handleWithdraw = async () => {
    const amt = parseFloat(withdrawAmount);
    if (isNaN(amt) || amt <= 0) return;
    try {
      await withdraw(amt);
      setWithdrawAmount("");
      setShowDepositModal(false);
    } catch (e) {
      console.error("Withdraw failed:", e);
    }
  };

  return (
    <div className="max-w-[1440px] mx-auto px-4 py-4">
      {/* Market Header */}
      <div className="flex items-center justify-between mb-4">
        <div className="flex items-center gap-4">
          {/* Market Selector */}
          <div className="relative">
            <button
              onClick={() => setMarketSelectorOpen(!marketSelectorOpen)}
              className="flex items-center gap-2 px-4 py-2 rounded-lg bg-surface-2 border border-surface-4 hover:bg-surface-3 transition-colors"
            >
              <span className="font-bold text-lg text-white">
                {selectedMarket}
              </span>
              <svg
                className={`w-4 h-4 text-gray-400 transition-transform ${
                  marketSelectorOpen ? "rotate-180" : ""
                }`}
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  strokeWidth={2}
                  d="M19 9l-7 7-7-7"
                />
              </svg>
            </button>

            {marketSelectorOpen && (
              <div className="absolute top-full left-0 mt-1 w-56 bg-surface-2 border border-surface-4 rounded-xl shadow-2xl z-50 overflow-hidden">
                {Object.entries(MARKETS).map(([key, market]) => (
                  <button
                    key={key}
                    onClick={() => {
                      setSelectedMarket(key);
                      setMarketSelectorOpen(false);
                    }}
                    className={`w-full flex items-center justify-between px-4 py-3 hover:bg-surface-3 transition-colors ${
                      key === selectedMarket ? "bg-surface-3" : ""
                    }`}
                  >
                    <div className="text-left">
                      <div className="font-semibold text-white text-sm">
                        {key}
                      </div>
                      <div className="text-xs text-gray-500">{market.name}</div>
                    </div>
                    <span className="text-xs text-gray-500">
                      {market.maxLeverage}x
                    </span>
                  </button>
                ))}
              </div>
            )}
          </div>

          {/* Mark Price */}
          <div>
            <div className="text-2xl font-bold font-mono text-white">
              {markPrice
                ? `$${markPrice.toLocaleString(undefined, {
                    minimumFractionDigits: 2,
                    maximumFractionDigits: 2,
                  })}`
                : "--"}
            </div>
          </div>
        </div>

        {/* Market Info Pills */}
        <div className="hidden md:flex items-center gap-3">
          <div className="px-3 py-1.5 rounded-lg bg-surface-2 border border-surface-4">
            <span className="text-xs text-gray-500">24h Volume</span>
            <span className="block text-sm font-mono text-white">
              $12.4M
            </span>
          </div>
          <div className="px-3 py-1.5 rounded-lg bg-surface-2 border border-surface-4">
            <span className="text-xs text-gray-500">Open Interest</span>
            <span className="block text-sm font-mono text-white">
              $8.2M
            </span>
          </div>
          <div className="px-3 py-1.5 rounded-lg bg-surface-2 border border-surface-4">
            <span className="text-xs text-gray-500">Funding Rate</span>
            <span className="block text-sm font-mono text-long">
              +0.0012%
            </span>
          </div>
        </div>
      </div>

      {/* Main Grid */}
      <div className="grid grid-cols-1 lg:grid-cols-[1fr_340px] gap-4">
        {/* Left Column: Chart + Positions */}
        <div className="space-y-4">
          {/* Chart */}
          <div className="bg-surface-2 rounded-xl border border-surface-4 overflow-hidden">
            <div className="px-4 py-2.5 border-b border-surface-4 flex items-center gap-3">
              {["1m", "5m", "15m", "1H", "4H", "1D"].map((tf) => (
                <button
                  key={tf}
                  className={`text-xs px-2 py-1 rounded ${
                    tf === "5m"
                      ? "bg-surface-4 text-white"
                      : "text-gray-500 hover:text-gray-300"
                  } transition-colors`}
                >
                  {tf}
                </button>
              ))}
            </div>
            <div ref={chartContainerRef} className="w-full" />
          </div>

          {/* Positions */}
          <PositionTable />
        </div>

        {/* Right Column: Trade Form + Account */}
        <div className="space-y-4">
          <TradeForm selectedMarket={selectedMarket} markPrice={markPrice} />

          {/* Account Info */}
          <div className="bg-surface-2 rounded-xl border border-surface-4 p-5">
            <h3 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
              Account
            </h3>

            {connected ? (
              <div className="space-y-3">
                <div className="flex justify-between items-center">
                  <span className="text-sm text-gray-500">Collateral</span>
                  <span className="text-sm font-mono font-semibold text-white">
                    ${collateral.toFixed(2)}
                  </span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-sm text-gray-500">Unrealized PnL</span>
                  <span
                    className={`text-sm font-mono font-semibold ${
                      unrealizedPnl >= 0 ? "text-long" : "text-short"
                    }`}
                  >
                    {unrealizedPnl >= 0 ? "+" : ""}${unrealizedPnl.toFixed(2)}
                  </span>
                </div>
                <div className="flex justify-between items-center">
                  <span className="text-sm text-gray-500">Account Value</span>
                  <span className="text-sm font-mono font-semibold text-white">
                    ${(collateral + unrealizedPnl).toFixed(2)}
                  </span>
                </div>

                <div className="border-t border-surface-4 pt-3 flex gap-2">
                  <button
                    onClick={() => setShowDepositModal(true)}
                    className="flex-1 py-2 rounded-lg bg-volt-600 text-white text-sm font-semibold hover:bg-volt-500 transition-colors"
                  >
                    Deposit
                  </button>
                  <button
                    onClick={() => setShowDepositModal(true)}
                    className="flex-1 py-2 rounded-lg bg-surface-3 text-gray-300 text-sm font-semibold hover:bg-surface-4 transition-colors"
                  >
                    Withdraw
                  </button>
                </div>
              </div>
            ) : (
              <p className="text-center text-sm text-gray-500 py-4">
                Connect wallet to view account
              </p>
            )}
          </div>
        </div>
      </div>

      {/* Deposit/Withdraw Modal */}
      {showDepositModal && (
        <div className="fixed inset-0 bg-black/60 backdrop-blur-sm z-50 flex items-center justify-center p-4">
          <div className="bg-surface-2 border border-surface-4 rounded-2xl w-full max-w-md p-6">
            <div className="flex justify-between items-center mb-5">
              <h2 className="text-lg font-bold text-white">
                Manage Collateral
              </h2>
              <button
                onClick={() => setShowDepositModal(false)}
                className="text-gray-500 hover:text-white transition-colors"
              >
                <svg
                  className="w-5 h-5"
                  fill="none"
                  viewBox="0 0 24 24"
                  stroke="currentColor"
                >
                  <path
                    strokeLinecap="round"
                    strokeLinejoin="round"
                    strokeWidth={2}
                    d="M6 18L18 6M6 6l12 12"
                  />
                </svg>
              </button>
            </div>

            <div className="space-y-4">
              {/* Deposit */}
              <div>
                <label className="block text-xs text-gray-500 mb-1.5">
                  Deposit USDC
                </label>
                <div className="flex gap-2">
                  <input
                    type="number"
                    value={depositAmount}
                    onChange={(e) => setDepositAmount(e.target.value)}
                    placeholder="0.00"
                    className="input-field flex-1"
                  />
                  <button
                    onClick={handleDeposit}
                    disabled={loading || !depositAmount}
                    className="btn-primary"
                  >
                    {loading ? "..." : "Deposit"}
                  </button>
                </div>
              </div>

              {/* Withdraw */}
              <div>
                <label className="block text-xs text-gray-500 mb-1.5">
                  Withdraw USDC
                </label>
                <div className="flex gap-2">
                  <input
                    type="number"
                    value={withdrawAmount}
                    onChange={(e) => setWithdrawAmount(e.target.value)}
                    placeholder="0.00"
                    className="input-field flex-1"
                  />
                  <button
                    onClick={handleWithdraw}
                    disabled={loading || !withdrawAmount}
                    className="px-4 py-2 rounded-lg bg-surface-3 text-gray-300 font-semibold text-sm hover:bg-surface-4 transition-colors disabled:opacity-40"
                  >
                    {loading ? "..." : "Withdraw"}
                  </button>
                </div>
              </div>

              <div className="text-xs text-gray-600 pt-2">
                Current balance: ${collateral.toFixed(2)} USDC
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default function TradePage() {
  return (
    <Suspense
      fallback={
        <div className="flex items-center justify-center min-h-[60vh]">
          <div className="animate-spin w-8 h-8 border-2 border-accent border-t-transparent rounded-full" />
        </div>
      }
    >
      <TradeContent />
    </Suspense>
  );
}
