"use client";

import { useEffect, useRef, useState } from "react";
import { api, describeError } from "@/lib/api/client";

interface GsiId {
  initialize(cfg: { client_id: string; callback: (r: { credential: string }) => void }): void;
  renderButton(el: HTMLElement, opts: Record<string, string | number>): void;
}
declare global {
  interface Window {
    google?: { accounts: { id: GsiId } };
  }
}

export const GOOGLE_CLIENT_ID = process.env.NEXT_PUBLIC_GOOGLE_CLIENT_ID ?? "";
const SRC = "https://accounts.google.com/gsi/client";

/** Google Identity Services button. */
export function GoogleButton({ onSignedIn }: { onSignedIn: () => void }) {
  const holder = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  const done = useRef(onSignedIn);
  done.current = onSignedIn;

  useEffect(() => {
    if (!GOOGLE_CLIENT_ID) return;
    let cancelled = false;
    const init = () => {
      if (cancelled || !window.google || !holder.current) return;
      window.google.accounts.id.initialize({
        client_id: GOOGLE_CLIENT_ID,
        callback: async (resp) => {
          setError(null);
          try {
            await api.authGoogle(resp.credential);
            done.current();
          } catch (e) {
            setError(describeError(e).message);
          }
        },
      });
      const width = Math.min(400, holder.current.getBoundingClientRect().width || 320);
      window.google.accounts.id.renderButton(holder.current, { theme: "outline", size: "large", text: "signin_with", shape: "pill", width });
    };
    if (window.google) init();
    else {
      let script = document.querySelector<HTMLScriptElement>(`script[src="${SRC}"]`);
      if (!script) {
        script = document.createElement("script");
        script.src = SRC;
        script.async = true;
        document.head.appendChild(script);
      }
      script.addEventListener("load", init);
      script.addEventListener("error", () => !cancelled && setError("Google sign-in could not load."));
    }
    return () => {
      cancelled = true;
    };
  }, []);

  if (!GOOGLE_CLIENT_ID) {
    return (
      <button
        type="button"
        className="google-unconfigured"
        onClick={() => setError("Google sign-in is not configured for this site yet.")}
      >
        <span aria-hidden="true">G</span>
        Sign in with Google
        <style jsx>{`
          .google-unconfigured {
            width: 100%;
            min-height: 62px;
            border: 0;
            border-radius: 999px;
            background: #fff;
            color: #101116;
            font: 600 18px/1 system-ui, sans-serif;
            cursor: pointer;
          }
          .google-unconfigured span { color: #4285f4; font-size: 25px; margin-right: 14px; }
        `}</style>
        {error ? <span className="err">{error}</span> : null}
      </button>
    );
  }
  return (
    <div style={{ display: "grid", gap: 6 }}>
      <div ref={holder} style={{ minHeight: 44, display: "flex", justifyContent: "center" }} />
      {error ? <p className="err">{error}</p> : null}
    </div>
  );
}
