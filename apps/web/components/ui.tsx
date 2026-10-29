"use client";

import Link from "next/link";
import { useId, useRef, type KeyboardEvent, type ReactNode } from "react";
import type { ApiState } from "@/hooks/useApi";
import { describeError } from "@/lib/api/client";
import { hasEnvelope } from "@/lib/api/guards";
import type { DataStatus, Envelope, RiskState } from "@/lib/api/types";
import { formatClockUtc, formatPct, pctTone, shortenAddress } from "@/lib/format";

/* ---------------------------- badges and status ---------------------------- */

const STATUS_LABEL: Record<DataStatus, string> = {
  live: "Live",
  stale: "Stale",
  unavailable: "Data unavailable",
  partial: "Partial data",
  demo: "DEMO DATA",
};

export function StatusBadge({ status }: { status: DataStatus }) {
  return (
    <span className={`badge b-${status}`} data-status={status}>
      {STATUS_LABEL[status]}
    </span>
  );
}

export function ComingLaterBadge({ label = "Coming later" }: { label?: string }) {
  return <span className="badge b-soon">{label}</span>;
}

export function StatusLine({ env }: { env: Envelope }) {
  return (
    <div className="status-line">
      <StatusBadge status={env.data_status} />
      <span>As of {formatClockUtc(env.as_of)}</span>
      {env.data_status === "stale" ? <span>This data is older than expected. Check the time before relying on it.</span> : null}
      {env.coverage_note && env.data_status !== "demo" ? <span>{env.coverage_note}</span> : null}
    </div>
  );
}

const RISK_LABEL: Record<RiskState, string> = { pass: "Passed", warn: "Caution", block: "Failed", unknown: "Unknown" };

export function RiskPill({ state }: { state: RiskState }) {
  return <span className={`r-${state}`} style={{ fontWeight: 700 }}>{RISK_LABEL[state]}</span>;
}

export function riskLabel(state: RiskState): string {
  return RISK_LABEL[state];
}

export function Change({ value, digits = 1 }: { value: string | number | null | undefined; digits?: number }) {
  return <span className={`num ${pctTone(value)}`}>{formatPct(value, digits)}</span>;
}

/* ---------------------------------- tabs ----------------------------------- */

export function Tabs<T extends string>({
  tabs,
  value,
  onChange,
  label,
  small,
}: {
  tabs: ReadonlyArray<{ id: T; label: string }>;
  value: T;
  onChange: (v: T) => void;
  label: string;
  small?: boolean;
}) {
  const refs = useRef<Record<string, HTMLButtonElement | null>>({});
  const onKey = (e: KeyboardEvent<HTMLButtonElement>, i: number) => {
    let next = -1;
    if (e.key === "ArrowRight") next = (i + 1) % tabs.length;
    if (e.key === "ArrowLeft") next = (i - 1 + tabs.length) % tabs.length;
    if (e.key === "Home") next = 0;
    if (e.key === "End") next = tabs.length - 1;
    if (next >= 0) {
      e.preventDefault();
      const t = tabs[next];
      if (t) {
        onChange(t.id);
        refs.current[t.id]?.focus();
      }
    }
  };
  return (
    <div className={`tabs${small ? " sm" : ""}`} role="tablist" aria-label={label}>
      {tabs.map((t, i) => (
        <button
          key={t.id}
          ref={(el) => {
            refs.current[t.id] = el;
          }}
          role="tab"
          type="button"
          className="tab"
          aria-selected={value === t.id}
          tabIndex={value === t.id ? 0 : -1}
          onClick={() => onChange(t.id)}
          onKeyDown={(e) => onKey(e, i)}
        >
          {t.label}
        </button>
      ))}
    </div>
  );
}

/* ------------------------------ states and shells --------------------------- */

export function Skeleton({ h = 16, w = "100%", style }: { h?: number; w?: number | string; style?: React.CSSProperties }) {
  return <span className="skeleton" style={{ height: h, width: w, ...style }} />;
}

export function SkeletonRows({ rows = 4, h = 46 }: { rows?: number; h?: number }) {
  return (
    <div role="status" aria-live="polite" aria-label="Loading" style={{ display: "grid", gap: 8 }}>
      {Array.from({ length: rows }).map((_, i) => (
        <Skeleton key={i} h={h} />
      ))}
      <span className="sr-only">Loading</span>
    </div>
  );
}

export function ErrorState({ error, onRetry }: { error: unknown; onRetry?: () => void }) {
  const d = describeError(error);
  const unauthorized = d.title === "Sign in required";
  return (
    <div className="state error" role="alert">
      <h4>{d.title}</h4>
      <p>{d.message}</p>
      {d.requestId ? <span className="reqid">Request id: {d.requestId}</span> : null}
      {unauthorized ? (
        <Link className="btn btn-primary btn-sm" href="/sign-in" style={{ marginTop: 10 }}>
          Sign in
        </Link>
      ) : onRetry ? (
        <button type="button" className="btn btn-sm" onClick={onRetry}>
          Try again
        </button>
      ) : null}
    </div>
  );
}

export function EmptyState({ title, children, action }: { title: string; children?: ReactNode; action?: ReactNode }) {
  return (
    <div className="state">
      <h4>{title}</h4>
      {children ? <p>{children}</p> : null}
      {action}
    </div>
  );
}

export function UnavailableState({ env }: { env?: Envelope }) {
  return (
    <div className="state" role="status">
      <h4>
        <span className="badge b-unavailable" style={{ marginRight: 8 }}>Data unavailable</span>
        We could not load this right now
      </h4>
      <p>
        {env?.coverage_note ??
          "The data source did not return usable numbers, so TRACE shows nothing here instead of guessing."}
      </p>
    </div>
  );
}

export function Panel({
  title,
  sub,
  action,
  children,
  className = "",
  as: Tag = "section",
}: {
  title?: ReactNode;
  sub?: ReactNode;
  action?: ReactNode;
  children: ReactNode;
  className?: string;
  as?: "section" | "div" | "article";
}) {
  const id = useId();
  return (
    <Tag className={`panel ${className}`} aria-labelledby={title ? id : undefined}>
      {title ? (
        <div className="panel-h">
          <div>
            <h2 id={id}>{title}</h2>
            {sub ? <p className="panel-sub">{sub}</p> : null}
          </div>
          {action}
        </div>
      ) : null}
      {children}
    </Tag>
  );
}

/**
 * Renders the correct first-class state for a remote value: skeleton, error,
 * data unavailable, empty, or the data (with its honesty status line).
 */
export function Remote<T>({
  state,
  children,
  skeletonRows = 4,
  skeletonH = 46,
  isEmpty,
  empty,
  hideStatus,
}: {
  state: ApiState<T>;
  children: (data: T) => ReactNode;
  skeletonRows?: number;
  skeletonH?: number;
  isEmpty?: (data: T) => boolean;
  empty?: ReactNode;
  hideStatus?: boolean;
}) {
  if (state.status === "loading" && state.data === null) return <SkeletonRows rows={skeletonRows} h={skeletonH} />;
  if (state.status === "error") return <ErrorState error={state.error} onRetry={state.reload} />;
  const data = state.data;
  if (data === null) return <SkeletonRows rows={skeletonRows} h={skeletonH} />;
  const env: Envelope | null = hasEnvelope(data) ? (data as unknown as Envelope) : null;
  if (env && env.data_status === "unavailable") return <UnavailableState env={env} />;
  return (
    <>
      {env && !hideStatus ? <StatusLine env={env} /> : null}
      {isEmpty?.(data) ? (empty ?? <EmptyState title="Nothing to show yet" />) : children(data)}
    </>
  );
}

/* --------------------------------- visuals --------------------------------- */

const AVATAR_COLORS = ["#4be3a6", "#7aa8ff", "#f5b942", "#c9b6ff", "#ff9b8a", "#5fd4e8", "#b8e36a"];

function hash(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (h * 31 + s.charCodeAt(i)) >>> 0;
  return h;
}

export function Avatar({ seed, label, size }: { seed: string; label: string; size?: "sm" | "lg" }) {
  const h = hash(seed);
  const a = AVATAR_COLORS[h % AVATAR_COLORS.length];
  const b = AVATAR_COLORS[(h >>> 3) % AVATAR_COLORS.length];
  return (
    <span className={`avatar${size ? ` ${size}` : ""}`} style={{ background: `linear-gradient(135deg, ${a}, ${b})` }} aria-hidden="true">
      {label.slice(0, 2).toUpperCase()}
    </span>
  );
}

export function Sparkline({ points, tone = "up", width = 110, height = 34 }: { points: number[]; tone?: "up" | "down" | "flat"; width?: number; height?: number }) {
  if (!points || points.length < 2) return <span className="muted" aria-label="No chart data">n/a</span>;
  const min = Math.min(...points);
  const max = Math.max(...points);
  const span = max - min || 1;
  const step = width / (points.length - 1);
  const d = points.map((p, i) => `${i === 0 ? "M" : "L"}${(i * step).toFixed(1)} ${(height - 2 - ((p - min) / span) * (height - 4)).toFixed(1)}`).join(" ");
  const color = tone === "down" ? "var(--down)" : tone === "flat" ? "var(--muted)" : "var(--up)";
  const gid = `sg-${hash(d)}`;
  return (
    <svg width="100%" height={height} viewBox={`0 0 ${width} ${height}`} preserveAspectRatio="none" role="img" aria-label="Recent price shape" style={{ maxWidth: width * 2 }}>
      <defs>
        <linearGradient id={gid} x1="0" x2="0" y1="0" y2="1">
          <stop offset="0" stopColor={color} stopOpacity="0.35" />
          <stop offset="1" stopColor={color} stopOpacity="0" />
        </linearGradient>
      </defs>
      <path d={`${d} L${width} ${height} L0 ${height} Z`} fill={`url(#${gid})`} />
      <path d={d} fill="none" stroke={color} strokeWidth="1.8" strokeLinejoin="round" strokeLinecap="round" vectorEffect="non-scaling-stroke" />
    </svg>
  );
}

export function AddressText({ address, head = 4, tail = 4 }: { address: string; head?: number; tail?: number }) {
  return (
    <span className="mono" title={address}>
      {shortenAddress(address, head, tail)}
    </span>
  );
}

export function PageHead({ title, sub, action }: { title: string; sub?: ReactNode; action?: ReactNode }) {
  return (
    <header className="page-head">
      <div>
        <h1>{title}</h1>
        {sub ? <p>{sub}</p> : null}
      </div>
      {action}
    </header>
  );
}

export function HonestyNote({ children }: { children: ReactNode }) {
  return <p className="honesty-note">{children}</p>;
}

/** Fact vs hypothesis label used across research views. */
export function KindBadge({ kind }: { kind: "fact" | "hypothesis" }) {
  return kind === "fact" ? (
    <span className="badge b-fact" title="Observed on chain or in market data">Observed fact</span>
  ) : (
    <span className="badge b-hyp" title="A possible explanation that has not been confirmed">Hypothesis</span>
  );
}
