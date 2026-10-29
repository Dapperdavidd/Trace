import type { DataStatus } from "./types";

/** Runtime guards for the key fields the UI relies on. They check shape, not business truth. */

export type Guard<T> = (value: unknown) => value is T;

const STATUSES: readonly DataStatus[] = ["live", "stale", "unavailable", "partial", "demo"];

export function isRecord(v: unknown): v is Record<string, unknown> {
  return typeof v === "object" && v !== null && !Array.isArray(v);
}

export function hasEnvelope(v: unknown): v is Record<string, unknown> & {
  data_status: DataStatus;
  as_of: string;
  demo: boolean;
} {
  return (
    isRecord(v) &&
    typeof v.data_status === "string" &&
    (STATUSES as readonly string[]).includes(v.data_status) &&
    typeof v.as_of === "string" &&
    typeof v.demo === "boolean"
  );
}

function isTokenRef(v: unknown): boolean {
  return isRecord(v) && typeof v.chain === "string" && typeof v.address === "string";
}

function isWalletRef(v: unknown): boolean {
  return isRecord(v) && typeof v.chain === "string" && typeof v.address === "string";
}

function arrayOf(v: unknown, check: (x: unknown) => boolean): boolean {
  return Array.isArray(v) && v.every(check);
}

export const guardMovers: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) &&
  arrayOf(v.items, (i) =>
    isRecord(i) &&
    typeof i.rank === "number" &&
    isTokenRef(i.token) &&
    typeof i.price_usd === "string" &&
    typeof i.change_pct === "string" &&
    Array.isArray(i.sparkline),
  );

export const guardDiscoveryWallets: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) &&
  arrayOf(v.items, (i) => isRecord(i) && isWalletRef(i.wallet) && typeof i.decision === "string");

export const guardWalletDetail: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) && isWalletRef(v.wallet) && isRecord(v.metrics);

export const guardWalletTrades: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) &&
  arrayOf(v.items, (i) => isRecord(i) && typeof i.tx_id === "string" && (i.side === "buy" || i.side === "sell"));

export const guardWalletMetrics: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) && typeof v.total_trades === "number" && isRecord(v.wqs);

export const guardDna: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) && (v.state === "ok" || v.state === "insufficient_history") && Array.isArray(v.traits);

export const guardToken: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) &&
  isTokenRef(v.token) &&
  typeof v.price_usd === "string" &&
  isRecord(v.risk) &&
  Array.isArray((v.risk as Record<string, unknown>).checks) &&
  Array.isArray(v.why_moving) &&
  Array.isArray(v.buyers);

export const guardTimeline: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) &&
  arrayOf(v.items, (i) => isRecord(i) && typeof i.time === "string" && (i.kind === "fact" || i.kind === "hypothesis"));

export const guardCompare: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) && Array.isArray(v.items) && v.items.length >= 2 && typeof v.verdict_note === "string";

export const guardShadowResults: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) &&
  typeof v.starting_usd === "string" &&
  typeof v.ending_usd === "string" &&
  typeof v.return_pct === "string";

export const guardBriefing: Guard<unknown> = (v): v is unknown =>
  hasEnvelope(v) &&
  arrayOf(v.sections, (s) => isRecord(s) && typeof s.title === "string" && Array.isArray(s.items));

export const guardWatchlist: Guard<unknown> = (v): v is unknown =>
  isRecord(v) && arrayOf(v.items, (i) => isRecord(i) && typeof i.id === "string" && isWalletRef(i.wallet));

export const guardAlerts: Guard<unknown> = (v): v is unknown =>
  isRecord(v) && arrayOf(v.items, (i) => isRecord(i) && typeof i.id === "string" && typeof i.title === "string");

export const guardAuth: Guard<unknown> = (v): v is unknown =>
  isRecord(v) && typeof v.access_token === "string" && isRecord(v.user) && typeof v.user.email === "string";

export const guardMe: Guard<unknown> = (v): v is unknown =>
  isRecord(v) && typeof v.id === "string" && typeof v.email === "string";

export const guardIdCreated: Guard<unknown> = (v): v is unknown => isRecord(v) && typeof v.id === "string";

export const guardAny: Guard<unknown> = (v): v is unknown => v !== undefined;
