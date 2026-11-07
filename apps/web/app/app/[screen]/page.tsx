import { Suspense } from "react";
import { TraceScreen } from "@/components/trace/TraceScreen";
import { SCREENS } from "@/lib/trace/screens";

export const dynamicParams = false;

export function generateStaticParams() {
  return SCREENS.map((screen) => ({ screen }));
}

export default async function ScreenPage({ params }: { params: Promise<{ screen: string }> }) {
  const { screen } = await params;
  return (
    <Suspense fallback={null}>
      <TraceScreen screen={screen} />
    </Suspense>
  );
}
