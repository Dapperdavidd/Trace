"use client";
/**
 * NEAR wallet connection and mandate flow, shared by the Copy screen and the NEAR agent screen.
 * TRACE never signs: the API prepares a draft and a wallet-signable transaction, the user's own
 * NEAR wallet signs it, and the API verifies the anchor on chain.
 */
import { NearConnector, type NearWalletBase } from "@hot-labs/near-connect";
import { useCallback, useEffect, useRef, useState } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api, describeError } from "@/lib/api/client";
import type { CreateNearMandateBody, NearAsset, NearMandateCreated } from "@/lib/api/types";
import { readLocal, writeLocal, SHADOW_IDS_KEY } from "@/lib/local";

export type WalletState = { wallet: NearWalletBase; accountId: string } | null;

function outcomeHash(value: unknown): string | null {
  if (!value || typeof value !== "object") return null;
  const transaction = (value as { transaction?: unknown }).transaction;
  if (transaction && typeof transaction === "object") {
    const hash = (transaction as { hash?: unknown }).hash;
    if (typeof hash === "string") return hash;
  }
  const outcome = (value as { transaction_outcome?: { id?: unknown } }).transaction_outcome;
  return typeof outcome?.id === "string" ? outcome.id : null;
}

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

export function useNear() {
  const token = useAuthToken();
  const status = useApi(() => api.nearStatus(), []);
  const assets = useApi(() => api.nearAssets(), [token], !!token);
  const mandates = useApi(() => api.nearMandates(), [token], !!token);
  const connector = useRef<NearConnector | null>(null);
  const [wallet, setWallet] = useState<WalletState>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const network = status.data?.network ?? "testnet";

  useEffect(() => {
    if (!status.data) return;
    const near = new NearConnector({
      network,
      features: network === "testnet" ? { signAndSendTransaction: true, testnet: true } : { signAndSendTransaction: true, mainnet: true },
      footerBranding: { heading: "trace", link: window.location.origin, linkText: "Copy limits you sign on NEAR" },
    });
    connector.current = near;
    near
      .getConnectedWallet()
      .then(({ wallet: w, accounts }) => {
        const accountId = accounts[0]?.accountId;
        if (accountId) setWallet({ wallet: w, accountId });
      })
      .catch(() => undefined);
    return () => {
      near.removeAllListeners();
      if (connector.current === near) connector.current = null;
    };
  }, [network, status.data]);

  const connect = useCallback(async () => {
    if (!connector.current) return;
    setBusy(true);
    setError(null);
    try {
      const w = await connector.current.connect();
      const accounts = await w.getAccounts({ network });
      const accountId = accounts[0]?.accountId;
      if (!accountId) throw new Error("The wallet returned no NEAR account.");
      setWallet({ wallet: w, accountId });
    } catch (e) {
      setError(e instanceof Error ? e.message : "NEAR wallet connection failed.");
    } finally {
      setBusy(false);
    }
  }, [network]);

  const disconnect = useCallback(async () => {
    if (!wallet) return;
    setBusy(true);
    try {
      await wallet.wallet.signOut({ network });
      setWallet(null);
    } finally {
      setBusy(false);
    }
  }, [wallet, network]);

  const prepare = useCallback(
    async (p: { shadowId: string; mode: CreateNearMandateBody["mode"]; origin: NearAsset; destination: NearAsset; maxTrade: string; maxDaily: string; slippageBps: number; days?: number }) => {
      if (!wallet) throw new Error("Connect the NEAR wallet that will own this mandate.");
      return api.createNearMandate({
        shadow_strategy_id: p.shadowId,
        owner_near_account: wallet.accountId,
        agent_near_account: status.data?.agent.account_id ?? undefined,
        mode: p.mode,
        allowed_chains: Array.from(new Set([p.origin.blockchain, p.destination.blockchain])),
        allowed_assets: [
          { chain: p.origin.blockchain, asset_id: p.origin.assetId },
          { chain: p.destination.blockchain, asset_id: p.destination.assetId },
        ],
        max_trade_usd: p.maxTrade,
        max_daily_usd: p.maxDaily,
        max_slippage_bps: p.slippageBps,
        expires_at: new Date(Date.now() + (p.days ?? 7) * 86_400_000).toISOString(),
      });
    },
    [wallet, status.data],
  );

  const sign = useCallback(
    async (created: NearMandateCreated): Promise<string> => {
      if (!wallet) throw new Error("Connect your NEAR wallet first.");
      if (!created.near_transaction) throw new Error("The API did not return a transaction to sign. The NEAR contract may not be configured.");
      const tx = created.near_transaction;
      const outcome = await wallet.wallet.signAndSendTransaction({
        network,
        signerId: wallet.accountId,
        receiverId: tx.receiverId,
        actions: tx.actions,
      });
      const hash = outcomeHash(outcome);
      if (!hash) throw new Error("The wallet did not return a transaction hash.");
      await api.confirmNearAnchor(created.id, { tx_hash: hash, signer_account_id: wallet.accountId });
      mandates.reload();
      return hash;
    },
    [wallet, network, mandates],
  );

  return { token, status, assets, mandates, wallet, busy, error, setError, connect, disconnect, prepare, sign, network };
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

export function nearExplorerTx(network: string, hash: string): string {
  return network === "mainnet" ? `https://nearblocks.io/txns/${hash}` : `https://testnet.nearblocks.io/txns/${hash}`;
}
