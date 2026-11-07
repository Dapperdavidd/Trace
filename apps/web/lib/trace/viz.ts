/* Drawing and formatting helpers ported from design/trace-redesign-v11.html. */

export function rng(seed: number): () => number {
  let s = seed;
  return () => {
    s |= 0;
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export function walk(seed: number, n: number, drift: number, shape?: "breakout" | null): number[] {
  const r = rng(seed * 9973 + 7);
  let v = 0;
  const a: number[] = [];
  for (let i = 0; i < n; i++) {
    let d = drift;
    if (shape === "breakout") {
      const f = i / n;
      d = f < 0.34 ? 0.03 : f < 0.5 ? 1.1 : 0.3;
    }
    v += d + (r() - 0.5) * 2;
    a.push(v);
  }
  const mx = Math.max(...a);
  a[n - 1] = Math.max(a[n - 1]!, mx + 0.3);
  return a;
}

export function rescale(a: number[], t0: number, t1: number): number[] {
  const v0 = a[0]!;
  const v1 = a[a.length - 1]!;
  return a.map((v) => t0 + ((v - v0) / (v1 - v0 || 1)) * (t1 - t0));
}

/** Resample any numeric series to n points (linear). */
export function resample(a: number[], n: number): number[] {
  if (a.length === 0) return [];
  if (a.length === 1) return Array.from({ length: n }, () => a[0]!);
  return Array.from({ length: n }, (_, i) => {
    const x = (i / (n - 1)) * (a.length - 1);
    const lo = Math.floor(x);
    const hi = Math.min(a.length - 1, lo + 1);
    return a[lo]! + (a[hi]! - a[lo]!) * (x - lo);
  });
}

export const clamp = (v: number, a: number, b: number) => Math.max(a, Math.min(b, v));
export function ease(t: number): number {
  t = clamp(t, 0, 1);
  return t < 0.5 ? 2 * t * t : 1 - Math.pow(-2 * t + 2, 2) / 2;
}

export function sgn(n: number, dp = 1): string {
  return (n > 0 ? "+" : n < 0 ? "−" : "") + Math.abs(n).toFixed(dp);
}
export function money(n: number): string {
  return (
    (n < 0 ? "−" : "") +
    "$" +
    Math.abs(n).toLocaleString("en-US", { minimumFractionDigits: 2, maximumFractionDigits: 2 })
  );
}
export function pf(v: number): string {
  return "$" + (v < 0.01 ? v.toFixed(4) : v < 1 ? v.toFixed(3) : v.toFixed(2));
}
/** Minutes after 09:00 to "HH:MM". */
export function hm(t: number): string {
  const m = 540 + Math.round(t);
  return String(Math.floor(m / 60)).padStart(2, "0") + ":" + String(m % 60).padStart(2, "0");
}
export const cl = (n: number) => (n >= 0 ? "g" : "f2");

export function compactUsd(v: number | null | undefined): string {
  if (v === null || v === undefined || !Number.isFinite(v)) return "—";
  const a = Math.abs(v);
  if (a >= 1e9) return "$" + (v / 1e9).toFixed(1) + "B";
  if (a >= 1e6) return "$" + (v / 1e6).toFixed(1) + "M";
  if (a >= 1e3) return "$" + (v / 1e3).toFixed(1) + "K";
  return "$" + v.toFixed(a < 1 ? 4 : 0);
}

export function num(v: string | number | null | undefined): number | null {
  if (v === null || v === undefined || v === "") return null;
  const n = typeof v === "number" ? v : Number(v);
  return Number.isFinite(n) ? n : null;
}

export function shortAddr(a: string): string {
  return a.length > 12 ? `${a.slice(0, 4)}…${a.slice(-3)}` : a;
}

export function seedOf(s: string): number {
  let h = 0;
  for (let i = 0; i < s.length; i++) h = (Math.imul(h, 31) + s.charCodeAt(i)) | 0;
  return Math.abs(h % 97) + 1;
}

export function sparkSvg(values: number[], up = true): string {
  const a = values.length > 1 ? values : [0, 0];
  const mn = Math.min(...a);
  const mx = Math.max(...a);
  const n = a.length - 1;
  const d = a
    .map((v, i) => (i ? "L" : "M") + ((i / n) * 56).toFixed(1) + " " + (2 + (1 - (v - mn) / (mx - mn || 1)) * 18).toFixed(1))
    .join("");
  return `<svg class="spark" viewBox="0 0 56 22" aria-hidden="true"><path d="${d}" fill="none" stroke="${up ? "#4AE38F" : "rgba(237,242,239,.5)"}" stroke-width="1.3"/></svg>`;
}
export function sparkSeed(seed: number, up = true): string {
  return sparkSvg(walk(seed, 22, up ? 0.35 : -0.3), up);
}

export interface ChartSet {
  v: number[];
  c?: string;
  w?: number;
  dash?: boolean;
}

export function chart2Svg(sets: ChartSet[], opt: { base?: number } = {}): string {
  const all = sets.flatMap((s) => s.v);
  const mn = Math.min(...all);
  const mx = Math.max(...all);
  const W = 600;
  const H = 200;
  const p = 8;
  const X = (i: number, n: number) => (i / (n - 1)) * W;
  const Y = (v: number) => p + (1 - (v - mn) / (mx - mn || 1)) * (H - 2 * p);
  let g = "";
  for (let k = 0; k < 4; k++) {
    const y = p + ((H - 2 * p) * k) / 3;
    g += `<line x1="0" x2="${W}" y1="${y}" y2="${y}" stroke="rgba(237,242,239,.07)" vector-effect="non-scaling-stroke"/>`;
  }
  if (opt.base !== undefined) {
    const by = Y(opt.base);
    g += `<line x1="0" x2="${W}" y1="${by}" y2="${by}" stroke="rgba(237,242,239,.22)" stroke-dasharray="3 4" vector-effect="non-scaling-stroke"/>`;
  }
  const paths = sets
    .map((s) => {
      const n = s.v.length;
      return `<path d="${s.v.map((v, i) => (i ? "L" : "M") + X(i, n).toFixed(1) + " " + Y(v).toFixed(1)).join("")}" fill="none" stroke="${s.c || "#4AE38F"}" stroke-width="${s.w || 1.6}"${s.dash ? ' stroke-dasharray="4 4"' : ""} vector-effect="non-scaling-stroke"/>`;
    })
    .join("");
  const first = sets[0]!.v;
  const ey = ((Y(first[first.length - 1]!) / H) * 100).toFixed(1);
  return `<div class="chart2"><svg viewBox="0 0 ${W} ${H}" preserveAspectRatio="none" aria-hidden="true">${g}${paths}</svg><span class="endm" style="top:${ey}%"></span></div>`;
}
