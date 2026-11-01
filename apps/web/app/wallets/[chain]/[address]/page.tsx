"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { useState } from "react";
import { api, describeError } from "@/lib/api/client";
import type { Chain, WalletMetricsResponse, Score } from "@/lib/api/types";
import { useApi } from "@/hooks/useApi";
import { parseChainAddress } from "@/lib/params";
import {
  formatClockUtc,
  formatCompactUsd,
  formatRatioAsPct,
  formatRelativeTime,
  pctTone,
  shortenAddress,
  tokenLabel,
  walletLabel,
  formatInteger,
} from "@/lib/format";
import { tokenHref } from "@/components/MoversTable";
import { WATCH_STATUS_LABEL } from "@/components/Watch";
import { Avatar, EmptyState, ErrorState, HonestyNote, PageHead, Panel, Remote } from "@/components/ui";

const CONF_HELP: Record<string, string> = {
  high: "High: most trades could be matched to a cost basis.",
  medium: "Medium: some trades could not be matched, so the figure is an estimate.",
  low: "Low: many trades could not be matched. Treat the figure with caution.",
  insufficient: "Insufficient: there is not enough data to estimate profit.",
};

function ScoreCard({ title, score, note }: { title: string; score: Score | null; note: string }) {
  return (
    <div className="stat" style={{ padding: 12 }}>
      <small>{title}</small>
      {score ? (
        <>
          <b style={{ fontSize: 22 }} className="num">{score.value === null ? "n/a" : score.value}</b>
          <span className="muted"> / 100 (not a profit percentage)</span>
          <p style={{ fontSize: 12.5, margin: "6px 0" }}>{score.explanation}</p>
          <small className="muted" style={{ display: "block" }}>Decision: {score.decision} | Window {score.window} | Cohort {score.cohort} | Version {score.version}</small>
          <div style={{ marginTop: 6 }}>
            {Object.entries(score.components).map(([k, v]) => (
              <div className="kv" key={k}><span>{k.replace(/_/g, " ")}</span><b className="num">{v}</b></div>
            ))}
          </div>
        </>
      ) : (
        <p className="muted" style={{ fontSize: 12.5 }}>{note}</p>
      )}
    </div>
  );
}

function MetricsPanel({ m }: { m: WalletMetricsResponse }) {
  return (
    <>
      <div className="stat-grid three">
        <div className="stat"><small>Realized profit or loss (estimate)</small><b className={`num ${pctTone(m.realized_pnl_usd)}`}>{m.realized_pnl_usd === null ? "Unresolved" : formatCompactUsd(m.realized_pnl_usd)}</b></div>
        <div className="stat"><small>Unrealized (estimate)</small><b className={`num ${pctTone(m.unrealized_pnl_usd)}`}>{m.unrealized_pnl_usd === null ? "Unresolved" : formatCompactUsd(m.unrealized_pnl_usd)}</b></div>
        <div className="stat"><small>Total trades</small><b className="num">{formatInteger(m.total_trades)}</b></div>
        <div className="stat"><small>Confidence</small><b style={{ textTransform: "capitalize" }}>{m.pnl_confidence}</b></div>
        <div className="stat"><small>Coverage</small><b className="num">{formatRatioAsPct(m.coverage_ratio)}</b></div>
        <div className="stat"><small>Unresolved sales</small><b className="num">{m.unresolved_disposals}</b></div>
      </div>
      <p className="muted" style={{ fontSize: 12.5, margin: "8px 0 12px" }}>
        {CONF_HELP[m.pnl_confidence] ?? "Confidence describes how complete the data behind these estimates is."} Coverage is the share of the activity from this wallet that GiFTED! could read. Sales without a known purchase price stay unresolved instead of guessed.
      </p>
      <div className="cols-2">
        <ScoreCard title="Wallet quality score (WQS)" score={m.wqs} note="Not available." />
        <ScoreCard title="Wallet copyability score (WCS)" score={m.wcs} note="No copyability score yet. GiFTED! only scores wallets with enough evidence." />
      </div>
    </>
  );
}

export default function WalletPage() {
  const params = useParams<{ chain: string; address: string }>();
  const parsed = parseChainAddress(params?.chain, params?.address);
  const chain = (parsed.ok ? parsed.chain : "solana") as Chain;
  const address = parsed.ok ? parsed.address : "";

  const detail = useApi(() => api.wallet(chain, address), [chain, address], parsed.ok);
  const metrics = useApi(() => api.walletMetrics(chain, address), [chain, address], parsed.ok);
  const trades = useApi(() => api.walletTrades(chain, address), [chain, address], parsed.ok);
  const dna = useApi(() => api.walletDna(chain, address), [chain, address], parsed.ok);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);

  if (!parsed.ok) {
    return (
      <div className="stack">
        <PageHead title="Wallet" />
        <ErrorState error={new Error(parsed.reason)} />
      </div>
    );
  }

  const watch = async () => {
    setMsg(null);
    try {
      await api.addWatchlist({ chain, address });
      setMsg({ ok: true, text: "Added to your watchlist." });
    } catch (e) {
      setMsg({ ok: false, text: describeError(e).message });
    }
  };

  const w = detail.data?.wallet;
  const label = w ? walletLabel(w) : shortenAddress(address);

  return (
    <div className="stack">
      <PageHead
        title={label}
        sub={<><span className="mono">{address}</span> on {chain}</>}
        action={
          <div className="hero-cta" style={{ margin: 0 }}>
            <button type="button" className="btn" onClick={watch}>Watch this wallet</button>
            <Link href={`/shadow-trades?chain=${chain}&address=${encodeURIComponent(address)}`} className="btn btn-primary">Shadow with paper money</Link>
          </div>
        }
      />
      {msg ? <p role="status" className={msg.ok ? "up" : "down"}>{msg.text}</p> : null}
      <HonestyNote>
        Everything here comes from public on-chain activity. Names are user-chosen aliases, not identities. Profit figures are estimates with a confidence label. Past results do not predict future results.
      </HonestyNote>

      <Panel title="Overview">
        <Remote state={detail} skeletonRows={2} skeletonH={40}>
          {(d) => (
            <div style={{ display: "flex", gap: 14, alignItems: "center", flexWrap: "wrap" }}>
              <Avatar seed={d.wallet.address} label={label} size="lg" />
              <div className="stat-grid three" style={{ flex: 1, minWidth: 260 }}>
                <div className="stat"><small>Monitoring</small><b>{WATCH_STATUS_LABEL[d.monitoring as keyof typeof WATCH_STATUS_LABEL] ?? d.monitoring}</b></div>
                <div className="stat"><small>First seen</small><b>{d.first_seen ? formatRelativeTime(d.first_seen) : "n/a"}</b></div>
                <div className="stat"><small>Last seen</small><b>{d.last_seen ? formatRelativeTime(d.last_seen) : "n/a"}</b></div>
              </div>
            </div>
          )}
        </Remote>
      </Panel>

      <Panel title="Performance metrics" sub="Estimated metric. See the confidence and coverage below.">
        <Remote state={metrics} skeletonRows={3} skeletonH={60}>{(m) => <MetricsPanel m={m} />}</Remote>
      </Panel>

      <div className="cols-2">
        <Panel title="Recent trades" sub="Observed fact. Tentative trades may still change.">
          <Remote
            state={trades}
            skeletonRows={5}
            isEmpty={(d) => d.items.length === 0}
            empty={<EmptyState title="No trades yet">GiFTED! has not seen supported trades from this wallet.</EmptyState>}
          >
            {(d) => (
              <ul>
                {d.items.map((t) => (
                  <li key={t.tx_id} className="list-row">
                    <span className={`badge ${t.side === "buy" ? "b-live" : "b-unavailable"}`}>{t.side === "buy" ? "Bought" : "Sold"}</span>
                    <div className="grow">
                      <Link href={tokenHref(t.token)}><b>{tokenLabel(t.token)}</b></Link>
                      <small className="muted" style={{ display: "block" }}>
                        {formatRelativeTime(t.event_time)} ({formatClockUtc(t.event_time)}) {t.finality === "tentative" ? "| not final yet" : "| final"}
                      </small>
                    </div>
                    <div className="right num">{t.amount_usd ? formatCompactUsd(t.amount_usd) : "Value unknown"}</div>
                  </li>
                ))}
              </ul>
            )}
          </Remote>
        </Panel>

        <Panel title="Trading DNA" sub="Model assessment. Traits come from versioned rules applied to observed trades.">
          <Remote state={dna} skeletonRows={3}>
            {(d) =>
              d.state === "insufficient_history" ? (
                <EmptyState title="Not enough history yet">
                  GiFTED! needs more completed trades before it describes this wallet&apos;s style. Only {d.sample_size} trade{d.sample_size === 1 ? "" : "s"} could be used. It shows nothing instead of guessing.
                </EmptyState>
              ) : (
                <>
                  <ul>
                    {d.traits.map((t) => (
                      <li key={t.key} className="evidence">
                        <span className="badge b-neutral" title="A label produced by versioned rules from observed trades">Model assessment</span>
                        <div><b>{t.label}</b><p className="muted" style={{ fontSize: 13 }}>Evidence: {t.evidence}</p></div>
                      </li>
                    ))}
                  </ul>
                  <small className="muted">Rules version {d.rules_version}. Sample size {d.sample_size}.</small>
                </>
              )
            }
          </Remote>
        </Panel>
      </div>
    </div>
  );
}
