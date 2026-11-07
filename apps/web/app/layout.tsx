import type { Metadata, Viewport } from "next";
import type { ReactNode } from "react";
import { Sprite } from "@/components/trace/Sprite";
import "./design.css";
import "./shell.css";

export const metadata: Metadata = {
  title: { default: "trace · See who's buying before you buy", template: "%s · trace" },
  description:
    "trace reads Ethereum and Solana and tells you, in plain words, who is buying a coin, whether it looks safe, and what would have happened if you had copied them. Copy limits are signed on NEAR.",
  icons: { icon: "/favicon.svg" },
};

export const viewport: Viewport = {
  width: "device-width",
  initialScale: 1,
  viewportFit: "cover",
  themeColor: "#0A0B0B",
  colorScheme: "dark",
};

export default function RootLayout({ children }: { children: ReactNode }) {
  return (
    <html lang="en">
      <head>
        <link rel="preconnect" href="https://fonts.googleapis.com" />
        <link rel="preconnect" href="https://fonts.gstatic.com" crossOrigin="" />
        {/* eslint-disable-next-line @next/next/no-page-custom-font */}
        <link rel="stylesheet" href="https://fonts.googleapis.com/css2?family=Geist:wght@400;500;600&display=swap" />
      </head>
      <body>
        <Sprite />
        {children}
      </body>
    </html>
  );
}
