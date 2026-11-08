"use client";

import { useRouter, useSearchParams } from "next/navigation";
import { GoogleButton } from "./GoogleButton";

/** Google-only onboarding shared with the mobile visual direction. */
export function Welcome() {
  const router = useRouter();
  const params = useSearchParams();
  const next = params.get("next") || "/app/today/";

  return (
    <main className="trace-signin">
      <div className="trace-signin__glow" aria-hidden="true" />
      <div className="trace-signin__horizon" aria-hidden="true" />
      <section className="trace-signin__content" aria-labelledby="signin-title">
        {/* eslint-disable-next-line @next/next/no-img-element */}
        <img className="trace-signin__logo" src="/trace-ribbon-logo.png" alt="Trace" />
        <h1 id="signin-title">
          Look inside<br />the chain<span>.</span>
        </h1>
        <p>Trace, analyze, and understand<br />onchain activity.</p>
        <div className="trace-signin__action">
          <GoogleButton onSignedIn={() => router.push(next)} />
        </div>
      </section>
      <style jsx>{`
        .trace-signin {
          position: fixed;
          inset: 0;
          overflow: hidden;
          color: #f8f8fb;
          background: radial-gradient(circle at 65% 8%, #12152c 0, #070812 42%, #03040a 100%);
        }
        .trace-signin__glow {
          position: absolute;
          inset: 0;
          background: radial-gradient(circle at 50% 61%, rgba(47, 74, 255, .18), transparent 29%);
        }
        .trace-signin__horizon {
          position: absolute;
          left: 50%;
          top: 61%;
          width: min(1500px, 180vw);
          aspect-ratio: 1;
          transform: translateX(-50%);
          border: 1px solid #6284ff;
          border-radius: 50%;
          background: radial-gradient(circle at 50% 3%, #263578 0, #0e132d 29%, #050611 64%);
          box-shadow: 0 -4px 44px rgba(31, 65, 255, .9), 0 -20px 120px rgba(50, 90, 255, .55);
        }
        .trace-signin__content {
          position: relative;
          z-index: 1;
          width: min(100% - 48px, 560px);
          min-height: 100dvh;
          margin: 0 auto;
          display: flex;
          flex-direction: column;
          align-items: center;
          text-align: center;
          padding: max(36px, 5vh) 0 34px;
        }
        .trace-signin__logo {
          width: clamp(126px, 20vh, 190px);
          height: clamp(126px, 20vh, 190px);
          object-fit: contain;
          margin-top: clamp(20px, 7vh, 90px);
        }
        h1 {
          margin: clamp(22px, 4vh, 48px) 0 0;
          font-size: clamp(52px, 8vw, 82px);
          line-height: .97;
          letter-spacing: -.065em;
          font-weight: 650;
        }
        h1 span { color: #3968ff; }
        p {
          margin: 24px 0 0;
          color: #b6b8c3;
          font-size: clamp(18px, 2.3vw, 24px);
          line-height: 1.5;
          letter-spacing: -.02em;
        }
        .trace-signin__action {
          width: 100%;
          margin-top: auto;
        }
      `}</style>
    </main>
  );
}
