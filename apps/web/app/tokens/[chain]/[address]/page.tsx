"use client";

import Link from "next/link";
import { useParams } from "next/navigation";
import { api } from "@/lib/api/client";
import type { Chain } from "@/lib/api/types";
import { useApi } from "@/hooks/useApi";
import { parseChainAddress } from "@/lib/params";
import { formatPriceUsd, shortenAddress, tokenLabel } from "@/lib/format";
import { BuyersList, RiskChecksList, RiskGauge, TokenStats, WhyMovingList, gaugeText } from "@/components/TokenViews";
import { Avatar, Change, ComingLaterBadge, ErrorState, HonestyNote, PageHead, Panel, Remote } from "@/components/ui";

export default function TokenPage() {
  const params = useParams<{ chain: string; address: string }>();
  const parsed = parseChainAddress(params?.chain, params?.address);
  const chain = (parsed.ok ? parsed.chain : "solana") as Chain;
  const address = parsed.ok ? parsed.address : "";
  const state = useApi(() => api.token(chain, address), [chain, address], parsed.ok);

  if (!parsed.ok) {
    return (
      <div className="stack">
        <PageHead title="Token" />
        <ErrorState error={new Error(parsed.reason)} />
      </div>
    );
  }
  const t = state.data;
  const label = t ? tokenLabel(t.token) : shortenAddress(address);

  return (
    <div className="stack">
      <PageHead
        title={label}
        sub={<><span>{t?.token.name ?? "Token"}</span> on {chain}. <span className="mono">{address}</span></>}
        action={
          <div className="hero-cta" style={{ margin: 0 }}>
            <Link className="btn" href={`/tokens/${chain}/${encodeURIComponent(address)}/timeline`}>Opportunity Timeline</Link>
            <Link className="btn" href="/compare">Compare</Link>
            <button type="button" className="btn" disabled aria-disabled="true" title="Buying is coming later">Buy <ComingLaterBadge /></button>
          </div>
        }
      />
      <HonestyNote>
        Price, volume and liquidity are observed market data. Risk checks are automated estimates. The explanation for a move separates observed facts from hypotheses, and a hypothesis is not a confirmed cause.
      </HonestyNote>
      <Remote state={state} skeletonRows={5} skeletonH={60}>
        {(d) => (
          <>
            <Panel title="Price and market data">
              <div style={{ display: "flex", gap: 14, alignItems: "center", flexWrap: "wrap", marginBottom: 12 }}>
                <Avatar seed={d.token.address} label={label} size="lg" />
                <span className="big-result num">{formatPriceUsd(d.price_usd)}</span>
                <Change value={d.change_pct_24h} digits={1} />
                <span className="muted">over 24 hours</span>
              </div>
              <TokenStats t={d} />
            </Panel>
            <div className="cols-2">
              <Panel title="Why it is moving" sub="Observed facts first. Hypotheses are labelled.">
                <WhyMovingList items={d.why_moving} />
              </Panel>
              <Panel title="Risk checks" sub={`Fetched ${new Date(d.risk.fetched_at).toUTCString().slice(17, 22)} UTC. Estimates, not a guarantee.`}>
                <div className="gauge-wrap">
                  <RiskGauge state={d.risk.overall} size={140} />
                  <b style={{ fontSize: 17 }} className={`r-${d.risk.overall}`}>{gaugeText(d.risk.overall)}</b>
                </div>
                <RiskChecksList checks={d.risk.checks} />
              </Panel>
            </div>
            <Panel title="Wallets seen buying" sub="Tracked wallets with recent buys of this token.">
              <BuyersList buyers={d.buyers} />
            </Panel>
          </>
        )}
      </Remote>
    </div>
  );
}
