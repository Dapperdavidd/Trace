"use client";

import { memo, useEffect, useRef } from "react";
import { createRoot } from "react-dom/client";
import { D } from "@/lib/trace/example";
import { Field, has3D, isoField, type Engine } from "@/lib/trace/engine";
import { chart2Svg, clamp, rescale, rng, sgn, walk } from "@/lib/trace/viz";
import { api } from "@/lib/api/client";
import { LANDING_HTML } from "./landingMarkup";
import { MiniToday } from "./screens/MiniToday";

function tkHtml(s: string, on?: boolean) {
  const ch = ({ SOL: "S", USDC: "$", ETH: "Ξ" } as Record<string, string>)[s] || s[0];
  return `<span class="tk${on ? " on" : ""}">${ch}</span>`;
}

export function Landing() {
  const root = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = root.current;
    if (!el) return;
    const REDUCE = matchMedia("(prefers-reduced-motion: reduce)").matches;
    const cleanups: Array<() => void> = [];
    const $ = (id: string) => el.querySelector<HTMLElement>("#" + id);

    /* hero: 3D field */
    const peekTxt = $("peekTxt");
    const hero = $("heroField");
    let heroEng: Engine | null = null;
    if (hero) {
      if (has3D()) {
        heroEng = Field(hero, {
          seed: 7, cols: 26, rows: 14, fov: 40, lensR: 1.4, fogNear: 40, fogFar: 110, aspectFix: true, cardLift: 2.6,
          cardTop: () => {
            const t = el.querySelector(".Lfull-t .bset");
            return t && hero ? t.getBoundingClientRect().bottom - hero.getBoundingClientRect().top + 10 : 24;
          },
          keys: {}, hist: [], wash: [], chain: 0.9, autoOpen: 3,
          scenes: [{ wander: 1, focus: "center", dist: 1.75, lookY: 11.5 }], at: () => 0, wanderDur: 2.1,
          wanderPick: (b) => b.z > 1.5 && Math.abs(b.x) < 16,
          onPeek: (b) => { if (peekTxt) peekTxt.textContent = "Reading block #" + b.num.toLocaleString("en-US") + " · " + (120 + (b.num % 180)) + " trades"; },
          onClickBlock: (b) => { if (peekTxt) peekTxt.textContent = "Opened block #" + b.num.toLocaleString("en-US") + " · one trade inside"; },
        });
      } else hero.innerHTML = isoField(1200, 800);
    }
    cleanups.push(() => heroEng?.dispose());

    /* section visuals */
    const mb = $("mbChart");
    if (mb) {
      const a = walk(17, 48, 0.36);
      const mn = Math.min(...a);
      const mx = Math.max(...a);
      mb.innerHTML = '<svg viewBox="0 0 300 70" preserveAspectRatio="none"><path d="' + a.map((v, i) => (i ? "L" : "M") + ((i / 47) * 300).toFixed(1) + " " + (4 + (1 - (v - mn) / (mx - mn)) * 62).toFixed(1)).join("") + '" fill="none" stroke="#4AE38F" stroke-width="2" vector-effect="non-scaling-stroke"/></svg>';
    }
    const pm = $("pmChart");
    if (pm) pm.innerHTML = chart2Svg([{ v: rescale(walk(29, 30, 0.5), 100, 148.6) }, { v: rescale(walk(130, 30, 0.12), 100, 109.2), c: "rgba(237,242,239,.5)", dash: true, w: 1.3 }], { base: 100 });
    const wv = $("btWave");
    if (wv) {
      let h = "";
      for (let k = 0; k < 40; k++) h += '<i class="' + (k < 12 ? "p" : "") + '" style="height:' + (5 + Math.round(rng(k + 3)() * 18)) + 'px"></i>';
      wv.innerHTML = h;
    }
    const cw = $("cwr");
    if (cw)
      cw.innerHTML = ["NEURA", "SOLX", "FROG", "ORBIT"]
        .map((c) => {
          const d = D.coins[c]!;
          return "<div>" + tkHtml(c, c === "NEURA") + '<span class="grow"><b>' + c + "</b><small>" + d.why + '</small></span><em class="' + (d.risk === "High" ? "warn" : "") + '">' + (d.risk === "High" ? "Avoid" : sgn(d.ch) + "%") + "</em></div>";
        })
        .join("");
    const pmn = $("phMini");
    const phoneVp = pmn?.querySelector<HTMLElement>(".vp");
    if (phoneVp) {
      const mini = createRoot(phoneVp);
      mini.render(<MiniToday />);
      cleanups.push(() => setTimeout(() => mini.unmount(), 0));
    }

    /* trace your way: scroll picks the card */
    const sec = $("start");
    const slides = Array.from(el.querySelectorAll<HTMLElement>(".Ow-s"));
    const dots = Array.from(el.querySelectorAll<HTMLButtonElement>("#owDash button"));
    let cur = -1;
    const setSlide = (i: number) => {
      if (i === cur) return;
      cur = i;
      slides.forEach((s, k) => s.classList.toggle("on", k === i));
      dots.forEach((d, k) => {
        d.classList.toggle("on", k === i);
        d.setAttribute("aria-selected", k === i ? "true" : "false");
      });
    };
    const updOw = () => {
      if (!sec) return;
      if (innerWidth <= 820) return setSlide(0);
      const r = sec.getBoundingClientRect();
      const span = sec.offsetHeight - innerHeight;
      const p = clamp(-r.top / (span || 1), 0, 0.999);
      setSlide(Math.floor(p * slides.length));
    };
    dots.forEach((d, k) => {
      const fn = () => {
        if (!sec) return;
        const span = sec.offsetHeight - innerHeight;
        scrollTo({ top: sec.getBoundingClientRect().top + scrollY + (span * (k + 0.5)) / slides.length, behavior: REDUCE ? "auto" : "smooth" });
      };
      d.addEventListener("click", fn);
      cleanups.push(() => d.removeEventListener("click", fn));
    });

    /* set up in 5 steps: vertical scroll moves the track sideways */
    const su = $("setup") as (HTMLElement & { _extra?: number }) | null;
    const track = $("suTrack");
    const updSu = () => {
      if (!su || !track || innerWidth <= 820) return;
      const r = su.getBoundingClientRect();
      const span = su.offsetHeight - innerHeight;
      const p = clamp(-r.top / (span || 1), 0, 1);
      track.style.transform = "translateX(" + (-p * (su._extra || 0)).toFixed(1) + "px)";
    };
    const sizeSu = () => {
      if (!su || !track) return;
      if (innerWidth <= 820) {
        su.style.height = "";
        track.style.transform = "";
        return;
      }
      const extra = Math.max(0, track.scrollWidth - innerWidth + 40);
      su.style.height = innerHeight + extra + "px";
      su._extra = extra;
      updSu();
    };
    const onScroll = () => {
      updOw();
      updSu();
    };
    const onResize = () => {
      updOw();
      sizeSu();
    };
    addEventListener("scroll", onScroll, { passive: true });
    addEventListener("resize", onResize);
    cleanups.push(() => {
      removeEventListener("scroll", onScroll);
      removeEventListener("resize", onResize);
    });
    updOw();
    sizeSu();
    document.fonts?.ready.then(sizeSu).catch(() => undefined);

    /* email form: there is no mailing list yet, so say so */
    const form = $("hmail") as HTMLFormElement | null;
    const onSubmit = (e: Event) => {
      e.preventDefault();
      const v = ($("hmailIn") as HTMLInputElement | null)?.value.trim() ?? "";
      const n = $("hmailNote");
      if (n) n.textContent = /.+@.+\..+/.test(v) ? "Thanks. Download links are not ready yet, so we did not store " + v + ". The web app is open now." : "Type an email address, for example ayo@example.com.";
    };
    form?.addEventListener("submit", onSubmit);
    cleanups.push(() => form?.removeEventListener("submit", onSubmit));

    /* footer: real API status */
    const st = $("apiStatus");
    api
      .chains()
      .then(() => {
        if (st) st.innerHTML = '<i class="dot live"></i>trace API online';
      })
      .catch(() => {
        if (st) st.innerHTML = '<i class="dot"></i>trace API waking up';
      });

    return () => cleanups.forEach((f) => f());
  }, []);

  return <Markup innerRef={root} />;
}

/* Static markup from the design. Memoised so React never re-applies it over the live 3D and charts. */
const Markup = memo(function Markup({ innerRef }: { innerRef: React.RefObject<HTMLDivElement | null> }) {
  return <div ref={innerRef} suppressHydrationWarning dangerouslySetInnerHTML={{ __html: LANDING_HTML }} />;
});
