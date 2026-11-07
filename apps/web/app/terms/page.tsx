import Link from "next/link";
import { Logo } from "@/components/trace/ui";

export const metadata = { title: "Terms" };

export default function TermsPage() {
  return (
    <main className="legal">
      <Link href="/"><Logo /></Link>
      <h1>Terms</h1>
      <p className="lab">Last updated 7 November 2026</p>
      <h2>Research, not custody</h2>
      <p>trace provides research about public blockchain activity. It does not take custody of assets, sign transactions or submit trades. Swap quotes are dry previews from NEAR Intents. Copy limits are NEAR mandates that only your own wallet can sign, pause or revoke.</p>
      <h2>Not financial advice</h2>
      <p>Signals, scores, alerts, risk labels and market information are research tools, not financial, investment, legal or tax advice. Data can be delayed, partial, unavailable or incorrect.</p>
      <h2>Shadow is a simulation</h2>
      <p>Shadow tests are paper only. They do not move money. Simulated results are hypothetical and do not predict future performance.</p>
      <h2>Your account</h2>
      <p>You are responsible for activity under your sessions and for removing devices you do not recognise. Do not use trace to break laws, interfere with the service or harm other people.</p>
      <h2>Before public launch</h2>
      <p>Governing law, dispute terms, contact details, age requirements and regional consumer terms must be added after legal review. See <Link href="/privacy/">Privacy</Link>.</p>
    </main>
  );
}
