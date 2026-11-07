import Link from "next/link";
import { Logo } from "@/components/trace/ui";

export const metadata = { title: "Privacy" };

export default function PrivacyPage() {
  return (
    <main className="legal">
      <Link href="/"><Logo /></Link>
      <h1>Privacy</h1>
      <p className="lab">Last updated 7 November 2026</p>
      <p>trace is a research product. It never asks for private keys, never holds funds and never signs transactions. Your own NEAR wallet signs every copy mandate.</p>
      <h2>Account data</h2>
      <p>When you create an account with a passkey, trace stores your username, display name, the passkey&apos;s public key and opaque session records. It also stores the watchlists, alert preferences, alerts, paper-only Shadow tests, NEAR mandate drafts and agent connections you create.</p>
      <h2>Blockchain and market data</h2>
      <p>Wallet addresses and transactions on public blockchains are public. trace processes them to provide research, alerts and simulations. trace cannot remove public blockchain records from their networks.</p>
      <h2>Example and unavailable data</h2>
      <p>Example data is labelled “Example” and is never treated as live. Missing information is shown as unavailable. Market data can be delayed, partial or wrong and is not financial advice.</p>
      <h2>Delete your account</h2>
      <p>Signed-in users can sign out of every device from <Link href="/app/security/">Security and devices</Link>. Account deletion removes your account and all data keyed to it. A minimal audit record of the deletion is kept.</p>
      <h2>Before public launch</h2>
      <p>A privacy contact, the production service providers, regional rights and exact retention periods must be added before public launch. This page is not a substitute for legal review.</p>
    </main>
  );
}
