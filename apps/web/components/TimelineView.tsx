"use client";

import { api } from "@/lib/api/client";
import type { Chain } from "@/lib/api/types";
import { useApi } from "@/hooks/useApi";
import { formatClockUtc, formatRelativeTime } from "@/lib/format";
import { EmptyState, KindBadge, Remote } from "./ui";

export function TimelineView({ chain, address }: { chain: Chain; address: string }) {
  const state = useApi(() => api.tokenTimeline(chain, address), [chain, address]);
  return (
    <Remote
      state={state}
      skeletonRows={5}
      skeletonH={60}
      isEmpty={(d) => d.items.length === 0}
      empty={<EmptyState title="No timeline yet">There are no events to show for this token in the selected period.</EmptyState>}
    >
      {(d) => (
        <ol className="timeline" style={{ listStyle: "none", margin: 0 }}>
          {d.items.map((it, i) => (
            <li key={i} className={`tl-item ${it.kind === "hypothesis" ? "hyp" : ""}`}>
              <div className="tl-time">{formatClockUtc(it.time)} ({formatRelativeTime(it.time)})</div>
              <div style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap" }}>
                <b>{it.title}</b>
                <KindBadge kind={it.kind} />
              </div>
              <p style={{ fontSize: 13.5, color: "#cfe0d7" }}>{it.detail}</p>
              {it.possible_impact ? <p className="muted" style={{ fontSize: 12.5 }}>Possible impact: {it.possible_impact}</p> : null}
              <small className="muted">{it.evidence_ids.length > 0 ? `Evidence: ${it.evidence_ids.join(", ")}` : "No linked evidence."}</small>
            </li>
          ))}
        </ol>
      )}
    </Remote>
  );
}
