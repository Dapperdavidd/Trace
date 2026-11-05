import Link from "next/link";
import { PageHead, Panel } from "@/components/ui";

export default function TermsPage() {
  return (
    <div className="stack legal-page">
      <PageHead title="Terms" sub="Important limits for using TRACE." />
      <Panel title="Product terms summary" sub="Last updated 5 November 2026">
        <div className="legal-copy">
          <h2>Watch-only research</h2>
          <p>TRACE provides research about public blockchain activity. It does not provide a spendable wallet, take custody of assets, sign transactions, or place trades. Wallet, Send, Buy, and Swap actions are not available in this version.</p>
          <h2>Not financial advice</h2>
          <p>Signals, scores, alerts, risk labels, and market information are research tools, not financial, investment, legal, or tax advice. Data can be delayed, partial, unavailable, or incorrect. You are responsible for independently checking information before making decisions.</p>
          <h2>Shadow is a simulation</h2>
          <p>Shadow is paper-only. It does not copy a trader or move money. Simulated results are hypothetical, can use incomplete data, and do not predict future performance.</p>
          <h2>Your account</h2>
          <p>You are responsible for activity under your sessions and for revoking devices you do not recognise. Do not use TRACE to break laws, interfere with the service, bypass access controls, or harm other people.</p>
          <h2>Availability and changes</h2>
          <p>Features and coverage can change, and unsupported chains or protocols must not be treated as covered. See the status shown in the product for current coverage. TRACE may be unavailable during maintenance or failures.</p>
          <h2>Before public launch</h2>
          <p>Dapper must obtain legal review and add governing law, dispute terms, contact details, age requirements, and any regional consumer terms before public launch. See the <Link href="/privacy">Privacy page</Link> for the current data summary.</p>
        </div>
      </Panel>
    </div>
  );
}
