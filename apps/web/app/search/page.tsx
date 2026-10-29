import { Suspense } from "react";
import { SearchResults } from "@/components/SearchResults";
import { SkeletonRows } from "@/components/ui";

export default function Page() {
  return (
    <Suspense fallback={<SkeletonRows rows={3} />}>
      <SearchResults />
    </Suspense>
  );
}
