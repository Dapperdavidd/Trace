"use client";

import Link from "next/link";
import { useMemo, useState } from "react";
import { api } from "@/lib/api/client";
import type { MoverItem, RiskState } from "@/lib/api/types";
import { useApi } from "@/hooks/useApi";
import { tokenHref, tokenKey } from "@/components/MoversTable";
import { RiskChecksList, RiskGauge, gaugeText } from "@/components/TokenViews";
import { Avatar, EmptyState, HonestyNote, PageHead, Panel, Remote, RiskPill } from "@/components/ui";
import { formatPriceUsd, tokenLabel } from "@/lib/format";

const ORDER: Record<RiskState, number> = { block: 0, warn: 1, unknown: 2, pass: 3 };

export default function RiskRadarPage() {
  const movers = useApi(() => api.movers({ tab: "trending", window: "24h" }), []);
  const [key, setKey] = useState<string | null>(null);
  const sorted = useMemo<MoverItem[]>(
    () => (movers.data ? [...movers.data.items].sort((a, b) => ORDER[a.risk] - ORDER[b.risk]) : []),
    [movers.data],
  );
  const selected = sorted.find((m) => tokenKey(m.token) === key) ?? sorted[0] ?? null;
  const detail = useApi(() => api.token(selected!.token.chain, selected!.token.address), [selected?.token.chain, selected?.token.address], !!selected);

  return (
    <div className="stack">
      <PageHead title="Risk Radar" sub="Spot possible problems before you trade. Coins with warnings are listed first." />
      <HonestyNote>
        Risk checks are automated estimates from contract, liquidity and holder data. A pass is not a guarantee of safety, and an unknown means TRACE could not check. This is a risk warning, not advice.
      </HonestyNote>
      <div className="cols-2">
        <Panel title="Coins to check" sub="Trending coins, highest risk first.">
          <Remote
            state={movers}
            skeletonRows={5}
            isEmpty={(d) => d.items.length === 0}
            empty={<EmptyState title="No coins to check">No trending coins are available right now.</EmptyState>}
          >
            {() => (
              <ul>
                {sorted.map((m) => {
                  const k = tokenKey(m.token);
                  const label = tokenLabel(m.token);
                  return (
                    <li key={k} className={`list-row`} style={selected && tokenKey(selected.token) === k ? { background: "var(--emerald-dim)", borderRadius: 10 } : undefined}>
                      <Avatar seed={m.token.address} label={label} />
                      <button type="button" onClick={() => setKey(k)} aria-pressed={selected ? tokenKey(selected.token) === k : false} className="grow" style={{ all: "unset", cursor: "pointer", flex: 1, minWidth: 0 }}>
                        <b>{label}</b> <span className="muted">{formatPriceUsd(m.price_usd)}</span>
                        <small className="muted" style={{ display: "block" }}>{m.risk_notes[0] ?? "No warnings noted."}</small>
                      </button>
                      <RiskPill state={m.risk} />
                    </li>
                  );
                })}
              </ul>
            )}
          </Remote>
        </Panel>
        <Panel title={selected ? `${tokenLabel(selected.token)} checks` : "Checks"} action={selected ? <Link className="link" href={tokenHref(selected.token)}>Full analysis</Link> : undefined}>
          {!selected ? (
            <EmptyState title="Select a coin" />
          ) : (
            <Remote state={detail} skeletonRows={5} skeletonH={40}>
              {(t) => (
                <>
                  <div className="gauge-wrap">
                    <RiskGauge state={t.risk.overall} size={150} />
                    <div>
                      <b style={{ fontSize: 18 }} className={`r-${t.risk.overall}`}>{gaugeText(t.risk.overall)}</b>
                      <p className="muted" style={{ fontSize: 12.5 }}>Checks fetched {new Date(t.risk.fetched_at).toUTCString().slice(17, 22)} UTC</p>
                    </div>
                  </div>
                  <RiskChecksList checks={t.risk.checks} />
                </>
              )}
            </Remote>
          )}
        </Panel>
      </div>
    </div>
  );
}
