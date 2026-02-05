import { PublicKey } from "@solana/web3.js";

export const VOLTPERP_PROGRAM_ID = new PublicKey(
  "VoLTPErpxuqTzLBQ1k2meXYwVFj7803Yq7F7aYJbRMn"
);

export const USDC_MINT = new PublicKey(
  "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v"
);

export const SOL_ORACLE = new PublicKey(
  "H6ARHf6YXhGYeQfUzQNGk6rDNnLBQKrenN712K4AQJEG"
);

export const ETH_ORACLE = new PublicKey(
  "JBu1AL4obBcCMqKBBxhpWCNUt136ijcuMZLFvTP7iWdB"
);

export const BTC_ORACLE = new PublicKey(
  "GVXRSBjFk6e6J3NbVPXhDrgHfHMAPfCenHm9f2CDSRYV"
);

export interface MarketConfig {
  symbol: string;
  name: string;
  baseDecimals: number;
  quoteDecimals: number;
  oracle: PublicKey;
  maxLeverage: number;
  maintenanceMarginRatio: number;
  takerFee: number;
  makerFee: number;
}

export const MARKETS: Record<string, MarketConfig> = {
  "SOL-PERP": {
    symbol: "SOL-PERP",
    name: "Solana Perpetual",
    baseDecimals: 9,
    quoteDecimals: 6,
    oracle: SOL_ORACLE,
    maxLeverage: 10,
    maintenanceMarginRatio: 0.05,
    takerFee: 0.001,
    makerFee: 0.0005,
  },
  "ETH-PERP": {
    symbol: "ETH-PERP",
    name: "Ethereum Perpetual",
    baseDecimals: 8,
    quoteDecimals: 6,
    oracle: ETH_ORACLE,
    maxLeverage: 10,
    maintenanceMarginRatio: 0.05,
    takerFee: 0.001,
    makerFee: 0.0005,
  },
  "BTC-PERP": {
    symbol: "BTC-PERP",
    name: "Bitcoin Perpetual",
    baseDecimals: 8,
    quoteDecimals: 6,
    oracle: BTC_ORACLE,
    maxLeverage: 10,
    maintenanceMarginRatio: 0.05,
    takerFee: 0.001,
    makerFee: 0.0005,
  },
};

export const DEFAULT_MARKET = "SOL-PERP";

export const COMMITMENT = "confirmed" as const;

export const RPC_ENDPOINT =
  process.env.NEXT_PUBLIC_RPC_ENDPOINT || "https://api.devnet.solana.com";
