"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useMemo, useRef, useState } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import type { BriefingItem, RiskCheck } from "@/lib/api/types";
import { coinHref, useCoin, useCoins, useWallets, walletQuery, type CoinVM } from "@/lib/trace/data";
import type { ChartEvent } from "@/lib/trace/engine";
import { useWallet, yoctoToNear } from "@/lib/trace/wallet";
import { cl, clamp, money, pf, resample, sgn, shortAddr } from "@/lib/trace/viz";
import { useApp } from "../AppState";
import { usd0 } from "@/lib/trace/viz";
import { TraceChartView } from "../Three";
import { Av, Hd, Help, Ic, Lk, RiskTag, Row, Seg, Spark, Src, StateTag, Tk } from "../ui";

/* =============================== shared =============================== */

export function Empty({ title, children, action }: { title: string; children?: React.ReactNode; action?: React.ReactNode }) {
  return (
    <div className="pn" style={{ textAlign: "left" }}>
      <b style={{ fontWeight: 500, fontSize: 16 }}>{title}</b>
      {children ? (
        <span className="sub" style={{ whiteSpace: "normal" }}>
          {children}
        </span>
      ) : null}
      {action}
    </div>
  );
}

function greeting(d: Date) {
  const h = d.getHours();
  return h < 12 ? "Good morning." : h < 18 ? "Good afternoon." : "Good evening.";
}
function dateLabel(d: Date) {
  return d.toLocaleDateString("en-GB", { weekday: "short", day: "2-digit", month: "short" }) + " · " + d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
}

/* =============================== Today =============================== */

export function TodayScreen({ meInitials }: { meInitials?: string }) {
  const router = useRouter();
  const token = useAuthToken();
  const coins = useCoins("trending");
  const mandates = useApi(() => api.nearMandates(), [token], !!token);
  const { network, wallet, balance, usd } = useWallet();
  const [now, setNow] = useState<Date | null>(null);
  useEffect(() => setNow(new Date()), []);
  const signals = coins.data.slice(0, 3);
  const flagged = coins.data.find((c) => c.risk === "High");
  const live = mandates.data?.items ?? [];
  const near = balance ? Number(yoctoToNear(balance.available, 6).replace(/,/g, "")) : 0;

  return (
    <div className="scr s-today">
      <div className="hello">
        <div>
          <div className="hrow2">
            <span className="lab">{now ? dateLabel(now) : " "}</span>
            <button className="avb" type="button" onClick={() => router.push(token ? "/app/me/" : "/welcome/")} aria-label="Your profile">
              {meInitials || <Ic id="user" />}
            </button>
          </div>
          <h1>{now ? greeting(now) : "Hello."}</h1>
          <p>
            {coins.loading
              ? "Reading the chain…"
              : coins.data.length
                ? `${coins.data.length} coin${coins.data.length === 1 ? "" : "s"} moving right now.`
                : "No coins have cleared the safety filters yet today."}{" "}
            {token ? (live.length ? `You have ${live.length} copy mandate${live.length === 1 ? "" : "s"} on NEAR.` : "Test a wallet with pretend money, then copy it with limits.") : "Sign in to follow wallets and copy them with limits."}
          </p>
        </div>
        <div className="qa">
          <button className="qb" type="button" onClick={() => router.push("/app/wallet/?a=send")}>
            <Ic id="send" />
            <span>Send</span>
          </button>
          <button className="qb" type="button" onClick={() => router.push("/app/wallet/?a=receive")}>
            <Ic id="recv" />
            <span>Receive</span>
          </button>
          <button className="qb" type="button" onClick={() => router.push("/app/swap/")}>
            <Ic id="swap" />
            <span>Swap</span>
          </button>
          <button className="qb pri" type="button" onClick={() => router.push("/app/swap/?mode=buy")}>
            <Ic id="plus" />
            <span>Buy</span>
          </button>
        </div>
      </div>
      <div className="tg">
        <div className="pn">
          <div className="pn-h">
            <span className="lab">
              Your wallet <span className={`tag ${network === "mainnet" ? "g" : ""}`}>{network === "mainnet" ? "Mainnet" : "Testnet"}</span>
            </span>
            <Lk href="/app/wallet/">Wallet</Lk>
          </div>
          {wallet ? (
            <>
              <div className="vrow">
                <span className="big n">{balance ? `${yoctoToNear(balance.available, 2)} NEAR` : "…"}</span>
                {usd !== null && balance ? <span className="mono f2">{money(near * usd)}</span> : null}
              </div>
              <div className="kvl">
                <div>
                  <span>Account</span>
                  <span className="mono">{shortAddr(wallet.accountId)}</span>
                </div>
                <div>
                  <span>NEAR price</span>
                  <span className="mono">{usd === null ? "—" : money(usd)}</span>
                </div>
              </div>
            </>
          ) : (
            <>
              <span className="sub" style={{ whiteSpace: "normal" }}>
                Create your trace wallet. Your keys stay on this device. On testnet you get free tokens to try everything.
              </span>
              <button className="bfill" type="button" onClick={() => router.push("/app/wallet/")}>
                Create wallet
              </button>
            </>
          )}
        </div>
        <div className="pn">
          <div className="pn-h">
            <span className="lab">
              Moving now <Src example={false} loading={coins.loading} />
            </span>
            <Lk href="/app/briefing/">Briefing</Lk>
          </div>
          <div>
            {signals.map((c, i) => (
              <Row key={c.key} href={coinHref(c)}>
                <Tk s={c.sym} on={i === 0} />
                <div className="grow">
                  <div className="nm">{c.sym}</div>
                  <div className="sub">{c.why}</div>
                </div>
                <Spark values={c.spark} seed={c.seed} up={c.ch >= 0} />
                <span className={`mono ${cl(c.ch)}`}>{sgn(c.ch)}%</span>
              </Row>
            ))}
            {!coins.loading && !signals.length ? (
              <p className="sub" style={{ whiteSpace: "normal" }}>
                {coins.note ?? "Nothing has cleared the liquidity and freshness filters yet."} The daily briefing still has today&apos;s most traded coins.
              </p>
            ) : null}
          </div>
          {flagged ? (
            <div className="note">
              <Ic id="shield" />
              Risk alert · {flagged.sym}: {flagged.why.toLowerCase()}
            </div>
          ) : null}
        </div>
        <div className="pn">
          <div className="pn-h">
            <span className="lab">Wallets you copy</span>
            <Lk href="/app/wallets/">Find more</Lk>
          </div>
          <div>
            {live.slice(0, 3).map((m) => (
              <Row key={m.id} href="/app/near/">
                <Av t="N" />
                <div className="grow">
                  <div className="nm">{shortAddr(m.owner_near_account)}</div>
                  <div className="sub">
                    ${usd0(m.max_trade_usd)} per trade · {m.mode.replaceAll("_", " ")}
                  </div>
                </div>
                <span className={`tag ${m.status === "anchored" ? "g" : ""}`}>{m.status === "anchored" ? "Signed" : m.status}</span>
              </Row>
            ))}
            {!live.length ? (
              <p className="sub" style={{ whiteSpace: "normal" }}>
                {token ? "You are not copying anyone yet. Pick a smart wallet, test it with pretend money, then copy it with limits you sign on NEAR." : "Sign in to copy a wallet with limits you sign on NEAR."}
              </p>
            ) : null}
          </div>
          <div className="note">
            <span>Skip high risk</span>
            <span>Max per trade</span>
            <span>Signed on NEAR</span>
          </div>
        </div>
        <div className="pn">
          <div className="pn-h">
            <span className="lab">What you own</span>
            <Lk href="/app/wallet/">Wallet</Lk>
          </div>
          {wallet && balance ? (
            <>
              <div className="alloc">
                <i style={{ width: "100%", background: "var(--g)" }} />
              </div>
              <div className="hrow">
                <Tk s="NEAR" on />
                <div style={{ minWidth: 0 }}>
                  <div className="nm">NEAR</div>
                  <div className="mono">{yoctoToNear(balance.available, 4)} NEAR</div>
                </div>
                <div className="rt">
                  <div className="v">{usd === null ? "—" : money(near * usd)}</div>
                  <div className="mono f3">{network}</div>
                </div>
              </div>
            </>
          ) : (
            <p className="sub" style={{ whiteSpace: "normal" }}>
              {wallet ? "Reading your balance…" : "No wallet yet. Create one from the Wallet tab."}
            </p>
          )}
        </div>
      </div>
    </div>
  );
}

/* =============================== Briefing =============================== */

function itemHref(it: BriefingItem): string | undefined {
  if (it.token) return `/app/token/?chain=${it.token.chain}&address=${encodeURIComponent(it.token.address)}`;
  if (it.wallet) return `/app/profile/?chain=${it.wallet.chain}&address=${encodeURIComponent(it.wallet.address)}`;
  return undefined;
}
function sectionIcon(title: string): string {
  if (/risk/i.test(title)) return "shield";
  if (/wallet/i.test(title)) return "users";
  if (/move|coin|traded|crowd/i.test(title)) return "bolt";
  if (/chain/i.test(title)) return "globe";
  return "book";
}

export function BriefingScreen() {
  const b = useApi(() => api.briefings(), []);
  const [now, setNow] = useState<Date | null>(null);
  useEffect(() => setNow(new Date()), []);
  const live = b.data && !b.data.demo ? b.data : null;
  const sections = live?.sections ?? [];
  const short = sections.find((s) => s.title === "The short version");
  const rest = sections.filter((s) => s !== short);

  return (
    <div className="scr s-brief">
      <Hd
        title="Daily briefing"
        crumb={now ? `${now.toLocaleDateString("en-GB", { weekday: "long", day: "numeric", month: "long" })} · last 24 hours` : " "}
        right={<Src example={false} loading={b.status === "loading"} />}
        back
      />
      {b.status === "loading" ? <p className="sub">Writing today&apos;s report from the chain…</p> : null}
      {b.status === "error" ? <Empty title="The briefing could not load">trace could not reach its API. It may be waking up; try again in a moment.</Empty> : null}
      {b.status === "ok" && !sections.length ? <Empty title="Not enough activity yet">{live?.coverage_note ?? "The report fills in as trace reads more trades."}</Empty> : null}
      {sections.length ? (
        <div className="brief">
          <div style={{ display: "grid", gap: 14, alignContent: "start" }}>
            {short ? (
              <div className="sumcard">
                <span className="lab">
                  <b>The short version</b>
                </span>
                {short.items.map((i) => i.text).join(" ")}
              </div>
            ) : null}
            <span className="sub" style={{ whiteSpace: "normal" }}>
              Written by trace from finalized on-chain trades, prices and safety checks. Facts only, no predictions. Tap any line to open the coin or wallet.
            </span>
          </div>
          <div style={{ display: "grid", gap: 18 }}>
            {rest.map((s) => (
              <div key={s.title} style={{ display: "grid", gap: 4 }}>
                <span className="lab">
                  <b>{s.title}</b>
                </span>
                {s.items.map((it, i) => {
                  const href = itemHref(it);
                  return (
                    <Row key={i} href={href}>
                      <span className={`tk${i === 0 ? " on" : ""}`}>
                        <Ic id={sectionIcon(s.title)} />
                      </span>
                      <div className="grow">
                        <div className="sub" style={{ whiteSpace: "normal", color: "var(--fg)", fontSize: 14 }}>
                          {it.text}
                        </div>
                      </div>
                      {href ? <Ic id="arr" /> : null}
                    </Row>
                  );
                })}
              </div>
            ))}
          </div>
        </div>
      ) : null}
    </div>
  );
}

/* =============================== Discover =============================== */

export function DiscoverScreen() {
  const params = useSearchParams();
  const router = useRouter();
  const [f, setF] = useState("trending");
  const [q, setQ] = useState(params.get("q") ?? "");
  useEffect(() => setQ(params.get("q") ?? ""), [params]);
  const coins = useCoins(f);
  const { star, toggleStar, toast } = useApp();

  const list = useMemo(() => {
    const Q = q.trim().toUpperCase();
    if (!Q) return coins.data;
    return coins.data.filter((c) => c.sym.toUpperCase().includes(Q) || c.name.toUpperCase().includes(Q) || c.why.toUpperCase().includes(Q) || (c.address ?? "").toUpperCase() === Q);
  }, [coins.data, q]);

  const addr = q.trim();
  const looksLikeAddress = /^(0x[0-9a-fA-F]{40}|[1-9A-HJ-NP-Za-km-z]{32,44})$/.test(addr);
  const chainOf = addr.startsWith("0x") ? "ethereum" : "solana";

  return (
    <div className="scr s-disc">
      <Hd title="Discover" crumb="Coins people are buying right now" right={<Src example={false} loading={coins.loading} />} />
      <div className="topf">
        <label className="search">
          <Ic id="srch" />
          <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search coins, or paste a coin or wallet address" autoComplete="off" />
        </label>
        <Seg
          cls="box"
          opts={[
            ["trending", "Trending"],
            ["early", "Early"],
            ["smart", "Smart money"],
            ["new", "New"],
          ]}
          cur={f}
          onPick={(v) => {
            setF(v);
            setQ("");
          }}
        />
      </div>
      <span className="lab">
        <i className="dot live" />
        {coins.loading ? "Reading the chain…" : `Live from Ethereum and Solana${coins.window === "1h" ? ", last hour (the 24 hour view fills in after a day of prices)" : ""}. Tap a coin to see who is buying.`}
      </span>
      {f === "early" ? (
        <Help>
          <b style={{ color: "var(--fg)", fontWeight: 500 }}>Early</b> means proven wallets are buying but the price has barely moved yet.
        </Help>
      ) : null}
      {looksLikeAddress ? (
        <div className="bset">
          <button className="bout sm" type="button" onClick={() => router.push(`/app/token/?chain=${chainOf}&address=${encodeURIComponent(addr)}`)}>
            Open as a coin
          </button>
          <button className="bout sm" type="button" onClick={() => router.push(`/app/profile/?chain=${chainOf}&address=${encodeURIComponent(addr)}`)}>
            Open as a wallet
          </button>
        </div>
      ) : null}
      <div>
        {!coins.loading && !list.length ? (
          <Empty title={q ? `No live coins match “${q}”` : "No coins in this list right now"}>
            {q
              ? "Paste a full coin or wallet address to open it directly."
              : coins.note
                ? `${coins.note} trace only lists coins with fresh prices and enough liquidity to trade, so the list can be short.`
                : "trace only lists coins with fresh prices and enough liquidity to trade. Check the daily briefing for today's most traded coins."}
          </Empty>
        ) : list.length ? (
          <>
            <div className="dhead">
              <span />
              <span>Coin</span>
              <span>Price</span>
              <span>{coins.window}</span>
              <span>Volume</span>
              <span>Trend</span>
              <span>Why it is moving</span>
              <span>Risk</span>
              <span />
            </div>
            {list.map((c, i) => (
              <Row key={c.key} className="drow" href={coinHref(c)}>
                <Tk s={c.sym} on={i === 0} />
                <div style={{ minWidth: 0 }}>
                  <div className="nm">{c.sym}</div>
                  <div className="sub dm">{c.why}</div>
                  <div className="sub dc">
                    {c.name} · {c.chain}
                  </div>
                </div>
                <span className="mono dc">{pf(c.p)}</span>
                <span className={`mono ${cl(c.ch)} dc`}>{sgn(c.ch)}%</span>
                <span className="mono dc">{c.vol}</span>
                <Spark values={c.spark} seed={c.seed} up={c.ch >= 0} />
                <span className="sub dc" style={{ color: "var(--fg)" }}>
                  {c.why}
                </span>
                <span className="dc">
                  <RiskTag r={c.risk} short />
                </span>
                <span className="rt dm">
                  <span className={`mono ${cl(c.ch)}`} style={{ display: "block" }}>
                    {sgn(c.ch)}%
                  </span>
                  <span className="mono f3" style={{ fontSize: 11 }}>
                    {pf(c.p)}
                  </span>
                </span>
                <button
                  className={`hb dc${star[c.key] ? " on" : ""}`}
                  type="button"
                  aria-label={`Watch ${c.sym}`}
                  style={{ width: 30, height: 30 }}
                  onClick={() => toast(toggleStar(c.key) ? `${c.sym} added to watchlist` : "Removed from watchlist")}
                >
                  <Ic id="star" />
                </button>
              </Row>
            ))}
          </>
        ) : null}
      </div>
    </div>
  );
}

/* =============================== Token =============================== */

const CHECK_LABEL: Record<string, string> = {
  liquidity: "Liquidity",
  honeypot: "Can you sell?",
  holder_concentration: "Holder concentration",
  contract_safety: "Contract",
  suspicious_activity: "Suspicious trading",
};
const CHECK_STATE: Record<string, string> = { pass: "Good", warn: "Watch", block: "Flag", unknown: "Unknown" };

function liveRisk(checks: RiskCheck[]): { s: string; c: [string, string, string][] } {
  const flags = checks.filter((k) => k.state === "block").length;
  const warns = checks.filter((k) => k.state === "warn").length;
  const unknown = checks.filter((k) => k.state === "unknown").length;
  const s = !checks.length
    ? "These checks have not run for this coin yet."
    : flags
      ? `${flags} check${flags > 1 ? "s" : ""} failed. Read them before you buy.`
      : warns
        ? `${warns} thing${warns > 1 ? "s" : ""} to watch before you size up.`
        : unknown === checks.length
          ? "These checks have not run for this coin yet."
          : "No red flags in the checks that ran.";
  return { s, c: checks.map((k) => [CHECK_LABEL[k.key] ?? k.key, k.note, CHECK_STATE[k.state] ?? "Unknown"]) };
}

function minutesIntoDay(iso: string, nowMs: number): number {
  const age = (nowMs - new Date(iso).getTime()) / 60000;
  return clamp(190 - (age / 1440) * 190, 0, 190);
}

export function TokenScreen() {
  const params = useSearchParams();
  const router = useRouter();
  const ref = { chain: params.get("chain") ?? undefined, address: params.get("address") ?? undefined };
  const [tab, setTab] = useState(params.get("tab") ?? "ov");
  const s = useCoin(ref);
  const movers = useCoins("trending");
  const { star, toggleStar, toast } = useApp();

  if (!s.data) {
    return (
      <div className="scr s-token">
        <Hd title={s.loading ? "Loading coin" : "Coin not found"} crumb={ref.address ? `${ref.chain} · ${shortAddr(ref.address)}` : undefined} back />
        {!s.loading ? (
          <Empty
            title={ref.address ? "trace has no market data for this coin yet" : "No coin selected"}
            action={
              <button className="bout" type="button" onClick={() => router.push("/app/discover/")}>
                Back to Discover
              </button>
            }
          >
            {ref.address ? "It may trade on a protocol trace does not read yet, or nobody has traded it since trace started watching." : "Open a coin from Discover or the daily briefing."}
          </Empty>
        ) : null}
      </div>
    );
  }
  const { coin: c, detail, timeline } = s.data;
  const sym = c.sym;
  const nowMs = detail ? new Date(detail.as_of).getTime() : 0;
  const spark = movers.data.find((m) => m.address === c.address)?.spark;
  const series = spark && spark.length > 1 ? resample(spark, 64) : [];
  const events: ChartEvent[] = (timeline?.items ?? [])
    .filter((t) => t.kind === "fact")
    .slice(0, 6)
    .map((t) => ({ t: minutesIntoDay(t.time, nowMs), w: t.title, amt: "", label: `${t.title} at ${new Date(t.time).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" })}` }));
  const buyer = detail?.buyers[0];
  const shadowHref = buyer ? `/app/shadow/?chain=${buyer.chain}&address=${encodeURIComponent(buyer.address)}` : "/app/shadow/";
  const why: [string, string, string][] = (detail?.why_moving ?? []).map((w) => [w.kind === "fact" ? "bolt" : "chat", w.kind === "fact" ? "Fact." : "Possible reason.", w.text]);
  const tItems = (timeline?.items ?? []).map((it) => ({
    tm: new Date(it.time).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" }),
    b: it.title,
    p: it.detail + (it.kind === "hypothesis" ? " (possible reason, not confirmed)" : ""),
    hot: it.kind === "fact",
  }));
  const R = liveRisk(detail?.risk.checks ?? []);

  return (
    <div className="scr s-token" data-tab={tab}>
      <Hd
        title={
          <>
            <Tk s={sym} on /> {sym}
          </>
        }
        crumb={`${c.name} · ${c.chain}`}
        back
        right={
          <>
            <Src example={false} />
            <button className={`hb${star[c.key] ? " on" : ""}`} type="button" aria-label="Watch" onClick={() => toast(toggleStar(c.key) ? `${sym} added to watchlist` : "Removed from watchlist")}>
              <Ic id="star" />
            </button>
            <button
              className="hb"
              type="button"
              aria-label="Share"
              onClick={() => {
                navigator.clipboard?.writeText(window.location.href).catch(() => undefined);
                toast(`Link to ${sym} copied`);
              }}
            >
              <Ic id="share" />
            </button>
          </>
        }
      />
      <div className="pxrow">
        <span className="big n">{pf(c.p)}</span>
        <span className={`mono ${cl(c.ch)}`}>{sgn(c.ch)}% 24h</span>
        <RiskTag r={c.risk} />
      </div>
      <Seg
        cls="box tseg"
        opts={[
          ["ov", "Overview"],
          ["tl", "Timeline"],
          ["rk", "Risk"],
        ]}
        cur={tab}
        onPick={setTab}
      />
      <div className="tgrid">
        <div className="tp p-ov">
          <div className="tvwrap">
            <div className="tvh">
              <span className="lab">Who bought, and when</span>
              <span className="lk">Drag to turn</span>
            </div>
            {series.length ? (
              <TraceChartView series={series} events={events} seed={c.seed} />
            ) : (
              <div className="tv" style={{ display: "grid", placeItems: "center", padding: 20, textAlign: "center" }}>
                <span className="sub" style={{ whiteSpace: "normal" }}>
                  trace has not recorded a price path for this coin yet. The 3D view appears once it has.
                </span>
              </div>
            )}
            <div className="tvf legend">
              <span>
                <i />
                Price
              </span>
              <span>
                <i className="w" />
                Trading volume
              </span>
              <span>
                <i />
                Each green line is a recorded event
              </span>
            </div>
          </div>
          <div className="stat">
            <div>
              <span className="lab">Market cap</span>
              <b>{c.mc}</b>
            </div>
            <div>
              <span className="lab">Volume 24h</span>
              <b>{c.vol}</b>
            </div>
            <div>
              <span className="lab">Liquidity</span>
              <b>{c.liq}</b>
            </div>
            <div>
              <span className="lab">Holders</span>
              <b>{c.hold}</b>
            </div>
          </div>
          <div className="why">
            <span className="lab">Why it is moving</span>
            {why.length ? (
              why.map((w, i) => (
                <div key={i}>
                  <Ic id={w[0]} />
                  <span>
                    <b>{w[1]}</b> {w[2]}
                  </span>
                </div>
              ))
            ) : (
              <span className="sub">No reason recorded yet.</span>
            )}
          </div>
          {detail?.buyers.length ? (
            <div style={{ display: "grid", gap: 6 }}>
              <span className="lab">Wallets trace saw buying</span>
              {detail.buyers.slice(0, 5).map((w) => (
                <Row key={w.address} href={`/app/profile/?chain=${w.chain}&address=${encodeURIComponent(w.address)}`}>
                  <Av t={(w.alias || w.address.replace(/^0x/, "")).slice(0, 2).toUpperCase()} />
                  <div className="grow">
                    <div className="nm">{w.alias || shortAddr(w.address)}</div>
                    <div className="sub">{w.chain}</div>
                  </div>
                  <Ic id="arr" />
                </Row>
              ))}
            </div>
          ) : null}
        </div>
        <div className="tp p-tl">
          <span className="lab">
            <b>How it moved</b>
          </span>
          <div style={{ fontSize: 16, fontWeight: 500, letterSpacing: "-.02em" }}>
            {sgn(c.ch)}% in 24 hours. {c.why}.
          </div>
          <div className="tl">
            {tItems.length ? (
              tItems.map((it, i) => (
                <div key={i} className={`tli${it.hot ? " hot" : ""}`}>
                  <span className="tm">{it.tm}</span>
                  <span className="dt" />
                  <div>
                    <b>{it.b}</b>
                    <p>{it.p}</p>
                  </div>
                  <span className="px" />
                </div>
              ))
            ) : (
              <p className="sub">No events recorded today.</p>
            )}
          </div>
        </div>
        <div className="tp p-rk">
          <span className="lab">
            <b>Safety check</b> · we test every coin for common traps
          </span>
          <div style={{ display: "flex", justifyContent: "space-between", alignItems: "baseline" }}>
            <span style={{ fontSize: 24, fontWeight: 500, letterSpacing: "-.035em" }}>{c.risk === "Unrated" ? "Not rated yet" : `${c.risk} risk`}</span>
            <span className="mono f3">{c.rn} of 5</span>
          </div>
          <div className={`meter${c.risk === "Low" ? " low" : ""}`}>
            {Array.from({ length: 5 }, (_, k) => (
              <i key={k} className={k < c.rn ? "on" : ""} />
            ))}
          </div>
          <p style={{ margin: 0, color: "var(--fg2)", fontSize: 13.5 }}>{R.s}</p>
          <div>
            {R.c.map((k) => (
              <div className="ck" key={k[0]}>
                <div>
                  <b>{k[0]}</b>
                  <span className="sub">{k[1]}</span>
                </div>
                <StateTag s={k[2]} />
              </div>
            ))}
          </div>
        </div>
      </div>
      <div className="actbar">
        <button className="bout" type="button" onClick={() => router.push(`/app/compare/?a=${encodeURIComponent(c.key)}`)}>
          Compare
        </button>
        <button className="bout" type="button" onClick={() => router.push(shadowHref)}>
          Shadow test
        </button>
        <button className="bfill" type="button" onClick={() => router.push(`/app/swap/?mode=buy&sym=${encodeURIComponent(sym)}`)}>
          Buy
        </button>
      </div>
    </div>
  );
}

/* =============================== Compare =============================== */

function cycle<T>(list: T[], cur: T, skip?: T): T {
  const i = list.indexOf(cur);
  for (let k = 1; k <= list.length; k++) {
    const n = list[(i + k) % list.length] as T;
    if (n !== skip) return n;
  }
  return cur;
}

export function CompareScreen() {
  const params = useSearchParams();
  const router = useRouter();
  const coins = useCoins("trending");
  const keys = coins.data.map((c) => c.key);
  const [pair, setPair] = useState<[string, string] | null>(null);
  useEffect(() => {
    if (keys.length < 2) return;
    const a = params.get("a");
    const A = (a && keys.includes(a) ? a : keys[0]) as string;
    const B = keys.find((k) => k !== A) ?? A;
    setPair([A, B]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [keys.join("|")]);
  const byKey = (k: string) => coins.data.find((c) => c.key === k) as CoinVM;
  const verdict = useApi(() => api.compare(byKey(pair![0]).address as string, byKey(pair![1]).address as string), [pair?.[0], pair?.[1]], !!pair);

  if (!pair || !byKey(pair[0]) || !byKey(pair[1])) {
    return (
      <div className="scr s-cmp">
        <Hd title="Compare" crumb="Two live coins side by side" back />
        {coins.loading ? <p className="sub">Loading live coins…</p> : <Empty title="Not enough live coins to compare">Compare needs at least two coins that cleared trace&apos;s liquidity and freshness filters. Check back soon.</Empty>}
      </div>
    );
  }
  const a = byKey(pair[0]);
  const b = byKey(pair[1]);
  function row(lbl: string, va: string, vb: string, na: number, nb: number, hi: boolean) {
    const wa = hi ? na >= nb : na <= nb;
    const mxv = Math.max(Math.abs(na), Math.abs(nb)) || 1;
    return (
      <div className="cr" key={lbl}>
        <span className="lb2">{lbl}</span>
        <div className="vals">
          <span className={wa ? "g" : ""}>{va}</span>
          <span className={!wa ? "g" : ""}>{vb}</span>
        </div>
        <div className="bars">
          <i className={wa ? "w" : ""} style={{ width: `${(Math.abs(na) / mxv) * 100}%` }} />
          <i className={!wa ? "w" : ""} style={{ width: `${(Math.abs(nb) / mxv) * 100}%` }} />
        </div>
      </div>
    );
  }
  const mom = a.ch >= b.ch ? a : b;
  const safe = a.rn < b.rn ? a : b.rn < a.rn ? b : (a.liqN ?? 0) >= (b.liqN ?? 0) ? a : b;
  const v = verdict.data?.verdict_note || (mom === safe ? `${mom.sym} leads on momentum and on safety today.` : `${mom.sym} has more momentum. ${safe.sym} is the steadier pick.`);

  return (
    <div className="scr s-cmp">
      <Hd title="Compare" crumb="Tap a coin to change it" back right={<Src example={false} />} />
      <div className="cmpbody" style={{ display: "grid", gap: 14 }}>
        <div className="cmph">
          <button type="button" onClick={() => setPair([cycle(keys, pair[0], pair[1]), pair[1]])}>
            <Tk s={a.sym} on l />
            <b>{a.sym}</b>
            <span className="lab">Change</span>
          </button>
          <span className="vs">VS</span>
          <button type="button" onClick={() => setPair([pair[0], cycle(keys, pair[1], pair[0])])}>
            <Tk s={b.sym} l />
            <b>{b.sym}</b>
            <span className="lab">Change</span>
          </button>
        </div>
        <div>
          {row("Change", `${sgn(a.ch)}%`, `${sgn(b.ch)}%`, a.ch, b.ch, true)}
          {a.liqN !== null && b.liqN !== null ? row("Liquidity", a.liq, b.liq, a.liqN, b.liqN, true) : null}
          {row("Risk (lower is safer)", a.risk, b.risk, a.rn, b.rn, false)}
        </div>
        <div className="verdict">
          <span className="lab">Our take</span>
          {v}
        </div>
        <div className="bset">
          <button className="bout" type="button" onClick={() => router.push(coinHref(a))}>
            Open {a.sym}
          </button>
          <button className="bout" type="button" onClick={() => router.push(coinHref(b))}>
            Open {b.sym}
          </button>
        </div>
      </div>
    </div>
  );
}

/* =============================== Ask =============================== */

interface Answer {
  t: string;
  coins?: CoinVM[];
  wallets?: Array<{ name: string; sub: string; href: string; av: string }>;
  f: string[];
  go?: string;
}

function answer(q: string, coins: CoinVM[], walletsLive: ReturnType<typeof useWallets>["data"]): Answer {
  const Q = q.toUpperCase();
  const words = ` ${Q.replace(/[^A-Z0-9 ]/g, " ")} `;
  const c = coins.find((x) => {
    const s = x.sym.toUpperCase().replace(/[^A-Z0-9]/g, "");
    return s.length > 1 && words.includes(` ${s} `);
  });
  if (!coins.length && !walletsLive.length) return { t: "trace has no live coins or ranked wallets to answer from yet. Try the daily briefing for today's most traded coins.", f: [] };
  if (c && /^OPEN THE RISK CHECK/.test(Q)) return { t: "Opening the risk check.", coins: [c], f: [], go: coinHref(c, "&tab=rk") };
  if (c && /SAFE|RISK|SCAM|RUG/.test(Q)) return { t: `${c.sym} is ${c.risk === "Unrated" ? "not rated yet" : c.risk.toLowerCase() + " risk"}. Open it for each check.`, coins: [c], f: [`Open the risk check for ${c.sym}`] };
  if (c && /COMPARE/.test(Q)) return { t: "Opening a side by side view.", coins: [c], f: [], go: `/app/compare/?a=${encodeURIComponent(c.key)}` };
  if (c && /WALLET|WHO|BUY/.test(Q)) return { t: `Open ${c.sym} to see the wallets trace saw buying it.`, coins: [c], f: [`Is ${c.sym} safe?`] };
  if (c) return { t: `${c.sym} is ${sgn(c.ch)}% now. ${c.why}.`, coins: [c], f: [`Is ${c.sym} safe?`, `Compare ${c.sym}`] };
  if (/WALLET|SMART|COPY|TRADER/.test(Q)) {
    if (!walletsLive.length) return { t: "No wallets have enough evidence to be ranked yet.", f: ["What is moving right now?"] };
    return {
      t: "These are the top ranked wallets right now. Test one with pretend money before you copy it.",
      wallets: walletsLive.slice(0, 4).map((w) => ({ name: w.name, sub: `${w.chain} · ${w.win === null ? `${w.trades} trades` : `${w.win}% win`}`, href: `/app/profile/?${walletQuery(w)}`, av: w.av })),
      f: ["What is moving right now?"],
    };
  }
  if (/EARLY|HAVEN'T|NOT PUMPED|FLAT|QUIET/.test(Q)) {
    const early = coins.filter((x) => x.early || Math.abs(x.ch) < 5).slice(0, 3);
    if (early.length) return { t: `${early.length} live coin${early.length === 1 ? "" : "s"} with activity and little price move yet.`, coins: early, f: early[0] ? [`Is ${early[0].sym} safe?`] : [] };
    return { t: "No live coin fits that right now: every listed coin has already moved more than 5%.", coins: coins.slice(0, 3), f: [] };
  }
  const top = coins.slice(0, 3);
  return { t: top.length ? "Here is what is moving most right now." : "No coins have cleared the filters yet. Ask about wallets instead.", coins: top, f: top[0] ? [`Is ${top[0].sym} safe?`, "Which smart wallets should I look at?"] : ["Which smart wallets should I look at?"] };
}

export function AskScreen() {
  const router = useRouter();
  const coins = useCoins("trending");
  const ws = useWallets("top");
  const [thread, setThread] = useState<Array<{ q: string; a: Answer }>>([]);
  const [input, setInput] = useState("");
  const end = useRef<HTMLDivElement>(null);
  const ask = (q: string) => {
    const a = answer(q, coins.data, ws.data);
    setThread((t) => [...t, { q, a }].slice(-6));
    setInput("");
    if (a.go) setTimeout(() => router.push(a.go as string), 700);
    setTimeout(() => end.current?.scrollIntoView({ block: "end" }), 50);
  };
  const last = thread[thread.length - 1];
  const fol = last ? last.a.f : ["What is moving right now?", "Which smart wallets should I look at?", "Which coins haven't pumped yet?"];

  return (
    <div className="scr s-ask">
      <Hd title="Ask trace" crumb="Answers come from live coins, wallets and safety checks" right={<Src example={false} loading={coins.loading} />} />
      <div className="thread">
        {!thread.length ? (
          <p className="sub" style={{ whiteSpace: "normal" }}>
            Ask about any coin or wallet trace can see. Tap a suggestion to start.
          </p>
        ) : null}
        {thread.map((m, i) => (
          <div key={i} style={{ display: "contents" }}>
            <div className="qb2">{m.q}</div>
            <div className="ans">
              <span className="who">
                <Ic id="spark" />
                trace
              </span>
              <span>{m.a.t}</span>
              {m.a.coins?.length ? (
                <div className="box">
                  {m.a.coins.map((c, k) => (
                    <Row key={c.key} href={coinHref(c)}>
                      <Tk s={c.sym} on={k === 0} />
                      <div className="grow">
                        <div className="nm">{c.sym}</div>
                        <div className="sub">{c.why}</div>
                      </div>
                      <span className={`mono ${cl(c.ch)}`}>{sgn(c.ch)}%</span>
                    </Row>
                  ))}
                </div>
              ) : null}
              {m.a.wallets?.length ? (
                <div className="box">
                  {m.a.wallets.map((w) => (
                    <Row key={w.href} href={w.href}>
                      <Av t={w.av} />
                      <div className="grow">
                        <div className="nm">{w.name}</div>
                        <div className="sub">{w.sub}</div>
                      </div>
                      <Ic id="arr" />
                    </Row>
                  ))}
                </div>
              ) : null}
            </div>
          </div>
        ))}
        <div className="chips">
          {fol.map((f) => (
            <button key={f} type="button" onClick={() => ask(f)}>
              {f}
            </button>
          ))}
        </div>
        <div ref={end} />
      </div>
      <form
        className="composer"
        onSubmit={(e) => {
          e.preventDefault();
          if (input.trim()) ask(input.trim());
        }}
      >
        <label>
          <Ic id="spark" />
          <input value={input} onChange={(e) => setInput(e.target.value)} placeholder="Ask about any coin or wallet" autoComplete="off" />
        </label>
        <button className="barrow" type="submit" aria-label="Send">
          <Ic id="arr" />
        </button>
      </form>
    </div>
  );
}
