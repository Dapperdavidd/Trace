import Link from "next/link";
import type { RiskCheck, RiskState, TokenDetail, WhyMoving } from "@/lib/api/types";
import { formatCompactUsd, formatInteger } from "@/lib/format";
import { KindBadge, RiskPill } from "./ui";

export const CHECK_LABEL: Record<RiskCheck["key"], string> = {
  liquidity: "Liquidity",
  honeypot: "Honeypot check",
  holder_concentration: "Large holder concentration",
  contract_safety: "Contract safety",
  suspicious_activity: "Suspicious activity",
};

const GAUGE_TEXT: Record<RiskState, string> = {
  pass: "Fewer risk flags",
  warn: "Caution",
  block: "High risk",
  unknown: "Unknown",
};

const GAUGE_POS: Record<RiskState, number | null> = { pass: 0.12, warn: 0.5, block: 0.88, unknown: null };

/** Semicircle gauge. It only draws the API's overall state, it does not compute a score. */
export function RiskGauge({ state, size = 120 }: { state: RiskState; size?: number }) {
  const r = 46;
  const cx = 60;
  const cy = 58;
  const arc = (from: number, to: number, color: string) => {
    const a0 = Math.PI * (1 - from);
    const a1 = Math.PI * (1 - to);
    const x0 = cx + r * Math.cos(a0);
    const y0 = cy - r * Math.sin(a0);
    const x1 = cx + r * Math.cos(a1);
    const y1 = cy - r * Math.sin(a1);
    return <path d={`M${x0.toFixed(1)} ${y0.toFixed(1)} A${r} ${r} 0 0 1 ${x1.toFixed(1)} ${y1.toFixed(1)}`} stroke={color} strokeWidth="11" fill="none" strokeLinecap="round" />;
  };
  const pos = GAUGE_POS[state];
  const needle = pos === null ? null : (() => {
    const a = Math.PI * (1 - pos);
    return <line x1={cx} y1={cy} x2={cx + (r - 8) * Math.cos(a)} y2={cy - (r - 8) * Math.sin(a)} stroke="#eaf2ee" strokeWidth="3" strokeLinecap="round" />;
  })();
  const unknown = pos === null;
  return (
    <svg width={size} height={size * 0.6} viewBox="0 0 120 70" role="img" aria-label={`Overall risk: ${GAUGE_TEXT[state]}`}>
      {arc(0.02, 0.33, unknown ? "#3a4a42" : "#3fe39b")}
      {arc(0.37, 0.63, unknown ? "#3a4a42" : "#f5b942")}
      {arc(0.67, 0.98, unknown ? "#3a4a42" : "#ff6b6b")}
      {needle}
      <circle cx={cx} cy={cy} r="4.5" fill="#eaf2ee" />
    </svg>
  );
}

export function gaugeText(state: RiskState): string {
  return GAUGE_TEXT[state];
}

export function RiskChecksList({ checks }: { checks: RiskCheck[] }) {
  return (
    <ul>
      {checks.map((c) => (
        <li key={c.key} className="check-row">
          <span className="grow">
            {CHECK_LABEL[c.key]}
            <small>{c.note}</small>
          </span>
          <RiskPill state={c.state} />
        </li>
      ))}
    </ul>
  );
}

export function WhyMovingList({ items }: { items: WhyMoving[] }) {
  if (items.length === 0) {
    return <p className="muted">No explanation is available yet. GiFTED! shows nothing here rather than guess.</p>;
  }
  return (
    <ul>
      {items.map((w, i) => (
        <li key={i} className="evidence">
          <KindBadge kind={w.kind} />
          <div>
            <p>{w.text}</p>
            {w.evidence_ids.length > 0 ? (
              <small className="muted">Evidence: {w.evidence_ids.length} linked record{w.evidence_ids.length === 1 ? "" : "s"} ({w.evidence_ids.slice(0, 2).join(", ")}{w.evidence_ids.length > 2 ? ", ..." : ""})</small>
            ) : (
              <small className="muted">No linked on-chain evidence.</small>
            )}
          </div>
        </li>
      ))}
    </ul>
  );
}

export function TokenStats({ t }: { t: TokenDetail }) {
  return (
    <div className="stat-grid">
      <div className="stat"><small>Market cap</small><b className="num">{formatCompactUsd(t.market_cap_usd)}</b></div>
      <div className="stat"><small>Volume (24h)</small><b className="num">{formatCompactUsd(t.volume_24h_usd)}</b></div>
      <div className="stat"><small>Liquidity</small><b className="num">{formatCompactUsd(t.liquidity_usd)}</b></div>
      <div className="stat"><small>Holders</small><b className="num">{t.holders === null ? "n/a" : formatInteger(t.holders)}</b></div>
    </div>
  );
}

export function BuyersList({ buyers }: { buyers: TokenDetail["buyers"] }) {
  if (buyers.length === 0) return <p className="muted">No tracked wallets have been seen buying this token.</p>;
  return (
    <ul>
      {buyers.map((b) => (
        <li key={`${b.chain}:${b.address}`} className="list-row">
          <Link href={`/wallets/${b.chain}/${encodeURIComponent(b.address)}`} className="grow ellip">
            <b>{b.alias ?? `${b.address.slice(0, 4)}...${b.address.slice(-4)}`}</b>
          </Link>
          <span className="muted">{b.chain}</span>
        </li>
      ))}
    </ul>
  );
}
