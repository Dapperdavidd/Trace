import { Suspense } from "react";
import { ShadowTrades } from "@/components/ShadowTrades";
import { SkeletonRows } from "@/components/ui";

export default function Page() {
  return (
    <Suspense fallback={<SkeletonRows rows={4} />}>
      <ShadowTrades />
    </Suspense>
  );
}
