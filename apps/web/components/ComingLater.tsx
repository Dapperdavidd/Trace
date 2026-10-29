import Link from "next/link";
import type { ReactNode } from "react";
import { ComingLaterBadge, Panel } from "./ui";

export function ComingLater({ title, why, alternatives }: { title: string; why: ReactNode; alternatives?: Array<{ href: string; label: string }> }) {
  return (
    <Panel className="coming-hero" as="div">
      <ComingLaterBadge />
      <h1>{title}</h1>
      <p className="muted">{why}</p>
      <p className="muted" style={{ marginTop: 10 }}>
        TRACE is a watch-only research tool for now. It does not hold funds, sign transactions or place trades.
      </p>
      <div className="hero-cta" style={{ justifyContent: "center" }}>
        <button type="button" className="btn btn-primary" disabled aria-disabled="true">{title} (coming later)</button>
        {(alternatives ?? [{ href: "/top-movers", label: "Explore Top Movers" }, { href: "/shadow-trades", label: "Try a paper trade" }]).map((a) => (
          <Link key={a.href} href={a.href} className="btn">{a.label}</Link>
        ))}
      </div>
    </Panel>
  );
}
