import Link from "next/link";
import { PageHead, Panel } from "@/components/ui";

export default function PrivacyPage() {
  return (
    <div className="stack legal-page">
      <PageHead title="Privacy" sub="How TRACE handles account and public blockchain data." />
      <Panel title="Product privacy summary" sub="Last updated 5 November 2026">
        <div className="legal-copy">
          <p>TRACE is a watch-only research product. It does not ask for private keys, hold funds, sign transactions, or place trades.</p>
          <h2>Account data</h2>
          <p>When you sign in with Google, TRACE stores your Google account identifier, email address, display name, and opaque session records. TRACE also stores the watchlists, alert preferences, alerts, and paper-only Shadow strategies you create.</p>
          <h2>Blockchain and market data</h2>
          <p>Wallet addresses and transactions on supported public blockchains are public information. TRACE processes this information to provide research, alerts, and simulations. Public blockchain records are not private account data and cannot be removed from their networks by TRACE.</p>
          <h2>Demo and unavailable data</h2>
          <p>Demo information is visibly labelled and is not treated as live. Missing information is shown as unavailable. Market and blockchain information can be delayed, partial, or wrong, so it should not be treated as financial advice.</p>
          <h2>Delete your account</h2>
          <p>You can permanently delete your account from <Link href="/account">Account and sessions</Link>. This removes your active account and all data keyed to it, including sessions, watchlists, alert preferences, alerts, and Shadow strategies. A minimal audit record containing the deleted account ID and deletion action is retained.</p>
          <h2>Before public launch</h2>
          <p>Dapper must add a privacy contact, the production service providers, regional rights, and exact backup and log retention periods before public launch. This page describes the product as it exists today and is not a substitute for legal review.</p>
        </div>
      </Panel>
    </div>
  );
}
