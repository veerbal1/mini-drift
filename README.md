# mini-drift

A learning-first rebuild of Drift-style perpetual futures mechanics on Solana.

## What this is

Explain:
- Anchor/Solana perp protocol project
- rebuilt ring by ring from Drift v2 concepts
- goal is to understand the protocol deeply, not copy-paste production Drift
- currently focused on order placement, fill helpers, and position-update path

## Current status

Say:
- Rust unit tests: 96 passing
- current focus: position updates
- not production ready
- educational / portfolio protocol

## Completed so far

List:
- User account initialization
- PerpPosition storage
- OrderParams and stored Order model
- Market and limit order placement
- reduce-only validation
- open_bids / open_asks accounting
- order slot and perp position slot helpers
- auction price helper
- FillMode helper
- fulfillment method labels: AMM / Match
- maker/taker crossing checks
- resting limit order checks
- matched fill base/quote math
- order progress update after fill
- full-fill open-order counter cleanup
- expiry cleanup decision
- reduce-only cleanup decision
- keeper reward gate
- OrderRecord / OrderActionRecord event shapes
- devnet Pyth SOL/USD readout script
- wallet connect + SOL balance readout stub

## Pending

List:
- real position mutation: open / increase / reduce / close / flip
- real AMM reserve swap
- oracle validation and TWAP
- spreads
- fee pools and funding
- collateral deposits
- PnL calculation
- margin checks
- liquidation
- bankruptcy / insurance
- real localnet end-to-end trading demo
- serious UI

## Project layout

Mention:
- programs/mini-drift/src/state
- programs/mini-drift/src/controller
- programs/mini-drift/src/math
- scripts/pyth-sol-usd.ts
- app/index.html

## Running checks

Commands:
- cargo test
- npx tsc --noEmit
- npx prettier --check app/index.html scripts/pyth-sol-usd.ts

## Disclaimer

Say:
This is not production software. It is a study/build project for learning perpetual protocol mechanics.
