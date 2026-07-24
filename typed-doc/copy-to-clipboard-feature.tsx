/**
 * Issue #322: Add copy-to-clipboard on recommended fee
 *
 * Implements single-click copy of the raw stroop value from the
 * RecommendationPanel, with "Copied!" feedback and graceful fallback.
 */

// packages/ui/src/components/dashboard/RecommendationPanel.tsx
import { useState, useCallback } from "react";

interface RecommendationPanelProps {
  recommendedFee: number;       // raw stroop value (e.g. 3849)
  lowFee: number;
  mediumFee: number;
  highFee: number;
}

export function RecommendationPanel({
  recommendedFee,
  lowFee,
  mediumFee,
  highFee,
}: RecommendationPanelProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(String(recommendedFee));
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // clipboard API unavailable — fallback
      const ta = document.createElement("textarea");
      ta.value = String(recommendedFee);
      ta.style.position = "fixed";
      ta.style.opacity = "0";
      document.body.appendChild(ta);
      ta.select();
      document.execCommand("copy");
      document.body.removeChild(ta);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    }
  }, [recommendedFee]);

  return (
    <div className="rounded-lg border p-4 space-y-3">
      <h3 className="text-sm font-medium text-muted-foreground">
        Recommended Fee
      </h3>

      <div className="flex items-center gap-3">
        <span className="text-3xl font-bold tabular-nums">
          {recommendedFee.toLocaleString()}
        </span>
        <span className="text-sm text-muted-foreground">stroops</span>

        <button
          onClick={handleCopy}
          className="ml-auto inline-flex items-center gap-1.5 rounded-md px-3 py-1.5 text-sm font-medium transition-colors hover:bg-accent"
          aria-label={copied ? "Copied" : "Copy fee value"}
        >
          {copied ? (
            <>
              <CheckIcon className="h-4 w-4 text-green-500" />
              Copied!
            </>
          ) : (
            <>
              <CopyIcon className="h-4 w-4" />
              Copy
            </>
          )}
        </button>
      </div>

      <div className="grid grid-cols-3 gap-2 text-xs text-muted-foreground">
        <div>Low: {lowFee.toLocaleString()}</div>
        <div>Medium: {mediumFee.toLocaleString()}</div>
        <div>High: {highFee.toLocaleString()}</div>
      </div>
    </div>
  );
}

// Icons (inline to avoid import complexity)
function CopyIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
      <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
    </svg>
  );
}

function CheckIcon({ className }: { className?: string }) {
  return (
    <svg className={className} fill="none" viewBox="0 0 24 24" stroke="currentColor" strokeWidth={2}>
      <polyline points="20 6 9 17 4 12" />
    </svg>
  );
}
