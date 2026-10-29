"use client";

import { useState } from "react";
import { api } from "@/lib/api/client";
import type { Chain, MoverTab, MoverWindow } from "@/lib/api/types";
import { CHAINS } from "@/lib/api/types";
import { CHAIN_LABEL } from "@/lib/address";
import { useApi } from "@/hooks/useApi";
import { MoversTable } from "@/components/MoversTable";
import { EmptyState, HonestyNote, PageHead, Panel, Remote, Tabs } from "@/components/ui";

const TABS: ReadonlyArray<{ id: MoverTab; label: string }> = [
  { id: "trending", label: "Trending" },
  { id: "early_momentum", label: "Early momentum" },
  { id: "smart_money", label: "Smart money" },
  { id: "new", label: "New" },
];
const WINDOWS: ReadonlyArray<{ id: MoverWindow; label: string }> = [
  { id: "1h", label: "1 hour" },
  { id: "24h", label: "24 hours" },
  { id: "7d", label: "7 days" },
];

export default function TopMoversPage() {
  const [tab, setTab] = useState<MoverTab>("trending");
  const [win, setWin] = useState<MoverWindow>("24h");
  const [chain, setChain] = useState<Chain | "">("");
  const state = useApi(() => api.movers({ tab, window: win, chain: chain || undefined }), [tab, win, chain]);

  return (
    <div className="stack">
      <PageHead title="Top Movers" sub="Coins gaining attention right now, with the reason, liquidity and risk checks next to each one." />
      <HonestyNote>
        Price, volume and liquidity are observed market data from the listed pool. The reason is a short label for what the data shows, not a prediction. A large move on thin liquidity can reverse quickly.
      </HonestyNote>
      <Panel>
        <div className="toolbar">
          <Tabs label="Movers category" value={tab} onChange={setTab} tabs={TABS} />
          <Tabs small label="Time window" value={win} onChange={setWin} tabs={WINDOWS} />
          <div className="field" style={{ flexDirection: "row", alignItems: "center", gap: 8 }}>
            <label htmlFor="chain-filter">Chain</label>
            <select id="chain-filter" className="select" style={{ width: 150, height: 34 }} value={chain} onChange={(e) => setChain(e.target.value as Chain | "")}>
              <option value="">All chains</option>
              {CHAINS.map((c) => <option key={c} value={c}>{CHAIN_LABEL[c]}</option>)}
            </select>
          </div>
        </div>
        <Remote
          state={state}
          skeletonRows={6}
          isEmpty={(d) => d.items.length === 0}
          empty={<EmptyState title="No coins match">No coins passed the liquidity and data filters for this view. Try another tab, window or chain.</EmptyState>}
        >
          {(d) => (
            <>
              <MoversTable items={d.items} windowLabel={win} />
              {d.items.some((i) => i.risk_notes.length > 0) ? (
                <div style={{ marginTop: 16 }}>
                  <h2 style={{ fontSize: 15, marginBottom: 6 }}>Risk notes</h2>
                  <ul>
                    {d.items.filter((i) => i.risk_notes.length > 0).map((i) => (
                      <li key={`${i.token.chain}:${i.token.address}`} className="list-row">
                        <b style={{ minWidth: 64 }}>{i.token.symbol ?? "Token"}</b>
                        <span className="muted">{i.risk_notes.join(" ")}</span>
                      </li>
                    ))}
                  </ul>
                </div>
              ) : null}
            </>
          )}
        </Remote>
      </Panel>
    </div>
  );
}
