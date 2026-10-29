import { describe, expect, it } from "vitest";
import {
  formatClockUtc,
  formatCompactUsd,
  formatPct,
  formatPlainPct,
  formatPriceUsd,
  formatRatioAsPct,
  formatRelativeTime,
  formatUsd,
  pctTone,
  shortenAddress,
  walletLabel,
} from "@/lib/format";

describe("formatCompactUsd", () => {
  it("compacts thousands, millions and billions", () => {
    expect(formatCompactUsd("8400")).toBe("$8.4K");
    expect(formatCompactUsd("12400000")).toBe("$12.4M");
    expect(formatCompactUsd(3_100_000)).toBe("$3.1M");
    expect(formatCompactUsd("1230000000")).toBe("$1.23B");
  });
  it("keeps small values and negatives", () => {
    expect(formatCompactUsd("4.2")).toBe("$4.2");
    expect(formatCompactUsd("-4800000")).toBe("-$4.8M");
  });
  it("returns n/a for missing or invalid values", () => {
    expect(formatCompactUsd(null)).toBe("n/a");
    expect(formatCompactUsd(undefined)).toBe("n/a");
    expect(formatCompactUsd("abc")).toBe("n/a");
    expect(formatCompactUsd("")).toBe("n/a");
  });
});

describe("formatPct", () => {
  it("adds an explicit plus sign for gains", () => {
    expect(formatPct("42.7")).toBe("+42.7%");
    expect(formatPct(12.34, 1)).toBe("+12.3%");
  });
  it("keeps the minus sign for losses and no sign for zero", () => {
    expect(formatPct("-3.1")).toBe("-3.1%");
    expect(formatPct("0")).toBe("0.0%");
  });
  it("groups large values", () => {
    expect(formatPct("1324", 0)).toBe("+1,324%");
  });
  it("handles null", () => {
    expect(formatPct(null)).toBe("n/a");
  });
});

describe("other number helpers", () => {
  it("formats exact USD", () => {
    expect(formatUsd("148.6")).toBe("$148.60");
    expect(formatUsd("-4.2")).toBe("-$4.20");
  });
  it("formats token prices by magnitude", () => {
    expect(formatPriceUsd("0.0084")).toBe("$0.0084");
    expect(formatPriceUsd("0.132")).toBe("$0.132");
    expect(formatPriceUsd("0.21")).toBe("$0.21");
    expect(formatPriceUsd("1248.21")).toBe("$1,248.21");
  });
  it("formats plain percent and ratios", () => {
    expect(formatPlainPct("78")).toBe("78%");
    expect(formatRatioAsPct("0.916")).toBe("92%");
    expect(formatRatioAsPct(null)).toBe("n/a");
  });
  it("reports tone", () => {
    expect(pctTone("4")).toBe("up");
    expect(pctTone("-4")).toBe("down");
    expect(pctTone(null)).toBe("flat");
  });
});

describe("formatRelativeTime", () => {
  const now = Date.parse("2026-10-28T12:00:00Z");
  it("handles minutes, hours and days", () => {
    expect(formatRelativeTime("2026-10-28T11:57:00Z", now)).toBe("3 minutes ago");
    expect(formatRelativeTime("2026-10-28T11:59:00Z", now)).toBe("1 minute ago");
    expect(formatRelativeTime("2026-10-28T10:00:00Z", now)).toBe("2 hours ago");
    expect(formatRelativeTime("2026-10-23T12:00:00Z", now)).toBe("5 days ago");
  });
  it("says just now for very recent and handles future or bad input", () => {
    expect(formatRelativeTime("2026-10-28T11:59:40Z", now)).toBe("just now");
    expect(formatRelativeTime("2026-10-28T13:00:00Z", now)).toBe("in 1 hour");
    expect(formatRelativeTime("nope", now)).toBe("n/a");
    expect(formatRelativeTime(null, now)).toBe("n/a");
  });
  it("formats a UTC clock time", () => {
    expect(formatClockUtc("2026-10-28T12:10:00Z")).toBe("12:10 UTC");
  });
});

describe("addresses", () => {
  it("shortens long addresses and leaves short ones", () => {
    expect(shortenAddress("7xA2Kq9mQvB3dFhT8rLwYpN4cZsJ6uEgVoR1tXq5Wb9k")).toBe("7xA2...Wb9k");
    expect(shortenAddress("0xabc")).toBe("0xabc");
    expect(shortenAddress("")).toBe("");
  });
  it("prefers the alias for wallet labels", () => {
    expect(walletLabel({ alias: "0xSun", address: "7xA2Kq9mQvB3dFhT8rLwYpN4cZsJ6uEgVoR1tXq5Wb9k" })).toBe("0xSun");
    expect(walletLabel({ alias: null, address: "7xA2Kq9mQvB3dFhT8rLwYpN4cZsJ6uEgVoR1tXq5Wb9k" })).toBe("7xA2...Wb9k");
  });
});
