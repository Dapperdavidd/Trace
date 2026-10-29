# TRACE API contract v1 (client-facing)

Base path `/v1`. JSON only. Amounts that can exceed 2^53 are strings. Percentages and USD values are
decimal strings. Every analytic response carries the honesty envelope below. Errors use
`{ "code", "message", "details?", "request_id" }`.

```ts
type Chain = "solana" | "ethereum" | "base" | "bnb";
type DataStatus = "live" | "stale" | "unavailable" | "partial" | "demo";
interface Envelope { data_status: DataStatus; as_of: string /* ISO UTC */; demo: boolean; coverage_note?: string }
interface TokenRef { chain: Chain; address: string; symbol: string | null; name: string | null }
interface WalletRef { chain: Chain; address: string; alias: string | null }
type RiskState = "pass" | "warn" | "block" | "unknown";
```

## Auth and account
- `POST /auth/google` body `{ id_token }` -> `{ access_token, expires_at, user: { id, email, name } }`
- `POST /auth/logout`, `DELETE /me` (account deletion), `GET /me` -> `{ id, email, name, wallets: { watch_only: number, connected: 0 } }`
- A signed-in user has NO spendable wallet. `GET /me/wallets` -> `{ watch_only: WalletRef[], connected: [], embedded: { available: false, reason: string } }`

## Watchlist and alerts
- `GET /watchlists` -> `{ items: [{ id, wallet: WalletRef, status: "active"|"paused"|"partial_monitoring"|"unsupported_protocol"|"waiting_for_activity", last_checked_at }] }`
- `POST /watchlists` `{ chain, address, alias? }` (idempotent on user+chain+address), `PATCH /watchlists/{id}`, `DELETE /watchlists/{id}`
- `GET|PATCH /alert-preferences`, `GET /alerts?cursor=` -> `{ items: [{ id, type: "wallet_buy"|"wallet_sell"|"risk_event"|"signal", title, body, evidence_tx: string[], event_time, published_at }], next_cursor }`

## Discovery (Top Movers and Smart Wallets)
- `GET /discovery/movers?tab=trending|early_momentum|smart_money|new&window=1h|24h|7d&chain=`
  -> `Envelope & { items: [{ rank, token: TokenRef, price_usd, change_pct, volume_usd, liquidity_usd, pool: string, reason: { code: string; label: string }, sparkline: number[], risk: RiskState, risk_notes: string[], source_timestamp: string }] }`
- `GET /discovery/wallets?tab=top|new|most_followed&chain=&cursor=`
  -> `Envelope & { items: [{ wallet: WalletRef, realized_return_pct: string | null, win_rate_pct: string | null, sample_size: number, tags: string[], decision: "eligible"|"watch_only"|"insufficient_evidence"|"rejected", pnl_confidence: "high"|"medium"|"low"|"insufficient", coverage_ratio: string | null }], next_cursor }`

## Wallet research
- `GET /wallets/{chain}/{address}` -> `Envelope & { wallet: WalletRef, first_seen: string|null, last_seen: string|null, monitoring: string, metrics: Metrics }`
- `GET /wallets/{chain}/{address}/trades?cursor=` -> `Envelope & { items: [{ tx_id, event_time, side: "buy"|"sell", token: TokenRef, amount_usd: string|null, finality: "tentative"|"final" }], next_cursor }`
- `GET /wallets/{chain}/{address}/metrics` -> `Envelope & Metrics & { wqs: Score, wcs: Score | null }`
- `GET /wallets/{chain}/{address}/dna` -> `Envelope & { state: "ok"|"insufficient_history", rules_version: string, sample_size: number, traits: [{ key: string, label: string, evidence: string }] }`
- `Metrics = { realized_pnl_usd: string|null, unrealized_pnl_usd: string|null, total_trades: number, coverage_ratio: string|null, pnl_confidence: string, unresolved_disposals: number }`
- `Score = { decision: string, value: number|null, version: string, components: Record<string, string>, cohort: string, window: string, explanation: string }`

## Tokens
- `GET /tokens/{chain}/{address}` -> `Envelope & { token: TokenRef, price_usd, change_pct_24h, market_cap_usd: string|null, volume_24h_usd, liquidity_usd, holders: number|null, risk: { overall: RiskState, checks: [{ key: "liquidity"|"honeypot"|"holder_concentration"|"contract_safety"|"suspicious_activity", state: RiskState, note: string }], fetched_at: string }, why_moving: [{ kind: "fact"|"hypothesis", text: string, evidence_ids: string[] }], buyers: WalletRef[] }`
- `GET /tokens/{chain}/{address}/timeline` -> `Envelope & { items: [{ time, kind: "fact"|"hypothesis", title, detail, evidence_ids: string[], possible_impact: string|null }] }`
- `GET /tokens/compare?a=chain:address&b=chain:address` -> `Envelope & { items: TokenSummary[2..4], verdict_note: string }`

## Signals and Shadow Wallets
- `GET /signals?cursor=&state=` and `GET /signals/{id}` (immutable receipt: evidence, rules_version, risk verdict, published_at) and `GET /signals/{id}/outcomes` (1h, 24h, 7d, 30d incl. losses and unpriced)
- `POST /shadow-strategies` `{ wallet: {chain,address}, budget_usd, size_rule, delay_seconds, slippage_bps, exit_rule }` -> `{ id }`
- `GET /shadow-strategies/{id}/results` -> `Envelope & { starting_usd, ending_usd, return_pct, trades: number, win_rate_pct, skipped_unexecutable: number, fees_and_slippage_usd, baseline: {...}, assumptions: {...} }`
- Paper only. Nothing here signs or broadcasts.

## Briefings and coverage
- `GET /briefings` -> `Envelope & { sections: [{ title, items: [{ text, link? }] }] }`
- `GET /chains` (implemented): chain and protocol coverage truth.

## Out of scope for MVP (UI must show "Coming later", never fake it)
Spendable wallet creation, send, receive, buy, swap, live copy trading, natural-language "Ask TRACE".


## Settled in milestone 5 (previously ambiguous)

- **Auth:** `Authorization: Bearer <access_token>`. Sessions last 30 days and only a hash is stored. Sign-in answers 503
  `dependency_unavailable` when `GOOGLE_CLIENT_ID` is not configured. `DELETE /me` removes the account, sessions,
  watchlist, alerts and paper strategies.
- **Errors:** every error carries `request_id`, including those raised inside handlers. 409 `conflict`, 422 `limit_reached`.
- **Watchlist:** up to 200 wallets. `POST` returns 201 for a new wallet and 200 with the same `id` when it already exists.
  `PATCH` body `{ alias?, paused? }`; an empty alias clears it. Status is derived from real coverage and activity:
  `paused`, `unsupported_protocol` (no enabled protocol on that chain), `waiting_for_activity` (no swaps yet),
  `partial_monitoring`, `active`.
- **Alert preferences:** `{ wallet_buy, wallet_sell, risk_event, signal }`, all booleans, default true. Alerts of a switched-off
  type are not returned by `GET /alerts`. `GET /alerts?limit=` accepts 1 to 50.
- **Pagination:** `cursor` is opaque. Discovery wallets and wallet trades use an offset cursor, alerts and signals a keyset cursor.
- **TokenSummary** (compare): `{ token, price_usd, change_pct_24h, market_cap_usd|null, volume_24h_usd, liquidity_usd, risk, note|null }`.
  Compare takes `a`, `b`, optional `c`, `d` as `chain:address`. A token with no market data makes the request 404.
- **Token risk checks** may also include `quote_freshness`. `risk.policy_version` is returned.
- **Top Movers:** `coverage_note` states what was excluded (stale, thin liquidity, invalid price, blocked by risk checks).
  A token with no explainable reason is not listed.
- **Discovery wallets:** `top` ranks only wallets that could be scored. `new` and `most_followed` also list wallets with
  `decision: insufficient_evidence`. A wallet that looks like a bot is tagged `possible_bot` and capped at `watch_only`.
- **Wallet `wcs`** is `null` until follower quotes exist. Wallet endpoints for an address with no history return 200 with
  `data_status: unavailable`, never zeros.
- **Shadow strategies:** `size_rule` is `fixed_pct` (5% of budget per trade) or `fixed_usd` (5% of budget as dollars).
  `exit_rule` is `mirror_sells`, `time_24h` or `time_exit_7d`. `budget_usd` is a decimal string or number in (0, 1,000,000].
  `delay_seconds` 0 to 3600, `slippage_bps` 0 to 2000. Other values are 400. `GET /shadow-strategies` lists your strategies.
  Results add `skipped` (with reasons), `open_positions`, `unmarked_open_positions`, and `baseline = { label, return_pct,
  ideal_leader_execution_return_pct, note }` which is a comparison only. Only final trades are replayed.
- **Signals:** `GET /signals/{id}` returns the receipt, `receipt_hash` and events. A receipt whose hash no longer matches is
  withheld: the detail call fails with 500 and the list marks the item `integrity: "failed"`. Outcomes list all four
  horizons with `status` `recorded | pending | awaiting_evaluation` and a `summary` that counts losses, unpriced and untradeable.
- **Demo mode:** with `TRACE_SEED_DEMO=true` the API seeds synthetic tokens, wallets, swaps, prices, snapshots, signals and
  starter alerts, all flagged `demo`. Responses built from them have `data_status: "demo"` and `demo: true`.
