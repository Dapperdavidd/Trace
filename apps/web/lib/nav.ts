export type NavIcon =
  | "home" | "trend" | "users" | "shadow" | "radar" | "star" | "brief" | "compare" | "ask"
  | "timeline" | "bell" | "wallet" | "send" | "receive" | "swap" | "clock";

export interface NavItem {
  href: string;
  label: string;
  icon: NavIcon;
  soon?: boolean;
}

export const MAIN_NAV: NavItem[] = [
  { href: "/", label: "Home", icon: "home" },
  { href: "/top-movers", label: "Top Movers", icon: "trend" },
  { href: "/smart-wallets", label: "Smart Wallets", icon: "users" },
  { href: "/shadow-trades", label: "Shadow Trades", icon: "shadow" },
  { href: "/risk-radar", label: "Risk Radar", icon: "radar" },
  { href: "/timeline", label: "Timeline", icon: "timeline" },
  { href: "/watchlist", label: "Watchlist", icon: "star" },
  { href: "/alerts", label: "Alerts", icon: "bell" },
  { href: "/briefing", label: "Daily Briefing", icon: "brief" },
  { href: "/compare", label: "Compare Coins", icon: "compare" },
  { href: "/ask", label: "Ask TRACE", icon: "ask", soon: true },
];

export const LATER_NAV: NavItem[] = [
  { href: "/wallet", label: "Wallet", icon: "wallet", soon: true },
  { href: "/swap", label: "Swap", icon: "swap", soon: true },
  { href: "/receive", label: "Receive", icon: "receive", soon: true },
  { href: "/send", label: "Send", icon: "send", soon: true },
];

export function isActive(pathname: string, href: string): boolean {
  if (href === "/") return pathname === "/";
  return pathname === href || pathname.startsWith(`${href}/`);
}
