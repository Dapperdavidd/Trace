/**
 * Deterministic DEMO fixtures for every endpoint. Everything returned here carries
 * demo: true and data_status "demo". Token names are obviously fictional.
 * Times are derived from the `now` argument so a given clock always yields the same output.
 */
import type {
  AlertItem,
  AlertPreferences,
  Chain,
  DiscoveredWallet,
  MoverItem,
  RiskCheck,
  RiskState,
  ShadowStrategyBody,
  TokenRef,
  TokenSummary,
  WalletRef,
  WalletTrade,
  WatchlistItem,
} from "./types";

export type DemoResult = { found: true; data: unknown } | { found: false };

const B58 = "123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";
const HEX = "0123456789abcdef";

function lcg(seed: number): () => number {
  let s = (seed * 2654435761) >>> 0;
  return () => {
    s = (Math.imul(s, 1664525) + 1013904223) >>> 0;
    return s / 4294967296;
  };
}

export function fakeSolAddress(seed: number, prefix = "", suffix = ""): string {
  const r = lcg(seed);
  const len = 44;
  let out = prefix;
  while (out.length < len - suffix.length) out += B58[Math.floor(r() * B58.length)];
  return out + suffix;
}

export function fakeEvmAddress(seed: number): string {
  const r = lcg(seed);
  let out = "0x";
  for (let i = 0; i < 40; i++) out += HEX[Math.floor(r() * 16)];
  return out;
}

interface DemoToken {
  ref: TokenRef;
  price: string;
  change24h: number;
  volume: number;
  liquidity: number;
  marketCap: number | null;
  holders: number | null;
  pool: string;
  reason: { code: string; label: string };
  risk: RiskState;
  riskNotes: string[];
  checks: RiskCheck[];
  why: Array<{ kind: "fact" | "hypothesis"; text: string; ids: string[] }>;
  seed: number;
}

function check(key: RiskCheck["key"], state: RiskState, note: string): RiskCheck {
  return { key, state, note };
}

const TOKENS: DemoToken[] = [
  {
    ref: { chain: "solana", address: fakeSolAddress(11, "NEURA", "demo"), symbol: "NEURA", name: "Neura (demo token)" },
    price: "0.0084", change24h: 42.7, volume: 12_400_000, liquidity: 3_100_000, marketCap: 8_400_000, holders: 4382,
    pool: "Demo DEX pool NEURA/USDC", reason: { code: "smart_wallets_buying", label: "Smart wallets buying" },
    risk: "warn", riskNotes: ["A few wallets hold a large share of supply."],
    checks: [
      check("liquidity", "pass", "Pool liquidity is above the minimum depth filter."),
      check("honeypot", "pass", "A sell simulation did not find a restriction."),
      check("holder_concentration", "warn", "The top 10 holders own a large share of supply."),
      check("contract_safety", "pass", "No known risky contract functions were found."),
      check("suspicious_activity", "pass", "No suspicious activity was detected in the checked window."),
    ],
    why: [
      { kind: "fact", text: "3 wallets with a reviewed track record bought this token in the last hour.", ids: ["demo_tx_01", "demo_tx_02", "demo_tx_03"] },
      { kind: "fact", text: "Trading volume is about 4x the 7 day average.", ids: ["demo_vol_01"] },
      { kind: "hypothesis", text: "Buying by these wallets may be drawing attention from other traders. This is a hypothesis, not a confirmed cause.", ids: [] },
    ],
    seed: 1,
  },
  {
    ref: { chain: "solana", address: fakeSolAddress(12, "SxLX", "demo"), symbol: "SOLX", name: "Solx (demo token)" },
    price: "0.132", change24h: 28.4, volume: 8_100_000, liquidity: 4_200_000, marketCap: 12_000_000, holders: 9120,
    pool: "Demo DEX pool SOLX/USDC", reason: { code: "volume_spike", label: "High volume spike" },
    risk: "pass", riskNotes: [],
    checks: [
      check("liquidity", "pass", "Pool liquidity is above the minimum depth filter."),
      check("honeypot", "pass", "A sell simulation did not find a restriction."),
      check("holder_concentration", "pass", "Supply is spread across many holders."),
      check("contract_safety", "pass", "No known risky contract functions were found."),
      check("suspicious_activity", "pass", "No suspicious activity was detected in the checked window."),
    ],
    why: [
      { kind: "fact", text: "Volume in the last 24 hours is far above its recent average.", ids: ["demo_vol_02"] },
      { kind: "hypothesis", text: "A short burst of larger trades may explain the move. This has not been confirmed.", ids: [] },
    ],
    seed: 2,
  },
  {
    ref: { chain: "base", address: fakeEvmAddress(13), symbol: "FROG", name: "Frog (demo token)" },
    price: "0.0031", change24h: 19.1, volume: 6_300_000, liquidity: 2_400_000, marketCap: 5_700_000, holders: 15210,
    pool: "Demo DEX pool FROG/WETH", reason: { code: "large_buys", label: "Large wallet buys" },
    risk: "pass", riskNotes: [],
    checks: [
      check("liquidity", "pass", "Pool liquidity is above the minimum depth filter."),
      check("honeypot", "pass", "A sell simulation did not find a restriction."),
      check("holder_concentration", "pass", "Supply is spread across many holders."),
      check("contract_safety", "pass", "No known risky contract functions were found."),
      check("suspicious_activity", "pass", "No suspicious activity was detected in the checked window."),
    ],
    why: [
      { kind: "fact", text: "Two of the largest buys in the last 24 hours came from watched wallets.", ids: ["demo_tx_11", "demo_tx_12"] },
    ],
    seed: 3,
  },
  {
    ref: { chain: "ethereum", address: fakeEvmAddress(14), symbol: "NOVA", name: "Nova (demo token)" },
    price: "0.21", change24h: 15.8, volume: 4_900_000, liquidity: 1_100_000, marketCap: 21_000_000, holders: 6004,
    pool: "Demo DEX pool NOVA/WETH", reason: { code: "buyer_count_rising", label: "Rising buyer count" },
    risk: "warn", riskNotes: ["Liquidity is thin compared with daily volume."],
    checks: [
      check("liquidity", "warn", "Liquidity is thin compared with daily volume."),
      check("honeypot", "pass", "A sell simulation did not find a restriction."),
      check("holder_concentration", "pass", "Supply is spread across many holders."),
      check("contract_safety", "pass", "No known risky contract functions were found."),
      check("suspicious_activity", "unknown", "Not enough data to check for suspicious activity."),
    ],
    why: [
      { kind: "fact", text: "The number of distinct buyers rose in the last 6 hours.", ids: ["demo_buyers_01"] },
      { kind: "hypothesis", text: "Thin liquidity may be exaggerating the price move. This is a model assessment, not a confirmed cause.", ids: [] },
    ],
    seed: 4,
  },
  {
    ref: { chain: "bnb", address: fakeEvmAddress(15), symbol: "ORBIT", name: "Orbit (demo token)" },
    price: "0.045", change24h: 12.3, volume: 3_100_000, liquidity: 900_000, marketCap: null, holders: null,
    pool: "Demo DEX pool ORBIT/WBNB", reason: { code: "unusual_activity", label: "Unusual activity" },
    risk: "unknown", riskNotes: ["Risk checks could not be completed for this token."],
    checks: [
      check("liquidity", "warn", "Liquidity is low."),
      check("honeypot", "unknown", "The sell simulation could not run."),
      check("holder_concentration", "unknown", "Holder data is unavailable."),
      check("contract_safety", "unknown", "Contract data is unavailable."),
      check("suspicious_activity", "unknown", "Not enough data to check for suspicious activity."),
    ],
    why: [{ kind: "fact", text: "Trading activity is higher than this token's recent baseline.", ids: ["demo_act_01"] }],
    seed: 5,
  },
];

const WINDOW_FACTOR: Record<string, number> = { "1h": 0.18, "24h": 1, "7d": 2.4 };

function sparkline(seed: number, up: boolean): number[] {
  const r = lcg(seed * 97);
  const pts: number[] = [];
  let v = 40;
  for (let i = 0; i < 24; i++) {
    v += (up ? 1.6 : -1.6) + (r() - 0.5) * 7;
    pts.push(Math.round(v * 10) / 10);
  }
  return pts;
}

function minutesAgo(now: Date, mins: number): string {
  const base = new Date(Math.floor(now.getTime() / 60000) * 60000);
  return new Date(base.getTime() - mins * 60000).toISOString();
}

function envelope(now: Date, note?: string) {
  return {
    data_status: "demo" as const,
    as_of: minutesAgo(now, 0),
    demo: true as const,
    coverage_note: note ?? "Demo fixtures. Not real market data.",
  };
}

/* -------------------------------- wallets ---------------------------------- */

interface DemoWallet {
  ref: WalletRef;
  ret: string | null;
  win: string | null;
  sample: number;
  tags: string[];
  decision: DiscoveredWallet["decision"];
  conf: DiscoveredWallet["pnl_confidence"];
  coverage: string | null;
  dna: "ok" | "insufficient_history";
  realized: string | null;
  unrealized: string | null;
  trades: number;
}

const WALLETS: DemoWallet[] = [
  {
    ref: { chain: "solana", address: fakeSolAddress(21, "7a2", "9kP"), alias: "0xSun" },
    ret: "1324.0", win: "78", sample: 432, tags: ["Early Buyer", "Holds 1-7d"], decision: "eligible",
    conf: "medium", coverage: "0.91", dna: "ok", realized: "84210.55", unrealized: "12044.10", trades: 432,
  },
  {
    ref: { chain: "solana", address: fakeSolAddress(22, "3mB", "2dQ"), alias: "CryptoNomad" },
    ret: "892.0", win: "72", sample: 318, tags: ["Swing Trader", "Large Caps"], decision: "eligible",
    conf: "high", coverage: "0.96", dna: "ok", realized: "52980.00", unrealized: "3300.42", trades: 318,
  },
  {
    ref: { chain: "base", address: fakeEvmAddress(23), alias: "DeFiWhale" },
    ret: "684.0", win: "66", sample: 9, tags: ["Mid Caps", "Holds 3-14d"], decision: "watch_only",
    conf: "low", coverage: "0.58", dna: "insufficient_history", realized: null, unrealized: "1204.00", trades: 9,
  },
  {
    ref: { chain: "ethereum", address: fakeEvmAddress(24), alias: "AlphaHunter" },
    ret: "512.0", win: "71", sample: 205, tags: ["Memecoins", "Early Entry"], decision: "eligible",
    conf: "medium", coverage: "0.84", dna: "ok", realized: "31870.20", unrealized: "-420.15", trades: 205,
  },
];

function findWallet(chain: string, address: string): DemoWallet | undefined {
  return WALLETS.find((w) => w.ref.chain === chain && w.ref.address === address);
}

function findToken(chain: string, address: string): DemoToken | undefined {
  return TOKENS.find((t) => t.ref.chain === chain && t.ref.address === address);
}

function walletTrades(w: DemoWallet, now: Date): WalletTrade[] {
  const rows: Array<[number, "buy" | "sell", number, string | null, "tentative" | "final"]> = [
    [3, "buy", 0, "12450", "tentative"],
    [28, "buy", 1, "8920", "final"],
    [60, "sell", 2, "15300", "final"],
    [120, "buy", 3, "6200", "final"],
    [1500, "buy", 4, null, "final"],
  ];
  return rows.map(([mins, side, ti, amt, fin], i) => ({
    tx_id: `demo_tx_${w.ref.address.slice(0, 6)}_${i}`,
    event_time: minutesAgo(now, mins),
    side,
    token: (TOKENS[ti] as DemoToken).ref,
    amount_usd: amt,
    finality: fin,
  }));
}

function walletDetail(chain: string, address: string, now: Date) {
  const w = findWallet(chain, address);
  if (!w) {
    return {
      ...envelope(now, "Demo fixtures. This address is not in the demo set, so no activity is shown."),
      wallet: { chain, address, alias: null },
      first_seen: null,
      last_seen: null,
      monitoring: "waiting_for_activity",
      metrics: { realized_pnl_usd: null, unrealized_pnl_usd: null, total_trades: 0, coverage_ratio: null, pnl_confidence: "insufficient", unresolved_disposals: 0 },
    };
  }
  return {
    ...envelope(now),
    wallet: w.ref,
    first_seen: minutesAgo(now, 60 * 24 * 160),
    last_seen: minutesAgo(now, 3),
    monitoring: "active",
    metrics: {
      realized_pnl_usd: w.realized,
      unrealized_pnl_usd: w.unrealized,
      total_trades: w.trades,
      coverage_ratio: w.coverage,
      pnl_confidence: w.conf,
      unresolved_disposals: w.conf === "low" ? 4 : 1,
    },
  };
}

/* ------------------------------ mutable state ------------------------------ */

const watchlist: WatchlistItem[] = [];
let watchlistSeeded = false;
let prefs: AlertPreferences = { wallet_buy: true, wallet_sell: true, risk_event: true, signal: false };
const strategies = new Map<string, ShadowStrategyBody>();
let strategyCounter = 0;

function seedWatchlist(now: Date) {
  if (watchlistSeeded) return;
  watchlistSeeded = true;
  const [a, b] = WALLETS;
  if (a && b) {
    watchlist.push(
      { id: "demo_watch_1", wallet: a.ref, status: "active", last_checked_at: minutesAgo(now, 2) },
      { id: "demo_watch_2", wallet: b.ref, status: "partial_monitoring", last_checked_at: minutesAgo(now, 9) },
    );
  }
}

/** Test helper: reset the mutable demo state. */
export function resetDemoFixtures() {
  watchlist.length = 0;
  watchlistSeeded = false;
  strategies.clear();
  strategyCounter = 0;
  prefs = { wallet_buy: true, wallet_sell: true, risk_event: true, signal: false };
}

/* --------------------------------- router ---------------------------------- */

type Query = Record<string, string | number | undefined | null>;

function str(q: Query, k: string): string | undefined {
  const v = q[k];
  return v === undefined || v === null || v === "" ? undefined : String(v);
}

function moverItems(tab: string, window: string, chain: string | undefined, now: Date): MoverItem[] {
  const subsets: Record<string, string[]> = {
    trending: ["NEURA", "SOLX", "FROG", "NOVA", "ORBIT"],
    early_momentum: ["NEURA", "FROG", "ORBIT"],
    smart_money: ["NEURA", "FROG", "NOVA"],
    new: ["ORBIT", "NOVA"],
  };
  const symbols = subsets[tab] ?? subsets.trending ?? [];
  const f = WINDOW_FACTOR[window] ?? 1;
  const picked = symbols
    .map((s) => TOKENS.find((t) => t.ref.symbol === s))
    .filter((t): t is DemoToken => !!t)
    .filter((t) => !chain || t.ref.chain === chain);
  return picked.map((t, i) => ({
    rank: i + 1,
    token: t.ref,
    price_usd: t.price,
    change_pct: (Math.round(t.change24h * f * 10) / 10).toFixed(1),
    volume_usd: String(Math.round(t.volume * (window === "1h" ? 0.1 : window === "7d" ? 4.5 : 1))),
    liquidity_usd: String(t.liquidity),
    pool: t.pool,
    reason: t.reason,
    sparkline: sparkline(t.seed, true),
    risk: t.risk,
    risk_notes: t.riskNotes,
    source_timestamp: minutesAgo(now, 1 + i),
  }));
}

function tokenSummary(t: DemoToken): TokenSummary {
  return {
    token: t.ref,
    price_usd: t.price,
    change_pct_24h: t.change24h.toFixed(1),
    market_cap_usd: t.marketCap === null ? null : String(t.marketCap),
    volume_24h_usd: String(t.volume),
    liquidity_usd: String(t.liquidity),
    risk: t.risk,
    note: t.riskNotes[0] ?? null,
  };
}

function notFound(): DemoResult {
  return { found: false };
}

function parseRef(v: string | undefined): { chain: string; address: string } | null {
  if (!v) return null;
  const i = v.indexOf(":");
  if (i < 0) return null;
  return { chain: v.slice(0, i), address: v.slice(i + 1) };
}

export function demoResponse(
  method: string,
  rawPath: string,
  query: Query,
  body: unknown,
  now: Date = new Date(),
): DemoResult {
  const path = rawPath.replace(/^\/v1/, "");
  const ok = (data: unknown): DemoResult => ({ found: true, data });
  let m: RegExpMatchArray | null;

  if (method === "GET" && path === "/discovery/movers") {
    const items = moverItems(str(query, "tab") ?? "trending", str(query, "window") ?? "24h", str(query, "chain"), now);
    return ok({ ...envelope(now), items });
  }

  if (method === "GET" && path === "/discovery/wallets") {
    const tab = str(query, "tab") ?? "top";
    let list = WALLETS.slice();
    if (tab === "new") list = [WALLETS[2], WALLETS[3]].filter((w): w is DemoWallet => !!w);
    if (tab === "most_followed") list = [WALLETS[1], WALLETS[0], WALLETS[3], WALLETS[2]].filter((w): w is DemoWallet => !!w);
    const chain = str(query, "chain");
    if (chain) list = list.filter((w) => w.ref.chain === chain);
    const items: DiscoveredWallet[] = list.map((w) => ({
      wallet: w.ref,
      realized_return_pct: w.ret,
      win_rate_pct: w.win,
      sample_size: w.sample,
      tags: w.tags,
      decision: w.decision,
      pnl_confidence: w.conf,
      coverage_ratio: w.coverage,
    }));
    return ok({ ...envelope(now), items, next_cursor: null });
  }

  if ((m = path.match(/^\/wallets\/([^/]+)\/([^/]+)$/)) && method === "GET") {
    return ok(walletDetail(m[1] as string, decodeURIComponent(m[2] as string), now));
  }
  if ((m = path.match(/^\/wallets\/([^/]+)\/([^/]+)\/trades$/)) && method === "GET") {
    const w = findWallet(m[1] as string, decodeURIComponent(m[2] as string));
    return ok({ ...envelope(now), items: w ? walletTrades(w, now) : [], next_cursor: null });
  }
  if ((m = path.match(/^\/wallets\/([^/]+)\/([^/]+)\/metrics$/)) && method === "GET") {
    const w = findWallet(m[1] as string, decodeURIComponent(m[2] as string));
    const d = walletDetail(m[1] as string, decodeURIComponent(m[2] as string), now);
    return ok({
      ...envelope(now),
      ...d.metrics,
      wqs: {
        decision: w?.decision ?? "insufficient_evidence",
        value: w ? (w.conf === "low" ? 41 : 72) : null,
        version: "wqs-demo-1",
        components: { consistency: "0.71", sample_size: w && w.sample < 20 ? "0.12" : "0.78", data_coverage: w?.coverage ?? "0" },
        cohort: "demo_active_traders",
        window: "90d",
        explanation: "Demo score. It summarises how consistent the observed trades were, with a penalty for small samples. It is not a percentage of profit.",
      },
      wcs: w && w.decision === "eligible"
        ? {
            decision: "copyable",
            value: 64,
            version: "wcs-demo-1",
            components: { follower_price_gap: "0.63", delay_sensitivity: "0.55" },
            cohort: "demo_active_traders",
            window: "30d",
            explanation: "Demo score. It estimates whether a follower could still get a similar price after a delay. It is an estimate, not a promise.",
          }
        : null,
    });
  }
  if ((m = path.match(/^\/wallets\/([^/]+)\/([^/]+)\/dna$/)) && method === "GET") {
    const w = findWallet(m[1] as string, decodeURIComponent(m[2] as string));
    const ok_ = !!w && w.dna === "ok";
    return ok({
      ...envelope(now),
      state: ok_ ? "ok" : "insufficient_history",
      rules_version: "dna-rules-demo-1",
      sample_size: w?.sample ?? 0,
      traits: ok_
        ? [
            { key: "early_buyer", label: "Early Buyer", evidence: "Bought within the first 2 hours of pool activity in 9 of 14 reviewed tokens." },
            { key: "holds_short", label: "Holds 1 to 7 days", evidence: "Median holding time across closed positions was 2.8 days." },
            { key: "size_bands", label: "Mid sized positions", evidence: "Most buys fell between $3K and $15K." },
          ]
        : [],
    });
  }

  if ((m = path.match(/^\/tokens\/([^/]+)\/([^/]+)$/)) && method === "GET" && m[1] !== "compare") {
    const t = findToken(m[1] as string, decodeURIComponent(m[2] as string));
    if (!t) return notFound();
    return ok({
      ...envelope(now),
      token: t.ref,
      price_usd: t.price,
      change_pct_24h: t.change24h.toFixed(1),
      market_cap_usd: t.marketCap === null ? null : String(t.marketCap),
      volume_24h_usd: String(t.volume),
      liquidity_usd: String(t.liquidity),
      holders: t.holders,
      risk: { overall: t.risk, checks: t.checks, fetched_at: minutesAgo(now, 4) },
      why_moving: t.why.map((w) => ({ kind: w.kind, text: w.text, evidence_ids: w.ids })),
      buyers: WALLETS.filter((_, i) => i < 3).map((w) => w.ref),
    });
  }
  if ((m = path.match(/^\/tokens\/([^/]+)\/([^/]+)\/timeline$/)) && method === "GET") {
    const t = findToken(m[1] as string, decodeURIComponent(m[2] as string));
    if (!t) return notFound();
    return ok({
      ...envelope(now),
      items: [
        { time: minutesAgo(now, 180), kind: "fact", title: "Unusual activity", detail: "Volume rose above its normal level.", evidence_ids: ["demo_vol_01"], possible_impact: null },
        { time: minutesAgo(now, 166), kind: "fact", title: "Smart wallets buying", detail: "3 wallets with a reviewed track record made purchases.", evidence_ids: ["demo_tx_01", "demo_tx_02", "demo_tx_03"], possible_impact: "Buying pressure can lift the price in the short term." },
        { time: minutesAgo(now, 158), kind: "hypothesis", title: "Attention may have increased", detail: "More traders may have noticed the token. This is a hypothesis and is not confirmed by on-chain data.", evidence_ids: [], possible_impact: "Possible follow-on buying, or a quick reversal." },
        { time: minutesAgo(now, 115), kind: "fact", title: "Price started moving", detail: "Price rose 18.4% in 45 minutes.", evidence_ids: ["demo_px_01"], possible_impact: null },
        { time: minutesAgo(now, 0), kind: "fact", title: "Now", detail: `Up ${t.change24h.toFixed(1)}% over 24 hours. Still active.`, evidence_ids: [], possible_impact: null },
      ],
    });
  }
  if (method === "GET" && path === "/tokens/compare") {
    const a = parseRef(str(query, "a"));
    const b = parseRef(str(query, "b"));
    const ta = a && findToken(a.chain, a.address);
    const tb = b && findToken(b.chain, b.address);
    if (!ta || !tb) return notFound();
    return ok({
      ...envelope(now),
      items: [tokenSummary(ta), tokenSummary(tb)],
      verdict_note:
        "Demo note. The token with the larger 24 hour move also shows the higher risk flags in this demo set. Compare liquidity and risk checks before drawing conclusions. This is not advice.",
    });
  }

  if (method === "POST" && path === "/shadow-strategies") {
    const id = `demo-${++strategyCounter}`;
    strategies.set(id, body as ShadowStrategyBody);
    return ok({ id });
  }
  if ((m = path.match(/^\/shadow-strategies\/([^/]+)\/results$/)) && method === "GET") {
    const id = decodeURIComponent(m[1] as string);
    if (!id.startsWith("demo")) return notFound();
    const strat = strategies.get(id);
    const start = strat ? Number(strat.budget_usd) || 100 : 100;
    const ret = 48.6;
    const end = Math.round(start * (1 + ret / 100) * 100) / 100;
    return ok({
      ...envelope(now, "Demo simulation. A fixed sample result, scaled to the budget you entered."),
      starting_usd: start.toFixed(2),
      ending_usd: end.toFixed(2),
      return_pct: ret.toFixed(1),
      trades: 12,
      win_rate_pct: "66.7",
      skipped_unexecutable: 2,
      fees_and_slippage_usd: (Math.round(start * 0.042 * 100) / 100).toFixed(2),
      baseline: { label: "Hold the same tokens from the first follow-up price", return_pct: "31.2" },
      assumptions: {
        delay_seconds: strat?.delay_seconds ?? 30,
        slippage_bps: strat?.slippage_bps ?? 50,
        size_rule: strat?.size_rule ?? "fixed_pct",
        exit_rule: strat?.exit_rule ?? "mirror_sells",
        price_source: "Follower-available price after detection",
      },
    });
  }

  if (method === "GET" && path === "/briefings") {
    return ok({
      ...envelope(now),
      sections: [
        { title: "New opportunities", items: [{ text: "3 coins show unusual buying activity.", link: "/top-movers" }] },
        { title: "Wallet moves to watch", items: [{ text: "2 watched wallets made new buys.", link: "/watchlist" }] },
        { title: "Risk alerts", items: [{ text: "1 coin on your lists has a risk warning.", link: "/risk-radar" }] },
        { title: "Yesterday's paper results", items: [{ text: "Your paper trades finished +8.4% (simulated).", link: "/shadow-trades" }] },
      ],
    });
  }

  seedWatchlist(now);
  if (method === "GET" && path === "/watchlists") return ok({ items: watchlist.slice() });
  if (method === "POST" && path === "/watchlists") {
    const b = body as { chain: Chain; address: string; alias?: string };
    let item = watchlist.find((w) => w.wallet.chain === b.chain && w.wallet.address === b.address);
    if (!item) {
      item = {
        id: `demo_watch_${watchlist.length + 1}`,
        wallet: { chain: b.chain, address: b.address, alias: b.alias ?? null },
        status: "waiting_for_activity",
        last_checked_at: null,
      };
      watchlist.push(item);
    }
    return ok(item);
  }
  if ((m = path.match(/^\/watchlists\/([^/]+)$/))) {
    const id = decodeURIComponent(m[1] as string);
    const idx = watchlist.findIndex((w) => w.id === id);
    if (idx < 0) return notFound();
    if (method === "DELETE") {
      watchlist.splice(idx, 1);
      return ok(undefined);
    }
    if (method === "PATCH") {
      const patch = body as { alias?: string; status?: "active" | "paused" };
      const cur = watchlist[idx] as WatchlistItem;
      const next: WatchlistItem = {
        ...cur,
        wallet: { ...cur.wallet, alias: patch.alias ?? cur.wallet.alias },
        status: patch.status ?? cur.status,
      };
      watchlist[idx] = next;
      return ok(next);
    }
  }
  if (path === "/alert-preferences") {
    if (method === "PATCH") prefs = { ...prefs, ...(body as Partial<AlertPreferences>) };
    return ok(prefs);
  }
  if (method === "GET" && path === "/alerts") {
    const [w0, w1] = WALLETS as [DemoWallet, DemoWallet, ...DemoWallet[]];
    const items: AlertItem[] = [
      { id: "demo_alert_1", type: "wallet_buy", title: `${w0.ref.alias} bought NEURA`, body: "Observed on chain. The transaction is still tentative.", evidence_tx: ["demo_tx_01"], event_time: minutesAgo(now, 3), published_at: minutesAgo(now, 2) },
      { id: "demo_alert_2", type: "wallet_sell", title: `${w1.ref.alias} sold FROG`, body: "Observed on chain and final.", evidence_tx: ["demo_tx_21"], event_time: minutesAgo(now, 60), published_at: minutesAgo(now, 58) },
      { id: "demo_alert_3", type: "risk_event", title: "NOVA liquidity dropped", body: "Pool liquidity fell below the warning level.", evidence_tx: [], event_time: minutesAgo(now, 180), published_at: minutesAgo(now, 175) },
    ];
    return ok({ items, next_cursor: null });
  }

  if (method === "GET" && path === "/me") {
    return ok({ id: "demo_user", email: "demo@example.invalid", name: "Demo user", wallets: { watch_only: watchlist.length, connected: 0 } });
  }
  if (method === "GET" && path === "/me/wallets") {
    return ok({ watch_only: watchlist.map((w) => w.wallet), connected: [], embedded: { available: false, reason: "Wallet creation is coming later." } });
  }
  if (method === "GET" && path === "/chains") {
    return ok({
      chains: [
        { chain: "solana", status: "demo", note: "Demo coverage list." },
        { chain: "ethereum", status: "demo" },
        { chain: "base", status: "demo" },
        { chain: "bnb", status: "demo" },
      ],
    });
  }
  return notFound();
}

export const DEMO_TOKENS: readonly TokenRef[] = TOKENS.map((t) => t.ref);
export const DEMO_WALLET_REFS: readonly WalletRef[] = WALLETS.map((w) => w.ref);
