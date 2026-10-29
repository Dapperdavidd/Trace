"use client";

import Link from "next/link";
import { api } from "@/lib/api/client";
import { useApi } from "@/hooks/useApi";
import { EmptyState, HonestyNote, PageHead, Panel, Remote } from "@/components/ui";

export default function BriefingPage() {
  const state = useApi(() => api.briefings(), []);
  return (
    <div className="stack">
      <PageHead title="Daily Briefing" sub="A short summary of what changed, in plain language." />
      <HonestyNote>Each line is generated from observed data. Paper results are simulations, not earnings.</HonestyNote>
      <Remote
        state={state}
        skeletonRows={4}
        skeletonH={80}
        isEmpty={(d) => d.sections.every((s) => s.items.length === 0)}
        empty={<EmptyState title="Nothing new to report">There is no briefing yet. Check back after more data arrives.</EmptyState>}
      >
        {(d) => (
          <div className="cols-2">
            {d.sections.map((s) => (
              <Panel key={s.title} title={s.title}>
                {s.items.length === 0 ? (
                  <p className="muted">Nothing in this section.</p>
                ) : (
                  <ul>
                    {s.items.map((it, i) => (
                      <li key={i} className="list-row">
                        <span className="grow">{it.text}</span>
                        {it.link && it.link.startsWith("/") ? <Link className="link" href={it.link}>Open</Link> : null}
                      </li>
                    ))}
                  </ul>
                )}
              </Panel>
            ))}
          </div>
        )}
      </Remote>
    </div>
  );
}
