"use client";

import { DashboardPreview } from "./DashboardPreview";
import { FeatureOrbit } from "./FeatureOrbit";
import { HowItWorks } from "./HowItWorks";
import { OpenSourceCTA } from "./OpenSourceCTA";
import { Footer } from "./Footer";
import Hero from "./ui/Hero";
import StatStrip from "./StatStrip";
import  Nav  from "./Nav";

export function LandingPage() {
  return (
    <div
      style={{
        fontFamily: "'JetBrains Mono', monospace",
        background: "#080b0f",
        color: "#e2eaf4",
      }}
      className="min-h-screen overflow-x-hidden"
    >
      {/* Scanline texture */}
      <div
        className="pointer-events-none fixed inset-0 z-50"
        style={{
          background:
            "repeating-linear-gradient(0deg,transparent,transparent 2px,rgba(0,0,0,0.025) 2px,rgba(0,0,0,0.025) 4px)",
        }}
      />

      {/* Grid background */}
      <div
        className="pointer-events-none fixed inset-0"
        style={{
          backgroundImage:
            "linear-gradient(rgba(0,255,157,0.025) 1px,transparent 1px),linear-gradient(90deg,rgba(0,255,157,0.025) 1px,transparent 1px)",
          backgroundSize: "48px 48px",
        }}
      />

      <Nav />
      <Hero />
      <StatStrip />
      <DashboardPreview />
      <FeatureOrbit />
      <HowItWorks />
      <OpenSourceCTA />
      <Footer />
    </div>
  );
}
