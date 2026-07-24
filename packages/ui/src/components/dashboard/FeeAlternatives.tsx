'use client'

import type { FeeAlternative } from '@/lib/types'
import { formatStroops } from '@/lib/utils'

const TIER_STYLES: Record<string, { border: string; label: string }> = {
  economy: { border: 'border-cyan-500/30',   label: 'text-cyan-400'   },
  standard:{ border: 'border-green-500/30',  label: 'text-green-400'  },
  fast:    { border: 'border-yellow-500/30', label: 'text-yellow-400' },
}

function tierStyle(label: string) {
  const key = label.toLowerCase()
  return TIER_STYLES[key] ?? { border: 'border-border-base', label: 'text-text-base' }
}

interface Props {
  alternatives: FeeAlternative[]
}

export function FeeAlternatives({ alternatives }: Props) {
  if (!alternatives.length) return null

  const handleCopy = (fee: string) => {
    navigator.clipboard.writeText(fee).catch(() => {})
  }

  return (
    <div>
      <p className="text-xs text-text-muted mb-2 font-medium">Alternatives</p>
      <div className="grid grid-cols-3 gap-2">
        {alternatives.map((alt, i) => {
          const { border, label: labelColor } = tierStyle(alt.label)
          return (
            <button
              key={i}
              onClick={() => handleCopy(alt.fee)}
              title="Click to copy fee"
              className={`rounded-lg border ${border} bg-bg-base px-2 py-2.5 text-left transition-colors hover:bg-bg-card`}
            >
              <p className={`text-xs font-semibold capitalize ${labelColor}`}>{alt.label}</p>
              <p className="text-sm font-bold text-text-base tabular-nums mt-1">
                {formatStroops(Number(alt.fee))}
              </p>
              <p className="text-xs text-text-muted mt-0.5">
                {(alt.confidence * 100).toFixed(0)}% · ~{alt.estimated_wait_ledgers}L
              </p>
            </button>
          )
        })}
      </div>
    </div>
  )
}
