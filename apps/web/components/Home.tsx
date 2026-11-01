"use client";

import Link from "next/link";
import { useEffect, useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import type {
  DataStatus,
  DiscoveredWallet,
  MoverItem,
  MoverTab,
  WalletTab,
  WalletTrade,
  WalletRef,
  ChainsResponse,
} from "@/lib/api/types";
import { useApi, useDemoActive } from "@/hooks/useApi";
import { readLocal, SHADOW_IDS_KEY } from "@/lib/local";
import {
  formatCompactUsd,
  formatPct,
  formatRelativeTime,
  pctTone,
  tokenLabel,
  walletLabel,
} from "@/lib/format";
import { MoversTable, tokenHref, tokenKey } from "./MoversTable";
import { WalletList, walletHref } from "./WalletList";
import { BuyersList, RiskChecksList, RiskGauge, TokenStats, WhyMovingList, gaugeText } from "./TokenViews";
import { ShadowResultView } from "./ShadowResultView";
import { WatchRow } from "./Watch";
import { NavIcon } from "./NavIcon";
import { IconBuy, IconReceive, IconSend, IconSwap } from "./icons";
import {
  Avatar,
  ComingLaterBadge,
  Change,
  EmptyState,
  Panel,
  Remote,
  SkeletonRows,
  Sparkline,
  Tabs,
} from "./ui";

const MOVER_TABS: ReadonlyArray<{ id: MoverTab; label: string }> = [
  { id: "trending", label: "Trending" },
  { id: "early_momentum", label: "Early Momentum" },
  { id: "smart_money", label: "Smart Money" },
  { id: "new", label: "New" },
];

const WALLET_TABS: ReadonlyArray<{ id: WalletTab; label: string }> = [
  { id: "top", label: "Top Performers" },
  { id: "new", label: "New Wallets" },
  { id: "most_followed", label: "Most Followed" },
];

/* ---------------------------------- hero ---------------------------------- */

function Hero() {
  return (
    <section className="hero" aria-labelledby="hero-title">
      <div className="hero-text">
      <h1 id="hero-title">
        Discover what is moving, <em>and why.</em>
      </h1>
      <p>Follow capable wallets, research coins in plain language, and test ideas with paper money. Watch-only: GiFTED! never holds your funds.</p>
      <div className="hero-cta">
        <Link href="/top-movers" className="btn btn-primary">Explore Top Movers</Link>
        <button type="button" className="btn" disabled aria-disabled="true" title="Connecting a wallet is coming later. GiFTED! is watch-only for now.">
          Connect Wallet <ComingLaterBadge />
        </button>
        <button type="button" className="btn" disabled aria-disabled="true" title="Creating a wallet is coming later. GiFTED! does not hold funds yet.">
          Create New Wallet <ComingLaterBadge />
        </button>
      </div>
      </div>
      <div className="hero-art" aria-hidden="true">
        <div className="float-card fc-1">
          <Avatar seed="shadow" label="SH" size="sm" />
          <div><b>Shadow Trades</b><span>Paper only</span></div>
        </div>
        <div className="phone">
          <small>Research view</small>
          <div className="big">Smart wallets</div>
          <svg viewBox="0 0 160 80" width="100%" height="80" style={{ marginTop: 8 }}>
            <path d="M0 70 L20 60 L40 64 L60 44 L80 50 L100 30 L120 34 L140 12 L160 8" stroke="#3fe39b" strokeWidth="2.4" fill="none" strokeLinejoin="round" />
            <path d="M0 70 L20 60 L40 64 L60 44 L80 50 L100 30 L120 34 L140 12 L160 8 L160 80 L0 80Z" fill="rgba(63,227,155,0.15)" />
          </svg>
          <div style={{ display: "grid", gap: 6, marginTop: 10 }}>
            <span className="skeleton" style={{ height: 22, animation: "none" }} />
            <span className="skeleton" style={{ height: 22, animation: "none" }} />
            <span className="skeleton" style={{ height: 22, animation: "none", width: "70%" }} />
          </div>
        </div>
        <div className="float-card fc-2">
          <Avatar seed="radar" label="RR" size="sm" />
          <div><b>Risk Radar</b><span>Checks before you decide</span></div>
        </div>
        <span className="coin-dot" style={{ right: 4, top: 52, width: 28, height: 28, background: "#4be3a6", color: "#04140d" }}>T</span>
        <span className="coin-dot" style={{ right: 230, top: 18, width: 22, height: 22, background: "#7aa8ff", color: "#04140d" }}>S</span>
        <span className="coin-dot" style={{ left: 6, top: 112, width: 36, height: 36, background: "#f5b942", color: "#04140d" }}>B</span>
      </div>
    </section>
  );
}

/* ------------------------------- quick cards ------------------------------ */

function QuickCards({ movers, wallets }: { movers: MoverItem[] | null; wallets: DiscoveredWallet[] | null }) {
  const risky = movers ? movers.filter((m) => m.risk === "warn" || m.risk === "block").length : null;
  const cards: Array<{ href: string; title: string; sub: string; icon: "trend" | "users" | "radar" | "shadow" | "brief"; color: string; bg: string }> = [
    { href: "/top-movers", title: "Top Movers", sub: movers ? `${movers.length} coins gaining attention` : "Coins gaining attention", icon: "trend", color: "#4be3a6", bg: "rgba(75,227,166,0.14)" },
    { href: "/smart-wallets", title: "Smart Wallets", sub: wallets ? `${wallets.length} wallets to review` : "Wallets worth reviewing", icon: "users", color: "#7aa8ff", bg: "rgba(122,168,255,0.14)" },
    { href: "/risk-radar", title: "Risk Radar", sub: risky === null ? "Check risks first" : `${risky} with risk flags`, icon: "radar", color: "#ff7b7b", bg: "rgba(255,107,107,0.14)" },
    { href: "/shadow-trades", title: "Shadow Trades", sub: "Test with paper money", icon: "shadow", color: "#a78bfa", bg: "rgba(167,139,250,0.16)" },
    { href: "/briefing", title: "Daily Briefing", sub: "Your market summary", icon: "brief", color: "#4be3a6", bg: "rgba(75,227,166,0.14)" },
  ];
  return (
    <div className="quick">
      {cards.map((c) => (
        <Link key={c.href} href={c.href}>
          <span className="qi" style={{ background: c.bg, color: c.color }}><NavIcon name={c.icon} size={24} /></span>
          <span style={{ minWidth: 0 }}>
            <b>{c.title}</b>
            <span className="s">{c.sub}</span>
          </span>
        </Link>
      ))}
    </div>
  );
}

/* ------------------------------ recent activity ---------------------------- */

type ActivityRow = { wallet: WalletRef; trade: WalletTrade };
interface ActivityData { data_status: DataStatus; as_of: string; demo: boolean; coverage_note?: string; items: ActivityRow[] }

const STATUS_RANK: DataStatus[] = ["demo", "unavailable", "stale", "partial", "live"];

function RecentActivity({ wallets }: { wallets: DiscoveredWallet[] | null }) {
  const [filter, setFilter] = useState<"all" | "buy" | "sell">("all");
  const refs = useMemo(() => (wallets ?? []).slice(0, 3).map((w) => w.wallet), [wallets]);
  const key = refs.map((r) => r.address).join(",");
  const state = useApi<ActivityData>(
    async () => {
      const settled = await Promise.allSettled(refs.map((r) => api.walletTrades(r.chain, r.address)));
      const ok = settled.flatMap((s, i) => (s.status === "fulfilled" ? [{ res: s.value, ref: refs[i] as WalletRef }] : []));
      if (ok.length === 0) {
        const first = settled.find((s) => s.status === "rejected");
        throw first && first.status === "rejected" ? first.reason : new Error("No activity");
      }
      const items = ok.flatMap(({ res, ref }) => res.items.map((trade) => ({ wallet: ref, trade })));
      items.sort((a, b) => Date.parse(b.trade.event_time) - Date.parse(a.trade.event_time));
      let status: DataStatus = "live";
      for (const { res } of ok) {
        if (STATUS_RANK.indexOf(res.data_status) < STATUS_RANK.indexOf(status)) status = res.data_status;
      }
      if (ok.length < settled.length && status === "live") status = "partial";
      return { data_status: status, as_of: ok[0]?.res.as_of ?? new Date().toISOString(), demo: status === "demo", items: items.slice(0, 8) };
    },
    [key],
    refs.length > 0,
  );
  return (
    <Panel
      title="Recent Activity"
      action={<Link href="/smart-wallets" className="link">View All</Link>}
    >
      <Tabs
        small
        label="Activity filter"
        value={filter}
        onChange={setFilter}
        tabs={[{ id: "all", label: "All" }, { id: "buy", label: "Buys" }, { id: "sell", label: "Sells" }]}
      />
      <div style={{ marginTop: 10 }}>
        {wallets === null ? (
          <SkeletonRows rows={4} />
        ) : (
          <Remote
            state={state}
            skeletonRows={4}
            isEmpty={(d) => d.items.filter((r) => filter === "all" || r.trade.side === filter).length === 0}
            empty={<EmptyState title="No recent trades">Tracked wallets have not made supported trades yet.</EmptyState>}
          >
            {(d) => (
              <ul>
                {d.items
                  .filter((r) => filter === "all" || r.trade.side === filter)
                  .slice(0, 4)
                  .map((r) => (
                    <li key={r.trade.tx_id} className="list-row">
                      <Avatar seed={r.wallet.address} label={walletLabel(r.wallet)} />
                      <div className="grow">
                        <Link href={walletHref(r.wallet)}><b>{walletLabel(r.wallet)}</b></Link>{" "}
                        <span className={r.trade.side === "buy" ? "up" : "down"}>{r.trade.side === "buy" ? "bought" : "sold"}</span>{" "}
                        <Link href={tokenHref(r.trade.token)}>{tokenLabel(r.trade.token)}</Link>
                        <small className="muted" style={{ display: "block" }}>
                          {formatRelativeTime(r.trade.event_time)}
                          {r.trade.finality === "tentative" ? " (not final yet)" : ""}
                        </small>
                      </div>
                      <div className="right num">{r.trade.amount_usd ? formatCompactUsd(r.trade.amount_usd) : "Value unknown"}</div>
                    </li>
                  ))}
              </ul>
            )}
          </Remote>
        )}
      </div>
    </Panel>
  );
}

/* ------------------------------ market research ---------------------------- */

function MarketResearch({ selected }: { selected: MoverItem | null }) {
  const [tab, setTab] = useState<"why" | "data" | "buyers" | "risk">("why");
  const chain = selected?.token.chain;
  const address = selected?.token.address;
  const state = useApi(() => api.token(chain as never, address as string), [chain, address], !!selected);
  const label = selected ? tokenLabel(selected.token) : "";
  return (
    <Panel title="Market Research" action={selected ? <Link className="link" href={tokenHref(selected.token)}>View All</Link> : undefined}>
      {!selected ? (
        <EmptyState title="Pick a coin">Choose a coin from Top Movers to see why it is moving.</EmptyState>
      ) : (
        <>
          <div style={{ display: "flex", gap: 12, alignItems: "center", marginBottom: 8 }}>
            <Avatar seed={selected.token.address} label={label} size="lg" />
            <div style={{ minWidth: 0 }}>
              <b style={{ fontSize: 18 }}>{label}</b>{" "}
              <Change value={selected.change_pct} />
              <small className="muted ellip" style={{ display: "block" }}>{selected.token.name}</small>
            </div>
            <div style={{ marginLeft: "auto", width: 130, minWidth: 0 }}>
              <Sparkline points={selected.sparkline} tone={pctTone(selected.change_pct)} width={130} height={48} />
            </div>
          </div>
          <Remote state={state} skeletonRows={3} skeletonH={40}>
            {(t) => (
              <>
                <p style={{ fontSize: 13.5, color: "#cfe0d7", marginBottom: 10 }}>
                  {t.why_moving[0]?.text ?? "No explanation is available yet."}
                </p>
                <div className="hero-cta" style={{ marginTop: 0, marginBottom: 10 }}>
                  <Link href={tokenHref(t.token)} className="btn btn-primary btn-sm">View Full Analysis</Link>
                  <button type="button" className="btn btn-sm" disabled aria-disabled="true" title="Coin watchlists are not available yet. You can watch wallets instead.">
                    Add to Watchlist <ComingLaterBadge />
                  </button>
                </div>
                <TokenStats t={t} />
                <div style={{ margin: "12px 0 6px" }}>
                  <Tabs
                    small
                    label="Research sections"
                    value={tab}
                    onChange={setTab}
                    tabs={[
                      { id: "why", label: "Why It's Moving" },
                      { id: "data", label: "Key Data" },
                      { id: "buyers", label: "Buyers" },
                      { id: "risk", label: "Risk Check" },
                    ]}
                  />
                </div>
                {tab === "why" ? <WhyMovingList items={t.why_moving} /> : null}
                {tab === "data" ? (
                  <div>
                    <div className="kv"><span>Price source time</span><b>{formatRelativeTime(selected.source_timestamp)}</b></div>
                    <div className="kv"><span>Pool</span><b>{selected.pool}</b></div>
                    <div className="kv"><span>Chain</span><b>{t.token.chain}</b></div>
                  </div>
                ) : null}
                {tab === "buyers" ? <BuyersList buyers={t.buyers} /> : null}
                {tab === "risk" ? <RiskChecksList checks={t.risk.checks} /> : null}
              </>
            )}
          </Remote>
        </>
      )}
    </Panel>
  );
}

/* -------------------------------- risk radar -------------------------------- */

function RiskRadarPanel({ selected }: { selected: MoverItem | null }) {
  const chain = selected?.token.chain;
  const address = selected?.token.address;
  const state = useApi(() => api.token(chain as never, address as string), [chain, address], !!selected);
  return (
    <Panel title="Risk Radar" action={<Link className="link" href="/risk-radar">View All</Link>}>
      {!selected ? (
        <EmptyState title="No coin selected" />
      ) : (
        <Remote state={state} skeletonRows={4} skeletonH={34}>
          {(t) => (
            <>
              <div className="gauge-wrap">
                <RiskGauge state={t.risk.overall} size={110} />
                <div>
                  <b style={{ fontSize: 15 }} className={`r-${t.risk.overall}`}>{gaugeText(t.risk.overall)}</b>
                  <p className="muted" style={{ fontSize: 12 }}>Estimated from automated checks. Not a guarantee of safety.</p>
                </div>
              </div>
              <RiskChecksList checks={t.risk.checks} />
            </>
          )}
        </Remote>
      )}
    </Panel>
  );
}

/* ------------------------------- shadow summary ------------------------------ */

function useSavedShadowIds(): string[] {
  const [ids, setIds] = useState<string[]>([]);
  useEffect(() => setIds(readLocal<string[]>(SHADOW_IDS_KEY, [])), []);
  return ids;
}

function ShadowSummary({ top }: { top: WalletRef | null }) {
  const ids = useSavedShadowIds();
  const { active: demo } = useDemoActive();
  const id = ids[0] ?? (demo ? "demo-sample" : null);
  const state = useApi(() => api.shadowResults(id as string), [id], !!id);
  const href = top ? `/shadow-trades?chain=${top.chain}&address=${encodeURIComponent(top.address)}` : "/shadow-trades";
  return (
    <Panel title="Shadow Trades" sub="See what would have happened if you followed a trader on paper." action={<Link className="link" href="/shadow-trades">View All</Link>}>
      {id ? (
        <Remote state={state} skeletonRows={3} skeletonH={30}>{(r) => <ShadowResultView r={r} compact />}</Remote>
      ) : (
        <EmptyState title="No paper trades yet">Pick a wallet and start a simulation. No real money is used.</EmptyState>
      )}
      <Link className="btn btn-primary" href={href} style={{ width: "100%", marginTop: 12 }}>Start Shadowing</Link>
    </Panel>
  );
}

function CoveragePanel() {
  const state = useApi<ChainsResponse>(() => api.chains(), []);
  return (
    <Panel title="Coverage" sub="What GiFTED! can parse today">
      <Remote state={state} skeletonRows={4} skeletonH={34}>
        {(data) => (
          <ul>
            {data.chains.map((chain) => (
              <li className="list-row" key={chain.chain}>
                <div className="grow">
                  <b>{chain.display_name}</b>
                  <small className="muted" style={{ display: "block" }}>
                    {chain.protocols.map((p) => `${p.protocol}: ${p.status}`).join(", ")}
                  </small>
                </div>
                <span className={`badge ${chain.status === "partial" || chain.status === "enabled" ? "b-partial" : "b-unavailable"}`}>
                  {chain.status}
                </span>
              </li>
            ))}
          </ul>
        )}
      </Remote>
    </Panel>
  );
}

/* --------------------------------- right rail -------------------------------- */

function WalletRail() {
  const actions = [
    { label: "Send", icon: <IconSend size={20} /> },
    { label: "Receive", icon: <IconReceive size={20} /> },
    { label: "Buy", icon: <IconBuy size={20} /> },
    { label: "Swap", icon: <IconSwap size={20} /> },
  ];
  return (
    <Panel title="Wallet" action={<ComingLaterBadge />} className="rail-wallet">
      <p className="panel-sub">Balance</p>
      <div className="soon-balance">Not available</div>
      <p className="muted" style={{ fontSize: 12.5 }}>
        GiFTED! is watch-only for now. It does not hold funds, so there is no balance to show. Sending, receiving, buying and swapping are coming later.
      </p>
      <div className="rail-actions">
        {actions.map((a) => (
          <button key={a.label} type="button" className="btn" disabled aria-disabled="true" title={`${a.label} is coming later`}>
            {a.icon}
            {a.label}
          </button>
        ))}
      </div>
    </Panel>
  );
}

function WatchlistRail() {
  const state = useApi(() => api.watchlists(), []);
  return (
    <Panel title="Watchlist" sub="Wallets you watch" action={<Link className="link" href="/watchlist">Manage</Link>}>
      <Remote
        state={state}
        skeletonRows={3}
        isEmpty={(d) => d.items.length === 0}
        empty={<EmptyState title="No watched wallets" action={<Link className="btn btn-sm" href="/watchlist">Add a wallet</Link>}>Add any public address to follow its buys and sells.</EmptyState>}
      >
        {(d) => <ul>{d.items.slice(0, 4).map((i) => <WatchRow key={i.id} item={i} />)}</ul>}
      </Remote>
    </Panel>
  );
}

function PaperRow({ id }: { id: string }) {
  const state = useApi(() => api.shadowResults(id), [id]);
  return (
    <li className="list-row">
      <div className="grow">
        <b>Paper strategy</b>
        <small className="muted mono" style={{ display: "block" }}>{id}</small>
      </div>
      <div className="right">
        {state.status === "ok" && state.data ? (
          <>
            <b className={`num ${pctTone(state.data.return_pct)}`}>{formatPct(state.data.return_pct)}</b>
            <small className="muted" style={{ display: "block" }}>{state.data.demo ? "Demo" : "Simulated"}</small>
          </>
        ) : state.status === "error" ? (
          <small className="muted">Unavailable</small>
        ) : (
          <small className="muted">Loading</small>
        )}
      </div>
    </li>
  );
}

function PaperPositions() {
  const ids = useSavedShadowIds();
  const { active: demo } = useDemoActive();
  const list = ids.length > 0 ? ids.slice(0, 3) : demo ? ["demo-sample"] : [];
  return (
    <Panel title="Paper Positions" sub="Simulated only. No real funds." action={<Link className="link" href="/shadow-trades">Open</Link>}>
      {list.length === 0 ? (
        <EmptyState title="No paper positions" action={<Link className="btn btn-sm" href="/shadow-trades">Start a simulation</Link>}>
          Paper trades you create will show here.
        </EmptyState>
      ) : (
        <ul>{list.map((id) => <PaperRow key={id} id={id} />)}</ul>
      )}
    </Panel>
  );
}

/* ----------------------------------- page ------------------------------------ */

export function Home() {
  const [moverTab, setMoverTab] = useState<MoverTab>("trending");
  const [walletTab, setWalletTab] = useState<WalletTab>("top");
  const [selectedKey, setSelectedKey] = useState<string | null>(null);

  const movers = useApi(() => api.movers({ tab: moverTab, window: "24h" }), [moverTab]);
  const wallets = useApi(() => api.discoveryWallets({ tab: walletTab }), [walletTab]);

  const items = movers.data?.items ?? null;
  const selected = useMemo(() => {
    if (!items || items.length === 0) return null;
    return items.find((m) => tokenKey(m.token) === selectedKey) ?? items[0] ?? null;
  }, [items, selectedKey]);
  const walletItems = wallets.data?.items ?? null;

  return (
    <div className="dash-wrap">
    <div className="dash">
      <div className="dash-main">
        <Hero />
        <QuickCards movers={items} wallets={walletItems} />
        <div className="row-2">
          <Panel
            title="Top Movers"
            sub="Coins gaining attention right now."
            action={
              <div style={{ display: "flex", gap: 10, alignItems: "center", flexWrap: "wrap" }}>
                <Tabs small label="Top movers category" value={moverTab} onChange={setMoverTab} tabs={MOVER_TABS} />
                <Link className="link" href="/top-movers">View All</Link>
              </div>
            }
          >
            <Remote
              state={movers}
              skeletonRows={5}
              isEmpty={(d) => d.items.length === 0}
              empty={<EmptyState title="No movers in this view">Try another tab. GiFTED! only lists coins with enough liquidity and data.</EmptyState>}
            >
              {(d) => (
                <MoversTable items={d.items} windowLabel="24h" compact selectedKey={selected ? tokenKey(selected.token) : null} onSelect={(m) => setSelectedKey(tokenKey(m.token))} />
              )}
            </Remote>
          </Panel>
          <Panel
            title="Smart Wallets"
            sub="Wallets with a track record worth watching. Return figures are estimates."
            action={<Link className="link" href="/smart-wallets">View All</Link>}
          >
            <Tabs small label="Smart wallet category" value={walletTab} onChange={setWalletTab} tabs={WALLET_TABS} />
            <div style={{ marginTop: 6 }}>
              <Remote
                state={wallets}
                skeletonRows={4}
                skeletonH={64}
                isEmpty={(d) => d.items.length === 0}
                empty={<EmptyState title="No wallets in this view">No wallets passed the evidence checks yet.</EmptyState>}
              >
                {(d) => <WalletList items={d.items.slice(0, 4)} />}
              </Remote>
            </div>
          </Panel>
        </div>
      </div>
      <aside className="dash-rail" aria-label="Wallet, watchlist and paper positions">
        <WalletRail />
        <WatchlistRail />
        <PaperPositions />
        <CoveragePanel />
      </aside>
    </div>
    <div className="row-4">
      <RecentActivity wallets={walletItems} />
      <MarketResearch selected={selected} />
      <RiskRadarPanel selected={selected} />
      <ShadowSummary top={walletItems?.[0]?.wallet ?? null} />
    </div>
    </div>
  );
}


