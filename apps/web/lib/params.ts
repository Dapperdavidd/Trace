import type { Chain } from "./api/types";
import { isValidChain, validateAddress } from "./address";

export function parseChainAddress(
  chain: string | string[] | undefined,
  address: string | string[] | undefined,
): { ok: true; chain: Chain; address: string } | { ok: false; reason: string } {
  const c = Array.isArray(chain) ? chain[0] : chain;
  const aRaw = Array.isArray(address) ? address[0] : address;
  if (!c || !isValidChain(c)) return { ok: false, reason: "This chain is not supported. GiFTED! lists Solana, Ethereum, Base and BNB Chain, but protocol coverage varies." };
  let a = aRaw ?? "";
  try {
    a = decodeURIComponent(a);
  } catch {
    /* keep raw */
  }
  const check = validateAddress(c, a);
  if (!check.ok) return { ok: false, reason: check.reason };
  return { ok: true, chain: c, address: a.trim() };
}
