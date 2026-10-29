import type { NavIcon as Name } from "@/lib/nav";
import * as I from "./icons";

const MAP = {
  home: I.IconHome, trend: I.IconTrend, users: I.IconUsers, shadow: I.IconShadow, radar: I.IconRadar,
  star: I.IconStar, brief: I.IconBrief, compare: I.IconCompare, ask: I.IconAsk, timeline: I.IconTimeline,
  bell: I.IconBell, wallet: I.IconWallet, send: I.IconSend, receive: I.IconReceive, swap: I.IconSwap, clock: I.IconClock,
} as const;

export function NavIcon({ name, size = 20 }: { name: Name; size?: number }) {
  const C = MAP[name];
  return <C size={size} />;
}
