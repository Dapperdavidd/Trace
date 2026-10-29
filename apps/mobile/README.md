# TRACE mobile (Flutter)

Flutter client for TRACE, a discovery-first crypto research wallet. This build is
**watch-only research and paper trading**. It has no spendable wallet, no signing,
no live trades and no automatic trading.

Visual language: Emerald Green on Charcoal Black (see `lib/core/tokens.dart`).
Design source: the 14-screen mobile mockup. API source of truth:
`docs/api-contract.md` (base path `/v1`).

## First run

The `android/` and `ios/` folders are not committed. Generate them once:

```sh
cd apps/mobile
flutter create . --platforms=ios,android
flutter pub get
```

Run against the Rust backend (Android emulator reaches the host at 10.0.2.2):

```sh
flutter run --dart-define=TRACE_API_URL=http://10.0.2.2:8080
```

Run with demo fixtures only (no network calls at all):

```sh
flutter run --dart-define=TRACE_DEMO=true
```

Google Sign-In (optional, otherwise the button shows a "not configured" state):

```sh
flutter run \
  --dart-define=TRACE_API_URL=https://api.example.com \
  --dart-define=TRACE_GOOGLE_CLIENT_ID=<ios-or-web-client-id> \
  --dart-define=TRACE_GOOGLE_SERVER_CLIENT_ID=<web-client-id-used-by-the-backend>
```

Platform setup for `google_sign_in` (URL scheme on iOS, SHA-1 on Android) is
standard and must be done after `flutter create`. The session token is stored with
`flutter_secure_storage`. On iOS use `http://localhost:8080` or HTTPS; on Android
cleartext HTTP to a non-localhost host needs a network security config.

Tests (pure Dart logic plus API client with a mock HTTP client):

```sh
flutter test
flutter analyze
```

## Build-time flags

| Flag | Default | Meaning |
| --- | --- | --- |
| `TRACE_API_URL` | `http://10.0.2.2:8080` | Backend base URL (the app appends `/v1`) |
| `TRACE_DEMO` | `false` | Force demo fixtures, never call the network |
| `TRACE_GOOGLE_CLIENT_ID` | empty | Google client id (iOS or web) |
| `TRACE_GOOGLE_SERVER_CLIENT_ID` | empty | Server client id so Google returns an ID token |

## Real versus demo

- **Real:** everything in `lib/api/api_client.dart` talks to the contract endpoints
  when the backend is reachable. Values are shown as the server sends them. The
  client never computes authoritative PnL or scores.
- **Demo:** `lib/api/demo_api.dart` holds fictional fixtures (invented token
  names, addresses and numbers modelled on the mockup). Every fixture is flagged
  `data_status: "demo"`, `demo: true`.
- **Fallback rule** (`lib/api/fallback_policy.dart`): demo data is used when
  `TRACE_DEMO=true`, or when the API is unreachable (network error, timeout, HTTP
  502/503/504). Real application errors (400, 401, 403, 404, 409, 422, 429, 500)
  are shown as errors and are never hidden behind demo data.
- A yellow **DEMO DATA** banner appears on every screen while demo data is in use,
  and each response also shows its own status bar (LIVE, STALE, PARTIAL,
  UNAVAILABLE, DEMO DATA) with the as-of time. A missing or unknown status is
  treated as unavailable, never as live.

## Screens

Welcome, Home, Top Movers, Token Research, Smart Wallet profile (with Trading DNA
and the `insufficient_history` state), Shadow setup, Shadow Trades results,
Opportunity Timeline, Risk Radar, Compare Coins, Daily Briefing, Watchlist (with
chain-specific address validation), Alerts, More, plus the Coming later screens
Wallet, Trade & Swap and Ask TRACE. Bottom navigation: Home, Market, Shadow,
Watchlist, More.

Design changes made on purpose, to stay honest with the MVP scope:

- "Copy" becomes **Shadow** (paper simulation via `POST /shadow-strategies`) or
  **Watch** (watchlist). Copy risk controls become simulation assumptions.
- Home shows paper results instead of copy P&L, and no balances anywhere.
- Facts, hypotheses, estimates, model assessments and risk warnings are labelled
  separately. There is no profit or win-probability language.

## Coming later (visible but disabled, with a one-line reason)

Create wallet, Send, Receive, Buy, Swap, wallet balances, Auto Copy and live copy
trading, Ask TRACE natural-language chat, push notifications, news and social
feed, token price charts (the contract has no price series). Alerts are in-app
polling of `GET /alerts` every 30 seconds while the Alerts screen is open.

## Accessibility

Semantics labels on cards, status badges and charts; 48dp minimum touch targets
(theme minimum sizes, `InkWell` constraints, padded tap targets); layouts reflow
at large text scale (stat grid becomes one column above 1.3x); sparkline
animation is disabled when the system asks for reduced motion; status is never
conveyed by color alone (every badge has text).

## Layout

```
lib/api        models, ApiClient, DemoApi, error envelope, fallback policy
lib/core       config, tokens, theme, formatters, address validation, copy
lib/state      Riverpod providers, session (secure storage, Google), FallbackApi
lib/widgets    shared UI (scaffold, banners, pills, tiles, async states)
lib/screens    one file per screen group
test/          unit tests
```

## Contract ambiguities and assumptions

- `TokenSummary` (compare) is undefined. The client assumes it mirrors token
  detail headline fields and accepts `change_pct` or `change_pct_24h`, and `risk`
  as a string or `{ overall }`.
- `POST /watchlists` response body is unspecified. The client ignores it and
  refetches `GET /watchlists`.
- Shadow request wire values are unspecified. The client sends `budget_usd` as a
  decimal string, `size_rule` as `fixed_usd` or `fixed_pct`, `exit_rule` as
  `mirror_sells`, `time_24h` or `time_exit_7d` (the API refuses other values).
- `baseline` and `assumptions` in shadow results are free-form. The client shows
  `baseline.label` and `baseline.return_pct` when present and lists assumptions as
  key and value pairs. There is no equity-curve series, so no chart is drawn.
- Wallet tags and win rate exist only in discovery rows, so the profile shows them
  only when opened from a list.
- Token price history is not in the contract, so Token Research has no chart.
- `GET /signals*` and `GET /chains` are not used by the UI yet.
- `/me/wallets` is not used: the client treats a signed-in user as watch-only.

## Verification status

Written without a Flutter or Dart toolchain. In the authoring environment
`which flutter dart` found nothing and outbound access to the Dart SDK download
host and pub.dev was blocked (HTTP 403 from the proxy), so no SDK could be
installed.

NOT run: `flutter pub get`, `flutter analyze`, `flutter test`, `flutter run`, any
build, any device or emulator, any Google Sign-In flow, and no live call to the
Rust backend. **No test in this repository has been executed.** The tests were
written carefully and their expected values were checked by hand, but they may
contain mistakes.

Done by hand only: a scripted check that all `.dart` files are ASCII only and
have balanced brackets, a scan for em dash characters, and manual review of
imports, null safety and const usage. Expect to fix a few compile or lint issues
on the first `flutter analyze`. Dependency versions in `pubspec.yaml` were
chosen from memory and are unresolved.
