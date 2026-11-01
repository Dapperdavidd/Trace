import type { Metadata } from "next";
import { ComingLater } from "@/components/ComingLater";

export const metadata: Metadata = { title: "Swap (coming later)" };

export default function Page() {
  return <ComingLater title="Swap" why="Swapping needs a connected wallet that can sign transactions. GiFTED! cannot do that yet. Shadow Trades lets you test ideas with paper money instead." />;
}
