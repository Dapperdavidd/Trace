import { GoogleSignIn } from "@/components/GoogleSignIn";
import { PageHead, Panel } from "@/components/ui";

export default function SignInPage() {
  return (
    <div className="stack" style={{ maxWidth: 560 }}>
      <PageHead title="Sign in" sub="Save your watchlist and alert settings across web and mobile." />
      <Panel>
        <GoogleSignIn />
        <p className="muted" style={{ fontSize: 12.5, marginTop: 16 }}>
          Signing in with Google identifies your TRACE account. It is not a wallet. TRACE never asks for private keys or seed phrases, and a signed-in account has no spendable wallet.
        </p>
      </Panel>
    </div>
  );
}
