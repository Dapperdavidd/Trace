"use client";

import Link from "next/link";
import type { MoverItem } from "@/lib/api/types";
import { formatCompactUsd, formatPriceUsd, pctTone, tokenLabel } from "@/lib/format";
import { Avatar, Change, RiskPill, Sparkline } from "./ui";

export function tokenKey(t: { chain: string; address: string }): string {
  return `${t.chain}:${t.address}`;
}

export function tokenHref(t: { chain: string; address: string }): string {
  return `/tokens/${t.chain}/${encodeURIComponent(t.address)}`;
}

export function MoversTable({
  items,
  windowLabel,
  selectedKey,
  onSelect,
  compact,
}: {
  items: MoverItem[];
  windowLabel: string;
  selectedKey?: string | null;
  onSelect?: (item: MoverItem) => void;
  compact?: boolean;
}) {
  const cls = `tbl-row movers-grid${compact ? " compact" : ""}`;
  return (
    <div className="cq"><div className="tbl" role="table" aria-label="Top movers">
      <div className={`tbl-head movers-grid${compact ? " compact" : ""}`} role="row">
        <span role="columnheader">#</span>
        <span role="columnheader">Coin</span>
        <span role="columnheader">Price</span>
        <span role="columnheader">{windowLabel}</span>
        <span role="columnheader" className="hide-m hide-c2">Volume</span>
        <span role="columnheader" className="hide-m hide-c">Chart</span>
        <span role="columnheader" className="hide-m hide-c">Why moving</span>
        {compact ? null : <span role="columnheader" className="hide-m hide-c">Risk</span>}
        <span role="columnheader" className="sr-only">Open</span>
      </div>
      {items.map((m) => {
        const key = tokenKey(m.token);
        const label = tokenLabel(m.token);
        const selected = selectedKey === key;
        return (
          <div key={key} className={`${cls}${selected ? " selected" : ""}`} role="row">
            <span className="muted num" role="cell">{m.rank}</span>
            <span className="cell-coin" role="cell">
              <Avatar seed={m.token.address} label={label} />
              {onSelect ? (
                <button
                  type="button"
                  onClick={() => onSelect(m)}
                  aria-pressed={selected}
                  aria-label={`Show research for ${label}`}
                  style={{ all: "unset", cursor: "pointer", minWidth: 0 }}
                >
                  <b>{label}</b>
                  <small>{m.token.name ?? m.token.chain}</small>
                </button>
              ) : (
                <Link href={tokenHref(m.token)} style={{ minWidth: 0 }}>
                  <b>{label}</b>
                  <small>{m.token.name ?? m.token.chain}</small>
                </Link>
              )}
            </span>
            <span className="num" role="cell">{formatPriceUsd(m.price_usd)}</span>
            <span role="cell"><Change value={m.change_pct} /></span>
            <span className="num hide-m hide-c2" role="cell">{formatCompactUsd(m.volume_usd)}</span>
            <span className="hide-m hide-c" role="cell"><Sparkline points={m.sparkline} tone={pctTone(m.change_pct)} /></span>
            <span className="hide-m hide-c" role="cell">{m.reason.label}</span>
            {compact ? null : <span className="hide-m hide-c" role="cell"><RiskPill state={m.risk} /></span>}
            <Link href={tokenHref(m.token)} aria-label={`Open ${label} research page`} role="cell" className="muted" style={{ textAlign: "right" }}>
              &rsaquo;
            </Link>
          </div>
        );
      })}
    </div></div>
  );
}
