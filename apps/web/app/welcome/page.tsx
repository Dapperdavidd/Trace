import { Suspense } from "react";
import { Welcome } from "@/components/trace/Welcome";

export const metadata = { title: "Welcome" };

export default function WelcomePage() {
  return (
    <Suspense fallback={null}>
      <Welcome />
    </Suspense>
  );
}
