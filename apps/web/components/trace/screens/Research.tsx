"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { useEffect, useMemo, useRef, useState } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import type { RiskCheck } from "@/lib/api/types";
import { D, RANGE } from "@/lib/trace/example";
import {
  coinHref,
  exampleCoin,
  exampleEvents,
  exampleSeries,
  useCoin,
  useCoins,
  type CoinVM,
} from "@/lib/trace/data";
import type { ChartEvent } from "@/lib/trace/engine";
import { cl, clamp, hm, money, pf, resample, rng, sgn, walk } from "@/lib/trace/viz";
import { useApp } from "../AppState";
import { TraceChartView } from "../Three";
import { Av, Chart2, Hd, Help, Ic, Lk, RiskTag, Row, Seg, Spark, Src, StateTag, Tk } from "../ui";

/* =============================== Today =============================== */

const EX_COPIES = [
  { w: "0xSun", amt: 120.25, ch: 48.6 },
  { w: "CryptoNomad", amt: 80.1, ch: 22.3 },
  { w: "DeFiWhale", amt: 98.4, ch: -3.1 },
];

function greeting(d: Date) {
  const h = d.getHours();
  return h < 12 ? "Good morning." : h < 18 ? "Good afternoon." : "Good evening.";
}
function dateLabel(d: Date) {
  return d.toLocaleDateString("en-GB", { weekday: "short", day: "2-digit", month: "short" }) + " · " + d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" });
}

export function TodayScreen({ meInitials }: { meInitials?: string }) {
  const router = useRouter();
  const token = useAuthToken();
  const coins = useCoins("trending");
  const mandates = useApi(() => api.nearMandates(), [token], !!token);
  const { toast } = useApp();
  const [range, setRange] = useState("1D");
  const [now, setNow] = useState<Date | null>(null);
  useEffect(() => setNow(new Date()), []);
  const signals = coins.data.filter((c) => c.ch > 0).slice(0, 3);
  const flagged = coins.data.find((c) => c.risk === "High");
  const aw = ["var(--g)", "rgba(237,242,239,.55)", "var(--g-line)", "rgba(237,242,239,.28)", "rgba(237,242,239,.14)"];
  const live = mandates.data?.items ?? [];

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
            {signals.length} signal{signals.length === 1 ? "" : "s"} today.{" "}
            {token ? (live.length ? `You have ${live.length} copy mandate${live.length === 1 ? "" : "s"} on NEAR.` : "Test a wallet with pretend money, then copy it with limits.") : "Sign in to follow wallets and copy them with limits."}
          </p>
        </div>
        <div className="qa">
          <button className="qb" type="button" onClick={() => router.push("/app/discover/")}>
            <Ic id="grid" />
            <span>Discover</span>
          </button>
          <button className="qb" type="button" onClick={() => router.push("/app/shadow/")}>
            <Ic id="flask" />
            <span>Test</span>
          </button>
          <button className="qb" type="button" onClick={() => router.push("/app/swap/")}>
            <Ic id="swap" />
            <span>Swap</span>
          </button>
          <button className="qb pri" type="button" onClick={() => router.push(signals[0] ? `/app/swap/?to=${encodeURIComponent(signals[0].sym)}` : "/app/swap/")}>
            <Ic id="plus" />
            <span>Buy</span>
          </button>
        </div>
      </div>
      <div className="tg">
        <div className="pn">
          <div className="pn-h">
            <span className="lab">
              Portfolio <Src example />
            </span>
            <Seg opts={["1H", "1D", "1W", "1M", "1Y"].map((r) => [r, r] as [string, string])} cur={range} onPick={setRange} />
          </div>
          <div className="vrow">
            <span className="big n">$2,840.25</span>
            <span className="mono g">{RANGE[range]!.d}</span>
          </div>
          <Chart2 sets={[{ v: walk(RANGE[range]!.seed, 72, 0.36) }]} />
        </div>
        <div className="pn">
          <div className="pn-h">
            <span className="lab">
              Today&apos;s signals <Src example={coins.example} loading={coins.loading} />
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
                <Spark values={c.spark} seed={c.seed} />
                <span className="mono g">{sgn(c.ch)}%</span>
              </Row>
            ))}
            {!signals.length ? <p className="sub">Nothing is up today yet.</p> : null}
          </div>
          {flagged ? (
            <div className="note">
              <Ic id="shield" />1 risk alert · {flagged.sym} {flagged.why.toLowerCase()}
            </div>
          ) : null}
        </div>
        <div className="pn">
          <div className="pn-h">
            <span className="lab">
              Wallets you copy {token && live.length ? <Src example={false} /> : <Src example />}
            </span>
            <Lk href="/app/wallets/">Find more</Lk>
          </div>
          <div>
            {token && live.length
              ? live.slice(0, 3).map((m) => (
                  <Row key={m.id} href="/app/near/">
                    <Av t="N" />
                    <div className="grow">
                      <div className="nm">{m.owner_near_account}</div>
                      <div className="sub">
                        ${m.max_trade_usd} per trade · {m.mode.replaceAll("_", " ")}
                      </div>
                    </div>
                    <span className={`tag ${m.status === "anchored" ? "g" : ""}`}>{m.status === "anchored" ? "Signed" : m.status}</span>
                  </Row>
                ))
              : EX_COPIES.map((c) => (
                  <Row key={c.w} href={`/app/profile/?w=${c.w}`}>
                    <Av t={D.wallets[c.w]!.av} />
                    <div className="grow">
                      <div className="nm">{c.w}</div>
                      <div className="sub">{money(c.amt)} allocated</div>
                    </div>
                    <span className={`mono ${cl(c.ch)}`}>{sgn(c.ch)}%</span>
                    <button type="button" className="sw on" aria-label={`Copy ${c.w}`} onClick={() => toast("Example copy. Set up a real one from Smart wallets.")} />
                  </Row>
                ))}
          </div>
          <div className="note">
            <span>Skip high risk</span>
            <span>Max per trade</span>
            <span>Signed on NEAR</span>
          </div>
        </div>
        <div className="pn">
          <div className="pn-h">
            <span className="lab">
              What you own <Src example />
            </span>
            <Lk href="/app/wallet/">Wallet</Lk>
          </div>
          <div className="alloc">
            {D.holdings.map((h, k) => (
              <i key={h[0]} style={{ width: `${h[5]}%`, background: aw[k] }} />
            ))}
          </div>
          <div>
            {D.holdings.map((h) => (
              <div className="hrow" key={h[0]}>
                <Tk s={h[0]} />
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
          </div>
        </div>
      </div>
    </div>
  );
}

/* =============================== Briefing =============================== */

const EX_BRIEF = {
  short: "A quiet morning for majors. Smart money is rotating into two small caps, and one coin on your watchlist failed a risk check.",
  items: [
    { acc: true, icon: "bolt", t: "3 new opportunities", s: "NEURA, SOLX, BETA", href: "/app/discover/" },
    { acc: false, icon: "users", t: "2 wallet moves to watch", s: "0xSun and CryptoNomad bought", href: "/app/wallets/" },
    { acc: false, icon: "shield", t: "1 risk alert", s: "ORBIT: the same 4 wallets are trading it", href: "/app/token/?sym=ORBIT&tab=rk" },
    { acc: false, icon: "copy", t: "Yesterday's copy trades", s: "+8.4% across 3 wallets", href: "/app/wallet/?t=copies" },
  ],
};

export function BriefingScreen() {
  const b = useApi(() => api.briefings(), []);
  const { prefs, setPrefs } = useApp();
  const [now, setNow] = useState<Date | null>(null);
  const [elapsed, setElapsed] = useState(0);
  useEffect(() => setNow(new Date()), []);
  const live = b.data && !b.data.demo && b.data.sections.length ? b.data : null;
  const sections = live?.sections ?? [];
  const short = live ? sections.map((s) => s.items[0]?.text).filter(Boolean).join(" ") : EX_BRIEF.short;
  const items = live
    ? sections.flatMap((s, si) =>
        s.items.map((it, k) => ({
          acc: si === 0 && k === 0,
          icon: /risk/i.test(s.title) ? "shield" : /wallet/i.test(s.title) ? "users" : /signal|mover|opportun/i.test(s.title) ? "bolt" : "book",
          t: s.title,
          s: it.text,
          href: it.link || undefined,
        })),
      )
    : EX_BRIEF.items;
  const speech = typeof window !== "undefined" && "speechSynthesis" in window;
  const text = [short, ...items.map((i) => `${i.t}. ${i.s}.`)].join(" ");
  const secs = Math.max(20, Math.round(text.split(/\s+/).length / 2.6));
  const timer = useRef<ReturnType<typeof setInterval> | null>(null);

  const stop = () => {
    if (speech) window.speechSynthesis.cancel();
    if (timer.current) clearInterval(timer.current);
    setPrefs({ playing: false });
  };
  useEffect(() => () => {
    if (speech) window.speechSynthesis.cancel();
    if (timer.current) clearInterval(timer.current);
  }, [speech]);
  const play = () => {
    if (prefs.playing) return stop();
    if (!speech) return;
    const u = new SpeechSynthesisUtterance(text);
    u.rate = 1;
    u.onend = stop;
    window.speechSynthesis.cancel();
    window.speechSynthesis.speak(u);
    setElapsed(0);
    timer.current = setInterval(() => setElapsed((e) => e + 1), 1000);
    setPrefs({ playing: true });
  };
  const fmt = (s: number) => `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
  const playedBars = prefs.playing ? Math.min(40, Math.round((elapsed / secs) * 40)) : 11;

  return (
    <div className="scr s-brief">
      <Hd
        title="Daily briefing"
        crumb={now ? `${now.toLocaleDateString("en-GB", { weekday: "long", day: "numeric", month: "long" })} · ${prefs.bt}` : " "}
        right={<Src example={!live} loading={b.status === "loading"} />}
        back
      />
      <div className="brief">
        <div style={{ display: "grid", gap: 14 }}>
          <div className="sumcard">
            <span className="lab">
              <b>The short version</b>
            </span>
            {short}
          </div>
          <div className={`player${prefs.playing ? " on" : ""}`}>
            <button className="pb" type="button" onClick={play} aria-label={`${prefs.playing ? "Pause" : "Play"} briefing`} disabled={!speech}>
              <Ic id={prefs.playing ? "pause" : "play"} />
            </button>
            <div className="wave">
              {Array.from({ length: 40 }, (_, k) => (
                <i key={k} className={k < playedBars ? "p" : ""} style={{ height: 5 + Math.round(rng(k + 3)() * 18) }} />
              ))}
            </div>
            <span className="mono f3" style={{ fontSize: 11 }}>
              {prefs.playing ? fmt(elapsed) : fmt(secs)}
            </span>
          </div>
          {live?.coverage_note ? <span className="sub" style={{ whiteSpace: "normal" }}>{live.coverage_note}</span> : null}
        </div>
        <div>
          {items.map((it, i) => (
            <Row key={i} href={it.href}>
              <span className={`tk${it.acc ? " on" : ""}`}>
                <Ic id={it.icon} />
              </span>
              <div className="grow">
                <div className="nm">{it.t}</div>
                <div className="sub">{it.s}</div>
              </div>
              {it.href ? <Ic id="arr" /> : null}
            </Row>
          ))}
        </div>
      </div>
    </div>
  );
}

/* =============================== Discover =============================== */

const NEW_EX = ["BETA", "MIND", "LUNA", "ZETA"];

export function DiscoverScreen() {
  const params = useSearchParams();
  const router = useRouter();
  const [f, setF] = useState("trending");
  const [q, setQ] = useState(params.get("q") ?? "");
  useEffect(() => setQ(params.get("q") ?? ""), [params]);
  const coins = useCoins(f);
  const { star, toggleStar, toast } = useApp();

  const list = useMemo(() => {
    let l = coins.data.slice();
    if (coins.example) {
      if (f === "early") l = l.filter((c) => c.early);
      else if (f === "smart") l.sort((a, b) => (b.sm ?? 0) - (a.sm ?? 0));
      else if (f === "new") l = NEW_EX.map((s) => exampleCoin(s) as CoinVM);
      else l = l.slice(0, 6);
    }
    const Q = q.trim().toUpperCase();
    if (Q) l = coins.data.filter((c) => c.sym.toUpperCase().includes(Q) || c.name.toUpperCase().includes(Q) || c.why.toUpperCase().includes(Q) || (c.address ?? "").toUpperCase() === Q);
    return l;
  }, [coins.data, coins.example, f, q]);

  const looksLikeAddress = /^(0x[0-9a-fA-F]{40}|[1-9A-HJ-NP-Za-km-z]{32,44})$/.test(q.trim());

  return (
    <div className="scr s-disc">
      <Hd title="Discover" crumb="Coins people are buying right now" right={<Src example={coins.example} loading={coins.loading} />} />
      <div className="topf">
        <label className="search">
          <Ic id="srch" />
          <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="Search coins or reasons" autoComplete="off" />
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
        {coins.example ? (coins.note ? `${coins.note} Showing example coins so you can explore.` : "Example coins. Live coins appear here as soon as trace has price snapshots.") : `Live from the chain${coins.window === "1h" ? ", last hour (the 24 hour view fills in after a day of prices)" : ""}. Tap a coin to see who is buying.`}
      </span>
      {f === "early" ? (
        <Help>
          <b style={{ color: "var(--fg)", fontWeight: 500 }}>Early</b> means proven wallets are buying but the price has barely moved yet.
        </Help>
      ) : null}
      <div>
        {!list.length ? (
          <div style={{ padding: "16px 0", display: "grid", gap: 10 }}>
            <p className="f2" style={{ margin: 0 }}>
              No coins match “{q}”. {coins.example ? "Try NEURA or SOLX." : ""}
            </p>
            {looksLikeAddress ? (
              <div className="bset">
                <button className="bout sm" type="button" onClick={() => router.push(`/app/token/?chain=${q.startsWith("0x") ? "ethereum" : "solana"}&address=${encodeURIComponent(q.trim())}`)}>
                  Open as a coin
                </button>
                <button className="bout sm" type="button" onClick={() => router.push(`/app/profile/?chain=${q.startsWith("0x") ? "ethereum" : "solana"}&address=${encodeURIComponent(q.trim())}`)}>
                  Open as a wallet
                </button>
              </div>
            ) : null}
          </div>
        ) : (
          <>
            <div className="dhead">
              <span />
              <span>Coin</span>
              <span>Price</span>
              <span>{coins.window}</span>
              <span>Volume</span>
              <span>7d</span>
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
                  <div className="sub dc">{c.name}</div>
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
        )}
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

function exampleRisk(c: CoinVM): { s: string; c: [string, string, string][] } {
  return (
    D.risk[c.sym] || {
      s: c.risk === "Low" ? "No red flags. Liquidity is locked and the contract is verified." : "A couple of things to watch before you size up.",
      c: [
        ["Liquidity", `${c.liq}, locked 6 months`, (c.liqN ?? 0) > 1 ? "Good" : "Watch"],
        ["Can you sell?", "A test sell went through", "Pass"],
        ["Holder concentration", `Top 10 wallets hold ${20 + c.rn * 6}%`, c.rn > 2 ? "Watch" : "Good"],
        ["Contract", "Verified, no mint function", "Good"],
        ["Suspicious trading", "No wash trading found", "None"],
      ],
    }
  );
}

function liveRisk(checks: RiskCheck[]): { s: string; c: [string, string, string][] } {
  const flags = checks.filter((k) => k.state === "block").length;
  const warns = checks.filter((k) => k.state === "warn").length;
  const unknown = checks.filter((k) => k.state === "unknown").length;
  const s = flags ? `${flags} check${flags > 1 ? "s" : ""} failed. Read them before you buy.` : warns ? `${warns} thing${warns > 1 ? "s" : ""} to watch before you size up.` : unknown === checks.length ? "These checks have not run for this coin yet." : "No red flags in the checks that ran.";
  return { s, c: checks.map((k) => [CHECK_LABEL[k.key] ?? k.key, k.note, CHECK_STATE[k.state] ?? "Unknown"]) };
}

function minutesIntoDay(iso: string, nowMs: number): number {
  const age = (nowMs - new Date(iso).getTime()) / 60000;
  return clamp(190 - (age / 1440) * 190, 0, 190);
}

export function TokenScreen() {
  const params = useSearchParams();
  const router = useRouter();
  const ref = { sym: params.get("sym") ?? undefined, chain: params.get("chain") ?? undefined, address: params.get("address") ?? undefined };
  const [tab, setTab] = useState(params.get("tab") ?? "ov");
  const s = useCoin(ref);
  const movers = useCoins("trending");
  const { star, toggleStar, toast } = useApp();

  if (!s.data) {
    return (
      <div className="scr s-token">
        <Hd title={s.loading ? "Loading coin" : "Coin not found"} crumb={ref.address ? `${ref.chain} · ${ref.address.slice(0, 10)}…` : undefined} back />
        {!s.loading ? (
          <div className="pn">
            <p className="f2" style={{ margin: 0 }}>
              trace has no market data for this coin yet. It may be on a protocol trace does not read, or nobody has traded it since we started watching.
            </p>
            <div className="bset">
              <button className="bout" type="button" onClick={() => router.push("/app/discover/")}>
                Back to Discover
              </button>
            </div>
          </div>
        ) : null}
      </div>
    );
  }
  const { coin: c, detail, timeline } = s.data;
  const sym = c.sym;
  const nowMs = detail ? new Date(detail.as_of).getTime() : 0;

  /* chart data */
  let series: number[] = [];
  let events: ChartEvent[] = [];
  if (c.example) {
    series = exampleSeries(sym, 64);
    events = exampleEvents(sym);
  } else {
    const spark = movers.data.find((m) => m.address === c.address)?.spark;
    if (spark && spark.length > 1) series = resample(spark, 64);
    events = (timeline?.items ?? [])
      .filter((t) => t.kind === "fact")
      .slice(0, 6)
      .map((t) => ({ t: minutesIntoDay(t.time, nowMs), w: t.title, amt: "", label: `${t.title} at ${new Date(t.time).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" })}` }));
  }
  const ev = c.example ? D.events[sym] || [] : [];
  const firstWallet = ev[0]?.w || "0xSun";
  const buyer = detail?.buyers[0];
  const shadowHref = c.example ? `/app/shadow/?w=${firstWallet}` : buyer ? `/app/shadow/?chain=${buyer.chain}&address=${encodeURIComponent(buyer.address)}` : "/app/shadow/";

  const why: [string, string, string][] = c.example
    ? sym === "NEURA"
      ? [
          ["users", "Smart wallets buying.", "3 with a 70%+ win rate, 51 min before the breakout"],
          ["bolt", "Volume spike.", "4.8x its normal level"],
          ["chat", "Social attention.", "Mentions up 320%"],
        ]
      : [
          ["bolt", `${c.why}.`, `Volume ${c.vx} normal`],
          ["users", `${c.sm} smart wallet${c.sm === 1 ? "" : "s"}.`, "Bought in the last 24 hours"],
        ]
    : (detail?.why_moving ?? []).map((w) => [w.kind === "fact" ? "bolt" : "chat", w.kind === "fact" ? "Fact." : "Possible reason.", w.text]);

  /* timeline */
  const P = (t: number) => (series.length ? pf(series[Math.round(clamp(t / 190, 0, 1) * (series.length - 1))]!) : "");
  const tItems: Array<{ tm: string; b: string; p: string; hot: boolean; px: string }> = c.example
    ? (sym === "NEURA"
        ? ([
            [10, "Unusual activity", "Volume crossed 2x its 7-day average"],
            [24, "Smart wallets buying", "0xSun, CryptoNomad and AlphaHunter bought $41K", 1],
            [32, "Social attention", "Mentions up 320% in 30 minutes"],
            [75, "Breakout", "Up 18.4% in 45 minutes"],
            [176, "0xSun added", "Bought another $127K", 1],
            [190, "Now", "Up 42.7%, buying still active", 1],
          ] as Array<[number, string, string, number?]>)
        : [...ev.map((e) => [e.t, `${e.w}${e.sell ? " sold" : " bought"}`, `${e.amt} at ${hm(e.t)}`, e.sell ? 0 : 1] as [number, string, string, number]), [190, "Now", `${sgn(c.ch)}% today`, 1] as [number, string, string, number]]
      ).map((it) => ({ tm: hm(it[0]), b: it[1], p: it[2], hot: !!it[3], px: P(it[0]) }))
    : (timeline?.items ?? []).map((it) => ({
        tm: new Date(it.time).toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit" }),
        b: it.title,
        p: it.detail + (it.kind === "hypothesis" ? " (possible reason, not confirmed)" : ""),
        hot: it.kind === "fact",
        px: "",
      }));
  const sum = c.example ? (sym === "NEURA" ? "+42.7% in 3 hours. Smart money arrived 51 minutes before the breakout." : `${sgn(c.ch)}% today. ${c.why}.`) : `${sgn(c.ch)}% in 24 hours. ${c.why}.`;

  const R = c.example ? exampleRisk(c) : liveRisk(detail?.risk.checks ?? []);
  const starKey = c.key;

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
            <Src example={c.example} />
            <button
              className={`hb${star[starKey] ? " on" : ""}`}
              type="button"
              aria-label="Watch"
              onClick={() => toast(toggleStar(starKey) ? `${sym} added to watchlist` : "Removed from watchlist")}
            >
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
        <span className={`mono ${cl(c.ch)}`}>{sgn(c.ch)}% today</span>
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
              <TraceChartView series={series} events={events} seed={c.seed} heavy={sym === "ORBIT"} />
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
                Price today
              </span>
              <span>
                <i className="w" />
                Trading volume
              </span>
              <span>
                <i />
                Each green line is a proven wallet buying
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
              {c.vx ? <em>{c.vx} normal</em> : null}
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
        </div>
        <div className="tp p-tl">
          <span className="lab">
            <b>How it moved today</b>
          </span>
          <div style={{ fontSize: 16, fontWeight: 500, letterSpacing: "-.02em" }}>{sum}</div>
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
                  <span className="px">{it.px}</span>
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
        <button className="bout" type="button" onClick={() => router.push(`/app/compare/?a=${encodeURIComponent(c.example ? sym : c.key)}`)}>
          Compare
        </button>
        <button className="bout" type="button" onClick={() => router.push(shadowHref)}>
          Shadow test
        </button>
        <button className="bfill" type="button" onClick={() => router.push(`/app/swap/?to=${encodeURIComponent(sym)}`)}>
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
    if (!keys.length) return;
    const a = params.get("a");
    const A = (a && keys.includes(a) ? a : keys[0]) as string;
    const B = keys.find((k) => k !== A) ?? A;
    setPair([A, B]);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [keys.join("|")]);
  const byKey = (k: string) => coins.data.find((c) => c.key === k) as CoinVM;
  const verdictLive = useApi(
    () => api.compare(byKey(pair![0]).address as string, byKey(pair![1]).address as string),
    [pair?.[0], pair?.[1]],
    !!pair && !coins.example,
  );
  if (!pair || !byKey(pair[0]) || !byKey(pair[1])) return <div className="scr s-cmp"><Hd title="Compare" crumb="Loading coins" back /></div>;
  const a = byKey(pair[0]);
  const b = byKey(pair[1]);
  const A = a.sym;
  const B = b.sym;

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
  const v =
    verdictLive.data?.verdict_note ||
    (mom === safe
      ? `${mom.sym} leads on momentum and on safety. ${mom === a ? B : A} trails on both today.`
      : `${mom.sym} has more momentum. ${safe.sym} is the steadier pick, with ${(safe.liqN ?? 0) > (mom.liqN ?? 0) ? "deeper liquidity and " : ""}lower risk.`);

  return (
    <div className="scr s-cmp">
      <Hd title="Compare" crumb="Tap a coin to change it" back right={<Src example={coins.example} loading={coins.loading} />} />
      <div className="cmpbody" style={{ display: "grid", gap: 14 }}>
        <div className="cmph">
          <button type="button" onClick={() => setPair([cycle(keys, pair[0], pair[1]), pair[1]])}>
            <Tk s={A} on l />
            <b>{A}</b>
            <span className="lab">Change</span>
          </button>
          <span className="vs">VS</span>
          <button type="button" onClick={() => setPair([pair[0], cycle(keys, pair[1], pair[0])])}>
            <Tk s={B} l />
            <b>{B}</b>
            <span className="lab">Change</span>
          </button>
        </div>
        <div>
          {row("24h change", `${sgn(a.ch)}%`, `${sgn(b.ch)}%`, a.ch, b.ch, true)}
          {a.mcN !== null && b.mcN !== null ? row("Market cap", a.mc, b.mc, a.mcN, b.mcN, true) : null}
          {a.liqN !== null && b.liqN !== null ? row("Liquidity", a.liq, b.liq, a.liqN, b.liqN, true) : null}
          {a.sm !== null && b.sm !== null ? row("Smart wallets buying", String(a.sm), String(b.sm), a.sm, b.sm, true) : null}
          {a.vx && b.vx ? row("Volume vs normal", a.vx, b.vx, parseFloat(a.vx), parseFloat(b.vx), true) : null}
          {row("Risk (lower is safer)", a.risk, b.risk, a.rn, b.rn, false)}
        </div>
        <div className="verdict">
          <span className="lab">Our take</span>
          {v}
        </div>
        <div className="bset">
          <button className="bout" type="button" onClick={() => router.push(coinHref(a))}>
            Open {A}
          </button>
          <button className="bout" type="button" onClick={() => router.push(coinHref(b))}>
            Open {B}
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
  wallets?: Array<{ w: string; sub: string; win: string; href: string; av: string }>;
  f: string[];
  go?: string;
}

function answer(q: string, coins: CoinVM[]): Answer {
  const Q = q.toUpperCase();
  const c = coins.find((x) => Q.includes(x.sym.toUpperCase()));
  if (c && /SAFE|RISK|SCAM|RUG/.test(Q)) {
    const R = c.example ? exampleRisk(c) : null;
    return { t: `${c.sym} is ${c.risk === "Unrated" ? "not rated yet" : c.risk.toLowerCase() + " risk"}. ${R ? R.s : ""}`, coins: [c], f: [`Open the full risk check for ${c.sym}`, `Compare ${c.sym} and ${coins.find((x) => x !== c)?.sym ?? "SOLX"}`] };
  }
  if (c && /WALLET|WHO|BUY/.test(Q)) {
    const ev = c.example ? D.events[c.sym] || [] : [];
    if (!c.example) return { t: `Open ${c.sym} to see the wallets trace saw buying it.`, coins: [c], f: [`Is ${c.sym} safe?`] };
    return {
      t: `${ev.length} tracked wallet${ev.length === 1 ? "" : "s"} traded ${c.sym} today.`,
      wallets: ev.map((e) => ({ w: e.w, sub: `${e.sell ? "sold " : "bought "}${e.amt} at ${hm(e.t)}`, win: `${D.wallets[e.w]!.win}% win`, href: `/app/profile/?w=${e.w}`, av: D.wallets[e.w]!.av })),
      f: [`Shadow test ${ev[0]?.w || "0xSun"}`, `Is ${c.sym} safe?`],
    };
  }
  if (/COMPARE/.test(Q) && c) {
    const other = coins.find((x) => x !== c);
    return { t: "Opening a side by side view.", coins: other ? [c, other] : [c], f: [`Is ${c.sym} safe?`], go: `/app/compare/?a=${encodeURIComponent(c.example ? c.sym : c.key)}` };
  }
  if (/^OPEN THE FULL RISK CHECK FOR/.test(Q) && c) return { t: "Opening the risk check.", coins: [c], f: [], go: coinHref(c, "&tab=rk") };
  if (/^SHADOW TEST/.test(Q)) {
    const w = Object.keys(D.wallets).find((n) => Q.includes(n.toUpperCase())) ?? "0xSun";
    return { t: `Opening a shadow test for ${w}.`, f: [], go: `/app/shadow/?w=${w}` };
  }
  if (c) return { t: `${c.sym} is ${sgn(c.ch)}% today. ${c.why}.`, coins: [c], f: [`Which wallets are buying ${c.sym}?`, `Is ${c.sym} safe?`] };
  const early = coins.filter((x) => x.early || (x.ch < 5 && (x.sm ?? 0) >= 3)).slice(0, 3);
  if (early.length)
    return {
      t: `${early.length} coin${early.length === 1 ? "" : "s"} match. Each has proven wallets buying, and the price is up less than 5%.`,
      coins: early,
      f: [`Is ${early[early.length - 1]!.sym} safe?`, `Which wallets are buying ${early[0]!.sym}?`],
    };
  const top = coins.slice(0, 3);
  return { t: "Here is what is moving most right now.", coins: top, f: top[0] ? [`Is ${top[0].sym} safe?`] : [] };
}

export function AskScreen() {
  const router = useRouter();
  const coins = useCoins("trending");
  const [thread, setThread] = useState<Array<{ q: string; a: Answer }>>([]);
  const [input, setInput] = useState("");
  const end = useRef<HTMLDivElement>(null);
  const seeded = useRef(false);
  useEffect(() => {
    if (seeded.current || coins.loading) return;
    seeded.current = true;
    const q = "Which coins are smart wallets buying that haven't pumped yet?";
    setThread([{ q, a: answer(q, coins.data) }]);
  }, [coins.loading, coins.data]);
  const ask = (q: string) => {
    const a = answer(q, coins.data);
    setThread((t) => [...t, { q, a }].slice(-4));
    setInput("");
    if (a.go) setTimeout(() => router.push(a.go as string), 700);
    setTimeout(() => end.current?.scrollIntoView({ block: "end" }), 50);
  };
  const last = thread[thread.length - 1];
  const fol = last ? last.a.f : ["Which coins are smart wallets buying that haven't pumped yet?"];

  return (
    <div className="scr s-ask">
      <Hd title="Ask trace" crumb="Answers come from the coins and wallets trace can see" right={<Src example={coins.example} loading={coins.loading} />} />
      <div className="thread">
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
                    <Row key={w.w + w.sub} href={w.href}>
                      <Av t={w.av} />
                      <div className="grow">
                        <div className="nm">{w.w}</div>
                        <div className="sub">{w.sub}</div>
                      </div>
                      <span className="mono">{w.win}</span>
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
