"use client";

import Link from "next/link";
import { usePathname, useRouter } from "next/navigation";
import { useEffect, useRef, useState, type FormEvent, type ReactNode } from "react";
import { api, isDemoEnv } from "@/lib/api/client";
import { markDemoActive } from "@/lib/api/demo-state";
import { useApi, useAuthToken, useDemoActive } from "@/hooks/useApi";
import { LATER_NAV, MAIN_NAV, isActive, type NavItem } from "@/lib/nav";
import { NavIcon } from "./NavIcon";
import { IconBell, IconClose, IconMenu, IconSearch, IconUser } from "./icons";

export function Logo({ className = "logo" }: { className?: string }) {
  return (
    <Link href="/" className={className} aria-label="GiFTED! home">
      GiFTED<span className="dot">!</span>
    </Link>
  );
}

function NavLinks({ pathname, onNavigate }: { pathname: string; onNavigate?: () => void }) {
  const render = (item: NavItem) => (
    <Link
      key={item.href}
      href={item.href}
      className={`nav-link${item.soon ? " soon" : ""}`}
      aria-current={isActive(pathname, item.href) ? "page" : undefined}
      onClick={onNavigate}
    >
      <NavIcon name={item.icon} />
      <span>{item.label}</span>
      {item.soon ? <span className="nav-soon">Coming later</span> : null}
    </Link>
  );
  return (
    <>
      <nav className="nav" aria-label="Main">{MAIN_NAV.map(render)}</nav>
      <div className="nav-group" id="later-group">Wallet (not available yet)</div>
      <nav className="nav" aria-labelledby="later-group">{LATER_NAV.map(render)}</nav>
    </>
  );
}

function Sidebar({ pathname }: { pathname: string }) {
  return (
    <aside className="sidebar" aria-label="Sidebar">
      <Logo />
      <NavLinks pathname={pathname} />
      <div className="side-note">
        <strong>Watch-only research</strong>
        GiFTED! does not hold funds, sign transactions or place trades. Paper trades are simulations.
      </div>
    </aside>
  );
}

function AccountMenu() {
  const [open, setOpen] = useState(false);
  const token = useAuthToken();
  const router = useRouter();
  const ref = useRef<HTMLDivElement>(null);
  const me = useApi(() => api.me(), [token], !!token);

  useEffect(() => {
    if (!open) return;
    const onDown = (e: MouseEvent) => {
      if (ref.current && !ref.current.contains(e.target as Node)) setOpen(false);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    document.addEventListener("mousedown", onDown);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("mousedown", onDown);
      document.removeEventListener("keydown", onKey);
    };
  }, [open]);

  const signOut = async () => {
    setOpen(false);
    try {
      await api.logout();
    } catch {
      /* token is cleared even if the request fails */
    }
    router.push("/");
  };

  return (
    <div className="menu-wrap" ref={ref}>
      <button type="button" className="avatar-btn" aria-haspopup="menu" aria-expanded={open} aria-label="Account menu" onClick={() => setOpen((o) => !o)}>
        <span className="avatar" style={{ background: "linear-gradient(135deg,#4be3a6,#2fcf8f)" }}>
          {token && me.data ? me.data.name.slice(0, 1).toUpperCase() : <IconUser size={18} />}
        </span>
      </button>
      {open ? (
        <div className="menu" role="menu">
          {token ? (
            <>
              <div className="menu-head">
                Signed in
                <b>{me.data?.name ?? "Your account"}</b>
                {me.data?.email}
              </div>
              <div className="menu-head" style={{ borderBottom: 0, paddingTop: 0 }}>
                Watch-only account. No spendable wallet.
              </div>
              <Link role="menuitem" href="/watchlist" onClick={() => setOpen(false)}>Watchlist</Link>
              <Link role="menuitem" href="/alerts" onClick={() => setOpen(false)}>Alerts</Link>
              <button role="menuitem" type="button" onClick={signOut}>Sign out</button>
            </>
          ) : (
            <>
              <div className="menu-head">
                Not signed in
                <b>Guest</b>
                Sign in to save a watchlist.
              </div>
              <Link role="menuitem" href="/sign-in" onClick={() => setOpen(false)}>Sign in with Google</Link>
            </>
          )}
        </div>
      ) : null}
    </div>
  );
}

function Topbar() {
  const router = useRouter();
  const [q, setQ] = useState("");
  const input = useRef<HTMLInputElement>(null);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const el = document.activeElement as HTMLElement | null;
      const typing = el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable);
      if (e.key === "/" && !typing) {
        e.preventDefault();
        input.current?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const submit = (e: FormEvent) => {
    e.preventDefault();
    const value = q.trim();
    if (!value) return;
    router.push(`/search?q=${encodeURIComponent(value)}`);
  };

  return (
    <header className="topbar">
      <Logo className="m-logo" />
      <form className="search" role="search" onSubmit={submit}>
        <IconSearch size={18} />
        <input
          ref={input}
          type="search"
          name="q"
          value={q}
          onChange={(e) => setQ(e.target.value)}
          placeholder="Search coins, wallets, or ask a question"
          aria-label="Search coins, wallets, or ask a question"
          autoComplete="off"
          spellCheck={false}
        />
        <span className="kbd" aria-hidden="true">/</span>
      </form>
      <nav className="topnav" aria-label="Quick links">
        <Link href="/top-movers">Explore</Link>
        <Link href="/smart-wallets">Research</Link>
        <Link href="/shadow-trades">Shadow</Link>
        <span className="soon" title="Wallet is coming later">Wallet (coming later)</span>
        <Link href="/">Dashboard</Link>
      </nav>
      <div className="top-actions">
        <Link href="/alerts" className="icon-btn" aria-label="Alerts">
          <IconBell size={21} />
        </Link>
        <AccountMenu />
      </div>
    </header>
  );
}

function DemoBanner() {
  const { active, reason } = useDemoActive();
  if (!active) return null;
  return (
    <div className="demo-banner" role="status" data-testid="demo-banner">
      <strong>DEMO DATA</strong>
      <span>The numbers on this page are fictional samples, not live market data. {reason ?? ""}</span>
    </div>
  );
}

function MobileNav({ pathname }: { pathname: string }) {
  const [open, setOpen] = useState(false);
  const closeBtn = useRef<HTMLButtonElement>(null);
  useEffect(() => {
    if (!open) return;
    closeBtn.current?.focus();
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [open]);
  const primary = MAIN_NAV.filter((n) => ["/", "/top-movers", "/smart-wallets", "/risk-radar"].includes(n.href));
  return (
    <>
      <nav className="mobile-nav" aria-label="Primary">
        {primary.map((n) => (
          <Link key={n.href} href={n.href} aria-current={isActive(pathname, n.href) ? "page" : undefined}>
            <NavIcon name={n.icon} size={22} />
            {n.label === "Smart Wallets" ? "Wallets" : n.label === "Top Movers" ? "Movers" : n.label === "Risk Radar" ? "Radar" : n.label}
          </Link>
        ))}
        <button type="button" onClick={() => setOpen(true)} aria-haspopup="dialog" aria-expanded={open}>
          <IconMenu size={22} />
          More
        </button>
      </nav>
      {open ? (
        <>
          <div className="drawer-backdrop" onClick={() => setOpen(false)} />
          <div className="drawer" role="dialog" aria-modal="true" aria-label="All sections">
            <div className="drawer-head">
              <Logo />
              <button ref={closeBtn} type="button" className="icon-btn" aria-label="Close menu" onClick={() => setOpen(false)}>
                <IconClose />
              </button>
            </div>
            <NavLinks pathname={pathname} onNavigate={() => setOpen(false)} />
            <div className="side-note" style={{ marginTop: 14 }}>
              <strong>Watch-only research</strong>
              GiFTED! does not hold funds, sign transactions or place trades.
            </div>
          </div>
        </>
      ) : null}
    </>
  );
}

export function AppShell({ children }: { children: ReactNode }) {
  const pathname = usePathname() ?? "/";
  useEffect(() => {
    if (isDemoEnv()) markDemoActive("Demo mode is on (NEXT_PUBLIC_TRACE_DEMO=1).");
    // Cheap coverage probe so the DEMO banner is also correct on pages that load no data.
    api.chains().catch(() => undefined);
  }, []);
  return (
    <>
      <a className="skip-link" href="#main">Skip to content</a>
      <div className="shell">
        <Sidebar pathname={pathname} />
        <div className="main">
          <DemoBanner />
          <Topbar />
          <main id="main" className="content" tabIndex={-1}>
            {children}
          </main>
        </div>
      </div>
      <MobileNav pathname={pathname} />
    </>
  );
}

