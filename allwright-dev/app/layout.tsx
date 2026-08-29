import type { Metadata, Viewport } from "next";
import { IBM_Plex_Mono, Space_Grotesk } from "next/font/google";
import { GoogleAnalytics } from "@next/third-parties/google";

import "./globals.css";
import {
  GITHUB_URL,
  SITE_DESCRIPTION,
  SITE_NAME,
  SITE_TITLE,
  SITE_URL,
} from "./brand";
import { SiteFooter } from "./site-footer";
import { SiteHeader } from "./site-header";
import { ThemeProvider } from "./theme-provider";

const spaceGrotesk = Space_Grotesk({
  subsets: ["latin"],
  variable: "--font-display",
});

const ibmPlexMono = IBM_Plex_Mono({
  subsets: ["latin"],
  weight: ["400", "500"],
  variable: "--font-mono",
});

export const metadata: Metadata = {
  metadataBase: new URL(SITE_URL),
  title: {
    default: SITE_TITLE,
    template: `%s · ${SITE_NAME}`,
  },
  description: SITE_DESCRIPTION,
  keywords: [
    "test automation",
    "web automation",
    "mobile automation",
    "desktop automation",
    "API testing",
    "QA automation engine",
    "end-to-end testing",
  ],
  category: "technology",
  applicationName: SITE_NAME,
  alternates: {
    canonical: SITE_URL,
  },
  openGraph: {
    type: "website",
    url: SITE_URL,
    siteName: SITE_NAME,
    title: SITE_TITLE,
    description: SITE_DESCRIPTION,
    locale: "en_US",
  },
  twitter: {
    card: "summary_large_image",
    title: SITE_TITLE,
    description: SITE_DESCRIPTION,
  },
  robots: {
    index: true,
    follow: true,
    googleBot: {
      index: true,
      follow: true,
    },
  },
  other: {
    "github:repo": GITHUB_URL,
  },
};

export const viewport: Viewport = {
  colorScheme: "light dark",
  themeColor: [
    { media: "(prefers-color-scheme: light)", color: "#eef8f4" },
    { media: "(prefers-color-scheme: dark)", color: "#06171a" },
  ],
};

// Single env var drives Google Analytics: set NEXT_PUBLIC_GA_ID (a GA4
// measurement ID, e.g. "G-XXXXXXXXXX") to turn it on. Unset it — locally,
// in preview deploys, wherever — and analytics stays off with no code
// changes.
const GA_ID = process.env.NEXT_PUBLIC_GA_ID;

export default function RootLayout({
  children,
}: Readonly<{
  children: React.ReactNode;
}>) {
  return (
    <html lang="en" suppressHydrationWarning>
      <body className={`${spaceGrotesk.variable} ${ibmPlexMono.variable}`}>
        <ThemeProvider attribute="class" defaultTheme="system" enableSystem>
          <div className="relative min-h-screen overflow-hidden bg-[radial-gradient(circle_at_top_left,var(--accent-soft),transparent_38%),radial-gradient(circle_at_85%_15%,var(--accent-2-soft),transparent_32%),linear-gradient(180deg,var(--background)_0%,var(--background-deep)_100%)]">
            <div className="grid-overlay pointer-events-none absolute inset-0 opacity-40" />
            <div className="ambient-left pointer-events-none absolute left-[-10%] top-[6%] h-96 w-96 rounded-full bg-[radial-gradient(circle,var(--accent-soft),transparent_68%)] blur-md" />
            <div className="ambient-right pointer-events-none absolute bottom-[-4%] right-[-10%] h-104 w-104 rounded-full bg-[radial-gradient(circle,var(--accent-2-soft),transparent_68%)] blur-md" />
            <SiteHeader />
            <main className="relative px-4 sm:px-6">{children}</main>
            <SiteFooter />
          </div>
        </ThemeProvider>
      </body>
      {GA_ID ? <GoogleAnalytics gaId={GA_ID} /> : null}
    </html>
  );
}
