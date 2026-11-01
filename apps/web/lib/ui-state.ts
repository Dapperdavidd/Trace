import type { DataStatus } from "./api/types";

export const STATUS_LABEL: Record<DataStatus, string> = {
  live: "Live",
  stale: "Stale",
  unavailable: "Data unavailable",
  partial: "Partial data",
  demo: "DEMO DATA",
};

export const DEFAULT_EMPTY_TITLE = "Nothing to show yet";
export const UNAVAILABLE_TITLE = "Data unavailable";
export const UNAVAILABLE_EXPLANATION =
  "The data source did not return usable numbers, so TRACE shows nothing here instead of guessing.";
