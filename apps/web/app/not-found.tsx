import Link from "next/link";
import { Logo } from "@/components/trace/ui";

export default function NotFound() {
  return (
    <main className="legal" style={{ minHeight: "100dvh", alignContent: "center", justifyItems: "start" }}>
      <Logo />
      <h1>This page does not exist.</h1>
      <p>It may have moved when trace was redesigned.</p>
      <div className="bset">
        <Link className="bfill" href="/app/today/">Open trace</Link>
        <Link className="bout" href="/">Home</Link>
      </div>
    </main>
  );
}
