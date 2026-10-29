"use client";

import Link from "next/link";
import type { DiscoveredWallet } from "@/lib/api/types";
import { formatPct, formatPlainPct, pctTone, walletLabel, shortenAddress } from "@/lib/format";
import { Avatar } from "./ui";

export const DECISION_LABEL: Record<DiscoveredWallet["decision"], string> = {
  eligible: "Eligible to follow on paper",
  watch_only: "Watch only",
  insufficient_evidence: "Not enough evidence",
  rejected: "Rejected",
};

export const CONFIDENCE_LABEL: Record<DiscoveredWallet["pnl_confidence"], string> = {
  high: "High confidence",
  medium: "Medium confidence",
  low: "Low confidence",
  insufficient: "Insufficient data",
};

export function walletHref(w: { chain: string; address: string }): string {
  return `/wallets/${w.chain}/${encodeURIComponent(w.address)}`;
}

export function WalletList({ items }: { items: DiscoveredWallet[] }) {
  return (
    <ul className="cq">
      {items.map((w) => {
        const label = walletLabel(w.wallet);
        const tone = pctTone(w.realized_return_pct);
        return (
          <li key={`${w.wallet.chain}:${w.wallet.address}`} className="wallet-row">
            <Avatar seed={w.wallet.address} label={label} size="lg" />
            <div className="w-name">
              <Link href={walletHref(w.wallet)}><b className="ellip" style={{ display: "block" }}>{label}</b></Link>
              <small className="muted mono">{shortenAddress(w.wallet.address)}</small>
            </div>
            <div className="w-metric">
              <b className={`num ${tone}`}>{formatPct(w.realized_return_pct, 0)}</b>
              <small className="muted" style={{ display: "block" }} title={`${w.sample_size} observed trades`}>
                {formatPlainPct(w.win_rate_pct)} win rate
              </small>
            </div>
            <div className="tags">
              {w.tags.slice(0, 2).map((t) => <span key={t} className="tag">{t}</span>)}
              <span className="tag" title={`How much to trust the profit estimate (${w.sample_size} trades observed)`}>{CONFIDENCE_LABEL[w.pnl_confidence]}</span>
            </div>
            <Link className="btn btn-sm" href={`/shadow-trades?chain=${w.wallet.chain}&address=${encodeURIComponent(w.wallet.address)}`} aria-label={`Shadow ${label} with paper money`}>
              Shadow
            </Link>
          </li>
        );
      })}
    </ul>
  );
}
