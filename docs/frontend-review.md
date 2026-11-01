# Frontend review

Reviewed on 2026-11-01 against the supplied desktop dashboard and mobile screen montage.

## Before and after

Before this pass, the apps already used the dark emerald visual direction, but several surfaces still used the old product name, compact controls missed the 44 point target, coverage was not visible on the main surfaces, money strings could pass through floating point values, and mobile API errors did not show their request ID.

After this pass, both apps use the TRACE name, keep the dense dark card layout from the designs, expose freshness and data status on API-backed surfaces, show protocol coverage, preserve decimal money strings, show reportable API errors, and keep Wallet, Send, Receive, Buy, Swap, copy trading, and Ask TRACE visibly unavailable. Shadow is paper-only everywhere.

The designs contain balances, real trading, auto-copy, and AI chat actions that the current product does not provide. Those areas match the designs in hierarchy and styling, but deliberately show `Coming later` or `Unavailable` instead of pretending to work.

## Web screens

| Screen | Matches design | States covered | Notes |
| --- | --- | --- | --- |
| Home | Yes | loading, empty, error, offline, demo, partial, unavailable | Dense dashboard, honest wallet rail, freshness, and coverage panel. |
| Top Movers | Yes | loading, empty, error, offline, demo | Tabs, ranked rows, compact charts, risk, status, and freshness. |
| Smart Wallets | Yes | loading, empty, error, offline, demo | Confidence and evidence remain visible. Shadow replaces copy. |
| Wallet profile | Yes | loading, empty, error, offline, demo, partial | Public address only. No balance or custody claim. |
| Token research | Yes | loading, empty, error, offline, demo, unavailable | Exact decimal prices and observed versus hypothesis labels. |
| Token timeline | Yes | loading, empty, error, offline, demo | Facts and hypotheses remain distinct. |
| Timeline chooser | Yes | loading, empty, error, offline, demo | Chooses from available tokens without implying wider coverage. |
| Risk Radar | Yes | loading, empty, error, offline, demo, unavailable | Unknown checks stay unknown rather than passing. |
| Compare Coins | Yes | loading, empty, error, offline, demo, unavailable | Missing values read `Unavailable`. |
| Shadow Trades | Yes | loading, empty, error, offline, demo | Paper-only language is persistent. |
| Daily Briefing | Yes | loading, empty, error, offline, demo | Status and update time appear with the briefing. |
| Watchlist | Yes | loading, empty, error, offline, demo, partial | Monitoring status is explicit. |
| Alerts | Yes | loading, empty, error, offline, demo | API errors include request IDs. |
| Search | Yes | loading, empty, error, offline, demo | Results are limited to listed coins and public wallets. |
| Sign in | Yes | loading, API error, unconfigured, development-only | Google 503 is explained. Local sign-in requires the explicit dev flag. |
| Wallet | Yes | static coming-later state | No balance is invented. |
| Send | Yes | static coming-later state | No action is enabled. |
| Receive | Yes | static coming-later state | No address is invented. |
| Swap | Yes | static coming-later state | No transaction can be submitted. |
| Ask TRACE | Yes | static coming-later state | Example questions are disabled. |

Web captures are in [`docs/screens/web`](screens/web).

## Mobile screens

| Screen | Matches design | States covered | Notes |
| --- | --- | --- | --- |
| Welcome | Yes | signed out, unconfigured Google, development-only, demo | Watch-only promise replaces wallet creation claims. |
| Home | Yes | loading, empty, error, offline, demo, unavailable | Research and paper results lead the screen. |
| Top Movers | Yes | loading, empty, error, offline, demo | 44 point filters and tap targets. |
| Token Research | Yes | loading, empty, error, offline, demo, unavailable | Exact decimal amounts and evidence labels. |
| Smart Wallets | Yes | loading, empty, error, offline, demo | Confidence and small-sample warnings are visible. |
| Wallet profile | Yes | loading, empty, error, offline, demo, partial | Public address and observed behaviour only. |
| Shadow Trades | Yes | loading, empty, error, offline, demo | Paper-only statement stays above the flow. |
| Shadow setup | Yes | validation, submit error, demo | Real-money auto-copy remains disabled. |
| Shadow results | Yes | loading, empty, error, offline, demo | Clearly labelled simulated result and assumptions. |
| Watchlist | Yes | loading, empty, error, offline, demo, partial | Monitoring state and freshness are explicit. |
| Opportunity Timeline | Yes | loading, empty, error, offline, demo | Fact and hypothesis timeline matches the reference hierarchy. |
| Risk Radar | Yes | loading, empty, error, offline, demo, unavailable | Unknown is never converted to safe. |
| Compare Coins | Yes | loading, empty, error, offline, demo, unavailable | Exact amount formatting and cautious assessment. |
| Daily Briefing | Yes | loading, empty, error, offline, demo | Opportunities, wallet moves, risk, and paper results. |
| Alerts | Yes | loading, empty, error, offline, demo | Phone push remains coming later. |
| More and coverage | Yes | loading, error, offline, demo, partial | Only Ethereum Uniswap V2 is enabled. |
| Wallet | Yes | static coming-later state | No balance or custody action. |
| Trade and Swap | Yes | static coming-later state | Real-money controls are disabled. |
| Ask TRACE | Yes | static coming-later state | Input and examples remain disabled. |

Mobile captures are in [`docs/screens/mobile`](screens/mobile).

## Known intentional differences

- The supplied designs show real balances and trading controls. The implemented surfaces say `Unavailable` or `Coming later` because those capabilities do not exist.
- The supplied designs say Copy Trade in a few places. The implemented product uses Shadow and describes it as a paper simulation.
- Live market charts are not available from the API contract, so the UI does not fabricate them.
- Dark mode is the only design mode in the supplied references. Both clients consistently use it and support increased text size and keyboard or screen-reader navigation.
