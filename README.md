# VoltPerp

On-chain perpetual futures engine on Solana. VoltPerp provides a fully decentralized derivatives trading platform powered by a virtual AMM (vAMM) architecture, cross-margin collateral management, multi-tiered liquidation, and autonomous keeper infrastructure.

## Architecture

```
                          ┌───────────────────────────────────┐
                          │          Frontend (Next.js)       │
                          │   Trading UI / Portfolio / Charts │
                          └──────────────┬────────────────────┘
                                         │ RPC / WebSocket
                          ┌──────────────▼────────────────────┐
                          │         Solana Cluster             │
                          │                                    │
                          │  ┌──────────────────────────────┐  │
                          │  │      VoltPerp Program         │  │
                          │  │         (Anchor)              │  │
                          │  │                                │  │
                          │  │  ┌──────────┐  ┌───────────┐  │  │
                          │  │  │ Exchange │  │   Perp     │  │  │
                          │  │  │  State   │  │  Markets   │  │  │
                          │  │  └────┬─────┘  └─────┬─────┘  │  │
                          │  │       │              │         │  │
                          │  │  ┌────▼──────────────▼─────┐  │  │
                          │  │  │     vAMM Engine          │  │  │
                          │  │  │  (x * y = k pricing)     │  │  │
                          │  │  └────┬─────────────────────┘  │  │
                          │  │       │                        │  │
                          │  │  ┌────▼─────┐  ┌───────────┐  │  │
                          │  │  │  User    │  │  Oracle    │  │  │
                          │  │  │ Accounts │  │ (Pyth)    │  │  │
                          │  │  └──────────┘  └───────────┘  │  │
                          │  └──────────────────────────────┘  │
                          └──────────────┬────────────────────┘
                                         │
                 ┌───────────────────────┼───────────────────────┐
                 │                       │                       │
        ┌────────▼─────────┐   ┌─────────▼────────┐   ┌─────────▼────────┐
        │   Liquidator     │   │  Funding Crank   │   │  TWAP Updater    │
        │   Keeper Bot     │   │  Keeper Bot      │   │  Keeper Bot      │
        └──────────────────┘   └──────────────────┘   └──────────────────┘
                          Keeper Infrastructure (Rust)
```

## Features

- **Virtual AMM (vAMM)**: Constant-product pricing without requiring real liquidity providers. The vAMM uses `x * y = k` invariant to provide deterministic price impact and slippage.
- **Cross-Margin Collateral**: Single collateral vault supports positions across multiple perpetual markets, improving capital efficiency.
- **Multi-Tiered Liquidation**: Three-tier liquidation system — partial liquidation, full liquidation, and socialized loss — to protect the insurance fund and minimize cascading failures.
- **Funding Rate Mechanism**: Hourly funding payments that converge mark price to oracle price. Calculated as `(mark_price - oracle_price) / oracle_price` with time-weighted averaging.
- **Oracle Integration**: Pyth Network price feeds for reliable, low-latency oracle data with TWAP smoothing to prevent manipulation.
- **Keeper Infrastructure**: Production-grade Rust keeper bots for liquidation execution and funding rate settlement with built-in metrics and monitoring.
- **Position Management**: Open long/short positions, partial close, and full close with realized PnL tracking.
- **Risk Engine**: Pre-trade margin checks, maintenance margin monitoring, and account health calculation.

## Tech Stack

| Layer          | Technology                          |
|----------------|-------------------------------------|
| Smart Contract | Anchor (Rust), Solana runtime       |
| Keeper Bots    | Rust, Tokio, solana-client          |
| Frontend       | Next.js, React, TypeScript          |
| Testing        | Anchor TS (Mocha/Chai), Rust tests  |
| Oracle         | Pyth Network                        |
| Deployment     | Solana CLI, Anchor CLI              |

## Directory Structure

```
voltperp/
├── programs/
│   └── voltperp/
│       └── src/
│           ├── lib.rs              # Program entry point and instruction dispatch
│           ├── state/              # Account structures (Exchange, Market, User)
│           ├── instructions/       # Instruction handlers
│           ├── math/               # vAMM math, funding calculations
│           └── errors.rs           # Custom error codes
├── keeper/
│   ├── Cargo.toml                  # Keeper binary crate
│   └── src/
│       ├── main.rs                 # CLI entry point, task orchestration
│       ├── liquidator.rs           # Liquidation engine
│       ├── funding_crank.rs        # Funding rate settlement
│       └── metrics.rs              # Operational metrics
├── tests/
│   └── voltperp.ts                 # Anchor integration tests
├── app/                            # Next.js frontend (coming soon)
├── migrations/
│   └── deploy.ts                   # Deployment script
├── Anchor.toml                     # Anchor workspace config
├── Cargo.toml                      # Workspace Cargo config
├── package.json                    # Node dependencies
├── tsconfig.json                   # TypeScript configuration
└── README.md
```

## Prerequisites

- [Rust](https://rustup.rs/) 1.75+
- [Solana CLI](https://docs.solanalabs.com/cli/install) 2.1+
- [Anchor CLI](https://www.anchor-lang.com/docs/installation) 0.30+
- [Node.js](https://nodejs.org/) 18+
- [Yarn](https://yarnpkg.com/) or npm

## Build

### Smart Contract

```bash
# Build the Anchor program
anchor build

# Verify program keypair
solana address -k target/deploy/voltperp-keypair.json
```

### Keeper Bot

```bash
cd keeper
cargo build --release
```

## Run the Keeper

The keeper bot runs two concurrent tasks: a **liquidator** that monitors account health and executes liquidations, and a **funding crank** that settles funding rates on schedule.

```bash
# Using cargo
cd keeper
cargo run --release -- \
  --rpc-url https://api.mainnet-beta.solana.com \
  --keypair-path ~/.config/solana/keeper.json \
  --program-id <PROGRAM_ID> \
  --poll-interval-ms 2000 \
  --funding-interval-secs 60

# Or using the compiled binary
./target/release/voltperp-keeper \
  --rpc-url https://api.mainnet-beta.solana.com \
  --keypair-path ~/.config/solana/keeper.json \
  --program-id <PROGRAM_ID> \
  --verbose

# Dry-run mode (simulates but does not submit transactions)
./target/release/voltperp-keeper \
  --rpc-url https://api.devnet.solana.com \
  --keypair-path ~/.config/solana/keeper.json \
  --program-id <PROGRAM_ID> \
  --dry-run

# Environment variables are also supported
export RPC_URL=https://api.mainnet-beta.solana.com
export KEYPAIR_PATH=~/.config/solana/keeper.json
export PROGRAM_ID=<PROGRAM_ID>
./target/release/voltperp-keeper
```

### Keeper CLI Options

| Flag                          | Default                                      | Description                         |
|-------------------------------|----------------------------------------------|-------------------------------------|
| `--rpc-url`                   | `https://api.mainnet-beta.solana.com`        | Solana RPC endpoint                 |
| `--keypair-path`              | `~/.config/solana/id.json`                   | Keeper wallet keypair               |
| `--program-id`                | (required)                                   | VoltPerp program address            |
| `--poll-interval-ms`          | `2000`                                       | Account polling interval (ms)       |
| `--funding-interval-secs`     | `60`                                         | Funding check interval (s)          |
| `--max-liquidations-per-cycle`| `10`                                         | Max liquidations per poll cycle     |
| `--commitment`                | `confirmed`                                  | RPC commitment level                |
| `--dry-run`                   | `false`                                      | Simulate without submitting txs     |
| `--verbose`                   | `false`                                      | Enable debug-level logging          |

## Run Tests

```bash
# Start local validator
solana-test-validator

# In another terminal, run Anchor tests
anchor test

# Run specific test
anchor test -- --grep "Open Long Position"

# Run Rust unit tests (program)
cargo test --manifest-path programs/voltperp/Cargo.toml

# Run keeper unit tests
cargo test --manifest-path keeper/Cargo.toml
```

## Design Decisions

### vAMM Architecture

VoltPerp uses a **virtual AMM** rather than an order book or traditional AMM with real LPs. The vAMM maintains virtual reserves (`base_reserve * quote_reserve = k`) that determine price impact without requiring counterparty liquidity. This design choice provides:

- **Guaranteed liquidity** at all price levels — no thin order books or missing counterparties.
- **Deterministic price impact** — traders know their slippage before execution.
- **Simplicity** — no market-maker incentive programs or LP token management.
- **Capital efficiency** — the protocol does not need to attract or lock TVL for liquidity.

The trade-off is that all risk is socialized: the insurance fund absorbs losses when liquidation proceeds are insufficient.

### Multi-Tiered Liquidation

Liquidation follows a three-tier escalation model to minimize market impact and protect the insurance fund:

1. **Partial Liquidation** (health factor 50-100 bps): Up to 25% of the position is closed. The liquidator receives a small fee from the user's remaining collateral. This tier handles most liquidations and minimizes cascading price impact.

2. **Full Liquidation** (health factor < 50 bps): The entire position is closed. The liquidator receives a larger fee. Any remaining collateral goes to the insurance fund.

3. **Socialized Loss** (negative equity after full liquidation): When a position's losses exceed its collateral, the deficit is distributed across all open positions in the same market, proportional to position size. This prevents the insurance fund from depleting during extreme volatility.

### Oracle Integration

VoltPerp uses **Pyth Network** as its primary oracle source, with the following safeguards:

- **TWAP Smoothing**: A 5-minute exponential moving average of oracle prices is used for funding rate calculations to prevent manipulation via short-term price spikes.
- **Staleness Checks**: Oracle prices older than 60 seconds are rejected. Positions cannot be opened or liquidated with stale prices.
- **Confidence Interval**: Pyth's confidence band is checked — if the confidence interval exceeds 2% of the price, the oracle is considered unreliable and high-risk operations are blocked.
- **Mark-Oracle Divergence**: If the vAMM mark price diverges more than 10% from the oracle price, position opening is restricted to the side that brings mark price closer to oracle price.

### Funding Rate Mechanism

Funding payments occur hourly and are calculated as:

```
funding_rate = (mark_twap - oracle_twap) / oracle_twap / 24
```

- When mark > oracle, longs pay shorts (indicating bullish premium).
- When mark < oracle, shorts pay longs (indicating bearish discount).
- The `/24` divisor normalizes the hourly rate to a daily percentage.
- Funding is settled per-account when a user interacts with the protocol, or by the keeper crank for idle accounts.

### Cross-Margin Design

All positions share a single USDC collateral pool per user account. This means:

- Unrealized profits on one position can serve as margin for another.
- A single deposit supports trading across all available markets.
- Account health is computed holistically across all positions.
- Withdrawals are only permitted if the account remains above initial margin after the withdrawal.

## License

MIT
