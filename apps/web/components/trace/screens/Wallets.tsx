"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useMemo, useState } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import type { Chain, NearMandateCreated, ShadowResults } from "@/lib/api/types";
import { D, HOLD_SOL } from "@/lib/trace/example";
import { chainName, exampleWallet, useWallets, walletQuery, type WalletVM } from "@/lib/trace/data";
import { defaultPair, errText, nearExplorerTx, savedShadows, saveShadow, useNear } from "@/lib/trace/near";
import { cl, money, num, rescale, sgn, shortAddr, walk } from "@/lib/trace/viz";
import { useApp } from "../AppState";
import { Av, Chart2, Hd, Ic, Row, Seg, Spark, Src, Tk } from "../ui";

/** Resolve the wallet a screen is about: example (?w=0xSun) or live (?chain=&address=). */
function useWalletRef(): { ex: WalletVM | null; chain: string | null; address: string | null } {
  const p = useSearchParams();
  const chain = p.get("chain");
  const address = p.get("address");
  if (chain && address) return { ex: null, chain, address };
  return { ex: exampleWallet(p.get("w") || "0xSun"), chain: null, address: null };
}

/* =============================== Smart wallets =============================== */

export function WalletsScreen() {
  const router = useRouter();
  const [tab, setTab] = useState("top");
  const ws = useWallets(tab);
  const list = useMemo(() => {
    const l = ws.data.slice();
    if (!ws.example) return l;
    if (tab === "new") return l.filter((w) => w.isNew);
    if (tab === "copied") return l.sort((a, b) => parseInt((b.copiers || "0").replace(",", "")) - parseInt((a.copiers || "0").replace(",", "")));
    return l.sort((a, b) => (b.d30 ?? 0) - (a.d30 ?? 0));
  }, [ws.data, ws.example, tab]);

  return (
    <div className="scr s-wal">
      <Hd title="Smart wallets" crumb="Wallets that make money on most of their trades" back right={<Src example={ws.example} loading={ws.loading} />} />
      <Seg
        cls="box"
        opts={[
          ["top", "Top 30 days"],
          ["new", "New"],
          ["copied", "Most copied"],
        ]}
        cur={tab}
        onPick={setTab}
      />
      {ws.example && ws.note ? <span className="lab">{ws.note} Showing example wallets.</span> : null}
      <div>
        {list.map((w) => {
          const q = walletQuery(w);
          const d = w.d30 ?? 0;
          return (
            <Row key={w.key} className="wrow" href={`/app/profile/?${q}`}>
              <Av t={w.av} />
              <div style={{ minWidth: 0 }}>
                <div className="nm">{w.name}</div>
                <div className="sub">
                  {w.a} · {w.win === null ? `${w.trades} trades` : `${w.win}% win`}
                </div>
              </div>
              <span className={`mono ${cl(d)} wc`}>{w.d30 === null ? "—" : `${sgn(d)}% 30d`}</span>
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

  const follow = async (key: string, address?: string, chain?: string) => {
    const on = toggleStar(key);
    toast(on ? "Following" : "Unfollowed");
    if (on && token && address && chain) {
      api.addWatchlist({ chain: chain as Chain, address }).catch(() => undefined);
    }
  };

  if (!live) {
    const w = ref.ex as WalletVM;
    const n = w.name;
    const ex = D.wallets[n]!;
    const tr: [string, string, string, string, string?][] =
      n === "0xSun"
        ? [
            ["b", "NEURA", "$127K", "14 min ago"],
            ["b", "SOLX", "$89K", "3 h ago"],
            ["s", "FROG", "$45K", "1 day ago", "+31%"],
            ["b", "NOVA", "$73K", "2 days ago"],
          ]
        : D.order
            .filter((s) => (D.events[s] || []).some((e) => e.w === n))
            .map((s) => {
              const e = D.events[s]!.filter((x) => x.w === n)[0]!;
              return [e.sell ? "s" : "b", s, e.amt, "today"] as [string, string, string, string];
            })
            .slice(0, 4);
    return (
      <div className="scr s-prof">
        <Hd
          title={n}
          crumb={`${w.a} · ${w.copiers} copiers`}
          back
          right={
            <>
              <Src example />
              <button className={`hb${star["w" + n] ? " on" : ""}`} type="button" aria-label="Follow" onClick={() => follow("w" + n)}>
                <Ic id="star" />
              </button>
            </>
          }
        />
        <div className="prof">
          <div style={{ display: "grid", gap: 16 }}>
            <div className="phead">
              <Av t={w.av} l />
              <div style={{ display: "grid", gap: 6 }}>
                <span className="tag g" style={{ justifySelf: "start" }}>
                  {ex.d30 >= 40 ? "Top 1% · 30 days" : ex.isNew ? "New this month" : "Top 10% · 30 days"}
                </span>
                <div className="tags">
                  {ex.style.map((s) => (
                    <span key={s}>{s}</span>
                  ))}
                </div>
              </div>
            </div>
            <div className="stat">
              <div>
                <span className="lab">All time</span>
                <b className="g">+{ex.ret.toLocaleString()}%</b>
              </div>
              <div>
                <span className="lab">Win rate</span>
                <b>{ex.win}%</b>
              </div>
              <div>
                <span className="lab">Trades</span>
                <b>{ex.trades}</b>
              </div>
              <div>
                <span className="lab">Avg hold</span>
                <b>{ex.hold}</b>
              </div>
            </div>
            <div>
              <div className="pn-h" style={{ marginBottom: 8 }}>
                <span className="lab">Last 30 days</span>
                <span className={`mono ${cl(ex.d30)}`}>{sgn(ex.d30)}%</span>
              </div>
              <Chart2 sets={[{ v: walk(ex.seed, 40, ex.d30 >= 0 ? 0.4 : -0.1) }]} />
            </div>
          </div>
          <div style={{ display: "grid", gap: 10, alignContent: "start" }}>
            <span className="lab">
              <b>Recent trades</b>
            </span>
            <div>
              {tr.length ? (
                tr.map((t) => (
                  <Row key={t[1] + t[3]} className="trow" href={`/app/token/?sym=${t[1]}`}>
                    <Tk s={t[1]} on={t[0] === "b"} />
                    <div style={{ minWidth: 0 }}>
                      <div className="nm">
                        {t[0] === "b" ? "Bought " : "Sold "}
                        {t[1]}
                        {t[4] ? (
                          <span className="g mono" style={{ fontSize: 12 }}>
                            {" "}
                            {t[4]}
                          </span>
                        ) : null}
                      </div>
                      <div className="sub">{t[3]}</div>
                    </div>
                    <span className="mono">{t[2]}</span>
                  </Row>
                ))
              ) : (
                <p className="f2">No trades today.</p>
              )}
            </div>
          </div>
        </div>
        <div className="actbar">
          <button className="bout" type="button" onClick={() => router.push(`/app/shadow/?w=${n}`)}>
            <Ic id="flask" />
            Shadow test
          </button>
          <button className="bfill" type="button" onClick={() => router.push(`/app/copy/?w=${n}`)}>
            Copy wallet
          </button>
        </div>
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
  return (
    <div className="scr s-prof">
      <Hd
        title={name}
        crumb={`${shortAddr(addr)} · ${chainName(chain)}`}
        back
        right={
          <>
            <Src example={false} />
            <button className={`hb${star[key] ? " on" : ""}`} type="button" aria-label="Follow" onClick={() => follow(key, addr, chain)}>
              <Ic id="star" />
            </button>
          </>
        }
      />
      {detail.status === "error" ? <p className="err">trace has no record of this wallet yet. It appears once it trades on a protocol trace reads.</p> : null}
      <div className="prof">
        <div style={{ display: "grid", gap: 16 }}>
          <div className="phead">
            <Av t={(alias || addr.replace(/^0x/, "")).slice(0, 2).toUpperCase()} l />
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
  const liveWallets = useWallets("top");
  const exNames = D.wOrder;

  useEffect(() => {
    if (ref.ex) setShadow({ w: ref.ex.name });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [ref.ex?.name]);

  const [liveRes, setLiveRes] = useState<ShadowResults | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  useEffect(() => {
    setLiveRes(null);
    setErr(null);
  }, [ref.address, shadow.a]);

  const switchWallet = () => {
    if (!liveWallets.example) {
      const l = liveWallets.data;
      const i = l.findIndex((w) => w.address === ref.address);
      const n = l[(i + 1) % l.length];
      if (n) router.replace(`/app/shadow/?${walletQuery(n)}`);
      return;
    }
    const i = exNames.indexOf(shadow.w);
    const n = exNames[(i + 1) % exNames.length];
    setShadow({ w: n });
    router.replace(`/app/shadow/?w=${n}`);
  };

  const runLive = async () => {
    if (!ref.address || !ref.chain) return;
    if (!token) {
      router.push("/welcome/?next=" + encodeURIComponent(location.pathname + location.search));
      return;
    }
    setBusy(true);
    setErr(null);
    try {
      const created = await api.createShadowStrategy({
        wallet: { chain: ref.chain as Chain, address: ref.address },
        budget_usd: String(shadow.a),
        size_rule: "fixed_usd",
        delay_seconds: 30,
        slippage_bps: 100,
        exit_rule: "mirror_sells",
      });
      saveShadow({ id: created.id, chain: ref.chain, address: ref.address, budget: String(shadow.a), created_at: new Date().toISOString() });
      const r = await api.shadowResults(created.id);
      setLiveRes(r);
      toast("Shadow test saved. You can copy this wallet now.");
    } catch (e) {
      setErr(errText(e));
    } finally {
      setBusy(false);
    }
  };

  const amountSeg = (
    <div className="r">
      <span className="lab">Amount</span>
      <Seg
        cls="box"
        opts={[50, 100, 250, 500].map((a) => [a, `$${a}`] as [number, string])}
        cur={shadow.a}
        onPick={(a) => setShadow({ a })}
      />
    </div>
  );

  if (ref.address) {
    const r = liveRes;
    const start = num(r?.starting_usd) ?? shadow.a;
    const end = num(r?.ending_usd);
    const ret = num(r?.return_pct);
    const name = shortAddr(ref.address);
    return (
      <div className="scr s-sh">
        <Hd title="Shadow test" crumb="Try copying a wallet with pretend money" back right={<Src example={false} />} />
        <div className="shgrid">
          <div className="cfg">
            <div className="r">
              <span className="lab">Wallet</span>
              <button className="wpick" type="button" onClick={switchWallet}>
                <Av t={ref.address.replace(/^0x/, "").slice(0, 2).toUpperCase()} />
                <span className="grow" style={{ textAlign: "left" }}>
                  <b style={{ fontWeight: 500 }}>{name}</b>
                  <span className="sub" style={{ display: "block" }}>
                    {chainName(ref.chain as string)} · tap to switch
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
            {amountSeg}
            <button className="bfill" type="button" disabled={busy} onClick={runLive}>
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
            {r ? (
              <>
                {r.data_status === "unavailable" ? <p className="sub" style={{ whiteSpace: "normal" }}>{r.coverage_note ?? "Not enough priced trades to replay yet."}</p> : null}
                <div className="stat">
                  <div>
                    <span className="lab">Trades</span>
                    <b>{r.trades}</b>
                  </div>
                  <div>
                    <span className="lab">Win rate</span>
                    <b>{r.win_rate_pct === null ? "—" : `${Math.round(Number(r.win_rate_pct))}%`}</b>
                  </div>
                  <div>
                    <span className="lab">Could not copy</span>
                    <b>{r.skipped_unexecutable}</b>
                    <em>no price or route</em>
                  </div>
                  <div>
                    <span className="lab">Fees, slippage</span>
                    <b>−{money(Number(r.fees_and_slippage_usd))}</b>
                    <em>already counted</em>
                  </div>
                </div>
                <div className="kvl">
                  {Object.entries(r.baseline).map(([k, v]) => (
                    <div key={k}>
                      <span>{k.replaceAll("_", " ")}</span>
                      <span className="mono">{v === null ? "—" : String(v)}</span>
                    </div>
                  ))}
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

  const w = D.wallets[shadow.w] ?? D.wallets["0xSun"]!;
  const wn = D.wallets[shadow.w]! ? shadow.w : "0xSun";
  const r = (w[`d${shadow.d}` as "d7" | "d30" | "d90"] ?? w.d30) / 100;
  const res = shadow.a * (1 + r);
  const hold = shadow.a * (1 + HOLD_SOL[shadow.d]! / 100);
  const trades = Math.round(shadow.d * 0.4);
  const wins = Math.round((trades * (w.win - 11)) / 100);
  const missed = Math.max(1, Math.round(trades / 6));
  const fees = shadow.a * 0.042;
  const A = rescale(walk(w.seed + shadow.d, 30, r >= 0 ? 0.5 : -0.1), shadow.a, res);
  const B = rescale(walk(130 + shadow.d, 30, 0.12), shadow.a, hold);

  return (
    <div className="scr s-sh">
      <Hd title="Shadow test" crumb="Try copying a wallet with pretend money" back right={<Src example />} />
      <div className="shgrid">
        <div className="cfg">
          <div className="r">
            <span className="lab">Wallet</span>
            <button className="wpick" type="button" onClick={switchWallet}>
              <Av t={w.av} />
              <span className="grow" style={{ textAlign: "left" }}>
                <b style={{ fontWeight: 500 }}>{wn}</b>
                <span className="sub" style={{ display: "block" }}>
                  {w.win}% win · tap to switch
                </span>
              </span>
              <Ic id="swap" />
            </button>
          </div>
          <div className="r">
            <span className="lab">Period</span>
            <Seg
              cls="box"
              opts={[
                [7, "7 days"],
                [30, "30 days"],
                [90, "90 days"],
              ]}
              cur={shadow.d}
              onPick={(d) => setShadow({ d })}
            />
          </div>
          {amountSeg}
        </div>
        <div style={{ display: "grid", gap: 14 }}>
          <span className="lab">Your ${shadow.a} would now be</span>
          <div className="res">
            <span className="big n">{money(res)}</span>
            <span className={`mono ${cl(r)}`}>{sgn(r * 100)}%</span>
            <span className="mono f3">holding SOL {sgn(HOLD_SOL[shadow.d]!)}%</span>
          </div>
          <Chart2
            sets={[
              { v: A },
              { v: B, c: "rgba(237,242,239,.5)", dash: true, w: 1.3 },
            ]}
            base={shadow.a}
          />
          <div className="legend">
            <span>
              <i />
              copying {wn}
            </span>
            <span>
              <i className="d" />
              holding SOL · {money(hold)}
            </span>
          </div>
          <div className="stat">
            <div>
              <span className="lab">Trades</span>
              <b>{trades}</b>
            </div>
            <div>
              <span className="lab">Wins</span>
              <b>
                {wins} of {trades}
              </b>
            </div>
            <div>
              <span className="lab">Could not copy</span>
              <b>{missed}</b>
              <em>sold too fast</em>
            </div>
            <div>
              <span className="lab">Fees, slippage</span>
              <b>−{money(fees)}</b>
              <em>already counted</em>
            </div>
          </div>
        </div>
      </div>
      <div className="actbar">
        <button className="bout" type="button" onClick={switchWallet}>
          Try another wallet
        </button>
        <button className="bfill" type="button" onClick={() => router.push(`/app/copy/?w=${wn}`)}>
          Copy {wn}
        </button>
      </div>
    </div>
  );
}

/* =============================== Copy (NEAR mandate) =============================== */

const GUARDS = ["Only coins with good liquidity", "Skip high-risk coins", "Daily cap from your spending limit", "Expires in 7 days"];

export function CopyScreen() {
  const router = useRouter();
  const ref = useWalletRef();
  const { copy, setCopy, prefs, toast } = useApp();
  const near = useNear();
  const [created, setCreated] = useState<NearMandateCreated | null>(null);
  const [busy, setBusy] = useState(false);
  const [err, setErr] = useState<string | null>(null);
  const [done, setDone] = useState<string | null>(null);

  const live = !!ref.address;
  const ex = ref.ex;
  const name = live ? shortAddr(ref.address as string) : (ex as WalletVM).name;
  const sub = live ? `${chainName(ref.chain as string)} · ${shortAddr(ref.address as string)}` : (ex as WalletVM).a;
  const winLine = live ? "Limits are enforced by your NEAR mandate" : `${(ex as WalletVM).win}% win rate · about 4 trades a week`;
  const shadow = useMemo(() => (live ? savedShadows().find((s) => s.address === ref.address) : undefined), [live, ref.address, done]); // eslint-disable-line react-hooks/exhaustive-deps
  const delegation = !!near.status.data?.agent.delegation_enabled;
  const daily = prefs.lim > 0 ? Math.max(prefs.lim, copy.amt) : copy.amt * 5;

  const step = !near.token ? 0 : !live ? 1 : !shadow ? 1 : !near.wallet ? 2 : !created ? 3 : 4;
  const label = done
    ? "Open your copies"
    : ["Sign in to start", live ? "Run a shadow test first" : "Pick a live wallet to copy", "Connect NEAR wallet", "Prepare NEAR mandate", "Review and sign in NEAR wallet"][step];

  const act = async () => {
    setErr(null);
    if (done) return router.push("/app/wallet/?t=copies");
    if (step === 0) return router.push("/welcome/?next=" + encodeURIComponent(location.pathname + location.search));
    if (step === 1) return router.push(live ? `/app/shadow/?chain=${ref.chain}&address=${encodeURIComponent(ref.address as string)}` : "/app/wallets/");
    if (step === 2) return near.connect();
    setBusy(true);
    try {
      if (step === 3) {
        const items = near.assets.data?.items ?? [];
        const [origin, destination] = defaultPair(items);
        if (!origin || !destination) throw new Error("NEAR Intents did not return any assets. Try again in a minute.");
        const c = await near.prepare({
          shadowId: (shadow as { id: string }).id,
          mode: copy.mode === "auto" && delegation ? "delegated" : "approval_required",
          origin,
          destination,
          maxTrade: String(copy.amt),
          maxDaily: String(daily),
          slippageBps: copy.g[0] ? 100 : 300,
        });
        setCreated(c);
      } else if (step === 4 && created) {
        const hash = await near.sign(created);
        setDone(hash);
        setCreated(null);
        toast(`${copy.mode === "auto" ? "Copying" : "Alerts on for"} ${name} · $${copy.amt} per trade`);
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
        <Av t={live ? (ref.address as string).replace(/^0x/, "").slice(0, 2).toUpperCase() : (ex as WalletVM).av} />
        <div>
          <div className="nm">{name}</div>
          <div className="sub">{sub}</div>
        </div>
      </div>
      <div className="sheet">
        <span className="grab" />
        <Hd title={`Copy ${name}`} crumb={winLine} back />
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
          <span className="sub">
            Up to ${daily.toLocaleString()} a day · about ${(copy.amt * 4).toLocaleString()} a week at this pace
          </span>
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
            ["Sign in", near.token ? "Passkey account ready" : "A passkey keeps your account"],
            ["Shadow test", !live ? "Pick a live wallet first" : shadow ? `Tested with $${shadow.budget} pretend money` : "Prove it with pretend money"],
            ["NEAR wallet", near.wallet ? near.wallet.accountId : `Your wallet signs the limits on ${near.network}`],
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
        {err || near.error ? <p className="err">{err || near.error}</p> : null}
        {done ? (
          <p className="okmsg">
            Mandate anchored on NEAR.{" "}
            <a href={nearExplorerTx(near.network, done)} target="_blank" rel="noreferrer" style={{ textDecoration: "underline" }}>
              View the transaction
            </a>
          </p>
        ) : null}
        <button className="bfill" style={{ height: 50 }} type="button" disabled={busy || near.busy || (step === 3 && near.assets.status === "loading")} onClick={act}>
          {busy || near.busy ? "Waiting" : label}
        </button>
        <span className="sub" style={{ textAlign: "center", whiteSpace: "normal" }}>
          trace never holds your keys. Pause or revoke any time from Wallet · Copies.
        </span>
      </div>
    </div>
  );
}

