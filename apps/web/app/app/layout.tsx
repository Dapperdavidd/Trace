import type { ReactNode } from "react";
import { Suspense } from "react";
import { AppStateProvider } from "@/components/trace/AppState";
import { AppFrame } from "@/components/trace/AppFrame";

export default function AppLayout({ children }: { children: ReactNode }) {
  return (
    <AppStateProvider>
      <Suspense fallback={null}>
        <AppFrame>{children}</AppFrame>
      </Suspense>
    </AppStateProvider>
  );
}
