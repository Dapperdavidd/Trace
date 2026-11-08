"use client";
/**
 * View models for the trace screens. Every hook returns live API data only. When the API
 * has nothing yet the list is empty and `note` says why; screens show an honest empty state.
 */
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import type { Chain, DiscoveredWallet, MoverItem, MoverTab, RiskState, TokenDetail, TimelineResponse, WalletTab } from "@/lib/api/types";
import { compactUsd, num, seedOf, shortAddr } from "./viz";

export type RiskLabel = "Low" | "Medium" | "High" | "Unrated";

export interface CoinVM {
  key: string;
  sym: string;
  name: string;
  chain: string;
  address?: string;
  example: boolean;
  p: number;
  ch: number;
  vol: string;
  mc: string;
  liq: string;
  liqN: number | null;
  mcN: number | null;
  hold: string;
  why: string;
  sm: number | null;
  risk: RiskLabel;
  rn: number;
  seed: number;
  vx?: string;
  early?: boolean;
  spark?: number[];
}

export interface WalletVM {
  key: string;
  name: string;
  a: string;
  av: string;
  chain: string;
  address?: string;
  example: boolean;
  ret: number | null;
  win: number | null;
  trades: number;
  hold?: string;
  d7?: number;
  d30: number | null;
  d90?: number;
  style: string[];
  copiers?: string;
  seed: number;
  isNew?: boolean;
  decision?: string;
  confidence?: string;
}

/* ---------------- live adapters ---------------- */

const RISK: Record<RiskState, [RiskLabel, number]> = { pass: ["Low", 1], warn: ["Medium", 3], block: ["High", 5], unknown: ["Unrated", 0] };
const CHAIN_NAME: Record<string, string> = { solana: "Solana", ethereum: "Ethereum", base: "Base", bnb: "BNB Chain" };
export const chainName = (c: string) => CHAIN_NAME[c] ?? c;

export function moverToCoin(m: MoverItem): CoinVM {
  const sym = m.token.symbol || shortAddr(m.token.address);
  const [risk, rn] = RISK[m.risk] ?? RISK.unknown;
  const liqN = num(m.liquidity_usd);
  return {
    key: `${m.token.chain}:${m.token.address}`,
    sym,
    name: m.token.name || sym,
    chain: chainName(m.token.chain),
    address: m.token.address,
    example: false,
    p: num(m.price_usd) ?? 0,
    ch: num(m.change_pct) ?? 0,
    vol: compactUsd(num(m.volume_usd)),
    mc: "—",
    liq: compactUsd(liqN),
    liqN: liqN === null ? null : liqN / 1e6,
    mcN: null,
    hold: "—",
    why: m.reason.label,
    sm: null,
    risk,
    rn,
    seed: seedOf(m.token.address),
    early: m.reason.code === "early_momentum",
    spark: m.sparkline,
  };
}

export function tokenToCoin(t: TokenDetail): CoinVM {
  const sym = t.token.symbol || shortAddr(t.token.address);
  const [risk, rn] = RISK[t.risk.overall] ?? RISK.unknown;
  const liqN = num(t.liquidity_usd);
  const mcN = num(t.market_cap_usd);
  return {
    key: `${t.token.chain}:${t.token.address}`,
    sym,
    name: t.token.name || sym,
    chain: chainName(t.token.chain),
    address: t.token.address,
    example: false,
    p: num(t.price_usd) ?? 0,
    ch: num(t.change_pct_24h) ?? 0,
    vol: compactUsd(num(t.volume_24h_usd)),
    mc: compactUsd(mcN),
    liq: compactUsd(liqN),
    liqN: liqN === null ? null : liqN / 1e6,
    mcN: mcN === null ? null : mcN / 1e6,
    hold: t.holders === null ? "—" : t.holders.toLocaleString("en-US"),
    why: t.why_moving[0]?.text ?? "No reason recorded yet",
    sm: t.buyers.length,
    risk,
    rn,
    seed: seedOf(t.token.address),
  };
}

export function discoveredToWallet(d: DiscoveredWallet): WalletVM {
  const addr = d.wallet.address;
  const name = d.wallet.alias || shortAddr(addr);
  return {
    key: `${d.wallet.chain}:${addr}`,
    name,
    a: shortAddr(addr),
    av: (d.wallet.alias || addr.replace(/^0x/, "")).slice(0, 2).toUpperCase(),
    chain: chainName(d.wallet.chain),
    address: addr,
    example: false,
    ret: num(d.realized_return_pct),
    win: num(d.win_rate_pct),
    trades: d.sample_size,
    d30: num(d.realized_return_pct),
    style: (d.tags.length ? d.tags : [d.decision]).map(prettyTag),
    seed: seedOf(addr),
    decision: d.decision,
    confidence: d.pnl_confidence,
  };
}

/** "fast_flipper" -> "Fast flipper", "possible_bot" -> "Possible bot". */
export function prettyTag(t: string): string {
  const w = t.replaceAll("_", " ").trim();
  return w.charAt(0).toUpperCase() + w.slice(1);
}

/* ---------------- references in URLs ---------------- */

export interface Ref {
  sym?: string;
  chain?: string;
  address?: string;
}
export function coinHref(c: Pick<CoinVM, "example" | "sym" | "address"> & { chain: string }, extra = ""): string {
  if (c.example || !c.address) return `/app/token/?sym=${encodeURIComponent(c.sym)}${extra}`;
  return `/app/token/?chain=${encodeURIComponent(chainKey(c.chain))}&address=${encodeURIComponent(c.address)}${extra}`;
}
export function walletQuery(w: Pick<WalletVM, "example" | "name" | "address" | "chain">): string {
  if (w.example || !w.address) return `w=${encodeURIComponent(w.name)}`;
  return `chain=${encodeURIComponent(chainKey(w.chain))}&address=${encodeURIComponent(w.address)}`;
}
export function chainKey(c: string): string {
  const k = Object.entries(CHAIN_NAME).find(([, v]) => v === c)?.[0];
  return k ?? c.toLowerCase();
}

/* ---------------- hooks ---------------- */

/** Turn the API's ranking diagnostics ("5 excluded: stale.") into a sentence people can read. */
function friendlyNote(note: string | null | undefined): string | null {
  if (!note) return null;
  if (/excluded/i.test(note)) {
    const stale = /stale/i.test(note);
    const liq = /liquidity/i.test(note);
    return stale && liq
      ? "trace is waiting for fresh prices and liquidity on today's coins."
      : stale
        ? "trace is waiting for fresh prices on today's coins."
        : "Today's coins do not have enough liquidity to list safely yet.";
  }
  return note;
}

export interface Sourced<T> {
  data: T;
  example: boolean;
  loading: boolean;
  note: string | null;
}

const DISC_TAB: Record<string, MoverTab> = { trending: "trending", early: "early_momentum", smart: "smart_money", new: "new" };

/** 24h movers, falling back to the last hour while the 24h price window fills up. */
async function moversWithFallback(tab: MoverTab) {
  const day = await api.movers({ tab, window: "24h" });
  if (day.demo || day.items.length) return { res: day, window: "24h" as const };
  const hour = await api.movers({ tab, window: "1h" });
  return hour.items.length && !hour.demo ? { res: hour, window: "1h" as const } : { res: day, window: "24h" as const };
}

export function useCoins(tab = "trending"): Sourced<CoinVM[]> & { window: "24h" | "1h"; error: boolean } {
  const r = useApi(() => moversWithFallback(DISC_TAB[tab] ?? "trending"), [tab]);
  const res = r.data?.res ?? null;
  const window = r.data?.window ?? "24h";
  const live = res && !res.demo ? res.items.map(moverToCoin) : [];
  return {
    data: live,
    example: false,
    loading: r.status === "loading",
    note: res && !res.demo ? friendlyNote(res.coverage_note) : r.status === "error" ? "trace could not reach its API. It may be waking up; try again in a moment." : null,
    window,
    error: r.status === "error",
  };
}

export interface CoinDetail {
  coin: CoinVM;
  detail: TokenDetail | null;
  timeline: TimelineResponse | null;
}

export function useCoin(ref: Ref): Sourced<CoinDetail | null> & { error: unknown } {
  const liveRef = !!(ref.chain && ref.address);
  const t = useApi(() => api.token(ref.chain as Chain, ref.address as string), [ref.chain, ref.address], liveRef);
  const tl = useApi(() => api.tokenTimeline(ref.chain as Chain, ref.address as string), [ref.chain, ref.address], liveRef);
  if (liveRef && t.data && !t.data.demo) {
    return { data: { coin: tokenToCoin(t.data), detail: t.data, timeline: tl.data ?? null }, example: false, loading: false, note: t.data.coverage_note ?? null, error: null };
  }
  return { data: null, example: false, loading: liveRef && t.status === "loading", note: null, error: liveRef ? t.error : null };
}

const WALLET_TAB: Record<string, WalletTab> = { top: "top", new: "new", copied: "most_followed" };

export function useWallets(tab = "top"): Sourced<WalletVM[]> & { error: boolean } {
  const s = useApi(() => api.discoveryWallets({ tab: WALLET_TAB[tab] ?? "top" }), [tab]);
  const live = s.data && !s.data.demo ? s.data.items.map(discoveredToWallet) : [];
  return {
    data: live,
    example: false,
    loading: s.status === "loading",
    note: s.data && !s.data.demo ? s.data.coverage_note ?? null : s.status === "error" ? "trace could not reach its API. It may be waking up; try again in a moment." : null,
    error: s.status === "error",
  };
}

export function useSignedIn(): boolean {
  return !!useAuthToken();
}
