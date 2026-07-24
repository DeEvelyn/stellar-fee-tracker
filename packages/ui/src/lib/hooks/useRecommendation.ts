'use client'

import { useState, useCallback } from 'react'
import { api } from '@/lib/api'
import type { RecommendResponse, Urgency } from '@/lib/types'

interface UseRecommendationReturn {
  recommend: (targetLedgers?: number, urgency?: Urgency, maxFee?: string, confidence?: number) => Promise<void>
  result: RecommendResponse | null
  loading: boolean
  error: string | null
  clear: () => void
}

export function useRecommendation(): UseRecommendationReturn {
  const [result, setResult] = useState<RecommendResponse | null>(null)
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  const recommend = useCallback(
    async (targetLedgers?: number, urgency?: Urgency, maxFee?: string, confidence?: number) => {
      setLoading(true)
      setError(null)
      try {
        const res = await api.recommend({
          target_ledgers: targetLedgers,
          urgency,
          max_fee: maxFee,
          confidence: confidence !== undefined ? confidence / 100 : undefined,
        })
        setResult(res)
      } catch (err) {
        setError(err instanceof Error ? err.message : 'Failed to get fee recommendation')
      } finally {
        setLoading(false)
      }
    },
    [],
  )

  const clear = useCallback(() => {
    setResult(null)
    setError(null)
  }, [])

  return { recommend, result, loading, error, clear }
}
