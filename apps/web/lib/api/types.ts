/**
 * Types mirroring docs/api-contract.md (TRACE API contract v1).
 * Percentages and USD values are decimal strings. Amounts that can exceed 2^53 are strings.
 *
 * Where the contract leaves a shape open, the assumption is marked "ASSUMPTION" and is
 * listed in apps/web/README.md so the backend can confirm or change it.
 */

export type Chain = "solana" | "ethereum" | "base" | "bnb";
export const CHAINS: readonly Chain[] = ["solana", "ethereum", "base", "bnb"] as const;

export type DataStatus = "live" | "stale" | "unavailable" | "partial" | "demo";

export interface Envelope {
  data_status: DataStatus;
  /** ISO UTC timestamp */
  as_of: string;
  demo: boolean;
  coverage_note?: string;
}

export interface TokenRef {
  chain: Chain;
  address: string;
  symbol: string | null;
  name: string | null;
}

export interface WalletRef {
  chain: Chain;
  address: string;
  alias: string | null;
}

export type RiskState = "pass" | "warn" | "block" | "unknown";

/** Error envelope: `{ code, message, details?, request_id }` */
export interface ApiErrorBody {
  code: string;
  message: string;
  details?: unknown;
  request_id: string;
}

/* ------------------------------ Auth and account ------------------------------ */

export interface AuthSession {
  access_token: string;
  expires_at: string;
  user: { id: string; email: string; name: string };
}

export interface Me {
  id: string;
  email: string;
  name: string;
  wallets: { watch_only: number; connected: 0 };
}

export interface MyWallets {
  watch_only: WalletRef[];
  connected: [];
  embedded: { available: false; reason: string };
}

/* ---------------------------- Watchlist and alerts ---------------------------- */

export type WatchStatus =
  | "active"
  | "paused"
  | "partial_monitoring"
  | "unsupported_protocol"
  | "waiting_for_activity";

export interface WatchlistItem {
  id: string;
  wallet: WalletRef;
  status: WatchStatus;
  last_checked_at: string | null;
}

export interface WatchlistResponse {
  items: WatchlistItem[];
}

export interface AddWatchlistBody {
  chain: Chain;
  address: string;
  alias?: string;
}

/** ASSUMPTION: the contract does not define PATCH fields; alias and status are the obvious ones. */
export interface PatchWatchlistBody {
  alias?: string;
  status?: "active" | "paused";
}

/** ASSUMPTION: the contract names GET|PATCH /alert-preferences without a shape. */
export interface AlertPreferences {
  wallet_buy: boolean;
  wallet_sell: boolean;
  risk_event: boolean;
  signal: boolean;
}

export type AlertType = "wallet_buy" | "wallet_sell" | "risk_event" | "signal";

export interface AlertItem {
  id: string;
  type: AlertType;
  title: string;
  body: string;
  evidence_tx: string[];
  event_time: string;
  published_at: string;
}

export interface AlertsResponse {
  items: AlertItem[];
  next_cursor: string | null;
}

/* ------------------------------- Discovery ------------------------------------ */

export type MoverTab = "trending" | "early_momentum" | "smart_money" | "new";
export type MoverWindow = "1h" | "24h" | "7d";

export interface MoverItem {
  rank: number;
  token: TokenRef;
  price_usd: string;
  change_pct: string;
  volume_usd: string;
  liquidity_usd: string;
  pool: string;
  reason: { code: string; label: string };
  sparkline: number[];
  risk: RiskState;
  risk_notes: string[];
  source_timestamp: string;
}

export interface MoversResponse extends Envelope {
  items: MoverItem[];
}

export type WalletTab = "top" | "new" | "most_followed";
export type WalletDecision = "eligible" | "watch_only" | "insufficient_evidence" | "rejected";
export type PnlConfidence = "high" | "medium" | "low" | "insufficient";

export interface DiscoveredWallet {
  wallet: WalletRef;
  realized_return_pct: string | null;
  win_rate_pct: string | null;
  sample_size: number;
  tags: string[];
  decision: WalletDecision;
  pnl_confidence: PnlConfidence;
  coverage_ratio: string | null;
}

export interface DiscoveryWalletsResponse extends Envelope {
  items: DiscoveredWallet[];
  next_cursor: string | null;
}

/* ----------------------------- Wallet research -------------------------------- */

export interface Metrics {
  realized_pnl_usd: string | null;
  unrealized_pnl_usd: string | null;
  total_trades: number;
  coverage_ratio: string | null;
  pnl_confidence: string;
  unresolved_disposals: number;
}

export interface Score {
  decision: string;
  value: number | null;
  version: string;
  components: Record<string, string>;
  cohort: string;
  window: string;
  explanation: string;
}

export interface WalletDetail extends Envelope {
  wallet: WalletRef;
  first_seen: string | null;
  last_seen: string | null;
  monitoring: string;
  metrics: Metrics;
}

export interface WalletTrade {
  tx_id: string;
  event_time: string;
  side: "buy" | "sell";
  token: TokenRef;
  amount_usd: string | null;
  finality: "tentative" | "final";
}

export interface WalletTradesResponse extends Envelope {
  items: WalletTrade[];
  next_cursor: string | null;
}

export interface WalletMetricsResponse extends Envelope, Metrics {
  wqs: Score;
  wcs: Score | null;
}

export interface DnaTrait {
  key: string;
  label: string;
  evidence: string;
}

export interface WalletDna extends Envelope {
  state: "ok" | "insufficient_history";
  rules_version: string;
  sample_size: number;
  traits: DnaTrait[];
}

/* --------------------------------- Tokens ------------------------------------- */

export type RiskCheckKey =
  | "liquidity"
  | "honeypot"
  | "holder_concentration"
  | "contract_safety"
  | "suspicious_activity";

export interface RiskCheck {
  key: RiskCheckKey;
  state: RiskState;
  note: string;
}

export interface WhyMoving {
  kind: "fact" | "hypothesis";
  text: string;
  evidence_ids: string[];
}

export interface TokenDetail extends Envelope {
  token: TokenRef;
  price_usd: string;
  change_pct_24h: string;
  market_cap_usd: string | null;
  volume_24h_usd: string;
  liquidity_usd: string;
  holders: number | null;
  risk: { overall: RiskState; checks: RiskCheck[]; fetched_at: string };
  why_moving: WhyMoving[];
  buyers: WalletRef[];
}

export interface TimelineItem {
  time: string;
  kind: "fact" | "hypothesis";
  title: string;
  detail: string;
  evidence_ids: string[];
  possible_impact: string | null;
}

export interface TimelineResponse extends Envelope {
  items: TimelineItem[];
}

/** ASSUMPTION: the contract says `TokenSummary[2..4]` without defining it. */
export interface TokenSummary {
  token: TokenRef;
  price_usd: string;
  change_pct_24h: string;
  market_cap_usd: string | null;
  volume_24h_usd: string;
  liquidity_usd: string;
  risk: RiskState;
  /** short plain language note about this token, if the API provides one */
  note?: string | null;
}

export interface CompareResponse extends Envelope {
  items: TokenSummary[];
  verdict_note: string;
}

/* ------------------------ Signals and Shadow Wallets -------------------------- */

export interface ShadowStrategyBody {
  wallet: { chain: Chain; address: string };
  budget_usd: string;
  /** ASSUMPTION: free-form rule identifiers; the UI offers a fixed list. */
  size_rule: string;
  delay_seconds: number;
  slippage_bps: number;
  /** ASSUMPTION: free-form rule identifier. */
  exit_rule: string;
}

export interface ShadowStrategyCreated {
  id: string;
}

export interface ShadowResults extends Envelope {
  starting_usd: string;
  ending_usd: string;
  return_pct: string;
  trades: number;
  win_rate_pct: string | null;
  skipped_unexecutable: number;
  fees_and_slippage_usd: string;
  /** ASSUMPTION: contract shows `{...}`; rendered generically as label/value pairs. */
  baseline: Record<string, string | number | null>;
  assumptions: Record<string, string | number | null>;
}

/* ------------------------------ Briefings, chains ----------------------------- */

export interface BriefingItem {
  text: string;
  link?: string;
}

export interface BriefingSection {
  title: string;
  items: BriefingItem[];
}

export interface BriefingResponse extends Envelope {
  sections: BriefingSection[];
}

/** ASSUMPTION: GET /chains is "implemented" but its shape is not in the contract. */
export interface ChainsResponse {
  chains: Array<{ chain: Chain; status: string; note?: string }>;
}
