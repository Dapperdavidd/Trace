import type { ShadowResults } from "@/lib/api/types";
import { formatPct, formatPlainPct, formatUsd, pctTone } from "@/lib/format";

function fmt(key: string, v: string | number | null): string {
  if (v === null) return "n/a";
  if (key.endsWith("_pct")) return formatPct(v);
  return String(v);
}

function label(key: string): string {
  const s = key.replace(/_pct$/, "").replace(/_/g, " ");
  return s.charAt(0).toUpperCase() + s.slice(1);
}

export function ShadowResultView({ r, compact }: { r: ShadowResults; compact?: boolean }) {
  return (
    <div>
      <p className="muted" style={{ fontSize: 12.5 }}>Simulated result. Paper money only. Not a prediction.</p>
      <div style={{ display: "flex", alignItems: "baseline", gap: 14, flexWrap: "wrap", margin: "6px 0 10px" }}>
        <span className={`big-result num ${pctTone(r.return_pct)}`}>{formatPct(r.return_pct)}</span>
        <span>
          <small className="muted" style={{ display: "block" }}>Simulated ending value</small>
          <b className="num" style={{ fontSize: 18 }}>{formatUsd(r.ending_usd)}</b>
        </span>
        <span>
          <small className="muted" style={{ display: "block" }}>Starting with</small>
          <b className="num">{formatUsd(r.starting_usd)}</b>
        </span>
      </div>
      <div className="kv"><span>Total trades</span><b className="num">{r.trades}</b></div>
      <div className="kv"><span>Win rate</span><b className="num">{formatPlainPct(r.win_rate_pct, 0)}</b></div>
      <div className="kv"><span>Skipped (could not be copied at a follower price)</span><b className="num">{r.skipped_unexecutable}</b></div>
      <div className="kv"><span>Fees and slippage</span><b className="num down">-{formatUsd(r.fees_and_slippage_usd).replace("-", "")}</b></div>
      {compact ? null : (
        <div className="cols-2" style={{ marginTop: 16 }}>
          <div>
            <h3 style={{ fontSize: 14, marginBottom: 6 }}>Baseline for comparison</h3>
            {Object.entries(r.baseline).map(([k, v]) => (
              <div className="kv" key={k}><span>{label(k)}</span><b>{fmt(k, v)}</b></div>
            ))}
          </div>
          <div>
            <h3 style={{ fontSize: 14, marginBottom: 6 }}>Assumptions used</h3>
            {Object.entries(r.assumptions).map(([k, v]) => (
              <div className="kv" key={k}><span>{label(k)}</span><b>{fmt(k, v)}</b></div>
            ))}
          </div>
        </div>
      )}
    </div>
  );
}
