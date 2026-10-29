"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import type { Chain } from "@/lib/api/types";
import { parseChainAddress } from "@/lib/params";
import { shortenAddress } from "@/lib/format";
import { TimelineView } from "@/components/TimelineView";
import { ErrorState, HonestyNote, PageHead, Panel } from "@/components/ui";

export default function TokenTimelinePage() {
  const params = useParams<{ chain: string; address: string }>();
  const parsed = parseChainAddress(params?.chain, params?.address);
  if (!parsed.ok) {
    return (
      <div className="stack">
        <PageHead title="Opportunity Timeline" />
        <ErrorState error={new Error(parsed.reason)} />
      </div>
    );
  }
  return (
    <div className="stack">
      <PageHead
        title="Opportunity Timeline"
        sub={<>The sequence of events behind the move for <span className="mono">{shortenAddress(parsed.address)}</span> on {parsed.chain}.</>}
        action={<Link className="btn" href={`/tokens/${parsed.chain}/${encodeURIComponent(parsed.address)}`}>Back to token</Link>}
      />
      <HonestyNote>Observed facts are separated from hypotheses. A hypothesis is a possible explanation and is not confirmed. Failed or uncertain attributions stay visible.</HonestyNote>
      <Panel>
        <TimelineView chain={parsed.chain as Chain} address={parsed.address} />
      </Panel>
    </div>
  );
}
