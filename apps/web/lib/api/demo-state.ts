/** Tracks whether any demo fixture has been shown, so the global DEMO DATA banner can appear. */
let active = false;
let reason: string | null = null;
const listeners = new Set<() => void>();

export function markDemoActive(why: string): void {
  if (active && reason === why) return;
  active = true;
  reason = why;
  listeners.forEach((l) => l());
}

export function resetDemoState(): void {
  active = false;
  reason = null;
  listeners.forEach((l) => l());
}

export function isDemoActive(): boolean {
  return active;
}

export function demoReason(): string | null {
  return reason;
}

export function subscribeDemo(listener: () => void): () => void {
  listeners.add(listener);
  return () => listeners.delete(listener);
}
