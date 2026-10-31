# Pricing

Prices feed PnL, signal outcomes and shadow simulation through the `price_points` table. Nothing reads a provider at request time.

```
provider ─► parse ─► accept() ─► price_points ─► DbPrices / outcomes / shadow
              │          └─ rejected, counted by reason (never guessed)
              └─ missing assets reported
```

## Rules

* `observed_at` is the quote's own timestamp. Quotes older than 15 minutes, or from the future, are rejected.
* Provider confidence below 0.8 is rejected. A missing confidence field is not a rejection.
* Money is never parsed through float arithmetic. The price is read from the JSON number text.
* Our stored `tokens.decimals` always wins. If the provider disagrees, the quote is rejected.
  If we have none, the provider's decimals are accepted, written to `tokens`, and logged as `decimals taken from price provider; review`.
* A price that rounds to zero per base unit is rejected.
* Only real tokens (not demo) that traded in the last 7 days are requested, 200 at most.
* The lookup tolerance (6 hours) lives in `DbPrices`; a missing price makes a value `unavailable`, never zero.

## Turning it on

```
TRACE_PRICE_PROVIDER=defillama          # off when unset
TRACE_PRICE_API_BASE=https://coins.llama.fi   # optional
```

The DefiLlama response shape is parsed from documentation and tested on synthetic bodies. Before relying on it, capture one live response for a known token, compare the price and decimals to a second source, and keep it as a test body.

## Not done

* Historical backfill for tokens first seen before polling started. Outcomes for those stay `unpriced`.
* A second provider for cross-checking.
