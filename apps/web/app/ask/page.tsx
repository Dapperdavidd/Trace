import type { Metadata } from "next";
import { ComingLater } from "@/components/ComingLater";

export const metadata: Metadata = { title: "Ask TRACE (coming later)" };

export default function Page() {
  return <ComingLater title="Ask TRACE" why="Plain-language questions to an AI research assistant are planned for later. Today you can browse Top Movers, Smart Wallets and Risk Radar." />;
}
