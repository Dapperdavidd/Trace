type Num = string | number | null | undefined;

type DecimalParts = { negative: boolean; coefficient: bigint; scale: number };

function decimalParts(value: Num): DecimalParts | null {
  if (value === null || value === undefined || value === "") return null;
  const raw = String(value).trim();
  const match = /^([+-]?)(\d+)(?:\.(\d+))?$/.exec(raw);
  if (!match) return null;
  const whole = match[2] ?? "0";
  const fraction = match[3] ?? "";
  return {
    negative: match[1] === "-" && !/^0+$/.test(`${whole}${fraction}`),
    coefficient: BigInt(`${whole}${fraction}`),
    scale: fraction.length,
  };
}

function pow10(n: number): bigint {
  return 10n ** BigInt(n);
}

function atLeast(parts: DecimalParts, whole: bigint): boolean {
  return parts.coefficient >= whole * pow10(parts.scale);
}

function atLeastRatio(parts: DecimalParts, numerator: bigint, denominator: bigint): boolean {
  return parts.coefficient * denominator >= numerator * pow10(parts.scale);
}

function roundedScaled(parts: DecimalParts, divisor: bigint, decimals: number): bigint {
  const numerator = parts.coefficient * pow10(decimals);
  const denominator = pow10(parts.scale) * divisor;
  return (numerator + denominator / 2n) / denominator;
}

function grouped(s: string): string {
  return s.replace(/\B(?=(\d{3})+(?!\d))/g, ",");
}

function renderScaled(value: bigint, decimals: number, trim: boolean, group = false): string {
  const raw = value.toString().padStart(decimals + 1, "0");
  const whole = decimals === 0 ? raw : raw.slice(0, -decimals);
  let fraction = decimals === 0 ? "" : raw.slice(-decimals);
  if (trim) fraction = fraction.replace(/0+$/, "");
  return `${group ? grouped(whole) : whole}${fraction ? `.${fraction}` : ""}`;
}

function toNumber(value: Num): number | null {
  if (value === null || value === undefined || value === "") return null;
  const n = typeof value === "number" ? value : Number(value);
  return Number.isFinite(n) ? n : null;
}

export const NA = "Unavailable";

/** Compact USD: $12.4M, $8.4K, $1.23B. Small prices keep significant digits. */
export function formatCompactUsd(value: Num): string {
  const parts = decimalParts(value);
  if (!parts) return NA;
  const sign = parts.negative ? "-" : "";
  const units: Array<[bigint, string]> = [
    [1_000_000_000_000n, "T"],
    [1_000_000_000n, "B"],
    [1_000_000n, "M"],
    [1_000n, "K"],
  ];
  for (const [size, suffix] of units) {
    if (atLeast(parts, size)) {
      const decimals = atLeast(parts, size * 100n) ? 0 : atLeast(parts, size * 10n) ? 1 : 2;
      return `${sign}$${renderScaled(roundedScaled(parts, size, decimals), decimals, true)}${suffix}`;
    }
  }
  return `${sign}$${renderScaled(roundedScaled(parts, 1n, 2), 2, true)}`;
}

/** Token price: more precision for sub-dollar values. */
export function formatPriceUsd(value: Num): string {
  const parts = decimalParts(value);
  if (!parts) return NA;
  if (parts.coefficient === 0n) return "$0";
  const sign = parts.negative ? "-" : "";
  if (atLeast(parts, 1000n)) return `${sign}$${renderScaled(roundedScaled(parts, 1n, 2), 2, true, true)}`;
  if (atLeast(parts, 1n)) return `${sign}$${renderScaled(roundedScaled(parts, 1n, 2), 2, true)}`;
  const decimals = atLeastRatio(parts, 1n, 100n) ? 3 : Math.min(8, Math.max(4, parts.scale));
  return `${sign}$${renderScaled(roundedScaled(parts, 1n, decimals), decimals, true)}`;
}

/** Exact money with two decimals: $148.60 */
export function formatUsd(value: Num): string {
  const parts = decimalParts(value);
  if (!parts) return NA;
  const sign = parts.negative ? "-" : "";
  return `${sign}$${renderScaled(roundedScaled(parts, 1n, 2), 2, false, true)}`;
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
