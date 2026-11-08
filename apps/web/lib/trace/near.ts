"use client";
/**
 * NEAR mandate helpers. trace's server never signs: the API prepares a draft and a
 * wallet-signable transaction, the user's in-app wallet (lib/trace/wallet.ts) signs it on
 * their device, and the API verifies the anchor on chain.
 */
import { describeError } from "@/lib/api/client";
import type { NearAsset } from "@/lib/api/types";
import { readLocal, writeLocal, SHADOW_IDS_KEY } from "@/lib/local";

export function errText(e: unknown): string {
  const d = describeError(e);
  return `${d.message}${d.requestId ? ` (request ${d.requestId.slice(0, 8)})` : ""}`;
}

/* shadow strategy ids remembered per browser, with the wallet they test */
export interface SavedShadow {
  id: string;
  chain: string;
  address: string;
  budget: string;
  created_at: string;
}
const SHADOW_META_KEY = "trace.shadow_strategies.v2";
export function savedShadows(): SavedShadow[] {
  return readLocal<SavedShadow[]>(SHADOW_META_KEY, []);
}
export function saveShadow(s: SavedShadow) {
  const all = [s, ...savedShadows().filter((x) => x.id !== s.id)].slice(0, 20);
  writeLocal(SHADOW_META_KEY, all);
  writeLocal(SHADOW_IDS_KEY, all.map((x) => x.id));
}

/**
 * Default allowed pair for a copy mandate. Both assets live on NEAR so the owner's NEAR account
 * can be the quote recipient and refund address: USDC in, wNEAR out.
 */
export function defaultPair(items: NearAsset[]): [NearAsset | undefined, NearAsset | undefined] {
  const onNear = items.filter((a) => a.blockchain === "near");
  const origin = onNear.find((a) => a.symbol === "USDC") ?? onNear[0] ?? items[0];
  const destination = onNear.find((a) => a.symbol === "wNEAR") ?? onNear.find((a) => a.assetId !== origin?.assetId) ?? items.find((a) => a.assetId !== origin?.assetId);
  return [origin, destination];
}

/** Decimal string to integer base units, without floating point. */
export function toBaseUnits(amount: string, decimals: number): string | null {
  const m = /^(\d*)(?:\.(\d*))?$/.exec(amount.trim());
  if (!m || (!m[1] && !m[2])) return null;
  const frac = (m[2] ?? "").slice(0, decimals).padEnd(decimals, "0");
  const out = ((m[1] || "0") + frac).replace(/^0+/, "");
  return out || null;
}
