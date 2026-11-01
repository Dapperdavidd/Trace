import type { Metadata } from "next";
import { ComingLater } from "@/components/ComingLater";

export const metadata: Metadata = { title: "Wallet (coming later)" };

export default function Page() {
  return <ComingLater title="Wallet" why="A GiFTED! wallet that holds and manages your assets is not built yet. For now you can watch any public address and follow what it does." />;
}
