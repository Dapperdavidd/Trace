"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import { errText, toBaseUnits } from "@/lib/trace/near";
import {
  FAUCET_COOLDOWN_MS,
  createWallet,
  explorerAccount,
  explorerTx,
  forgetWallet,
  getActivity,
  importWallet,
  requestTestnetTokens,
  sendNear,
  setNetwork,
  useWallet,
  yoctoToNear,
  type Activity,
  type Network,
} from "@/lib/trace/wallet";
import { money, shortAddr } from "@/lib/trace/viz";
import { useApp } from "../AppState";
import { usd0 } from "@/lib/trace/viz";
import { Av, Hd, Ic, Lk, Row, Seg, Src, Tk } from "../ui";
import { Empty } from "./Research";

/* =============================== Trade / Swap (NEAR Intents) =============================== */

interface IntentAsset {
  assetId: string;
  decimals: number;
  blockchain: string;
  symbol: string;
  price?: number;
}
interface Quote {
  amountInFormatted: string;
  amountInUsd: string;
  amountOutFormatted: string;
  amountOutUsd: string;
  minAmountOut: string;
  timeEstimate: number;
}

const ONECLICK = "https://1click.chaindefuser.com/v0";
const PICKS: Array<[string, string]> = [
  ["near", "wNEAR"],
  ["near", "USDC"],
  ["near", "USDT"],
  ["eth", "ETH"],
  ["sol", "SOL"],
  ["btc", "BTC"],
  ["base", "ETH"],
];
const CHAIN_LABEL: Record<string, string> = { near: "NEAR", eth: "Ethereum", sol: "Solana", btc: "Bitcoin", base: "Base" };
const label = (a?: IntentAsset) => (a ? (a.symbol === "wNEAR" ? "NEAR" : a.symbol) : "—");

function useIntentAssets() {
  const [items, setItems] = useState<IntentAsset[]>([]);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    fetch(`${ONECLICK}/tokens`)
      .then((r) => r.json() as Promise<IntentAsset[]>)
      .then((all) => setItems(PICKS.map(([c, s]) => all.find((a) => a.blockchain === c && a.symbol === s)).filter(Boolean) as IntentAsset[]))
      .catch(() => setErr("NEAR Intents is not reachable right now."));
  }, []);
  return { items, err };
}

export function SwapScreen() {
  const router = useRouter();
  const params = useSearchParams();
  const { toast } = useApp();
  const { store, network } = useWallet();
  const mainnet = store.mainnet ?? null;
  const assets = useIntentAssets();
  const buyMode = params.get("mode") === "buy";
  const [mode, setMode] = useState<"swap" | "buy" | "sell">(buyMode ? "buy" : "swap");
  const [fromId, setFromId] = useState<string>("nep141:wrap.near");
  const [toId, setToId] = useState<string>("nep141:17208628f84f5d6ad33f0da3bbbeb27ffcb398eac501a31bd6ad2011e36133a1");
  const [amount, setAmount] = useState("1");
  const [quote, setQuote] = useState<Quote | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const from = assets.items.find((a) => a.assetId === fromId);
  const to = assets.items.find((a) => a.assetId === toId);

  useEffect(() => setQuote(null), [fromId, toId, amount]);
  useEffect(() => {
    if (mode === "buy") setFromId("nep141:wrap.near");
    if (mode === "sell") setToId("nep141:wrap.near");
  }, [mode]);

  const cycle = (cur: string, skip: string) => {
    const l = assets.items.filter((a) => a.assetId !== skip);
    const i = l.findIndex((a) => a.assetId === cur);
    return l[(i + 1) % l.length]?.assetId ?? cur;
  };

  const getQuote = async () => {
    if (!from || !to) return;
    if (!mainnet) return setErr("Create your mainnet wallet in the Wallet tab first. Swaps run on NEAR mainnet.");
    const base = toBaseUnits(amount, from.decimals);
    if (!base) return setErr("Type an amount, for example 1 or 0.5.");
    setBusy(true);
    setErr(null);
    try {
      const res = await fetch(`${ONECLICK}/quote`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          dry: true,
          swapType: "EXACT_INPUT",
          slippageTolerance: 100,
          originAsset: from.assetId,
          depositType: "INTENTS",
          destinationAsset: to.assetId,
          amount: base,
          recipient: mainnet.accountId,
          recipientType: "INTENTS",
          refundTo: mainnet.accountId,
          refundType: "INTENTS",
          deadline: new Date(Date.now() + 10 * 60_000).toISOString(),
        }),
      });
      const body = (await res.json()) as { quote?: Quote; message?: string };
      if (!res.ok || !body.quote) throw new Error(body.message ? `NEAR Intents: ${body.message}` : "NEAR Intents could not quote this swap.");
      setQuote(body.quote);
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };

  const usdIn = from?.price && Number(amount) ? Number(amount) * from.price : null;

  return (
    <div className="scr s-swap">
      <Hd title={mode === "buy" ? "Buy" : mode === "sell" ? "Sell" : "Trade"} crumb="Live quotes from NEAR Intents, across chains" right={<Src example={false} />} />
      {network === "testnet" ? (
        <div className="warnrow">
          <Ic id="globe" />
          <span className="grow">NEAR Intents only runs on mainnet. Quotes below are real mainnet prices.</span>
          <button className="lk" type="button" onClick={() => setNetwork("mainnet")}>
            Use mainnet <Ic id="arr" />
          </button>
        </div>
      ) : null}
      <div className="swgrid">
        <div style={{ display: "grid", gap: 14 }}>
          <Seg
            cls="box"
            opts={[
              ["swap", "Swap"],
              ["buy", "Buy"],
              ["sell", "Sell"],
            ]}
            cur={mode}
            onPick={setMode}
          />
          <div>
            <div className="sbox">
              <div className="pn-h">
                <span className="lab">You pay</span>
                <span className="mono f3" style={{ fontSize: 11 }}>
                  {from ? CHAIN_LABEL[from.blockchain] ?? from.blockchain : ""}
                </span>
              </div>
              <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                <input className="amt" inputMode="decimal" value={amount} onChange={(e) => setAmount(e.target.value.replace(",", "."))} aria-label="Amount to pay" />
                <button className="tsel" type="button" disabled={mode === "buy"} onClick={() => setFromId(cycle(fromId, toId))}>
                  <Tk s={label(from)} />
                  {label(from)}
                  {mode !== "buy" ? " ▾" : ""}
                </button>
              </div>
              <span className="mono f3" style={{ fontSize: 11.5 }}>
                {quote ? `$${Number(quote.amountInUsd).toFixed(2)}` : usdIn !== null ? `about ${money(usdIn)}` : " "}
              </span>
            </div>
            <button
              className="flip"
              type="button"
              aria-label="Flip"
              onClick={() => {
                setFromId(toId);
                setToId(fromId);
              }}
            >
              <Ic id="swap" />
            </button>
            <div className="sbox" style={{ paddingTop: 22 }}>
              <span className="lab">You get</span>
              <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                <span className="amt">{quote ? Number(quote.amountOutFormatted).toLocaleString("en-US", { maximumFractionDigits: 6 }) : "—"}</span>
                <button className="tsel" type="button" disabled={mode === "sell"} onClick={() => setToId(cycle(toId, fromId))}>
                  <Tk s={label(to)} on />
                  {label(to)}
                  {mode !== "sell" ? " ▾" : ""}
                </button>
              </div>
              <span className="mono f3" style={{ fontSize: 11.5 }}>
                {quote ? `$${Number(quote.amountOutUsd).toFixed(2)} · on ${CHAIN_LABEL[to?.blockchain ?? ""] ?? to?.blockchain}` : to?.price ? `1 ${label(to)} ≈ ${money(to.price)}` : " "}
              </span>
            </div>
          </div>
          <Seg
            cls="box"
            opts={[
              ["0.1", "0.1"],
              ["1", "1"],
              ["5", "5"],
              ["10", "10"],
            ]}
            cur={amount}
            onPick={(v) => setAmount(v)}
          />
        </div>
        <div style={{ display: "grid", gap: 12, alignContent: "start" }}>
          <div className="kvl">
            <div>
              <span>Route</span>
              <span className="mono">NEAR Intents 1Click</span>
            </div>
            <div>
              <span>Max slippage</span>
              <span className="mono">1%</span>
            </div>
            <div>
              <span>Time estimate</span>
              <span className="mono">{quote ? `${quote.timeEstimate}s` : "—"}</span>
            </div>
            <div>
              <span>Your mainnet wallet</span>
              <span className="mono">{mainnet ? shortAddr(mainnet.accountId) : "Not created"}</span>
            </div>
          </div>
          {assets.err ? <p className="err">{assets.err}</p> : null}
          {err ? <p className="err">{err}</p> : null}
          {!mainnet ? (
            <button className="bfill" style={{ height: 50 }} type="button" onClick={() => router.push("/app/wallet/?net=mainnet")}>
              Create mainnet wallet
            </button>
          ) : (
            <button
              className="bfill"
              style={{ height: 50 }}
              type="button"
              disabled={busy || !from || !to}
              onClick={() => (quote ? toast("Fund your mainnet wallet with NEAR to place this swap. Your wallet signs it; trace never holds funds.") : getQuote())}
            >
              {busy ? "Getting a live quote" : quote ? "Confirm swap" : "Get live quote"}
            </button>
          )}
          <span className="sub" style={{ textAlign: "center", whiteSpace: "normal" }}>
            Quotes are live from NEAR Intents and settle into your NEAR account. Your wallet signs every swap.
          </span>
        </div>
      </div>
    </div>
  );
}

/* =============================== Wallet =============================== */

function ago(d: Date) {
  const s = (Date.now() - d.getTime()) / 1000;
  if (s < 90) return "just now";
  if (s < 3600) return `${Math.round(s / 60)} min ago`;
  if (s < 86400) return `${Math.round(s / 3600)} h ago`;
  return d.toLocaleDateString("en-GB", { day: "numeric", month: "short" });
}

export function WalletScreen() {
  const router = useRouter();
  const params = useSearchParams();
  const { toast } = useApp();
  const token = useAuthToken();
  const { store, network, wallet, balance, usd, refresh } = useWallet();
  const me = useApi(() => api.me(), [token], !!token);
  const mandates = useApi(() => api.nearMandates(), [token], !!token);
  const [t, setT] = useState(params.get("t") ?? "tokens");
  const [panel, setPanel] = useState<"none" | "send" | "receive" | "import" | "secret">(() => (params.get("a") === "send" ? "send" : params.get("a") === "receive" ? "receive" : "none"));
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [to, setTo] = useState("");
  const [amt, setAmt] = useState("");
  const [secret, setSecret] = useState("");
  const [importId, setImportId] = useState("");
  const [activity, setActivity] = useState<Activity[] | null>(null);

  useEffect(() => {
    const n = params.get("net");
    if (n === "mainnet" || n === "testnet") setNetwork(n);
  }, [params]);
  useEffect(() => {
    setActivity(null);
    if (wallet) getActivity(network, wallet.accountId).then(setActivity).catch(() => setActivity([]));
  }, [network, wallet, balance]);

  const run = async (fn: () => Promise<void>) => {
    setBusy(true);
    setErr(null);
    try {
      await fn();
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };
  const nameHint = me.data?.name || me.data?.email?.split("@")[0] || "trace";
  const near = balance ? Number(yoctoToNear(balance.available, 6).replace(/,/g, "")) : 0;
  const cooling = !!store.lastFaucetAt && Date.now() - store.lastFaucetAt < FAUCET_COOLDOWN_MS;
  const copies = mandates.data?.items ?? [];

  const netSeg = (
    <Seg
      cls="box"
      opts={[
        ["mainnet", "Mainnet"],
        ["testnet", "Testnet"],
      ]}
      cur={network}
      onPick={(n: Network) => {
        setNetwork(n);
        setPanel("none");
        setErr(null);
      }}
    />
  );

  if (!wallet) {
    return (
      <div className="scr s-wlt">
        <Hd title="Wallet" crumb="Your keys stay on this device" />
        {netSeg}
        <div className="wgrid">
          <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
            <div className="pn">
              <span className="lab">{network === "testnet" ? "Testnet wallet" : "Mainnet wallet"}</span>
              <b style={{ fontWeight: 500, fontSize: 20, letterSpacing: "-.03em" }}>{network === "testnet" ? "Try everything with free test NEAR." : "Your real NEAR wallet."}</b>
              <span className="sub" style={{ whiteSpace: "normal" }}>
                {network === "testnet"
                  ? "trace creates a testnet account for you and asks the NEAR faucet for 10 test NEAR. Test tokens have no value."
                  : "trace creates a NEAR account from a key generated on this device. It starts empty; send NEAR to it to trade. trace never sees your key."}
              </span>
              <button className="bfill" type="button" disabled={busy} onClick={() => run(async () => void (await createWallet(network, nameHint), refresh(), toast(network === "testnet" ? "Testnet wallet ready with 10 test NEAR" : "Mainnet wallet created")))}>
                {busy ? (network === "testnet" ? "Asking the faucet" : "Creating") : network === "testnet" ? "Create testnet wallet" : "Create mainnet wallet"}
              </button>
              <button className="bout" type="button" onClick={() => setPanel(panel === "import" ? "none" : "import")}>
                I have a NEAR key
              </button>
              {panel === "import" ? (
                <div style={{ display: "grid", gap: 10 }}>
                  <label className="fld">
                    <span>Secret key (ed25519:…)</span>
                    <input value={secret} onChange={(e) => setSecret(e.target.value)} autoComplete="off" spellCheck={false} />
                  </label>
                  <label className="fld">
                    <span>Account (leave empty for an implicit account)</span>
                    <input value={importId} onChange={(e) => setImportId(e.target.value)} autoComplete="off" placeholder={network === "testnet" ? "name.testnet" : "name.near"} />
                  </label>
                  <button className="bfill" type="button" disabled={busy || !secret} onClick={() => run(async () => void (await importWallet(network, secret, importId), setSecret(""), refresh(), toast("Wallet restored")))}>
                    Restore wallet
                  </button>
                </div>
              ) : null}
              {err ? <p className="err">{err}</p> : null}
            </div>
          </div>
          <div className="pn">
            <span className="lab">How your wallet works</span>
            <div className="steps">
              {[
                ["Self-custody", "The key is created and stored only in this browser."],
                ["Two networks", "Switch between mainnet and testnet any time. Each has its own account."],
                ["Copy limits on NEAR", "Your wallet signs the mandate that caps every copied trade."],
              ].map(([a, b], i) => (
                <div key={a} className="done">
                  <i>{i + 1}</i>
                  <div>
                    <b>{a}</b>
                    <span className="sub">{b}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="scr s-wlt">
      <Hd
        title="Wallet"
        crumb={`${network === "mainnet" ? "Mainnet" : "Testnet"} · ${shortAddr(wallet.accountId)}`}
        right={
          <button
            className="hb"
            type="button"
            aria-label="Copy address"
            onClick={() => {
              navigator.clipboard?.writeText(wallet.accountId).catch(() => undefined);
              toast("Address copied");
            }}
          >
            <Ic id="copy" />
          </button>
        }
      />
      {netSeg}
      <div className="wgrid">
        <div style={{ display: "grid", gap: 14 }}>
          <div className="vrow">
            <span className="big n">{balance ? yoctoToNear(balance.available, 2) : "…"} NEAR</span>
            {usd !== null && balance ? <span className="mono f2">{money(near * usd)}</span> : null}
            {balance && !balance.exists ? <span className="tag">Not on chain yet</span> : null}
          </div>
          <div className="qa">
            <button className="qb" type="button" onClick={() => setPanel(panel === "send" ? "none" : "send")}>
              <Ic id="send" />
              <span>Send</span>
            </button>
            <button className="qb" type="button" onClick={() => setPanel(panel === "receive" ? "none" : "receive")}>
              <Ic id="recv" />
              <span>Receive</span>
            </button>
            <button className="qb" type="button" onClick={() => router.push("/app/swap/")}>
              <Ic id="swap" />
              <span>Swap</span>
            </button>
            {network === "testnet" ? (
              <button
                className="qb pri"
                type="button"
                disabled={busy || cooling}
                onClick={() =>
                  run(async () => {
                    const r = await requestTestnetTokens();
                    toast(`${r.amount} test NEAR is on its way`);
                    setTimeout(refresh, 2500);
                  })
                }
              >
                <Ic id="plus" />
                <span>{busy ? "Asking the faucet" : cooling ? "Tokens sent" : "Request tokens"}</span>
              </button>
            ) : (
              <button className="qb pri" type="button" onClick={() => router.push("/app/swap/?mode=buy")}>
                <Ic id="plus" />
                <span>Buy</span>
              </button>
            )}
          </div>
          {err ? <p className="err">{err}</p> : null}
          {panel === "send" ? (
            <div className="pn">
              <span className="lab">Send NEAR</span>
              <label className="fld">
                <span>To (NEAR account)</span>
                <input value={to} onChange={(e) => setTo(e.target.value.trim())} autoComplete="off" placeholder={network === "testnet" ? "friend.testnet" : "friend.near"} />
              </label>
              <label className="fld">
                <span>Amount (NEAR)</span>
                <input value={amt} onChange={(e) => setAmt(e.target.value.replace(",", "."))} inputMode="decimal" placeholder="1" />
              </label>
              <button
                className="bfill"
                type="button"
                disabled={busy || !to || !amt}
                onClick={() => {
                  if (!window.confirm(`Send ${amt} NEAR to ${to} on ${network}? This cannot be undone.`)) return;
                  run(async () => {
                    const hash = await sendNear(network, to, amt);
                    toast(`Sent ${amt} NEAR`);
                    setTo("");
                    setAmt("");
                    setPanel("none");
                    setTimeout(refresh, 1500);
                    if (hash) window.open(explorerTx(network, hash), "_blank", "noopener");
                  });
                }}
              >
                {busy ? "Sending" : "Send"}
              </button>
            </div>
          ) : null}
          {panel === "receive" ? (
            <div className="pn">
              <span className="lab">Your {network} address</span>
              <code className="hash" style={{ fontSize: 14, color: "var(--fg)" }}>
                {wallet.accountId}
              </code>
              <div className="bset">
                <button
                  className="bfill sm"
                  type="button"
                  onClick={() => {
                    navigator.clipboard?.writeText(wallet.accountId).catch(() => undefined);
                    toast("Address copied");
                  }}
                >
                  Copy address
                </button>
                <a className="bout sm" href={explorerAccount(network, wallet.accountId)} target="_blank" rel="noreferrer">
                  View on NearBlocks
                </a>
              </div>
              <span className="sub" style={{ whiteSpace: "normal" }}>
                Only send NEAR on {network} to this address.
              </span>
            </div>
          ) : null}
          <Seg
            cls="box"
            opts={[
              ["tokens", "Tokens"],
              ["copies", `Copies · ${copies.length}`],
              ["activity", "Activity"],
            ]}
            cur={t}
            onPick={setT}
          />
          <div>
            {t === "tokens" ? (
              <div className="hrow">
                <Tk s="NEAR" on />
                <div style={{ minWidth: 0 }}>
                  <div className="nm">NEAR</div>
                  <div className="mono">{balance ? yoctoToNear(balance.available, 4) : "…"} NEAR</div>
                </div>
                <div className="rt">
                  <div className="v">{usd === null || !balance ? "—" : money(near * usd)}</div>
                  <div className="mono f3">{usd === null ? "" : `${money(usd)} each`}</div>
                </div>
              </div>
            ) : null}
            {t === "copies" ? (
              copies.length ? (
                copies.map((m) => (
                  <Row key={m.id} href="/app/near/">
                    <Av t="N" />
                    <div className="grow">
                      <div className="nm">{m.mode === "delegated" ? "Auto copy" : m.mode === "paper" ? "Paper only" : "Alerts only"}</div>
                      <div className="sub">
                        ${usd0(m.max_trade_usd)} a trade · ${usd0(m.max_daily_usd)} a day · ends {new Date(m.expires_at).toLocaleDateString("en-GB", { day: "numeric", month: "short" })}
                      </div>
                    </div>
                    <span className={`tag ${m.status === "anchored" ? "g" : ""}`}>{m.status === "anchored" ? "Signed" : m.status}</span>
                  </Row>
                ))
              ) : (
                <p className="sub" style={{ whiteSpace: "normal" }}>
                  {token ? "No copies yet. Copy a smart wallet with limits your wallet signs." : "Sign in to copy a wallet."} <Lk href="/app/wallets/">Smart wallets</Lk>
                </p>
              )
            ) : null}
            {t === "activity" ? (
              activity === null ? (
                <p className="sub">Loading activity…</p>
              ) : activity.length ? (
                activity.map((a) => (
                  <a key={a.hash} className="li" href={explorerTx(network, a.hash)} target="_blank" rel="noreferrer">
                    <span className={`tk${a.incoming ? " on" : ""}`}>
                      <Ic id={a.incoming ? "recv" : "send"} />
                    </span>
                    <div className="grow">
                      <div className="nm" style={{ fontWeight: 400 }}>
                        {a.incoming ? "Received from" : a.kind === "transfer" ? "Sent to" : a.kind} <b style={{ fontWeight: 500 }}>{shortAddr(a.counterparty)}</b>
                      </div>
                      <div className="sub">
                        {ago(a.time)}
                        {a.ok ? "" : " · failed"}
                      </div>
                    </div>
                    <span className="mono">{a.amount > 0n ? `${a.incoming ? "+" : "−"}${yoctoToNear(a.amount, 3)}` : ""}</span>
                  </a>
                ))
              ) : (
                <p className="sub">No transactions yet.</p>
              )
            ) : null}
          </div>
        </div>
        <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
          <div className="pn">
            <span className="lab">
              <b>Backup</b>
            </span>
            <span className="sub" style={{ whiteSpace: "normal" }}>
              Your key lives only in this browser. Save it somewhere safe; without it, nobody can recover this wallet.
            </span>
            {panel === "secret" ? (
              <code className="hash" style={{ color: "var(--fg)" }}>
                {wallet.secretKey}
              </code>
            ) : null}
            <button
              className="bout"
              type="button"
              onClick={() => {
                if (panel === "secret") return setPanel("none");
                if (window.confirm("Show your secret key? Anyone who sees it can take everything in this wallet.")) setPanel("secret");
              }}
            >
              {panel === "secret" ? "Hide secret key" : "Show secret key"}
            </button>
          </div>
          <div className="pn">
            <span className="lab">Account</span>
            <div className="kvl">
              <div>
                <span>Network</span>
                <span className="mono">{network}</span>
              </div>
              <div>
                <span>Address</span>
                <span className="mono">{shortAddr(wallet.accountId)}</span>
              </div>
              <div>
                <span>Created</span>
                <span className="mono">{new Date(wallet.createdAt).toLocaleDateString("en-GB", { day: "numeric", month: "short" })}</span>
              </div>
            </div>
            <div className="bset">
              <a className="bout sm" href={explorerAccount(network, wallet.accountId)} target="_blank" rel="noreferrer">
                NearBlocks
              </a>
              <button
                className="blink"
                type="button"
                onClick={() => {
                  if (window.confirm("Remove this wallet from this device? Without your secret key backup it is gone for good.")) {
                    forgetWallet(network);
                    toast("Wallet removed from this device");
                  }
                }}
              >
                Remove from device
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}

/* =============================== NEAR agent =============================== */

export function NearScreen() {
  const router = useRouter();
  const token = useAuthToken();
  const status = useApi(() => api.nearStatus(), []);
  const mandates = useApi(() => api.nearMandates(), [token], !!token);
  const { store } = useWallet();
  const s = status.data;
  const list = mandates.data?.items ?? [];
  const testnet = store.testnet;
  const contractNote = useMemo(() => s?.coverage_note, [s]);

  return (
    <div className="scr s-me">
      <Hd title="NEAR agent" crumb="Your copy limits live on NEAR. Only your wallet can sign, pause or revoke them." back right={s ? <span className="tag g">{s.network}</span> : null} />
      <div className="mgrid2">
        <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
          <div className="pn">
            <span className="lab">How it works</span>
            <div className="steps">
              {[
                ["Discover", "trace finds wallets and explains why coins move, on Ethereum and Solana."],
                ["Prove", "A shadow test replays the wallet with pretend money first."],
                ["Authorize on NEAR", "Your wallet signs a mandate: amount per trade, daily cap, slippage, expiry."],
                ["Execute by intent", "NEAR Intents quotes the route. Your wallet signs. trace never holds keys."],
              ].map(([t, d], i) => (
                <div key={t} className="done">
                  <i>{i + 1}</i>
                  <div>
                    <b>{t}</b>
                    <span className="sub">{d}</span>
                  </div>
                </div>
              ))}
            </div>
          </div>
          <div className="pn">
            <span className="lab">Contract and agent</span>
            <div className="kvl">
              <div>
                <span>Network</span>
                <span className="mono">{s?.network ?? "—"}</span>
              </div>
              <div>
                <span>Mandate contract</span>
                <span className="mono">{s?.contract.account_id ?? "—"}</span>
              </div>
              <div>
                <span>Deployment verified</span>
                <span className={`mono${s?.contract.deployment_verified ? " g" : ""}`}>{s ? (s.contract.deployment_verified ? "Yes, code matches the reviewed build" : "Not verified") : "—"}</span>
              </div>
              <div>
                <span>trace agent</span>
                <span className="mono">{s?.agent.account_id ?? "—"}</span>
              </div>
              <div>
                <span>Auto copy</span>
                <span className="mono">{s ? (s.agent.delegation_enabled ? "On" : "Off until review") : "—"}</span>
              </div>
              <div>
                <span>Server signing</span>
                <span className="mono">Never</span>
              </div>
            </div>
            {contractNote ? (
              <span className="sub" style={{ whiteSpace: "normal" }}>
                {contractNote}
              </span>
            ) : null}
          </div>
        </div>
        <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
          <div className="pn">
            <span className="lab">Signing wallet</span>
            {testnet ? (
              <div className="grow2">
                <div>
                  <b style={{ fontWeight: 500 }}>{testnet.accountId}</b>
                  <div className="sub">Your trace testnet wallet signs your mandates</div>
                </div>
                <a className="bout sm" href={explorerAccount("testnet", testnet.accountId)} target="_blank" rel="noreferrer">
                  View
                </a>
              </div>
            ) : (
              <>
                <span className="sub" style={{ whiteSpace: "normal" }}>
                  Mandates are signed by your trace testnet wallet. Create it in one tap.
                </span>
                <button className="bfill" type="button" onClick={() => router.push("/app/wallet/?net=testnet")}>
                  Create testnet wallet
                </button>
              </>
            )}
          </div>
          <div className="pn">
            <div className="pn-h">
              <span className="lab">Your mandates</span>
              <Lk href="/app/wallets/">New copy</Lk>
            </div>
            {!token ? (
              <>
                <span className="sub">Sign in to see and create mandates.</span>
                <button className="bfill" type="button" onClick={() => router.push("/welcome/?next=/app/near/")}>
                  Sign in
                </button>
              </>
            ) : list.length ? (
              <div>
                {list.map((m) => (
                  <div className="li" key={m.id} style={{ alignItems: "flex-start" }}>
                    <span className={`tk${m.status === "anchored" ? " on" : ""}`}>
                      <Ic id="key" />
                    </span>
                    <div className="grow" style={{ display: "grid", gap: 4 }}>
                      <div className="nm">
                        ${usd0(m.max_trade_usd)} a trade · ${usd0(m.max_daily_usd)} a day
                      </div>
                      <div className="sub">
                        {m.mode.replaceAll("_", " ")} · {m.max_slippage_bps / 100}% slippage · ends {new Date(m.expires_at).toLocaleDateString("en-GB", { day: "numeric", month: "short" })}
                      </div>
                      {m.anchor_tx_hash ? (
                        <a className="hash" href={explorerTx("testnet", m.anchor_tx_hash)} target="_blank" rel="noreferrer">
                          NEAR tx {m.anchor_tx_hash.slice(0, 18)}…
                        </a>
                      ) : (
                        <span className="hash">Not signed yet. Drafts carry no authority.</span>
                      )}
                    </div>
                    <span className={`tag ${m.status === "anchored" ? "g" : ""}`}>{m.status === "anchored" ? "Signed" : m.status}</span>
                  </div>
                ))}
              </div>
            ) : (
              <Empty title="No mandates yet">Pick a smart wallet, run a shadow test, then copy it. Your wallet signs the limits.</Empty>
            )}
          </div>
          <div className="pn">
            <span className="lab">For developers</span>
            <span className="sub" style={{ whiteSpace: "normal" }}>
              Agents can connect through the trace MCP bridge with observe and propose scopes. They never get an execute scope, and every proposal is checked against your NEAR mandate.
            </span>
            <button className="bout" type="button" onClick={() => router.push("/app/security/")}>
              Manage connected agents
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
