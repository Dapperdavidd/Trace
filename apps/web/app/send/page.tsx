import type { Metadata } from "next";
import { ComingLater } from "@/components/ComingLater";

export const metadata: Metadata = { title: "Send (coming later)" };

export default function Page() {
  return <ComingLater title="Send" why="Sending crypto needs a secure wallet and transaction signing. GiFTED! does not offer either yet, so nothing can be sent from here." />;
}
