# TRACE web client

Next.js 15 (App Router) and TypeScript web client for TRACE, a discovery-first crypto research wallet.
The MVP is **watch-only research and paper trading**. There is no spendable wallet, no signing, no live
trading and no automatic trading in this app.

## Run it

```bash
cd apps/web
cp .env.example .env.local   # optional
npm install
npm run dev                  # http://localhost:3000
npm run build && npm run start
```

Checks:

```bash
npx tsc --noEmit
npm run lint
npm test                     # vitest: formatting, address validation, API client, demo fixtures
```

## Environment variables (all `NEXT_PUBLIC_*`, read at build time)

| Variable | Default | Purpose |
| --- | --- | --- |
| `NEXT_PUBLIC_TRACE_API_URL` | `http://localhost:8080` | Base URL of the TRACE Rust API (the client appends `/v1`). |
| `NEXT_PUBLIC_TRACE_DEMO` | unset | Set to `1` to always serve labelled demo fixtures and never call the API. |
| `NEXT_PUBLIC_GOOGLE_CLIENT_ID` | unset | Google Identity Services web client id. When unset, the Sign-in page shows "Google sign-in not configured". |
| `NEXT_PUBLIC_TRACE_DEV_AUTH` | unset | Set to `1` only with a local API started using `TRACE_DEV_AUTH=true`; reveals local development sign-in. |

## Real versus demo

* All data comes from the typed client in `lib/api/client.ts` against `docs/api-contract.md`
  (types in `lib/api/types.ts`, runtime shape guards in `lib/api/guards.ts`).
* **Demo fallback.** If `NEXT_PUBLIC_TRACE_DEMO=1`, or a request cannot reach the API (network error or
  timeout), the client serves deterministic fixtures from `lib/api/demo.ts`. Every fixture has
  `demo: true` and `data_status: "demo"`, and fictional token names. HTTP error responses (4xx, 5xx) are
  never replaced by demo data. Sign-in is never faked.
* **Honesty UI.** A global DEMO DATA banner appears on every page as soon as any demo data is served or the
  API reports `demo`. Each data panel shows its own status badge (Live, Stale, Partial data, Data
  unavailable, DEMO DATA) with the `as_of` time. `unavailable` renders a "data unavailable" state, never a
  blank or a made-up number.
* **No client-side authority.** PnL, scores, risk states and returns are displayed exactly as the API returns
  them. The only client logic is formatting and address format checks.
* Observed facts, hypotheses and model assessments are labelled separately. No profit guarantees.
* Auth: the bearer token lives in memory and `sessionStorage` (every access wrapped in try/catch).

## Routes

`/` Home, `/top-movers`, `/smart-wallets`, `/wallets/[chain]/[address]`, `/tokens/[chain]/[address]`,
`/tokens/[chain]/[address]/timeline`, `/timeline`, `/risk-radar`, `/shadow-trades`, `/compare`, `/briefing`,
`/watchlist`, `/alerts`, `/sign-in`, `/search`, and the Coming Later pages `/wallet`, `/send`, `/receive`,
`/swap`, `/ask`.

## Coming later (shown disabled, never faked)

Wallet balance, Send, Receive, Buy, Swap, Create New Wallet, Connect Wallet, live copy trading ("Copy" is
"Shadow", a paper simulation), Ask TRACE (natural-language assistant), coin watchlist (the contract only
covers wallet watchlists), push notifications.

## Contract assumptions that need backend confirmation

* `TokenSummary` (compare) is not defined in the contract. Assumed: token, price, 24h change, market cap,
  volume, liquidity, overall risk state and an optional `note`.
* `size_rule` and `exit_rule` for `POST /shadow-strategies` are now defined by the API (see
  `docs/api-contract.md`): `fixed_pct | fixed_usd` and `mirror_sells | time_24h | time_exit_7d`. Other values are
  refused with 400. `budget_usd` is sent as a decimal string.
* `GET|PATCH /alert-preferences` shape is not defined. Assumed booleans per alert type.
* `PATCH /watchlists/{id}` body is not defined. Assumed `{ alias?, status?: "active" | "paused" }`.
* No endpoint lists a user's shadow strategies, so Paper Positions remembers strategy ids in this browser
  (localStorage) and fetches each result.
* Recent Activity on Home is built from `/wallets/{chain}/{address}/trades` of the top discovered wallets.
  A single activity feed endpoint would be cleaner.
* Mover `price_usd`, `change_pct`, `volume_usd` and `liquidity_usd` are assumed to be decimal strings.
  `coverage_ratio` is assumed to be a 0 to 1 ratio.
* `GET /chains` response shape is assumed. The shell calls it once as a reachability probe.
* There is no price history endpoint, so charts use the mover `sparkline` only.

## Layout

`app/` routes, `components/` shared UI, `lib/api/` types, client, guards, demo fixtures, `lib/format.ts`
formatters, `lib/address.ts` validation, `tests/` vitest suites, `docs/screens/` verification screenshots.
