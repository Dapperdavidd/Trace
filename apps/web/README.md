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

## Design

The UI is a direct port of `design/trace-redesign-v11.html` (the approved Claude design). `app/design.css`
is that file's stylesheet, unchanged; `app/shell.css` only turns the prototype's desktop and phone frames into a
responsive full-screen app. The three.js block field and token trace chart live in `lib/trace/engine.ts`.

Every screen asks the live API first. When the API has nothing yet (no market snapshots, empty lists,
unreachable), the screen shows the design's fictional example data and labels it **Example**. Live data is
labelled **Live**.

## Routes

`/` landing, `/welcome/` onboarding and Google sign-in, and the app at `/app/<screen>/`:
`today`, `briefing`, `discover`, `token`, `compare`, `ask`, `wallets`, `profile`, `shadow`, `copy`, `swap`,
`wallet`, `near`, `me`, `notifs`, `security`, `following`, `prefs`. Coins and wallets are passed as query
parameters (`?chain=&address=` for live, `?sym=` or `?w=` for examples), so the whole site is a static export.

`copy` runs the NEAR mandate flow: Google account, shadow test, connect NEAR wallet, prepare the draft,
sign it in the wallet, verify the anchor. `swap` requests dry NEAR Intents quotes under a signed mandate.

## Not live yet

Wallet balances (trace never holds funds, so balances shown in Wallet and Today are labelled Example),
auto copy (needs the NEAR delegation review), executed swaps (quotes are dry previews; the owner's wallet signs),
currencies other than USD, languages other than English, push notifications.

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

`app/` routes and styles, `components/trace/` the app frame, screens, landing and onboarding, `lib/trace/` view
models, example data, NEAR flow and the 3D engine, `lib/api/` types, client, guards and demo fixtures,
`tests/` vitest suites, `design/` the source design file.
