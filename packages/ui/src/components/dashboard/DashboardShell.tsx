'use client'

import { useState, useEffect, useCallback } from 'react'
import { api } from '@/lib/api'
import type {
  CurrentFeeResponse,
  FeeHistoryResponse,
  FeeTrendResponse,
  InsightsResponse,
} from '@/lib/types'
import { TopBar }             from './TopBar'
import { StatCards }          from './StatCards'
import { PercentileRow }      from './PercentileRow'
import { RecommendationPanel } from './RecommendationPanel'
import { FeeChart }           from './FeeChart'
import { TrendPanel }         from './TrendPanel'
import { RollingAverages }    from './RollingAverages'

interface DashboardData {
  current:  CurrentFeeResponse  | null
  history:  FeeHistoryResponse  | null
  trend:    FeeTrendResponse    | null
  insights: InsightsResponse    | null
  error:    string | null
}

interface Props {
  initialData: DashboardData
}

const POLL_MS = 10_000

export function DashboardShell({ initialData }: Props) {
  const [data,        setData]        = useState<DashboardData>(initialData)
  const [window,      setWindow]      = useState<'1h' | '6h' | '24h'>('1h')
  const [lastUpdated, setLastUpdated] = useState<Date>(new Date())
  const [isRefreshing, setIsRefreshing] = useState(false)
  const [tick,        setTick]        = useState(0)

  const refresh = useCallback(async (win = window) => {
    setIsRefreshing(true)
    try {
      const [current, history, trend, insights] = await Promise.allSettled([
        api.currentFees(),
        api.feeHistory(win),
        api.feeTrend(),
        api.insights(),
      ])
      setData({
        current:  current.status  === 'fulfilled' ? current.value  : data.current,
        history:  history.status  === 'fulfilled' ? history.value  : data.history,
        trend:    trend.status    === 'fulfilled' ? trend.value    : data.trend,
        insights: insights.status === 'fulfilled' ? insights.value : data.insights,
        error:    null,
      })
      setLastUpdated(new Date())
      setTick(t => t + 1)
    } catch {
      // keep stale data
    } finally {
      setIsRefreshing(false)
    }
  }, [window, data])

  // Auto-poll every 10s
  useEffect(() => {
    const id = setInterval(() => refresh(), POLL_MS)
    return () => clearInterval(id)
  }, [refresh])

  // Re-fetch history when window changes
  const handleWindowChange = async (w: '1h' | '6h' | '24h') => {
    setWindow(w)
    refresh(w)
  }

  const { current, history, trend, insights } = data

  return (
    <div className="min-h-screen bg-bg-base grid-bg">
      <TopBar
        lastUpdated={lastUpdated}
        isRefreshing={isRefreshing}
        onRefresh={() => refresh()}
      />

      <main className="max-w-[1400px] mx-auto px-4 py-6 space-y-6">

        {/* Row 1 — Stat Cards */}
        <StatCards
          current={current}
          trend={trend}
          tick={tick}
        />

        {/* Row 2 — Percentile strip */}
        {current && (
          <PercentileRow percentiles={current.percentiles} tick={tick} />
        )}

        {/* Row 3 — Recommendation Panel */}
        <RecommendationPanel />

        {/* Row 4 — Chart */}
        <FeeChart
          history={history}
          window={window}
          onWindowChange={handleWindowChange}
        />

        {/* Row 5 — Trend + Averages */}
        <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
          <TrendPanel trend={trend} tick={tick} />
          <RollingAverages insights={insights} tick={tick} />
        </div>

        {/* Footer */}
        <div className="text-center text-text-muted text-xs pb-8 font-mono">
          <span className="text-accent-green/50">◆</span>
          {' '}STELLAR TESTNET · POLLING EVERY 10s · BUILT WITH RUST + NEXT.JS
        </div>
      </main>
    </div>
  )
}
