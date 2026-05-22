# Perpetual Protocol

Perpetual Protocol is a Solana/Anchor rebuild of the core mechanics behind an on-chain perpetual futures exchange. The goal is to understand the protocol layer deeply by implementing the important pieces directly: accounts, order state, AMM fills, position accounting, oracle checks, and protocol safety guards.

This started as `mini-drift`. The repository path and Anchor program crate still use `mini-drift` / `mini_drift` while the project-facing name is now **Perpetual Protocol**.

## Current Status

This is a serious learning and portfolio protocol, not production software.

- Anchor program with user accounts, perpetual market state, order state, and position state.
- Market and limit order placement with reduce-only validation.
- AMM fill path that updates orders, positions, market exposure, and AMM reserves.
- Position accounting for open, increase, reduce, close, and flip transitions.
- Oracle freshness/confidence checks and mark/oracle divergence validation.
- Spread helpers for inventory, volatility, and oracle confidence adjustments.
- Matching helpers for maker/taker crossing and fill sizing.
- Event models for order placement and AMM fills.
- Devnet Pyth SOL/USD readout script.
- Minimal wallet connect and SOL balance UI stub.
- `cargo test`: 164 passing Rust unit tests.

## Why This Exists

Perpetual futures protocols are not just "place order, update balance" applications. They combine several hard systems:

- Solana account modeling and Anchor instruction design.
- Perp-specific state machines for orders and positions.
- Constant-product AMM reserve movement and bounded reserve checks.
- Maker/taker logic, resting order rules, auctions, and fill limits.
- Signed base/quote accounting for long and short exposure.
- Oracle validity, confidence, and mark price guardrails.
- Mutation safety so failed fills do not half-update protocol state.

This repo is built to demonstrate that work directly in code. It is intentionally lower-level than a UI-only trading demo.

## What Is Built

### On-chain Program

The Anchor program exposes:

- `initialize`
- `initialize_user`
- `place_perp_order`
- `fill_perp_order`

The core state lives in:

- `User`: authority, perp positions, open orders, and order counters.
- `PerpPosition`: base/quote exposure, entry and break-even quote amounts, open bids/asks, and status flags.
- `Order`: order type, direction, price, amount, filled amounts, reduce-only/post-only/IOC flags, auction fields, and slot.
- `PerpMarket`: AMM reserves, peg, bounds, oracle settings, open interest aggregates, and mocked oracle data for local protocol work.

### Order Placement

`place_perp_order` currently supports market and limit orders. It handles:

- Order slot allocation.
- Perp position slot allocation and reuse.
- Reduce-only rejection when an order would increase exposure.
- Existing position direction tracking.
- `open_orders`, `open_bids`, and `open_asks` accounting.
- Order placement event emission.

### AMM Fill Path

`fill_perp_order` routes through the AMM fill handler and performs:

- Open-order validation.
- Market and position validation.
- Oracle freshness, confidence, and sufficient-data checks.
- Mark price versus oracle divergence guard.
- AMM quote calculation from base reserve movement.
- Spread application before position mutation.
- Limit-price protection for AMM fills.
- Atomic market, position, order, and counter updates.
- Fill event emission.

The fill path is written defensively: many tests assert that invalid fills return before mutating state.

### AMM And Price Math

The AMM module includes:

- Mark price calculation from reserves and peg.
- Base asset swap quote calculation.
- Reserve updates with min/max base reserve bounds.
- Long and short AMM exposure updates.
- Checked math paths for overflow and invalid reserve states.

### Position Accounting

Position logic supports:

- Opening a new position.
- Increasing an existing long or short.
- Reducing exposure and realizing PnL.
- Closing a position.
- Flipping from long to short or short to long.
- Updating market-level user counts and AMM quote aggregates.
- Step-size validation before mutation.

### Matching And Order Helpers

The protocol also has helpers for:

- Maker/taker crossing checks.
- Same-market opposite-side validation.
- Matched fill base and quote sizing.
- Resting limit order checks.
- Auction price calculation.
- Fill mode limit-price behavior.
- Order expiry decisions.
- Reduce-only cleanup decisions.
- Keeper reward eligibility.

### Tooling

- `scripts/pyth-sol-usd.ts` reads a Pyth SOL/USD price account on devnet and decodes the price update fields.
- `app/index.html` is a minimal browser UI for wallet connection and devnet SOL balance display.
- `tests/mini-drift.ts` contains the Anchor TypeScript smoke test.

## Repository Layout

```text
programs/mini-drift/src/
  controller/      Order and position mutation controllers
  instructions/    Anchor instruction handlers and fill/admin helpers
  math/            AMM, spread, oracle, auction, matching, and position math
  state/           User, order, market, oracle, fulfillment, and event types

scripts/
  pyth-sol-usd.ts  Devnet Pyth SOL/USD account decoder

app/
  index.html       Minimal wallet connect and SOL balance UI
```

## Running Checks

```bash
cargo test
yarn lint
anchor test
```

The primary protocol test suite is the Rust unit test suite:

```text
164 passed; 0 failed
```

## Roadmap

The next high-value protocol work:

- Expose market initialization as a complete Anchor instruction with PDA design.
- Add collateral deposits, withdrawals, margin checks, and account equity.
- Implement funding rate calculation and funding payment settlement.
- Add realized/unrealized PnL settlement.
- Replace mocked oracle data with real Pyth account validation.
- Build the maker/taker matched-fill instruction path.
- Add liquidations, bankruptcy handling, and insurance fund logic.
- Build a real localnet trading demo and a serious trader-facing UI.

## Disclaimer

This is not audited and not production ready. It is a study and portfolio implementation for learning perpetual protocol mechanics on Solana.
