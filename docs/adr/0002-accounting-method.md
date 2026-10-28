# ADR 0002: FIFO accounting with explicit uncertainty

Status: accepted, 2026-10-28

## Method
- Every credited (Direct attribution) swap disposes `token_in` and acquires `token_out`.
- Lots are consumed first in, first out per asset. Reorged swaps are ignored; tentative swaps are flagged.
- A swap is valued in USD from the quote-asset side when one exists, otherwise from the paid side.
  If both sides are priced and differ by more than 5% a `price_divergence` anomaly is recorded.
- Stable quote assets are neutral: they create no lots, closes or unresolved basis.

## Uncertainty states
- `unresolved_basis`: a disposal with no matching lot (transfer-in or unseen history). Never produces profit.
- `unpriced`: no price source for the swap. Its lots have unknown basis.
- `realized_pnl_usd` is `None`, not zero, when nothing could be resolved.
- `coverage_ratio` = reconstructable swaps / credited swaps.
- `pnl_confidence` is a rule label, not a probability: Insufficient below 5 reconstructable swaps, High at
  90% coverage or more with no tentative data, Medium at 70% or more, otherwise Low.
  Thresholds are defaults to be tested prospectively, not validated values.

## Limits
- Amounts above the exact decimal range (about 7.9e28 base units) are flagged, not wrapped.
- Multi-hop router routes are only partly attributed upstream, so basis can be unresolved for them.
