"use client";

import { useState } from "react";
import { api } from "@/lib/api/client";
import type { TokenSummary } from "@/lib/api/types";
import { useApi } from "@/hooks/useApi";
import { tokenKey } from "@/components/MoversTable";
import { Avatar, Change, EmptyState, HonestyNote, PageHead, Panel, Remote, RiskPill } from "@/components/ui";
import { formatCompactUsd, formatPriceUsd, tokenLabel } from "@/lib/format";

function ref(t: { chain: string; address: string }): string {
  return `${t.chain}:${t.address}`;
}

export default function ComparePage() {
  const movers = useApi(() => api.movers({ tab: "trending", window: "24h" }), []);
  const [a, setA] = useState("");
  const [b, setB] = useState("");
  const items = movers.data?.items ?? [];
  const aKey = a || (items[0] ? tokenKey(items[0].token) : "");
  const bKey = b || (items[1] ? tokenKey(items[1].token) : "");
  const ready = !!aKey && !!bKey && aKey !== bKey;
  const cmp = useApi(() => api.compare(aKey, bKey), [aKey, bKey], ready);

  return (
    <div className="stack">
      <PageHead title="Compare Coins" sub="See which opportunity looks stronger, side by side, in plain English." />
      <HonestyNote>The table shows observed and estimated numbers. The verdict note is a model assessment to help you think, not advice and not a prediction.</HonestyNote>
      <Panel>
        <Remote
          state={movers}
          skeletonRows={2}
          isEmpty={(d) => d.items.length < 2}
          empty={<EmptyState title="Not enough coins to compare">At least two listed coins are needed.</EmptyState>}
        >
          {(d) => (
            <div className="form-grid" style={{ maxWidth: 640 }}>
              {([["Coin A", aKey, setA], ["Coin B", bKey, setB]] as const).map(([label, val, set], i) => (
                <div className="field" key={label}>
                  <label htmlFor={`cmp-${i}`}>{label}</label>
                  <select id={`cmp-${i}`} className="select" value={val} onChange={(e) => set(e.target.value)}>
                    {d.items.map((m) => <option key={tokenKey(m.token)} value={tokenKey(m.token)}>{tokenLabel(m.token)} ({m.token.chain})</option>)}
                  </select>
                </div>
              ))}
            </div>
          )}
        </Remote>
      </Panel>
      {!ready && items.length >= 2 ? <EmptyState title="Pick two different coins" /> : null}
      {ready ? (
        <Panel title="Side by side">
          <Remote state={cmp} skeletonRows={5} skeletonH={40}>
            {(d) => (
              <>
                <div className="scroll-x">
                  <table style={{ width: "100%", borderCollapse: "collapse", minWidth: 420 }}>
                    <caption className="sr-only">Comparison of coins</caption>
                    <thead>
                      <tr>
                        <th scope="col" style={{ textAlign: "left", padding: 8 }} className="muted">Measure</th>
                        {d.items.map((t: TokenSummary) => (
                          <th scope="col" key={ref(t.token)} style={{ textAlign: "left", padding: 8 }}>
                            <span style={{ display: "inline-flex", alignItems: "center", gap: 8 }}><Avatar seed={t.token.address} label={tokenLabel(t.token)} size="sm" />{tokenLabel(t.token)}</span>
                          </th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {([
                        ["Price", (t: TokenSummary) => formatPriceUsd(t.price_usd)],
                        ["Change (24h)", (t: TokenSummary) => <Change value={t.change_pct_24h} />],
                        ["Market cap", (t: TokenSummary) => formatCompactUsd(t.market_cap_usd)],
                        ["Volume (24h)", (t: TokenSummary) => formatCompactUsd(t.volume_24h_usd)],
                        ["Liquidity", (t: TokenSummary) => formatCompactUsd(t.liquidity_usd)],
                        ["Risk checks", (t: TokenSummary) => <RiskPill state={t.risk} />],
                        ["Note", (t: TokenSummary) => t.note ?? "None"],
                      ] as Array<[string, (t: TokenSummary) => React.ReactNode]>).map(([name, fn]) => (
                        <tr key={name} style={{ borderTop: "1px solid var(--line)" }}>
                          <th scope="row" style={{ textAlign: "left", padding: 8, fontWeight: 500 }} className="muted">{name}</th>
                          {d.items.map((t) => <td key={ref(t.token)} style={{ padding: 8 }} className="num">{fn(t)}</td>)}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
                <div className="state" style={{ marginTop: 14 }}>
                  <h4>Our analysis (model assessment)</h4>
                  <p>{d.verdict_note}</p>
                </div>
              </>
            )}
          </Remote>
        </Panel>
      ) : null}
    </div>
  );
}
