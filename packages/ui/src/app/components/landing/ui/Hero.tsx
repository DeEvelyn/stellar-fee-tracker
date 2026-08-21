'use client'
import Link from "next/link";
import React, { useEffect, useState } from "react";
import { GridBeams } from "../GridBeams";

const FEES = [
  100, 3849, 8945, 11879, 42440, 61684, 100, 219192, 100, 3850, 137793,
];

function useTickingFee() {
  const [fee, setFee] = useState(100);
  const [prev, setPrev] = useState(100);

  useEffect(() => {
    const id = setInterval(() => {
      const next = FEES[Math.floor(Math.random() * FEES.length)];
      setPrev(fee);
      setFee(next);
    }, 1800);
    return () => clearInterval(id);
  }, [fee]);
  return { fee, prev, up: fee > prev };
}

function Sparkline({
  data,
  color = "#00ff9d",
}: {
  data: number[];
  color?: string;
}) {
  const max = Math.max(...data);
  const min = Math.min(...data);
  const w = 120,
    h = 40;
  const pts = data
    .map((v, i) => {
      const x = (i / (data.length - 1)) * w;
      const y = h - ((v - min) / (max - min || 1)) * h;
      return `${x},${y}`;
    })
    .join(" ");
  return (
    <svg width={w} height={h} viewBox={`0 0 ${w} ${h}`} fill="none">
      <polyline
        points={pts}
        stroke={color}
        strokeWidth="1.5"
        strokeLinejoin="round"
        strokeLinecap="round"
      />
      <polyline
        points={`0,${h} ${pts} ${w},${h}`}
        fill={`${color}10`}
        stroke="none"
      />
    </svg>
  );
}

function Hero() {
  const [mounted, setMounted] = useState(false);
  const { fee, up } = useTickingFee();

  useEffect(() => {
    setMounted(true);
  }, []);

  const sparkData = [
    100, 3849, 100, 8945, 100, 42440, 100, 3849, 219192, 100, 61684, 100, 3849,
    100, 11879,
  ];

  return (
    <section className="relative pt-24 pb-20 px-6 max-w-6xl mx-auto">
      <GridBeams />
      <div className="relative z-10 max-w-3xl">
        <div
          className={`flex items-center gap-2 mb-8 transition-all duration-700 ${mounted ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4"}`}
        >
          <span className="w-1.5 h-1.5 rounded-full bg-[#00ff9d] animate-pulse" />
          <span className="text-xs text-[#00ff9d] tracking-widest">
            STELLAR TESTNET · LIVE
          </span>
          <span className="text-[#1e2a36]">·</span>
          <span
            suppressHydrationWarning
            className={`text-xs font-bold transition-colors duration-300 ${up ? "text-[#ff4d6d]" : "text-[#00ff9d]"}`}
            style={{ fontFamily: "'Space Mono', monospace" }}
          >
            {fee.toLocaleString()} STROOPS
          </span>
        </div>

        <h1
          className={`text-5xl md:text-7xl font-bold leading-[1.05] mb-6 transition-all duration-700 delay-100 ${mounted ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4"}`}
          style={{
            fontFamily: "'Space Mono', monospace",
            letterSpacing: "-0.02em",
          }}
        >
          <span className="text-[#e2eaf4]">Know Before</span>
          <br />
          <span
            className="text-[#00ff9d]"
            style={{ textShadow: "0 0 40px rgba(0,255,157,0.3)" }}
          >
            You Send.
          </span>
        </h1>

        <p
          className={`text-base text-[#7a9ab5] leading-relaxed max-w-xl mb-10 transition-all duration-700 delay-200 ${mounted ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4"}`}
        >
          Real-time Stellar network fee intelligence. Track costs, detect
          congestion, and time your transactions with confidence.
        </p>

        <div
          className={`flex flex-wrap items-center gap-4 transition-all duration-700 delay-300 ${mounted ? "opacity-100 translate-y-0" : "opacity-0 translate-y-4"}`}
        >
          <Link
            href="/dashboard"
            className="group flex items-center gap-2 px-6 py-3 bg-[#00ff9d] text-[#080b0f] rounded font-bold text-sm hover:bg-[#00e88d] transition-all duration-200 hover:shadow-lg hover:shadow-[#00ff9d20] hover:-translate-y-0.5"
            style={{ fontFamily: "'Space Mono', monospace" }}
          >
            OPEN DASHBOARD
            <span className="group-hover:translate-x-1 transition-transform">
              →
            </span>
          </Link>
          <a
            href="https://github.com/StellarCommons/stellar-fee-tracker"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 px-6 py-3 border border-[#1e2a36] text-[#7a9ab5] rounded text-sm hover:border-[#3d5a73] hover:text-[#e2eaf4] transition-all duration-200"
            style={{ fontFamily: "'Space Mono', monospace" }}
          >
            <svg width="14" height="14" viewBox="0 0 24 24" fill="currentColor">
              <path d="M12 0C5.37 0 0 5.37 0 12c0 5.31 3.435 9.795 8.205 11.385.6.105.825-.255.825-.57 0-.285-.015-1.23-.015-2.235-3.015.555-3.795-.735-4.035-1.41-.135-.345-.72-1.41-1.23-1.695-.42-.225-1.02-.78-.015-.795.945-.015 1.62.87 1.845 1.23 1.08 1.815 2.805 1.305 3.495.99.105-.78.42-1.305.765-1.605-2.67-.3-5.46-1.335-5.46-5.925 0-1.305.465-2.385 1.23-3.225-.12-.3-.54-1.53.12-3.18 0 0 1.005-.315 3.3 1.23.96-.27 1.98-.405 3-.405s2.04.135 3 .405c2.295-1.56 3.3-1.23 3.3-1.23.66 1.65.24 2.88.12 3.18.765.84 1.23 1.905 1.23 3.225 0 4.605-2.805 5.625-5.475 5.925.435.375.81 1.095.81 2.22 0 1.605-.015 2.895-.015 3.3 0 .315.225.69.825.57A12.02 12.02 0 0 0 24 12c0-6.63-5.37-12-12-12z" />
            </svg>
            VIEW SOURCE
          </a>
        </div>
      </div>

      <div
        className={`hidden lg:block absolute right-6 top-24 transition-all duration-700 delay-500 ${mounted ? "opacity-100" : "opacity-0"}`}
      >
        <div className="text-[9px] text-[#3d5a73] tracking-widest mb-2 text-right">
          LAST 15 LEDGERS
        </div>
        <Sparkline data={sparkData} />
        <div className="flex justify-between mt-1">
          <span className="text-[8px] text-[#1e2a36]">100 str</span>
          <span className="text-[8px] text-[#1e2a36]">219K str</span>
        </div>
      </div>
    </section>
  );
}

export default Hero;
