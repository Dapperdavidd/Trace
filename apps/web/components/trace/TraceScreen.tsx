"use client";

import { useApi, useAuthToken } from "@/hooks/useApi";
import { api } from "@/lib/api/client";
import { initials } from "./AppFrame";
import { AskScreen, BriefingScreen, CompareScreen, DiscoverScreen, TodayScreen, TokenScreen } from "./screens/Research";
import { CopyScreen, ProfileScreen, ShadowScreen, WalletsScreen } from "./screens/Wallets";
import { NearScreen, SwapScreen, WalletScreen } from "./screens/Act";
import { FollowingScreen, MeScreen, NotifsScreen, PrefsScreen, SecurityScreen } from "./screens/You";


function Today() {
  const token = useAuthToken();
  const me = useApi(() => api.me(), [token], !!token);
  return <TodayScreen meInitials={token ? initials(me.data?.name || me.data?.email) : undefined} />;
}

const MAP: Record<string, () => React.JSX.Element> = {
  today: Today,
  briefing: BriefingScreen,
  discover: DiscoverScreen,
  token: TokenScreen,
  compare: CompareScreen,
  ask: AskScreen,
  wallets: WalletsScreen,
  profile: ProfileScreen,
  shadow: ShadowScreen,
  copy: CopyScreen,
  swap: SwapScreen,
  wallet: WalletScreen,
  near: NearScreen,
  me: MeScreen,
  notifs: NotifsScreen,
  security: SecurityScreen,
  following: FollowingScreen,
  prefs: PrefsScreen,
};

export function TraceScreen({ screen }: { screen: string }) {
  const S = MAP[screen] ?? Today;
  return <S />;
}
