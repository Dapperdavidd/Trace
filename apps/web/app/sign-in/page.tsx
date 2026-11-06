import { GoogleSignIn } from "@/components/GoogleSignIn";
import { PasskeySignIn } from "@/components/PasskeySignIn";
import { PageHead, Panel } from "@/components/ui";

export default function SignInPage() {
  return (
    <div className="stack" style={{ maxWidth: 560 }}>
      <PageHead title="Sign in" sub="Use your device to protect one TRACE account across web and mobile." />
      <Panel>
        <PasskeySignIn />
        <div className="auth-divider" role="separator"><span>Alternative sign-in</span></div>
        <GoogleSignIn />
        <p className="muted" style={{ fontSize: 12.5, marginTop: 16 }}>
          Signing in identifies your TRACE account. It does not authorize trades. Your NEAR wallet separately approves agent mandates, and TRACE never asks for private keys or seed phrases.
        </p>
      </Panel>
    </div>
  );
}
