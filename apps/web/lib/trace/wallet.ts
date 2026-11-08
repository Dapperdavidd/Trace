"use client";
/**
 * The trace in-app NEAR wallet. Keys are generated in the browser and stored only on this
 * device (localStorage). trace's server never sees them. One wallet per network:
 *  - testnet: a named account (`name-xxxx.testnet`) created and funded by the NEAR faucet
 *  - mainnet: an implicit account (64-hex address derived from the key), empty until funded
 */
import { Account, JsonRpcProvider, KeyPair, KeyPairSigner, actions, keyToImplicitAddress } from "near-api-js";
import { useCallback, useEffect, useState, useSyncExternalStore } from "react";

export type Network = "mainnet" | "testnet";

export interface StoredWallet {
  accountId: string;
  publicKey: string;
  secretKey: string;
  createdAt: string;
}

interface Store {
  network: Network;
  testnet?: StoredWallet;
  mainnet?: StoredWallet;
  lastFaucetAt?: number;
}

const KEY = "trace.wallet.v1";
const FAUCET = "https://helper.nearprotocol.com/account";
export const FAUCET_COOLDOWN_MS = 10 * 60_000;

export const RPC: Record<Network, string> = {
  mainnet: "https://free.rpc.fastnear.com",
  testnet: "https://test.rpc.fastnear.com",
};
const EXPLORER: Record<Network, string> = {
  mainnet: "https://nearblocks.io",
  testnet: "https://testnet.nearblocks.io",
};
const NEARBLOCKS_API: Record<Network, string> = {
  mainnet: "https://api.nearblocks.io",
  testnet: "https://api-testnet.nearblocks.io",
};

/* ---------------- storage ---------------- */

let cache: Store | null = null;
const listeners = new Set<() => void>();

function read(): Store {
  if (cache) return cache;
  try {
    const raw = typeof localStorage !== "undefined" ? localStorage.getItem(KEY) : null;
    cache = raw ? (JSON.parse(raw) as Store) : { network: "testnet" };
  } catch {
    cache = { network: "testnet" };
  }
  return cache;
}
function write(next: Store) {
  cache = next;
  try {
    localStorage.setItem(KEY, JSON.stringify(next));
  } catch {
    /* storage blocked: the wallet lives for this tab only */
  }
  listeners.forEach((l) => l());
}
function subscribe(l: () => void) {
  listeners.add(l);
  return () => listeners.delete(l);
}
const SERVER: Store = { network: "testnet" };

export function useWalletStore(): Store {
  return useSyncExternalStore(subscribe, read, () => SERVER);
}

export function setNetwork(network: Network) {
  write({ ...read(), network });
}

export function explorerTx(network: Network, hash: string) {
  return `${EXPLORER[network]}/txns/${hash}`;
}
export function explorerAccount(network: Network, id: string) {
  return `${EXPLORER[network]}/address/${id}`;
}

/* ---------------- accounts ---------------- */

function provider(network: Network) {
  return new JsonRpcProvider({ url: RPC[network] });
}

export function accountFor(network: Network, w: StoredWallet): Account {
  const signer = new KeyPairSigner(KeyPair.fromString(w.secretKey as `ed25519:${string}`));
  return new Account(w.accountId, provider(network), signer);
}

function slug(name: string): string {
  const s = name.toLowerCase().replace(/[^a-z0-9]/g, "").slice(0, 16);
  return s.length >= 2 ? s : "trace";
}
function rand(n: number): string {
  const a = "abcdefghijklmnopqrstuvwxyz0123456789";
  const b = new Uint8Array(n);
  crypto.getRandomValues(b);
  return Array.from(b, (x) => a[x % a.length]).join("");
}

async function faucetCreate(accountId: string, publicKey: string): Promise<void> {
  const res = await fetch(FAUCET, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ newAccountId: accountId, newAccountPublicKey: publicKey }),
  });
  if (!res.ok) {
    const text = await res.text().catch(() => "");
    throw new Error(res.status === 429 ? "The NEAR faucet is busy. Try again in a few minutes." : `The NEAR faucet refused the request. ${text.slice(0, 120)}`);
  }
}

/** Create this network's wallet. Testnet accounts are funded by the faucet right away. */
export async function createWallet(network: Network, nameHint: string): Promise<StoredWallet> {
  const existing = read()[network];
  if (existing) return existing;
  const kp = KeyPair.fromRandom("ed25519");
  const publicKey = kp.getPublicKey().toString();
  let accountId: string;
  if (network === "testnet") {
    accountId = `${slug(nameHint)}-${rand(5)}.testnet`;
    await faucetCreate(accountId, publicKey);
  } else {
    accountId = keyToImplicitAddress(publicKey);
  }
  const w: StoredWallet = { accountId, publicKey, secretKey: kp.toString(), createdAt: new Date().toISOString() };
  write({ ...read(), [network]: w, ...(network === "testnet" ? { lastFaucetAt: Date.now() } : {}) });
  return w;
}

/** Restore a wallet from its secret key (ed25519:...). */
export async function importWallet(network: Network, secretKey: string, accountId?: string): Promise<StoredWallet> {
  const kp = KeyPair.fromString(secretKey.trim() as `ed25519:${string}`);
  const publicKey = kp.getPublicKey().toString();
  const id = accountId?.trim() || keyToImplicitAddress(publicKey);
  const w: StoredWallet = { accountId: id, publicKey, secretKey: kp.toString(), createdAt: new Date().toISOString() };
  const acc = accountFor(network, w);
  const keys = await acc.getAccessKeyList().catch(() => null);
  if (keys && !keys.keys.some((k) => k.public_key === publicKey)) throw new Error("This key does not belong to that account.");
  write({ ...read(), [network]: w });
  return w;
}

export function forgetWallet(network: Network) {
  const s = { ...read() };
  delete s[network];
  write(s);
}

/**
 * Testnet tokens. The faucet funds new accounts, so trace asks it for a fresh helper
 * account and moves its 10 NEAR (minus a little for gas) into your wallet.
 */
export async function requestTestnetTokens(): Promise<{ hash: string; amount: string }> {
  const s = read();
  const w = s.testnet;
  if (!w) throw new Error("Create a testnet wallet first.");
  if (s.lastFaucetAt && Date.now() - s.lastFaucetAt < FAUCET_COOLDOWN_MS) {
    const mins = Math.ceil((FAUCET_COOLDOWN_MS - (Date.now() - s.lastFaucetAt)) / 60_000);
    throw new Error(`You just got testnet tokens. Try again in ${mins} minute${mins === 1 ? "" : "s"}.`);
  }
  const kp = KeyPair.fromRandom("ed25519");
  const helperId = `faucet-${rand(10)}.testnet`;
  await faucetCreate(helperId, kp.getPublicKey().toString());
  const helper = new Account(helperId, provider("testnet"), new KeyPairSigner(kp));
  // the faucet transaction lands optimistically; wait until the helper account is visible
  for (let i = 0; i < 10; i++) {
    const ok = await helper.getState().then(() => true, () => false);
    if (ok) break;
    await new Promise((r) => setTimeout(r, 1200));
  }
  const amount = "9.95";
  const out = await helper.transfer({ receiverId: w.accountId, amount: BigInt(nearToYocto(amount)) });
  write({ ...read(), lastFaucetAt: Date.now() });
  return { hash: txHash(out), amount };
}

/* ---------------- reads ---------------- */

export interface Balance {
  exists: boolean;
  available: bigint;
  total: bigint;
}

export async function getBalance(network: Network, accountId: string): Promise<Balance> {
  try {
    const st = await new Account(accountId, provider(network)).getState();
    return { exists: true, available: st.balance.available, total: st.balance.total };
  } catch (e) {
    const msg = e instanceof Error ? e.message : String(e);
    if (/does not exist|UNKNOWN_ACCOUNT|doesn't exist/i.test(msg)) return { exists: false, available: 0n, total: 0n };
    throw e;
  }
}

export interface Activity {
  hash: string;
  time: Date;
  kind: string;
  counterparty: string;
  incoming: boolean;
  amount: bigint;
  ok: boolean;
}

export async function getActivity(network: Network, accountId: string): Promise<Activity[]> {
  const res = await fetch(`${NEARBLOCKS_API[network]}/v3/accounts/${encodeURIComponent(accountId)}/txns?per_page=15`);
  if (!res.ok) return [];
  const body = (await res.json()) as { data?: Array<Record<string, unknown>> };
  return (body.data ?? []).map((t) => {
    const acts = (t.actions as Array<{ action: string; method: string | null }>) ?? [];
    const signer = String(t.signer_account_id ?? t.predecessor_account_id ?? "");
    const receiver = String(t.receiver_account_id ?? "");
    const incoming = receiver === accountId && signer !== accountId;
    const first = acts[0];
    const kind = first?.action === "FUNCTION_CALL" ? first.method ?? "Contract call" : (first?.action ?? "Transaction").replaceAll("_", " ").toLowerCase();
    const deposit = (t.actions_agg as { deposit?: string } | undefined)?.deposit ?? "0";
    const blockTs = String(t.block_timestamp ?? "0");
    return {
      hash: String(t.transaction_hash ?? ""),
      time: new Date(Number(BigInt(blockTs) / 1_000_000n)),
      kind,
      counterparty: incoming ? signer : receiver,
      incoming,
      amount: BigInt(String(deposit).split(".")[0] || "0"),
      ok: (t.outcomes as { status?: boolean } | undefined)?.status !== false,
    };
  });
}

let priceCache: { at: number; usd: number } | null = null;
export async function nearUsd(): Promise<number | null> {
  if (priceCache && Date.now() - priceCache.at < 60_000) return priceCache.usd;
  try {
    const r = await fetch("https://coins.llama.fi/prices/current/coingecko:near");
    const d = (await r.json()) as { coins?: Record<string, { price?: number }> };
    const p = d.coins?.["coingecko:near"]?.price;
    if (typeof p === "number") {
      priceCache = { at: Date.now(), usd: p };
      return p;
    }
  } catch {
    /* price unavailable */
  }
  return null;
}

/* ---------------- writes ---------------- */

function txHash(outcome: unknown): string {
  const o = outcome as { transaction?: { hash?: string }; transaction_outcome?: { id?: string } };
  return o.transaction?.hash ?? o.transaction_outcome?.id ?? "";
}

export async function sendNear(network: Network, to: string, amountNear: string): Promise<string> {
  const w = read()[network];
  if (!w) throw new Error("No wallet on this network.");
  const yocto = nearToYocto(amountNear);
  if (yocto <= 0n) throw new Error("Type an amount above zero.");
  const out = await accountFor(network, w).transfer({ receiverId: to.trim(), amount: yocto });
  return txHash(out);
}

/** Sign a contract call prepared by the trace API (for example a NEAR mandate). */
export async function signFunctionCall(
  network: Network,
  receiverId: string,
  call: { methodName: string; args: Record<string, unknown>; gas: string; deposit: string },
): Promise<string> {
  const w = read()[network];
  if (!w) throw new Error("Create a wallet on this network first.");
  const out = await accountFor(network, w).signAndSendTransaction({
    receiverId,
    actions: [actions.functionCall(call.methodName, call.args, BigInt(call.gas), BigInt(call.deposit))],
  });
  return txHash(out);
}

/* ---------------- amounts ---------------- */

export function nearToYocto(amount: string): bigint {
  const m = /^(\d*)(?:\.(\d*))?$/.exec(amount.trim());
  if (!m || (!m[1] && !m[2])) return 0n;
  const frac = (m[2] ?? "").slice(0, 24).padEnd(24, "0");
  return BigInt((m[1] || "0") + frac);
}

export function yoctoToNear(y: bigint, dp = 4): string {
  const neg = y < 0n;
  const v = neg ? -y : y;
  const whole = v / 10n ** 24n;
  const frac = (v % 10n ** 24n).toString().padStart(24, "0").slice(0, dp).replace(/0+$/, "");
  return `${neg ? "-" : ""}${whole.toLocaleString("en-US")}${frac ? "." + frac : ""}`;
}

/* ---------------- hook ---------------- */

export function useWallet() {
  const store = useWalletStore();
  const network = store.network;
  const wallet = store[network] ?? null;
  const [balance, setBalance] = useState<Balance | null>(null);
  const [usd, setUsd] = useState<number | null>(null);
  const [tick, setTick] = useState(0);
  const refresh = useCallback(() => setTick((t) => t + 1), []);

  useEffect(() => {
    let live = true;
    setBalance(null);
    if (wallet) {
      getBalance(network, wallet.accountId)
        .then((b) => live && setBalance(b))
        .catch(() => live && setBalance(null));
    }
    nearUsd().then((p) => live && setUsd(p));
    return () => {
      live = false;
    };
  }, [network, wallet, tick]);

  return { store, network, wallet, balance, usd, refresh };
}
