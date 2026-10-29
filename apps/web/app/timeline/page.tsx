"use client";

import { useState } from "react";
import { api } from "@/lib/api/client";
import { useApi } from "@/hooks/useApi";
import { tokenKey } from "@/components/MoversTable";
import { TimelineView } from "@/components/TimelineView";
import { EmptyState, HonestyNote, PageHead, Panel, Remote } from "@/components/ui";
import { tokenLabel } from "@/lib/format";

export default function TimelinePage() {
  const movers = useApi(() => api.movers({ tab: "trending", window: "24h" }), []);
  const [key, setKey] = useState<string>("");
  return (
    <div className="stack">
      <PageHead title="Opportunity Timeline" sub="See the story of what happened, step by step, in plain language." />
      <HonestyNote>Observed facts are separated from hypotheses. A hypothesis is a possible explanation and is not confirmed.</HonestyNote>
      <Panel>
        <Remote
          state={movers}
          skeletonRows={3}
          isEmpty={(d) => d.items.length === 0}
          empty={<EmptyState title="No coins available">There are no trending coins to build a timeline for right now.</EmptyState>}
        >
          {(d) => {
            const selected = d.items.find((m) => tokenKey(m.token) === key) ?? d.items[0];
            if (!selected) return null;
            return (
              <>
                <div className="field" style={{ maxWidth: 320, marginBottom: 16 }}>
                  <label htmlFor="tl-coin">Coin</label>
                  <select id="tl-coin" className="select" value={tokenKey(selected.token)} onChange={(e) => setKey(e.target.value)}>
                    {d.items.map((m) => <option key={tokenKey(m.token)} value={tokenKey(m.token)}>{tokenLabel(m.token)} ({m.token.chain})</option>)}
                  </select>
                </div>
                <TimelineView chain={selected.token.chain} address={selected.token.address} />
              </>
            );
          }}
        </Remote>
      </Panel>
    </div>
  );
}
