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

The DefiLlama response shape is tested on synthetic edge cases and a captured live response at `crates/pricing/tests/fixtures/defillama_weth_2026-11-03.json`.

## Live comparison

On 2026-11-03, the Ethereum WETH contract `0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2` was checked through both providers within three minutes:

| Source | Observed at UTC | USD price | Decimals |
| --- | --- | ---: | ---: |
| DefiLlama Coins API | 12:25:53 | 2275.8786226968355 | 18 |
| CoinGecko coin-by-contract API | 12:28:55 | 2279.55 | 18 |

The prices differed by 0.1611 percent and both sources reported 18 decimals. This comparison validates the response shape and a reasonable live result at that moment. It is not a permanent guarantee that providers will agree. Production can enable the adapter with `TRACE_PRICE_PROVIDER=defillama`; a second automated provider and disagreement policy are still required.

The checked endpoints were the [DefiLlama Coins API response](https://coins.llama.fi/prices/current/ethereum:0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2) and the [CoinGecko WETH contract response](https://api.coingecko.com/api/v3/coins/ethereum/contract/0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2).

## Historical adapter

The DefiLlama adapter also supports its documented `batchHistorical` endpoint. TRACE sends explicit asset and timestamp pairs, keeps the provider timestamp, and accepts a point only when it is within 30 minutes of the requested time. Historical points require token decimals already verified and stored by TRACE. The provider cannot teach decimals through this path.

A captured WETH response for timestamps `1700000000` and `1700086400` is stored as a parser fixture. The returned points were 100 seconds after each requested timestamp. This proves the response shape and approximate-time handling, not independent price accuracy. The resumable worker stores accepted points and counts every rejection. A second-source cross-check is still required before using historical prices as independently corroborated market truth.

## Not done

* A second provider for cross-checking.
