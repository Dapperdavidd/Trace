"use client";

import Link from "next/link";
import { usePathname, useRouter, useSearchParams } from "next/navigation";
import { useEffect, useRef, useState, type ReactNode } from "react";
import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import { useApp } from "./AppState";
import { Ic, Logo } from "./ui";

export const ROOTS = ["today", "discover", "swap", "wallet", "ask"];

const DESK_NAV: Array<[string, string?, string?]> = [
  ["Today", "today", "home"],
  ["Briefing", "briefing", "book"],
  ["#Discover"],
  ["Top movers", "discover", "grid"],
  ["Smart wallets", "wallets", "users"],
  ["Compare", "compare", "cols"],
  ["Ask trace", "ask", "spark"],
  ["#Test and act"],
  ["Shadow test", "shadow", "flask"],
  ["Swap", "swap", "swap"],
  ["#Track"],
  ["Wallet", "wallet", "wallet"],
  ["NEAR agent", "near", "key"],
  ["Profile", "me", "user"],
];
const NAVMAP: Record<string, string> = { today: "today", briefing: "briefing", discover: "discover", token: "discover", wallets: "wallets", profile: "wallets", copy: "wallets", compare: "compare", ask: "ask", shadow: "shadow", swap: "swap", wallet: "wallet", near: "near", me: "me", notifs: "me", security: "me", following: "me", prefs: "me" };
const TABMAP: Record<string, string> = { today: "today", briefing: "today", discover: "discover", token: "discover", compare: "discover", wallets: "discover", profile: "discover", shadow: "discover", copy: "wallet", near: "wallet", swap: "swap", wallet: "wallet", ask: "ask", me: "today", notifs: "today", security: "today", following: "today", prefs: "today" };

export function screenFromPath(path: string): string {
  const m = /\/app\/([a-z]+)/.exec(path);
  return m?.[1] ?? "today";
}

export function initials(name: string | null | undefined): string {
  if (!name) return "";
  const parts = name.trim().split(/\s+/);
  return ((parts[0]?.[0] ?? "") + (parts[1]?.[0] ?? "")).toUpperCase() || name.slice(0, 2).toUpperCase();
}

export function AppFrame({ children }: { children: ReactNode }) {
  const router = useRouter();
  const path = usePathname();
  const params = useSearchParams();
  const scr = screenFromPath(path);
  const { toastMsg, toastOn, toast } = useApp();
  const token = useAuthToken();
  const me = useApi(() => api.me(), [token], !!token);
  const alerts = useApi(() => api.alerts(), [token], !!token);
  const vp = useRef<HTMLDivElement>(null);
  const search = useRef<HTMLInputElement>(null);
  const [q, setQ] = useState(params.get("q") ?? "");

  useEffect(() => {
    vp.current?.scrollTo({ top: 0 });
  }, [path]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "/" && document.activeElement?.tagName !== "INPUT") {
        e.preventDefault();
        search.current?.focus();
      }
    };
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  const go = (s: string) => router.push(`/app/${s}/`);
  const onSearch = (v: string) => {
    setQ(v);
    router.replace(`/app/discover/?q=${encodeURIComponent(v)}`);
  };
  const meName = me.data?.name || me.data?.email || null;
  const alertCount = alerts.data?.items.length ?? 0;
  const notab = ROOTS.indexOf(scr) < 0;

  return (
    <div className={`app desk tapp${notab ? " notab" : ""}`}>
      <aside className="dside">
        <Link href="/" aria-label="trace home">
          <Logo />
        </Link>
        <nav className="dnav">
          {DESK_NAV.map(([label, s, icon]) =>
            label.startsWith("#") ? (
              <div className="gh" key={label}>
                {label.slice(1)}
              </div>
            ) : (
              <button key={label} type="button" className={NAVMAP[scr] === s ? "on" : ""} onClick={() => go(s as string)}>
                <Ic id={icon as string} />
                {label}
              </button>
            ),
          )}
        </nav>
        <div className="dpro">
          <b>trace Pro</b>Real-time alerts and unlimited shadow tests.
          <button className="bout" style={{ height: 32 }} type="button" onClick={() => toast("Pro is coming soon")}>
            Upgrade
          </button>
        </div>
      </aside>
      <div className="dmain">
        <header className="dtop">
          <label className="dsearch">
            <Ic id="srch" />
            <input ref={search} value={q} onChange={(e) => onSearch(e.target.value)} placeholder="Search coins and wallets" autoComplete="off" />
            <kbd>/</kbd>
          </label>
          <span className="sp" />
          <button className="sq" type="button" onClick={() => go("ask")} aria-label="Ask trace">
            <Ic id="spark" />
          </button>
          <button
            className="sq"
            type="button"
            aria-label="Alerts"
            onClick={() => {
              if (!token) toast("Sign in to get alerts from wallets you follow");
              else if (alertCount) router.push("/app/wallet/?t=activity");
              else toast("No alerts yet. Follow a wallet to get one.");
            }}
          >
            <Ic id="bell" />
          </button>
          <button className="sq me" type="button" onClick={() => (token ? go("me") : router.push("/welcome/"))} aria-label={token ? "Your profile" : "Sign in"}>
            {token ? initials(meName) || <Ic id="user" /> : <Ic id="user" />}
          </button>
        </header>
        <div className="vp" ref={vp}>
          {children}
        </div>
      </div>
      <nav className="tabs" aria-label="Main">
        <button type="button" className={TABMAP[scr] === "today" ? "on" : ""} onClick={() => go("today")}>
          <Ic id="home" />
          Today
        </button>
        <button type="button" className={TABMAP[scr] === "discover" ? "on" : ""} onClick={() => go("discover")}>
          <Ic id="grid" />
          Discover
        </button>
        <button type="button" className={`tr${TABMAP[scr] === "swap" ? " on" : ""}`} onClick={() => go("swap")} aria-label="Trade">
          <span>
            <Ic id="swap" />
          </span>
        </button>
        <button type="button" className={TABMAP[scr] === "wallet" ? "on" : ""} onClick={() => go("wallet")}>
          <Ic id="wallet" />
          Wallet
        </button>
        <button type="button" className={TABMAP[scr] === "ask" ? "on" : ""} onClick={() => go("ask")}>
          <Ic id="spark" />
          Ask
        </button>
      </nav>
      <div className={`toast${toastOn ? " on" : ""}`} role="status" aria-live="polite">
        {toastMsg}
      </div>
    </div>
  );
}
