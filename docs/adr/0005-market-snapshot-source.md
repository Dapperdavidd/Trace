# ADR 0005: Evidence-derived market snapshots

Status: accepted, 2026-11-02

## Context

The API reads `market_snapshots`, but production code previously wrote only demo rows. Discovery, token research, risk checks, outcomes, and Shadow depend on those rows.

TRACE currently has two reviewed real inputs: normalized swaps and accepted token price points. It does not yet have a reviewed source for pool reserves or liquidity. The original schema required liquidity, which made an honest partial snapshot impossible.

## Decision

The worker builds price and volume snapshots every five minutes when real price polling is enabled.

- Current and window-start prices come from accepted, non-demo `price_points` and token decimals. A point more than six hours before the requested time is not used.
- Window and baseline volume come from final, non-demo swaps valued with the most recent accepted price at or before each swap.
- Only non-stable tokens seen in final swaps during the last seven days are candidates.
- The worker writes `liquidity_usd = NULL` until a reviewed reserve or market-data source is available.
- Discovery excludes snapshots with unavailable liquidity and reports the exclusion.
- Risk, outcomes, and Shadow continue to abstain when liquidity is unavailable.
- Existing reviewed liquidity is preserved when the price and volume fields refresh.

## Consequences

Token research can show real price and volume data without claiming that missing liquidity is zero. Discovery and simulated execution remain safely gated. A later liquidity source can fill the nullable field without changing how price and volume are derived.

The current query is intentionally bounded to 200 active pool-token pairs. Scaling and retention need review after realistic-volume load testing.
