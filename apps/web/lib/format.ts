type Num = string | number | null | undefined;

function toNumber(value: Num): number | null {
  if (value === null || value === undefined || value === "") return null;
  const n = typeof value === "number" ? value : Number(value);
  return Number.isFinite(n) ? n : null;
}

export const NA = "n/a";

/** Compact USD: $12.4M, $8.4K, $1.23B. Small prices keep significant digits. */
export function formatCompactUsd(value: Num): string {
  const n = toNumber(value);
  if (n === null) return NA;
  const abs = Math.abs(n);
  const sign = n < 0 ? "-" : "";
  const units: Array<[number, string]> = [
    [1e12, "T"],
    [1e9, "B"],
    [1e6, "M"],
    [1e3, "K"],
  ];
  for (const [size, suffix] of units) {
    if (abs >= size) {
      const scaled = abs / size;
      const digits = scaled >= 100 ? 0 : scaled >= 10 ? 1 : 2;
      return `${sign}$${trimZeros(scaled.toFixed(digits))}${suffix}`;
    }
  }
  return `${sign}$${trimZeros(abs.toFixed(2))}`;
}

function trimZeros(s: string): string {
  if (!s.includes(".")) return s;
  return s.replace(/0+$/, "").replace(/\.$/, "");
}

/** Token price: more precision for sub-dollar values. */
export function formatPriceUsd(value: Num): string {
  const n = toNumber(value);
  if (n === null) return NA;
  const abs = Math.abs(n);
  if (abs === 0) return "$0";
  if (abs >= 1000) return `$${n.toLocaleString("en-US", { maximumFractionDigits: 2 })}`;
  if (abs >= 1) return `$${trimZeros(n.toFixed(2))}`;
  if (abs >= 0.01) return `$${trimZeros(n.toFixed(3))}`;
  // significant digits for tiny prices
  return `$${trimZeros(n.toPrecision(2))}`.replace(/e-(\d+)/, "e-$1");
}

/** Exact money with two decimals: $148.60 */
export function formatUsd(value: Num): string {
  const n = toNumber(value);
  if (n === null) return NA;
  const sign = n < 0 ? "-" : "";
  return `${sign}$${Math.abs(n).toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })}`;
}

/** Signed percent from a percent value (not a ratio): "42.7" -> "+42.7%". */
export function formatPct(value: Num, digits = 1): string {
  const n = toNumber(value);
  if (n === null) return NA;
  const rounded = Number(n.toFixed(digits));
  const sign = rounded > 0 ? "+" : "";
  const body = Math.abs(rounded) >= 1000
    ? rounded.toLocaleString("en-US", { maximumFractionDigits: digits })
    : rounded.toFixed(digits);
  return `${sign}${body}%`;
}

/** Unsigned percent, used for win rate and coverage: "78" -> "78%". */
export function formatPlainPct(value: Num, digits = 0): string {
  const n = toNumber(value);
  if (n === null) return NA;
  return `${n.toFixed(digits)}%`;
}

/** Coverage ratio is a 0..1 ratio in the API: "0.92" -> "92%". */
export function formatRatioAsPct(value: Num): string {
  const n = toNumber(value);
  if (n === null) return NA;
  return `${Math.round(n * 100)}%`;
}

export function pctTone(value: Num): "up" | "down" | "flat" {
  const n = toNumber(value);
  if (n === null || n === 0) return "flat";
  return n > 0 ? "up" : "down";
}

export function formatInteger(value: Num): string {
  const n = toNumber(value);
  if (n === null) return NA;
  return Math.round(n).toLocaleString("en-US");
}

const MINUTE = 60;
const HOUR = 3600;
const DAY = 86400;

/** "just now", "3 minutes ago", "2 hours ago", "5 days ago". */
export function formatRelativeTime(iso: string | null | undefined, now: Date | number = Date.now()): string {
  if (!iso) return NA;
  const t = Date.parse(iso);
  if (Number.isNaN(t)) return NA;
  const nowMs = typeof now === "number" ? now : now.getTime();
  const diff = Math.round((nowMs - t) / 1000);
  if (diff < -MINUTE) {
    const ahead = -diff;
    if (ahead < HOUR) return `in ${plural(Math.round(ahead / MINUTE), "minute")}`;
    if (ahead < DAY) return `in ${plural(Math.round(ahead / HOUR), "hour")}`;
    return `in ${plural(Math.round(ahead / DAY), "day")}`;
  }
  if (diff < 45) return "just now";
  if (diff < HOUR) return `${plural(Math.max(1, Math.round(diff / MINUTE)), "minute")} ago`;
  if (diff < DAY) return `${plural(Math.round(diff / HOUR), "hour")} ago`;
  return `${plural(Math.round(diff / DAY), "day")} ago`;
}

function plural(n: number, unit: string): string {
  return `${n} ${unit}${n === 1 ? "" : "s"}`;
}

/** Short UTC clock time, "12:10 UTC". */
export function formatClockUtc(iso: string | null | undefined): string {
  if (!iso) return NA;
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return NA;
  const hh = String(d.getUTCHours()).padStart(2, "0");
  const mm = String(d.getUTCMinutes()).padStart(2, "0");
  return `${hh}:${mm} UTC`;
}

/** "7xA2...9kPz". Short strings are returned unchanged. */
export function shortenAddress(address: string, head = 4, tail = 4): string {
  if (!address) return "";
  if (address.length <= head + tail + 3) return address;
  return `${address.slice(0, head)}...${address.slice(-tail)}`;
}

export function walletLabel(w: { alias: string | null; address: string }): string {
  return w.alias && w.alias.trim() ? w.alias : shortenAddress(w.address);
}

export function tokenLabel(t: { symbol: string | null; name: string | null; address: string }): string {
  return t.symbol ?? t.name ?? shortenAddress(t.address);
}
