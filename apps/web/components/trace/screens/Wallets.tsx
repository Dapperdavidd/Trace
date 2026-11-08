"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useState } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import type { Chain, NearMandateCreated, ShadowResults } from "@/lib/api/types";
import { chainName, useWallets, walletQuery } from "@/lib/trace/data";
import { defaultPair, errText, savedShadows, saveShadow } from "@/lib/trace/near";
import { createWallet, explorerTx, signFunctionCall, useWalletStore } from "@/lib/trace/wallet";
import { cl, money, num, sgn, shortAddr } from "@/lib/trace/viz";
import { useApp } from "../AppState";
import { Av, Hd, Ic, Row, Seg, Spark, Src, Tk } from "../ui";
import { Empty } from "./Research";

function useWalletRef(): { chain: string | null; address: string | null } {
  const p = useSearchParams();
  return { chain: p.get("chain"), address: p.get("address") };
}
const avOf = (addr: string, alias?: string | null) => (alias || addr.replace(/^0x/, "")).slice(0, 2).toUpperCase();

/* =============================== Smart wallets =============================== */

export function WalletsScreen() {
  const router = useRouter();
  const [tab, setTab] = useState("top");
  const ws = useWallets(tab);

  return (
    <div className="scr s-wal">
      <Hd title="Smart wallets" crumb="Wallets ranked by their real trading record" back right={<Src example={false} loading={ws.loading} />} />
      <Seg
        cls="box"
        opts={[
          ["top", "Top"],
          ["new", "New"],
          ["copied", "Most followed"],
        ]}
        cur={tab}
        onPick={setTab}
      />
      {ws.loading ? <p className="sub">Scoring wallets from their trades…</p> : null}
      {!ws.loading && !ws.data.length ? <Empty title="No ranked wallets yet">{ws.note ?? "A wallet needs enough closed trades to be scored. The list fills in as trace reads more of the chain."}</Empty> : null}
      <div>
        {ws.data.map((w) => {
          const q = walletQuery(w);
          const d = w.d30 ?? 0;
          return (
            <Row key={w.key} className="wrow" href={`/app/profile/?${q}`}>
              <Av t={w.av} />
              <div style={{ minWidth: 0 }}>
                <div className="nm">{w.name}</div>
                <div className="sub">
                  {w.chain} · {w.win === null ? `${w.trades} closed trades` : `${w.win}% win`}
                </div>
              </div>
              <span className={`mono ${cl(d)} wc`}>{w.d30 === null ? "—" : `${sgn(d)}%`}</span>
              <span className="mono wc">{w.win === null ? "—" : `${w.win}%`}</span>
              <span className="tags wc">
                {w.style.slice(0, 2).map((s) => (
                  <span key={s}>{s}</span>
                ))}
              </span>
              <span className="wc">
                <Spark seed={w.seed} up={d >= 0} />
              </span>
              <span className="rt" style={{ display: "flex", gap: 6, alignItems: "center" }}>
                <span className={`mono ${cl(d)} wm`}>{w.d30 === null ? "" : `${sgn(d)}%`}</span>
                <button className="bout wc" type="button" style={{ height: 32, padding: "0 10px" }} onClick={() => router.push(`/app/shadow/?${q}`)}>
                  Test
                </button>
                <button className="bfill wc" type="button" style={{ height: 32, padding: "0 10px" }} onClick={() => router.push(`/app/copy/?${q}`)}>
                  Copy
                </button>
              </span>
            </Row>
          );
        })}
      </div>
    </div>
  );
}

/* =============================== Wallet profile =============================== */

export function ProfileScreen() {
  const router = useRouter();
  const ref = useWalletRef();
  const live = !!ref.address;
  const { star, toggleStar, toast } = useApp();
  const token = useAuthToken();
  const detail = useApi(() => api.wallet(ref.chain as Chain, ref.address as string), [ref.address], live);
  const metrics = useApi(() => api.walletMetrics(ref.chain as Chain, ref.address as string), [ref.address], live);
  const trades = useApi(() => api.walletTrades(ref.chain as Chain, ref.address as string), [ref.address], live);
  const dna = useApi(() => api.walletDna(ref.chain as Chain, ref.address as string), [ref.address], live);

  if (!live) {
    return (
      <div className="scr s-prof">
        <Hd title="Wallet" back />
        <Empty title="No wallet selected" action={<button className="bout" type="button" onClick={() => router.push("/app/wallets/")}>Smart wallets</button>}>
          Open a wallet from Smart wallets, a coin page or the daily briefing, or paste an address into search.
        </Empty>
      </div>
    );
  }

  const addr = ref.address as string;
  const chain = ref.chain as string;
  const alias = detail.data?.wallet.alias;
  const name = alias || shortAddr(addr);
  const m = metrics.data;
  const pnl = num(m?.realized_pnl_usd);
  const q = `chain=${encodeURIComponent(chain)}&address=${encodeURIComponent(addr)}`;
  const key = `w${chain}:${addr}`;
  const follow = () => {
    const on = toggleStar(key);
    toast(on ? "Following" : "Unfollowed");
    if (on && token) api.addWatchlist({ chain: chain as Chain, address: addr }).catch(() => undefined);
  };
  return (
    <div className="scr s-prof">
      <Hd
        title={name}
        crumb={`${shortAddr(addr)} · ${chainName(chain)}`}
        back
        right={
          <>
            <Src example={false} />
            <button className={`hb${star[key] ? " on" : ""}`} type="button" aria-label="Follow" onClick={follow}>
              <Ic id="star" />
            </button>
          </>
        }
      />
      {detail.status === "error" ? <p className="err">trace has no record of this wallet yet. It appears once it trades on a protocol trace reads.</p> : null}
      <div className="prof">
        <div style={{ display: "grid", gap: 16 }}>
          <div className="phead">
            <Av t={avOf(addr, alias)} l />
            <div style={{ display: "grid", gap: 6 }}>
              <span className={`tag ${m?.wqs.decision === "eligible" ? "g" : ""}`} style={{ justifySelf: "start" }}>
                {m ? (m.wqs.decision === "eligible" ? "Proven wallet" : m.wqs.decision.replaceAll("_", " ")) : detail.data?.monitoring ?? "Loading"}
              </span>
              <div className="tags">
                {(dna.data?.traits ?? []).slice(0, 4).map((t) => (
                  <span key={t.key} title={t.evidence}>
                    {t.label}
                  </span>
                ))}
              </div>
            </div>
          </div>
          <div className="stat">
            <div>
              <span className="lab">Realized P&amp;L</span>
              <b className={pnl !== null && pnl >= 0 ? "g" : ""}>{pnl === null ? "—" : money(pnl)}</b>
            </div>
            <div>
              <span className="lab">Trades</span>
              <b>{m?.total_trades ?? detail.data?.metrics.total_trades ?? "—"}</b>
            </div>
            <div>
              <span className="lab">Coverage</span>
              <b>{m?.coverage_ratio ? `${Math.round(Number(m.coverage_ratio) * 100)}%` : "—"}</b>
            </div>
            <div>
              <span className="lab">Confidence</span>
              <b>{m?.pnl_confidence ?? "—"}</b>
            </div>
          </div>
          {m?.wqs.explanation ? (
            <div className="verdict">
              <span className="lab">Why this score</span>
              {m.wqs.explanation}
            </div>
          ) : null}
        </div>
        <div style={{ display: "grid", gap: 10, alignContent: "start" }}>
          <span className="lab">
            <b>Recent trades</b>
          </span>
          <div>
            {(trades.data?.items ?? []).slice(0, 8).map((t) => {
              const sym = t.token.symbol || shortAddr(t.token.address);
              return (
                <Row key={t.tx_id} className="trow" href={`/app/token/?chain=${t.token.chain}&address=${encodeURIComponent(t.token.address)}`}>
                  <Tk s={sym} on={t.side === "buy"} />
                  <div style={{ minWidth: 0 }}>
                    <div className="nm">
                      {t.side === "buy" ? "Bought " : "Sold "}
                      {sym}
                    </div>
                    <div className="sub">
                      {new Date(t.event_time).toLocaleString("en-GB", { day: "numeric", month: "short", hour: "2-digit", minute: "2-digit" })}
                      {t.finality === "tentative" ? " · not final yet" : ""}
                    </div>
                  </div>
                  <span className="mono">{t.amount_usd ? money(Number(t.amount_usd)) : "—"}</span>
                </Row>
              );
            })}
            {trades.status === "ok" && !trades.data?.items.length ? <p className="f2">No trades recorded yet.</p> : null}
          </div>
        </div>
      </div>
      <div className="actbar">
        <button className="bout" type="button" onClick={() => router.push(`/app/shadow/?${q}`)}>
          <Ic id="flask" />
          Shadow test
        </button>
        <button className="bfill" type="button" onClick={() => router.push(`/app/copy/?${q}`)}>
          Copy wallet
        </button>
      </div>
    </div>
  );
}

/* =============================== Shadow test =============================== */

export function ShadowScreen() {
  const router = useRouter();
  const ref = useWalletRef();
  const { shadow, setShadow, toast } = useApp();
  const token = useAuthToken();
  const ranked = useWallets("top");
  const [res, setRes] = useState<ShadowResults | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  // No wallet in the link: start with the top ranked wallet.
  useEffect(() => {
    if (!ref.address && ranked.data[0]) router.replace(`/app/shadow/?${walletQuery(ranked.data[0])}`);
  }, [ref.address, ranked.data, router]);
  useEffect(() => {
    setRes(null);
    setErr(null);
  }, [ref.address, shadow.a]);

  if (!ref.address || !ref.chain) {
    return (
      <div className="scr s-sh">
        <Hd title="Shadow test" crumb="Try copying a wallet with pretend money" back />
        {ranked.loading ? <p className="sub">Finding a wallet to test…</p> : <Empty title="No wallet to test yet">Open any wallet and tap Shadow test, or paste a wallet address into search.</Empty>}
      </div>
    );
  }

  const switchWallet = () => {
    const l = ranked.data;
    if (!l.length) return toast("No other ranked wallets yet");
    const i = l.findIndex((w) => w.address === ref.address);
    const n = l[(i + 1) % l.length];
    if (n) router.replace(`/app/shadow/?${walletQuery(n)}`);
  };

  const run = async () => {
    if (!token) return router.push("/welcome/?next=" + encodeURIComponent(location.pathname + location.search));
    setBusy(true);
    setErr(null);
    try {
      const created = await api.createShadowStrategy({
        wallet: { chain: ref.chain as Chain, address: ref.address as string },
        budget_usd: String(shadow.a),
        size_rule: "fixed_usd",
        delay_seconds: 30,
        slippage_bps: 100,
        exit_rule: "mirror_sells",
      });
      saveShadow({ id: created.id, chain: ref.chain as string, address: ref.address as string, budget: String(shadow.a), created_at: new Date().toISOString() });
      setRes(await api.shadowResults(created.id));
      toast("Shadow test saved. You can copy this wallet now.");
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };

  const start = num(res?.starting_usd) ?? shadow.a;
  const end = num(res?.ending_usd);
  const ret = num(res?.return_pct);
  const name = shortAddr(ref.address);
  return (
    <div className="scr s-sh">
      <Hd title="Shadow test" crumb="Try copying a wallet with pretend money" back right={<Src example={false} />} />
      <div className="shgrid">
        <div className="cfg">
          <div className="r">
            <span className="lab">Wallet</span>
            <button className="wpick" type="button" onClick={switchWallet}>
              <Av t={avOf(ref.address)} />
              <span className="grow" style={{ textAlign: "left" }}>
                <b style={{ fontWeight: 500 }}>{name}</b>
                <span className="sub" style={{ display: "block" }}>
                  {chainName(ref.chain)} · tap to switch
                </span>
              </span>
              <Ic id="swap" />
            </button>
          </div>
          <div className="r">
            <span className="lab">Period</span>
            <span className="sub" style={{ whiteSpace: "normal" }}>
              Every finalized trade trace has recorded for this wallet.
            </span>
          </div>
          <div className="r">
            <span className="lab">Amount</span>
            <Seg cls="box" opts={[50, 100, 250, 500].map((a) => [a, `$${a}`] as [number, string])} cur={shadow.a} onPick={(a) => setShadow({ a })} />
          </div>
          <button className="bfill" type="button" disabled={busy} onClick={run}>
            {busy ? "Replaying trades" : token ? "Run shadow test" : "Sign in to run it"}
          </button>
          {err ? <p className="err">{err}</p> : null}
        </div>
        <div style={{ display: "grid", gap: 14 }}>
          <span className="lab">Your ${start} would now be</span>
          <div className="res">
            <span className="big n">{end === null ? "—" : money(end)}</span>
            {ret !== null ? <span className={`mono ${cl(ret)}`}>{sgn(ret)}%</span> : null}
          </div>
          {res ? (
            <>
              {res.data_status === "unavailable" ? (
                <p className="sub" style={{ whiteSpace: "normal" }}>
                  {res.coverage_note ?? "Not enough priced trades to replay yet."}
                </p>
              ) : null}
              <div className="stat">
                <div>
                  <span className="lab">Trades</span>
                  <b>{res.trades}</b>
                </div>
                <div>
                  <span className="lab">Win rate</span>
                  <b>{res.win_rate_pct === null ? "—" : `${Math.round(Number(res.win_rate_pct))}%`}</b>
                </div>
                <div>
                  <span className="lab">Could not copy</span>
                  <b>{res.skipped_unexecutable}</b>
                  <em>no price or route</em>
                </div>
                <div>
                  <span className="lab">Fees, slippage</span>
                  <b>−{money(Number(res.fees_and_slippage_usd))}</b>
                  <em>already counted</em>
                </div>
              </div>
            </>
          ) : (
            <p className="sub" style={{ whiteSpace: "normal" }}>
              trace replays this wallet&apos;s real trades with your amount, a 30 second delay and 1% slippage. Nothing is spent.
            </p>
          )}
        </div>
      </div>
      <div className="actbar">
        <button className="bout" type="button" onClick={switchWallet}>
          Try another wallet
        </button>
        <button className="bfill" type="button" onClick={() => router.push(`/app/copy/?chain=${ref.chain}&address=${encodeURIComponent(ref.address as string)}`)}>
          Copy {name}
        </button>
      </div>
    </div>
  );
}

/* =============================== Copy (NEAR mandate, signed by the in-app wallet) =============================== */

const GUARDS = ["Only coins with good liquidity", "Skip high-risk coins", "Daily cap", "Expires in 7 days"];

export function CopyScreen() {
  const router = useRouter();
  const ref = useWalletRef();
  const token = useAuthToken();
  const { copy, setCopy, prefs, toast } = useApp();
  const store = useWalletStore();
  const testnet = store.testnet ?? null;
  const nearStatus = useApi(() => api.nearStatus(), []);
  const assets = useApi(() => api.nearAssets(), [token], !!token);
  const [created, setCreated] = useState<NearMandateCreated | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);

  const addr = ref.address;
  // Saved shadow tests live in localStorage: read them after mount so server and client HTML match.
  const [shadow, setSavedShadow] = useState<ReturnType<typeof savedShadows>[number] | undefined>(undefined);
  useEffect(() => setSavedShadow(addr ? savedShadows().find((s) => s.address === addr) : undefined), [addr, done]);
  const delegation = !!nearStatus.data?.agent.delegation_enabled;
  const daily = prefs.lim > 0 ? Math.max(prefs.lim, copy.amt) : copy.amt * 5;

  if (!addr || !ref.chain) {
    return (
      <div className="scr s-copy">
        <div className="sheet">
          <Hd title="Copy a wallet" back />
          <Empty title="Pick a wallet first" action={<button className="bfill" type="button" onClick={() => router.push("/app/wallets/")}>Smart wallets</button>}>
            Choose a ranked wallet, test it with pretend money, then copy it with limits you sign on NEAR.
          </Empty>
        </div>
      </div>
    );
  }
  const name = shortAddr(addr);

  // 0 sign in, 1 shadow test, 2 testnet wallet, 3 prepare, 4 sign
  const step = !token ? 0 : !shadow ? 1 : !testnet ? 2 : !created ? 3 : 4;
  const label = done ? "Open your copies" : ["Sign in to start", "Run a shadow test first", "Create your testnet wallet", "Prepare NEAR mandate", "Sign with your trace wallet"][step];

  const act = async () => {
    setErr(null);
    if (done) return router.push("/app/wallet/?t=copies");
    if (step === 0) return router.push("/welcome/?next=" + encodeURIComponent(location.pathname + location.search));
    if (step === 1) return router.push(`/app/shadow/?chain=${ref.chain}&address=${encodeURIComponent(addr)}`);
    setBusy(true);
    try {
      if (step === 2) {
        await createWallet("testnet", "trace");
        toast("Testnet wallet created with free test NEAR");
      } else if (step === 3) {
        const [origin, destination] = defaultPair(assets.data?.items ?? []);
        if (!origin || !destination) throw new Error("NEAR Intents did not return any assets. Try again in a minute.");
        const c = await api.createNearMandate({
          shadow_strategy_id: (shadow as { id: string }).id,
          owner_near_account: (testnet as { accountId: string }).accountId,
          agent_near_account: nearStatus.data?.agent.account_id ?? undefined,
          mode: copy.mode === "auto" && delegation ? "delegated" : "approval_required",
          allowed_chains: Array.from(new Set([origin.blockchain, destination.blockchain])),
          allowed_assets: [
            { chain: origin.blockchain, asset_id: origin.assetId },
            { chain: destination.blockchain, asset_id: destination.assetId },
          ],
          max_trade_usd: String(copy.amt),
          max_daily_usd: String(daily),
          max_slippage_bps: copy.g[0] ? 100 : 300,
          expires_at: new Date(Date.now() + 7 * 86_400_000).toISOString(),
        });
        setCreated(c);
      } else if (step === 4 && created) {
        const tx = created.near_transaction;
        const call = tx?.actions[0]?.params;
        if (!tx || !call) throw new Error("The API did not return a transaction to sign.");
        const hash = await signFunctionCall("testnet", tx.receiverId, call);
        await api.confirmNearAnchor(created.id, { tx_hash: hash, signer_account_id: (testnet as { accountId: string }).accountId });
        setDone(hash);
        setCreated(null);
        toast(`${copy.mode === "auto" && delegation ? "Copying" : "Alerts on for"} ${name} · $${copy.amt} per trade`);
      }
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="scr s-copy">
      <div className="cdim">
        <Av t={avOf(addr)} />
        <div>
          <div className="nm">{name}</div>
          <div className="sub">{chainName(ref.chain)}</div>
        </div>
      </div>
      <div className="sheet">
        <span className="grab" />
        <Hd title={`Copy ${name}`} crumb="Limits are enforced by a NEAR mandate your wallet signs" back />
        <div className="modes">
          <button type="button" className={copy.mode === "auto" ? "on" : ""} onClick={() => setCopy({ mode: "auto" })}>
            <b>Auto copy</b>
            <span>{delegation ? "Mirrors new buys for you" : "Opens after the security review"}</span>
          </button>
          <button type="button" className={copy.mode === "alerts" ? "on" : ""} onClick={() => setCopy({ mode: "alerts" })}>
            <b>Alerts only</b>
            <span>You approve each trade</span>
          </button>
        </div>
        <div style={{ display: "grid", gap: 8 }}>
          <span className="lab">Amount per trade</span>
          <Seg cls="box" opts={[50, 100, 250, 500].map((a) => [a, `$${a}`] as [number, string])} cur={copy.amt} onPick={(amt) => setCopy({ amt })} />
          <span className="sub">Up to ${daily.toLocaleString()} a day</span>
        </div>
        <div>
          <span className="lab">Guardrails</span>
          {GUARDS.map((t, k) => (
            <div className="grow2" key={t}>
              <span>{k === 2 ? `Stop for the day at $${daily.toLocaleString()}` : t}</span>
              <button
                type="button"
                className={`sw${copy.g[k] ? " on" : ""}`}
                aria-label={t}
                onClick={() => {
                  if (k >= 2) return toast("This limit is part of every NEAR mandate");
                  const g = copy.g.slice();
                  g[k] = !g[k];
                  setCopy({ g });
                }}
              />
            </div>
          ))}
        </div>
        <div className="steps" aria-label="Copy setup steps">
          {[
            ["Sign in", token ? "Google account ready" : "Google keeps your account connected"],
            ["Shadow test", shadow ? `Tested with $${shadow.budget} pretend money` : "Prove it with pretend money"],
            ["Testnet wallet", testnet ? testnet.accountId : "Created in one tap, funded by the NEAR faucet"],
            ["Mandate", done ? "Signed and verified on NEAR" : created ? "Draft ready. Your wallet signs next" : "Amount, daily cap, slippage, expiry"],
          ].map(([t, s], i) => {
            const at = Math.min(step, 3);
            const st = done || i < at ? "done" : i === at ? "cur" : "";
            return (
              <div key={t} className={st}>
                <i>{st === "done" ? "✓" : i + 1}</i>
                <div>
                  <b>{t}</b>
                  <span className="sub">{s}</span>
                </div>
              </div>
            );
          })}
        </div>
        {err ? <p className="err">{err}</p> : null}
        {done ? (
          <p className="okmsg">
            Mandate anchored on NEAR testnet.{" "}
            <a href={explorerTx("testnet", done)} target="_blank" rel="noreferrer" style={{ textDecoration: "underline" }}>
              View the transaction
            </a>
          </p>
        ) : null}
        <button className="bfill" style={{ height: 50 }} type="button" disabled={busy || (step === 3 && assets.status === "loading")} onClick={act}>
          {busy ? "Working" : label}
        </button>
        <span className="sub" style={{ textAlign: "center", whiteSpace: "normal" }}>
          Mandates live on NEAR testnet while the contract is reviewed. Your keys stay on this device.
        </span>
      </div>
    </div>
  );
}
