# Copy-to-Clipboard Feature — Specification

## Overview

Add a copy-to-clipboard button to the `RecommendationPanel` component that copies
the recommended fee (in stroops) to the user's clipboard with a visual confirmation.

## Component changes

### RecommendationPanel.tsx

Add after the recommended fee display:

```tsx
<CopyToClipboard
  text={String(result.fee_in_stroops)}
  label="Copy fee to clipboard"
/>
```

### CopyToClipboard component (new)

Create `packages/ui/src/components/shared/CopyToClipboard.tsx`:

```tsx
'use client';

import { useState, useCallback } from 'react';
import { Copy, Check } from 'lucide-react';

interface CopyToClipboardProps {
  text: string;
  label?: string;
}

export function CopyToClipboard({ text, label = 'Copy' }: CopyToClipboardProps) {
  const [copied, setCopied] = useState(false);

  const handleCopy = useCallback(async () => {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    } catch {
      // Fallback for older browsers
      const textarea = document.createElement('textarea');
      textarea.value = text;
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand('copy');
      document.body.removeChild(textarea);
      setCopied(true);
      setTimeout(() => setCopied(false), 2000);
    }
  }, [text]);

  return (
    <button
      onClick={handleCopy}
      className="inline-flex items-center gap-1.5 text-sm text-gray-400 hover:text-white transition-colors"
      aria-label={label}
      title={label}
    >
      {copied ? (
        <Check className="w-4 h-4 text-green-400" />
      ) : (
        <Copy className="w-4 h-4" />
      )}
      {copied ? 'Copied!' : 'Copy'}
    </button>
  );
}
```

### Recommendations types update

Ensure `lib/types.ts` exports the `RecommendResponse` interface (verify it already exists):

```ts
export interface RecommendResponse {
  recommended_fee: number;
  fee_in_stroops: number;
  estimated_wait_ledgers: number;
  confidence: number;
  network_condition: string;
  alternatives: FeeAlternative[];
}
```

## Visual design

- The button sits inline next to the recommended fee value
- Shows a Copy icon (from lucide-react) by default
- On click, transitions to a green Check icon with "Copied!" text
- After 2 seconds, reverts to the default state
- Uses Tailwind transition classes for smooth animation

## Acceptance criteria

- [ ] Clicking Copy copies the numeric fee value to clipboard
- [ ] Visual feedback shows copied state for 2 seconds
- [ ] Works in modern browsers (Clipboard API)
- [ ] Falls back to execCommand for older browsers
- [ ] Button has proper aria-label for accessibility
- [ ] Does not interfere with existing RecommendationPanel layout
