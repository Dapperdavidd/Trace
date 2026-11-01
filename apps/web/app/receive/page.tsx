import type { Metadata } from "next";
import { ComingLater } from "@/components/ComingLater";

export const metadata: Metadata = { title: "Receive (coming later)" };

export default function Page() {
  return <ComingLater title="Receive" why="Receiving crypto needs a GiFTED! wallet address, and GiFTED! does not create wallets yet. You can still watch any public address." />;
}
