import type { Chain } from "./api/types";

const EVM_RE = /^0x[0-9a-fA-F]{40}$/;
const SOLANA_RE = /^[1-9A-HJ-NP-Za-km-z]{32,44}$/;

export function isEvmAddress(value: string): boolean {
  return EVM_RE.test(value);
}

/** Format check only (base58 alphabet, 32 to 44 chars). It does not prove the key exists. */
export function isSolanaAddress(value: string): boolean {
  return SOLANA_RE.test(value);
}

export function isEvmChain(chain: Chain): boolean {
  return chain !== "solana";
}

export type AddressCheck = { ok: true } | { ok: false; reason: string };

export function validateAddress(chain: Chain, raw: string): AddressCheck {
  const value = raw.trim();
  if (!value) return { ok: false, reason: "Enter a wallet address." };
  if (chain === "solana") {
    if (isSolanaAddress(value)) return { ok: true };
    return {
      ok: false,
      reason: "Solana addresses use 32 to 44 letters and numbers (no 0, O, I or l).",
    };
  }
  if (isEvmAddress(value)) return { ok: true };
  return { ok: false, reason: "This chain uses addresses that start with 0x followed by 40 hex characters." };
}

/** Which chains could an address belong to, judging by format alone. */
export function candidateChains(raw: string): Chain[] {
  const value = raw.trim();
  if (isEvmAddress(value)) return ["ethereum", "base", "bnb"];
  if (isSolanaAddress(value)) return ["solana"];
  return [];
}

export function isValidChain(value: string): value is Chain {
  return value === "solana" || value === "ethereum" || value === "base" || value === "bnb";
}

export const CHAIN_LABEL: Record<Chain, string> = {
  solana: "Solana",
  ethereum: "Ethereum",
  base: "Base",
  bnb: "BNB Chain",
};
