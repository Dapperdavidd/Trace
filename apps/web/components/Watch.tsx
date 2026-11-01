"use client";

import Link from "next/link";
import type { WatchStatus, WatchlistItem } from "@/lib/api/types";
import { formatRelativeTime, shortenAddress, walletLabel } from "@/lib/format";
import { walletHref } from "./WalletList";
import { Avatar } from "./ui";

export const WATCH_STATUS_LABEL: Record<WatchStatus, string> = {
  active: "Monitoring",
  paused: "Paused",
  partial_monitoring: "Partly monitored",
  unsupported_protocol: "Protocol not supported yet",
  waiting_for_activity: "Waiting for activity",
};

export const WATCH_STATUS_HELP: Record<WatchStatus, string> = {
  active: "GiFTED! is checking this address for new buys and sells.",
  paused: "You paused alerts for this address.",
  partial_monitoring: "Only some of this address's activity is covered.",
  unsupported_protocol: "This address mostly uses a protocol GiFTED! cannot read yet.",
  waiting_for_activity: "No supported trades have been seen yet.",
};

export function WatchRow({ item, right }: { item: WatchlistItem; right?: React.ReactNode }) {
  const label = walletLabel(item.wallet);
  return (
    <li className="list-row">
      <Avatar seed={item.wallet.address} label={label} />
      <div className="grow">
        <Link href={walletHref(item.wallet)}><b className="ellip" style={{ display: "block" }}>{label}</b></Link>
        <small className="muted" style={{ display: "block" }}>
          {item.wallet.chain} <span className="mono">{shortenAddress(item.wallet.address)}</span>
        </small>
        <small className="muted" style={{ display: "block" }}>
          Checked {item.last_checked_at ? formatRelativeTime(item.last_checked_at) : "never"}
        </small>
      </div>
      <div className="right">
        <span className="badge b-neutral" title={WATCH_STATUS_HELP[item.status]}>{WATCH_STATUS_LABEL[item.status]}</span>
      </div>
      {right}
    </li>
  );
}
