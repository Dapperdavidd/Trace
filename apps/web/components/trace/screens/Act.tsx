"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import type { NearAsset, NearIntentPreview, NearMandate } from "@/lib/api/types";
import { D, SOLP } from "@/lib/trace/example";
import { coinHref, useCoins, type CoinVM } from "@/lib/trace/data";
import { errText, nearExplorerTx, savedShadows, toBaseUnits, useNear } from "@/lib/trace/near";
import { cl, money, pf, sgn, walk } from "@/lib/trace/viz";
import { useApp } from "../AppState";
import { Av, Chart2, Hd, Ic, Lk, Row, Seg, Src, Tk } from "../ui";

/* =============================== Swap =============================== */

function assetLabel(a: NearAsset | undefined) {
  return a ? `${a.symbol}` : "—";
}

export function SwapScreen() {
  const router = useRouter();
  const params = useSearchParams();
  const { toast } = useApp();
  const coins = useCoins("trending");
  const token = useAuthToken();
  const mandates = useApi(() => api.nearMandates(), [token], !!token);
  const assets = useApi(() => api.nearAssets(), [token], !!token);
  const usable = (mandates.data?.items ?? []).filter((m) => m.status === "anchored" || m.status === "draft");
  const mandate: NearMandate | undefined = usable.find((m) => m.status === "anchored") ?? usable[0];
  const liveMode = !!(token && mandate);

  /* example state (design behaviour) */
  const [to, setTo] = useState<string>(params.get("to") ?? "NEURA");
  const [amt, setAmt] = useState(1);
  const [confirm, setConfirm] = useState(false);
  const exList = coins.example ? coins.data : coins.data;
  const d: CoinVM = (exList.find((c) => c.sym === to) ?? exList[0])!;
  useEffect(() => {
    const t = params.get("to");
    if (t) setTo(t);
  }, [params]);

  /* live state (NEAR Intents dry quote under a mandate) */
  const find = (id: string | undefined) => assets.data?.items.find((a) => a.assetId === id);
  const pay = mandate ? find(mandate.allowed_assets[0]?.asset_id) : undefined;
  const get = mandate ? find(mandate.allowed_assets[1]?.asset_id) : undefined;
  const [liveAmt, setLiveAmt] = useState("1");
  const [quote, setQuote] = useState<NearIntentPreview | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);

  const reviewLive = async () => {
    if (!mandate || !pay || !get) return;
    if (quote) {
      toast("Quote saved. trace never submits trades. Your NEAR wallet signs when execution opens.");
      return;
    }
    const base = toBaseUnits(liveAmt, pay.decimals);
    if (!base) return setErr("Type an amount, for example 1 or 0.5.");
    setBusy(true);
    setErr(null);
    try {
      const q = await api.nearIntentPreview({
        mandate_id: mandate.id,
        quote: {
          originAsset: pay.assetId,
          destinationAsset: get.assetId,
          amount: base,
          slippageTolerance: Math.min(mandate.max_slippage_bps, 100),
          recipient: mandate.owner_near_account,
          refundTo: mandate.owner_near_account,
          deadline: new Date(Date.now() + 10 * 60_000).toISOString(),
        },
      });
      setQuote(q);
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };

  if (liveMode) {
    const q = quote?.quote.quote;
    return (
      <div className="scr s-swap">
        <Hd title="Trade" crumb="Swap one coin for another through NEAR Intents" right={<Src example={false} />} />
        <div className="swgrid">
          <div style={{ display: "grid", gap: 14 }}>
            <Seg cls="box" opts={[["swap", "Swap"], ["buy", "Buy"], ["sell", "Sell"]]} cur="swap" onPick={(v) => v !== "swap" && toast("Buy and sell use the same NEAR route. Pick the assets in your mandate.")} />
            <div>
              <div className="sbox">
                <div className="pn-h">
                  <span className="lab">You pay</span>
                  <span className="mono f3" style={{ fontSize: 11 }}>
                    Up to ${mandate.max_trade_usd} per trade
                  </span>
                </div>
                <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                  <input
                    className="amt"
                    inputMode="decimal"
                    value={liveAmt}
                    onChange={(e) => {
                      setLiveAmt(e.target.value.replace(",", "."));
                      setQuote(null);
                    }}
                    aria-label={`Amount of ${assetLabel(pay)}`}
                  />
                  <span className="tsel">
                    <Tk s={pay?.symbol ?? "?"} />
                    {assetLabel(pay)}
                  </span>
                </div>
                <span className="mono f3" style={{ fontSize: 11.5 }}>
                  {q ? `$${Number(q.amountInUsd).toFixed(2)}` : pay ? `on ${pay.blockchain}` : "Loading assets"}
                </span>
              </div>
              <div className="flip">
                <Ic id="swap" />
              </div>
              <div className="sbox" style={{ paddingTop: 22 }}>
                <span className="lab">You get</span>
                <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                  <span className="amt">{q ? Number(q.amountOutFormatted).toLocaleString("en-US", { maximumFractionDigits: 6 }) : "—"}</span>
                  <span className="tsel">
                    <Tk s={get?.symbol ?? "?"} on />
                    {assetLabel(get)}
                  </span>
                </div>
                <span className="mono f3" style={{ fontSize: 11.5 }}>
                  {q ? `$${Number(q.amountOutUsd).toFixed(2)} · at least ${q.minAmountOut} base units` : "Review to get a live quote"}
                </span>
              </div>
            </div>
          </div>
          <div style={{ display: "grid", gap: 12, alignContent: "start" }}>
            <div className="kvl">
              <div>
                <span>Route</span>
                <span className="mono">NEAR Intents 1Click</span>
              </div>
              <div>
                <span>Max slippage</span>
                <span className="mono">{(Math.min(mandate.max_slippage_bps, 100) / 100).toFixed(2)}%</span>
              </div>
              <div>
                <span>Time estimate</span>
                <span className="mono">{q ? `${q.timeEstimate}s` : "—"}</span>
              </div>
              <div>
                <span>Mandate</span>
                <span className="mono">{mandate.status === "anchored" ? "Signed on NEAR" : "Draft, not signed"}</span>
              </div>
            </div>
            <div className="warnrow okk">
              <Ic id="shield" />
              <span className="grow">Your NEAR mandate caps this at ${mandate.max_trade_usd} a trade and ${mandate.max_daily_usd} a day</span>
            </div>
            {err ? <p className="err">{err}</p> : null}
            <button className="bfill" style={{ height: 50 }} type="button" disabled={busy || !pay || !get} onClick={reviewLive}>
              {busy ? "Getting a quote" : quote ? "Confirm swap" : "Review swap"}
            </button>
            {quote ? (
              <span className="sub" style={{ textAlign: "center", whiteSpace: "normal" }}>
                {quote.note}
              </span>
            ) : null}
          </div>
        </div>
      </div>
    );
  }

  const out = (amt * SOLP) / d.p;
  return (
    <div className="scr s-swap">
      <Hd title="Trade" crumb="Swap one coin for another" right={<Src example />} />
      <div className="swgrid">
        <div style={{ display: "grid", gap: 14 }}>
          <Seg cls="box" opts={[["swap", "Swap"], ["buy", "Buy"], ["sell", "Sell"]]} cur="swap" onPick={(v) => v !== "swap" && toast("Coming soon")} />
          <div>
            <div className="sbox">
              <div className="pn-h">
                <span className="lab">You pay</span>
                <span className="mono f3" style={{ fontSize: 11 }}>
                  Balance 8.20 SOL
                </span>
              </div>
              <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                <input
                  className="amt"
                  inputMode="decimal"
                  value={amt}
                  onChange={(e) => {
                    const v = Math.max(0, Math.min(8.2, parseFloat(e.target.value.replace(",", ".")) || 0));
                    setAmt(v);
                    setConfirm(false);
                  }}
                  aria-label="Amount of SOL"
                />
                <span className="tsel">
                  <Tk s="SOL" />
                  SOL
                </span>
              </div>
              <span className="mono f3" style={{ fontSize: 11.5 }}>
                {money(amt * SOLP)}
              </span>
            </div>
            <div className="flip">
              <Ic id="swap" />
            </div>
            <div className="sbox" style={{ paddingTop: 22 }}>
              <span className="lab">You get</span>
              <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                <span className="amt">{Math.round(out).toLocaleString()}</span>
                <button
                  className="tsel"
                  type="button"
                  onClick={() => {
                    const i = exList.indexOf(d);
                    setTo(exList[(i + 1) % exList.length]!.sym);
                    setConfirm(false);
                  }}
                >
                  <Tk s={d.sym} on />
                  {d.sym} ▾
                </button>
              </div>
              <span className="mono f3" style={{ fontSize: 11.5 }}>
                {pf(d.p)} per {d.sym}
              </span>
            </div>
          </div>
          <Seg
            cls="box"
            opts={[
              [0.25, "25%"],
              [0.5, "50%"],
              [0.75, "75%"],
              [1, "Max"],
            ]}
            cur=""
            onPick={(v) => {
              setAmt(+(8.2 * v).toFixed(2));
              setConfirm(false);
            }}
          />
        </div>
        <div style={{ display: "grid", gap: 12, alignContent: "start" }}>
          <div className="kvl">
            <div>
              <span>Rate</span>
              <span className="mono">
                1 SOL = {Math.round(SOLP / d.p).toLocaleString()} {d.sym}
              </span>
            </div>
            <div>
              <span>Price impact</span>
              <span className="mono g">{(d.liqN ?? 0) > 2 ? "0.3%" : "1.4%"}</span>
            </div>
            <div>
              <span>Network fee</span>
              <span className="mono">$0.004</span>
            </div>
            <div>
              <span>Route</span>
              <span className="mono">NEAR Intents</span>
            </div>
          </div>
          {d.risk !== "Low" ? (
            <Row className="warnrow" href={coinHref(d, "&tab=rk")}>
              <Ic id="shield" />
              <span className="grow">
                {d.sym} is {d.risk === "Unrated" ? "not rated yet" : `${d.risk.toLowerCase()} risk`}
              </span>
              <span className="lk">
                Review <Ic id="arr" />
              </span>
            </Row>
          ) : (
            <div className="warnrow okk">
              <Ic id="shield" />
              <span className="grow">{d.sym} passed its risk check</span>
            </div>
          )}
          <button
            className="bfill"
            style={{ height: 50 }}
            type="button"
            onClick={() => {
              if (!confirm) return setConfirm(true);
              setConfirm(false);
              if (!token) return router.push("/welcome/?next=/app/copy/");
              toast("Example only. Copy a wallet to sign a NEAR mandate, then quotes are real.");
            }}
          >
            {confirm ? "Confirm swap" : "Review swap"}
          </button>
          {confirm ? (
            <span className="sub" style={{ textAlign: "center" }}>
              You pay {amt} SOL · you get about {Math.round(out).toLocaleString()} {d.sym}
            </span>
          ) : (
            <span className="sub" style={{ textAlign: "center", whiteSpace: "normal" }}>
              Real quotes come from NEAR Intents once you sign a copy mandate.
            </span>
          )}
        </div>
      </div>
    </div>
  );
}

/* =============================== Wallet =============================== */

export function WalletScreen() {
  const router = useRouter();
  const params = useSearchParams();
  const { toast } = useApp();
  const token = useAuthToken();
  const near = useNear();
  const alerts = useApi(() => api.alerts(), [token], !!token);
  const [t, setT] = useState(params.get("t") ?? "tokens");
  useEffect(() => {
    const p = params.get("t");
    if (p) setT(p);
  }, [params]);
  const mandates = near.mandates.data?.items ?? [];
  const liveCopies = !!token && mandates.length > 0;
  const liveActivity = !!token && (alerts.data?.items.length ?? 0) > 0;
  const acct = near.wallet?.accountId;

  return (
    <div className="scr s-wlt">
      <Hd
        title="Wallet"
        crumb={acct ? `NEAR · ${acct}` : "Connect a NEAR wallet to sign copy limits"}
        right={
          acct ? (
            <button
              className="hb"
              type="button"
              aria-label="Copy address"
              onClick={() => {
                navigator.clipboard?.writeText(acct).catch(() => undefined);
                toast(`Address ${acct} copied`);
              }}
            >
              <Ic id="copy" />
            </button>
          ) : (
            <button className="bout sm" type="button" onClick={near.connect} disabled={near.busy || !near.status.data}>
              {near.busy ? "Connecting" : "Connect NEAR"}
            </button>
          )
        }
      />
      <div className="wgrid">
        <div style={{ display: "grid", gap: 14 }}>
          <div className="vrow">
            <span className="big n">$2,840.25</span>
            <span className="mono g">+$313.33 · +12.4% 24h</span>
            <Src example />
          </div>
          <div className="qa">
            <button className="qb" type="button" onClick={() => router.push("/app/near/")}>
              <Ic id="key" />
              <span>Mandates</span>
            </button>
            <button className="qb" type="button" onClick={() => router.push("/app/shadow/")}>
              <Ic id="flask" />
              <span>Test</span>
            </button>
            <button className="qb" type="button" onClick={() => router.push("/app/swap/")}>
              <Ic id="swap" />
              <span>Swap</span>
            </button>
            <button className="qb pri" type="button" onClick={() => router.push("/app/wallets/")}>
              <Ic id="plus" />
              <span>Copy</span>
            </button>
          </div>
          <Seg
            cls="box"
            opts={[
              ["tokens", "Tokens"],
              ["copies", `Copies · ${liveCopies ? mandates.length : 3}`],
              ["activity", "Activity"],
            ]}
            cur={t}
            onPick={setT}
          />
          <div>
            {t === "tokens" ? (
              <>
                <span className="lab">
                  <Src example /> trace never holds tokens. Balances live in your own wallet.
                </span>
                {D.holdings.map((h) => (
                  <div className="hrow" key={h[0]}>
                    <Tk s={h[0]} on={h[0] === "NEURA"} />
                    <div style={{ minWidth: 0 }}>
                      <div className="nm">{h[1]}</div>
                      <div className="mono">{h[2]}</div>
                    </div>
                    <div className="rt">
                      <div className="v">{money(h[3])}</div>
                      <div className={`mono ${cl(h[4])}`}>{sgn(h[4])}%</div>
                    </div>
                  </div>
                ))}
              </>
            ) : null}
            {t === "copies" ? (
              liveCopies ? (
                mandates.map((m) => (
                  <Row key={m.id} href="/app/near/">
                    <Av t="N" />
                    <div className="grow">
                      <div className="nm">{m.mode === "delegated" ? "Auto copy" : m.mode === "paper" ? "Paper only" : "Alerts only"}</div>
                      <div className="sub">
                        ${m.max_trade_usd} a trade · ${m.max_daily_usd} a day · ends {new Date(m.expires_at).toLocaleDateString("en-GB", { day: "numeric", month: "short" })}
                      </div>
                    </div>
                    <span className={`tag ${m.status === "anchored" ? "g" : ""}`}>{m.status === "anchored" ? "Signed" : m.status}</span>
                  </Row>
                ))
              ) : (
                <>
                  <span className="lab">
                    <Src example /> {token ? "You have no copies yet." : "Sign in to copy a wallet."}
                  </span>
                  {[
                    { w: "0xSun", amt: 120.25, ch: 48.6 },
                    { w: "CryptoNomad", amt: 80.1, ch: 22.3 },
                    { w: "DeFiWhale", amt: 98.4, ch: -3.1 },
                  ].map((c) => (
                    <Row key={c.w} href={`/app/profile/?w=${c.w}`}>
                      <Av t={D.wallets[c.w]!.av} />
                      <div className="grow">
                        <div className="nm">{c.w}</div>
                        <div className="sub">auto copy · {money(c.amt)}</div>
                      </div>
                      <span className={`mono ${cl(c.ch)}`}>{sgn(c.ch)}%</span>
                    </Row>
                  ))}
                </>
              )
            ) : null}
            {t === "activity" ? (
              liveActivity ? (
                alerts.data?.items.map((a) => (
                  <div className="li" key={a.id}>
                    <span className="tk on">
                      <Ic id={a.type === "risk_event" ? "shield" : "bolt"} />
                    </span>
                    <div className="grow">
                      <div className="nm" style={{ fontWeight: 400 }}>
                        {a.title}
                      </div>
                      <div className="sub">
                        {new Date(a.event_time).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" })} · {a.body}
                      </div>
                    </div>
                  </div>
                ))
              ) : (
                <>
                  <span className="lab">
                    <Src example /> {token ? "Follow a wallet to see its trades here." : "Sign in to see alerts from wallets you follow."}
                  </span>
                  {D.activity.map((a) => (
                    <Row key={a[0]! + a[2]} href={`/app/token/?sym=${a[2]}`}>
                      <Av t={D.wallets[a[0]!]!.av} />
                      <div className="grow">
                        <div className="nm" style={{ fontWeight: 400 }}>
                          {a[0]} {a[1]} <b style={{ fontWeight: 500 }}>{a[2]}</b>
                        </div>
                        <div className="sub">
                          {a[4]}
                          {a[5] ? ` · ${a[5]}` : ""}
                        </div>
                      </div>
                      <span className="mono">{a[3]}</span>
                    </Row>
                  ))}
                </>
              )
            ) : null}
          </div>
        </div>
        <div className="pn">
          <span className="lab">
            <b>This week</b> <Src example />
          </span>
          <Chart2 sets={[{ v: walk(21, 40, 0.36) }]} />
          <div className="kvl">
            <div>
              <span>Copies</span>
              <span className="mono g">+18.7%</span>
            </div>
            <div>
              <span>Your own trades</span>
              <span className="mono g">+6.2%</span>
            </div>
            <div>
              <span>Fees paid</span>
              <span className="mono">$3.84</span>
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
  const { toast } = useApp();
  const near = useNear();
  const s = near.status.data;
  const mandates = near.mandates.data?.items ?? [];
  const shadows = useMemo(() => (typeof window === "undefined" ? [] : savedShadows()), []);
  const [anchoring, setAnchoring] = useState<string | null>(null);
  const [msg, setMsg] = useState<{ ok: boolean; t: string } | null>(null);

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
                ["Authorize on NEAR", "Your mandate sets assets, an amount per trade, a daily cap, slippage and expiry."],
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
            {s?.coverage_note ? (
              <span className="sub" style={{ whiteSpace: "normal" }}>
                {s.coverage_note}
              </span>
            ) : null}
          </div>
        </div>
        <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
          <div className="pn">
            <span className="lab">Your NEAR wallet</span>
            {near.wallet ? (
              <div className="grow2">
                <div>
                  <b style={{ fontWeight: 500 }}>{near.wallet.accountId}</b>
                  <div className="sub">Owns, pauses and revokes your mandates</div>
                </div>
                <button className="bout sm" type="button" onClick={near.disconnect} disabled={near.busy}>
                  Disconnect
                </button>
              </div>
            ) : (
              <>
                <p className="sub" style={{ whiteSpace: "normal", margin: 0 }}>
                  Connect the NEAR account that will own your copy limits. It keeps its own keys.
                </p>
                <button className="bfill" type="button" onClick={near.connect} disabled={near.busy || !s}>
                  {near.busy ? "Connecting" : "Connect NEAR wallet"}
                </button>
              </>
            )}
            {near.error ? <p className="err">{near.error}</p> : null}
          </div>
          <div className="pn">
            <div className="pn-h">
              <span className="lab">Your mandates</span>
              <Lk href="/app/wallets/">New copy</Lk>
            </div>
            {!near.token ? (
              <>
                <p className="sub" style={{ whiteSpace: "normal", margin: 0 }}>
                  Sign in to see and create mandates.
                </p>
                <button className="bfill" type="button" onClick={() => router.push("/welcome/?next=/app/near/")}>
                  Sign in
                </button>
              </>
            ) : mandates.length ? (
              <div>
                {mandates.map((m) => (
                  <div className="li" key={m.id} style={{ alignItems: "flex-start" }}>
                    <span className={`tk${m.status === "anchored" ? " on" : ""}`}>
                      <Ic id="key" />
                    </span>
                    <div className="grow" style={{ display: "grid", gap: 4 }}>
                      <div className="nm">
                        ${m.max_trade_usd} a trade · ${m.max_daily_usd} a day
                      </div>
                      <div className="sub">
                        {m.mode.replaceAll("_", " ")} · {m.max_slippage_bps / 100}% slippage · ends {new Date(m.expires_at).toLocaleDateString("en-GB", { day: "numeric", month: "short" })}
                      </div>
                      {m.anchor_tx_hash ? (
                        <a className="hash" href={nearExplorerTx(near.network, m.anchor_tx_hash)} target="_blank" rel="noreferrer">
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
              <p className="sub" style={{ whiteSpace: "normal", margin: 0 }}>
                No mandates yet. {shadows.length ? "You have a shadow test saved, so you can copy that wallet now." : "Run a shadow test on a wallet, then copy it."}
              </p>
            )}
            {mandates.some((m) => m.status === "draft") && near.wallet ? (
              <span className="sub" style={{ whiteSpace: "normal" }}>
                To sign a draft, open Copy for that wallet again and prepare a fresh mandate. Drafts expire unused.
              </span>
            ) : null}
            {msg ? <p className={msg.ok ? "okmsg" : "err"}>{msg.t}</p> : null}
            {anchoring ? <span className="sub">Signing {anchoring}</span> : null}
          </div>
          <div className="pn">
            <span className="lab">For developers</span>
            <p className="sub" style={{ whiteSpace: "normal", margin: 0 }}>
              Agents can connect through the trace MCP bridge with observe and propose scopes. They never get an execute scope, and every proposal is checked against your NEAR mandate.
            </p>
            <button
              className="bout"
              type="button"
              onClick={() => {
                setAnchoring(null);
                setMsg(null);
                router.push("/app/security/");
                toast("Connected apps live in Security");
              }}
            >
              Manage connected agents
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
