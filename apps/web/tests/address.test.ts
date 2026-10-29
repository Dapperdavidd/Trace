import { describe, expect, it } from "vitest";
import { candidateChains, isEvmAddress, isSolanaAddress, validateAddress } from "@/lib/address";
import { fakeEvmAddress, fakeSolAddress, DEMO_TOKENS, DEMO_WALLET_REFS } from "@/lib/api/demo";

const EVM = "0x" + "a1B2c3D4e5".repeat(4);
const SOL = "7xA2Kq9mQvB3dFhT8rLwYpN4cZsJ6uEgVoR1tXq5Wb9k";

describe("address validation", () => {
  it("accepts EVM 0x plus 40 hex characters", () => {
    expect(isEvmAddress(EVM)).toBe(true);
    expect(validateAddress("base", EVM)).toEqual({ ok: true });
  });
  it("rejects bad EVM addresses", () => {
    expect(isEvmAddress("0x123")).toBe(false);
    expect(isEvmAddress("0x" + "g".repeat(40))).toBe(false);
    expect(isEvmAddress("a1B2c3D4e5".repeat(4) + "0000")).toBe(false);
    expect(validateAddress("ethereum", SOL).ok).toBe(false);
  });
  it("accepts base58 Solana addresses of 32 to 44 characters", () => {
    expect(isSolanaAddress(SOL)).toBe(true);
    expect(isSolanaAddress("1".repeat(32))).toBe(true);
    expect(isSolanaAddress("1".repeat(44))).toBe(true);
    expect(validateAddress("solana", SOL)).toEqual({ ok: true });
  });
  it("rejects Solana addresses with a bad length or alphabet", () => {
    expect(isSolanaAddress("1".repeat(31))).toBe(false);
    expect(isSolanaAddress("1".repeat(45))).toBe(false);
    expect(isSolanaAddress("0" + "1".repeat(40))).toBe(false); // zero is not base58
    expect(isSolanaAddress("O" + "1".repeat(40))).toBe(false);
    expect(isSolanaAddress("I" + "1".repeat(40))).toBe(false);
    expect(isSolanaAddress("l" + "1".repeat(40))).toBe(false);
    expect(validateAddress("solana", EVM).ok).toBe(false);
  });
  it("rejects empty input with a message", () => {
    const r = validateAddress("solana", "   ");
    expect(r.ok).toBe(false);
  });
  it("trims whitespace", () => {
    expect(validateAddress("solana", `  ${SOL}  `)).toEqual({ ok: true });
  });
  it("suggests candidate chains by format", () => {
    expect(candidateChains(EVM)).toEqual(["ethereum", "base", "bnb"]);
    expect(candidateChains(SOL)).toEqual(["solana"]);
    expect(candidateChains("hello")).toEqual([]);
  });
  it("demo fixtures use valid addresses", () => {
    for (const t of DEMO_TOKENS) expect(validateAddress(t.chain, t.address).ok).toBe(true);
    for (const w of DEMO_WALLET_REFS) expect(validateAddress(w.chain, w.address).ok).toBe(true);
    expect(isSolanaAddress(fakeSolAddress(1, "7a2", "9kP"))).toBe(true);
    expect(isEvmAddress(fakeEvmAddress(5))).toBe(true);
  });
});
