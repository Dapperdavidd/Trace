"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import type { KeyboardEvent, ReactNode } from "react";
import { chart2Svg, sparkSeed, sparkSvg, type ChartSet } from "@/lib/trace/viz";
import type { RiskLabel } from "@/lib/trace/data";

export function Ic({ id }: { id: string }) {
  return (
    <svg className="i">
      <use href={`#${id}`} />
    </svg>
  );
}

export function Logo({ className = "logo" }: { className?: string }) {
  return (
    <span className={className}>
      <svg viewBox="0 0 24 24">
        <use href="#lens" />
      </svg>
      trace
    </span>
  );
}

export function Tk({ s, on, l }: { s: string; on?: boolean; l?: boolean }) {
  const ch = ({ SOL: "S", USDC: "$", ETH: "Ξ" } as Record<string, string>)[s] || (s[0] ?? "?");
  return <span className={`tk${on ? " on" : ""}${l ? " l" : ""}`}>{ch}</span>;
}

export function Av({ t, l }: { t: string; l?: boolean }) {
  return <span className={`av${l ? " l" : ""}`}>{t}</span>;
}

export function RiskTag({ r, short }: { r: RiskLabel; short?: boolean }) {
  const cls = r === "Low" ? "g" : r === "High" ? "w" : "";
  return <span className={`tag ${cls}`}>{r === "Unrated" ? "Not rated" : short ? r : `${r} risk`}</span>;
}

const STAG: Record<string, string> = { Good: "Looks good", Pass: "Passed", None: "None found", Watch: "Keep an eye on it", Flag: "Warning", Unknown: "Not checked yet" };
export function StateTag({ s }: { s: string }) {
  const cls = s === "Good" || s === "Pass" || s === "None" ? "g" : s === "Flag" ? "w" : "";
  return <span className={`tag ${cls}`}>{STAG[s] || s}</span>;
}

/** Live/Example source label. */
export function Src({ example, loading }: { example: boolean; loading?: boolean }) {
  if (!example) return <span className="tag g">Live</span>;
  return <span className="tag" title="Fictional data from the design. Live data replaces it as soon as the API has some.">{loading ? "Loading live data" : "Example"}</span>;
}

export function Spark({ values, seed, up = true }: { values?: number[]; seed?: number; up?: boolean }) {
  const html = values && values.length > 1 ? sparkSvg(values, up) : sparkSeed(seed ?? 1, up);
  return <span style={{ display: "contents" }} dangerouslySetInnerHTML={{ __html: html }} />;
}

export function Chart2({ sets, base }: { sets: ChartSet[]; base?: number }) {
  return <div style={{ display: "contents" }} dangerouslySetInnerHTML={{ __html: chart2Svg(sets, { base }) }} />;
}

export function Seg<T extends string | number>({
  opts,
  cur,
  onPick,
  cls = "",
}: {
  opts: Array<[T, string]>;
  cur: T | "";
  onPick: (v: T) => void;
  cls?: string;
}) {
  return (
    <div className={`seg ${cls}`}>
      {opts.map(([v, l]) => (
        <button key={String(v)} type="button" className={String(v) === String(cur) ? "on" : ""} onClick={() => onPick(v)}>
          {l}
        </button>
      ))}
    </div>
  );
}

export function Hd({
  title,
  crumb,
  right,
  back,
}: {
  title: ReactNode;
  crumb?: ReactNode;
  right?: ReactNode;
  back?: boolean;
}) {
  const router = useRouter();
  const goBack = () => {
    if (typeof window !== "undefined" && window.history.length > 1) router.back();
    else router.push("/app/today/");
  };
  return (
    <div className="hd">
      {back ? (
        <button className="hb" type="button" onClick={goBack} aria-label="Back">
          <Ic id="back" />
        </button>
      ) : null}
      <div className="ht">
        {crumb ? <span className="lab">{crumb}</span> : null}
        <h2>{title}</h2>
      </div>
      <div className="ha">{right}</div>
    </div>
  );
}

/** A clickable row that behaves like the design's [data-go][tabindex] rows. */
export function Row({ href, className = "li", children, onClick }: { href?: string; className?: string; children: ReactNode; onClick?: () => void }) {
  const router = useRouter();
  const go = () => {
    if (onClick) onClick();
    else if (href) router.push(href);
  };
  const onKey = (e: KeyboardEvent) => {
    if (e.target !== e.currentTarget) return;
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      go();
    }
  };
  return (
    <div
      className={className}
      tabIndex={0}
      role="link"
      onClick={(e) => {
        if ((e.target as HTMLElement).closest("button,a,input") && e.target !== e.currentTarget) return;
        go();
      }}
      onKeyDown={onKey}
    >
      {children}
    </div>
  );
}

export function Lk({ href, children, onClick }: { href?: string; children: ReactNode; onClick?: () => void }) {
  if (href)
    return (
      <Link className="lk" href={href}>
        {children} <Ic id="arr" />
      </Link>
    );
  return (
    <button type="button" className="lk" onClick={onClick}>
      {children} <Ic id="arr" />
    </button>
  );
}

export function Note({ children }: { children: ReactNode }) {
  return <div className="note">{children}</div>;
}

export function Help({ icon = "spark", children }: { icon?: string; children: ReactNode }) {
  return (
    <div className="help">
      <Ic id={icon} />
      <span>{children}</span>
    </div>
  );
}

export function Sw({ on, onClick, label }: { on: boolean; onClick: () => void; label: string }) {
  return <button type="button" className={`sw${on ? " on" : ""}`} onClick={onClick} aria-label={label} aria-pressed={on} />;
}
