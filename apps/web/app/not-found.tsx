import Link from "next/link";
import { EmptyState } from "@/components/ui";

export default function NotFound() {
  return (
    <EmptyState title="Page not found" action={<Link className="btn btn-primary btn-sm" href="/">Back to Home</Link>}>
      This page does not exist.
    </EmptyState>
  );
}
