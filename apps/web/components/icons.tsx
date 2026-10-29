import type { ReactNode, SVGProps } from "react";

function Svg({ children, size = 20, ...rest }: SVGProps<SVGSVGElement> & { size?: number; children: ReactNode }) {
  return (
    <svg width={size} height={size} viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={1.8} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false" {...rest}>
      {children}
    </svg>
  );
}
type P = { size?: number };

export const IconHome = (p: P) => <Svg {...p}><path d="M3 11l9-8 9 8" /><path d="M5 10v10h14V10" /></Svg>;
export const IconTrend = (p: P) => <Svg {...p}><path d="M3 17l6-6 4 4 8-8" /><path d="M15 7h6v6" /></Svg>;
export const IconUsers = (p: P) => <Svg {...p}><circle cx="9" cy="8" r="3.5" /><path d="M2.5 20c0-3.5 3-6 6.5-6s6.5 2.5 6.5 6" /><path d="M16 4.5a3.5 3.5 0 010 7M18.5 14.5c2 .7 3.5 2.5 3.5 5.5" /></Svg>;
export const IconShadow = (p: P) => <Svg {...p}><rect x="3" y="3" width="13" height="13" rx="3" /><path d="M8 21h10a3 3 0 003-3V8" strokeDasharray="3 3" /></Svg>;
export const IconRadar = (p: P) => <Svg {...p}><path d="M12 3l8 3v6c0 5-3.5 8-8 9-4.5-1-8-4-8-9V6l8-3z" /><path d="M12 8v5M12 16h.01" /></Svg>;
export const IconStar = (p: P) => <Svg {...p}><path d="M12 3l2.8 5.7 6.2.9-4.5 4.4 1.1 6.2L12 17.2 6.4 20.2l1.1-6.2L3 9.6l6.2-.9L12 3z" /></Svg>;
export const IconBrief = (p: P) => <Svg {...p}><path d="M4 5a2 2 0 012-2h12a2 2 0 012 2v14l-4-2-4 2-4-2-4 2V5z" /><path d="M8 8h8M8 12h5" /></Svg>;
export const IconCompare = (p: P) => <Svg {...p}><circle cx="8" cy="12" r="5" /><circle cx="16" cy="12" r="5" /></Svg>;
export const IconAsk = (p: P) => <Svg {...p}><path d="M21 12a8 8 0 01-11.6 7.1L4 20l1-4.6A8 8 0 1121 12z" /></Svg>;
export const IconBell = (p: P) => <Svg {...p}><path d="M6 9a6 6 0 0112 0c0 6 2 7 2 7H4s2-1 2-7z" /><path d="M10 20a2 2 0 004 0" /></Svg>;
export const IconSearch = (p: P) => <Svg {...p}><circle cx="11" cy="11" r="7" /><path d="M20 20l-3.5-3.5" /></Svg>;
export const IconWallet = (p: P) => <Svg {...p}><path d="M4 7a2 2 0 012-2h11v4" /><rect x="3" y="7" width="18" height="13" rx="3" /><circle cx="16.5" cy="13.5" r="1.2" /></Svg>;
export const IconSend = (p: P) => <Svg {...p}><path d="M7 17L17 7M9 7h8v8" /></Svg>;
export const IconReceive = (p: P) => <Svg {...p}><path d="M12 4v13M6 12l6 6 6-6" /></Svg>;
export const IconBuy = (p: P) => <Svg {...p}><circle cx="12" cy="12" r="9" /><path d="M12 8v8M8 12h8" /></Svg>;
export const IconSwap = (p: P) => <Svg {...p}><path d="M4 8h14l-3-3M20 16H6l3 3" /></Svg>;
export const IconClock = (p: P) => <Svg {...p}><circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 2" /></Svg>;
export const IconTimeline = (p: P) => <Svg {...p}><path d="M6 3v18" /><circle cx="6" cy="6" r="2" /><circle cx="6" cy="12" r="2" /><circle cx="6" cy="18" r="2" /><path d="M11 6h9M11 12h6M11 18h9" /></Svg>;
export const IconUser = (p: P) => <Svg {...p}><circle cx="12" cy="8" r="4" /><path d="M4 21c0-4 3.5-7 8-7s8 3 8 7" /></Svg>;
export const IconMenu = (p: P) => <Svg {...p}><path d="M4 7h16M4 12h16M4 17h16" /></Svg>;
export const IconClose = (p: P) => <Svg {...p}><path d="M6 6l12 12M18 6L6 18" /></Svg>;
export const IconLock = (p: P) => <Svg {...p}><rect x="5" y="11" width="14" height="9" rx="2" /><path d="M8 11V8a4 4 0 018 0v3" /></Svg>;
