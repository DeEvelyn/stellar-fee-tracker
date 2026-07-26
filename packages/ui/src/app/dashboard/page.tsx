import { api } from '@/lib/api'
import { DashboardShell } from '@/components/dashboard/DashboardShell'
import type { Metadata } from 'next'

export const dynamic = 'force-dynamic'
export const revalidate = 0

export const metadata: Metadata = {
  title: 'Dashboard | StellarFees',
  description: 'Real-time Stellar network fee intelligence dashboard',
}

async function fetchDashboardData() {
  try {
    const [current, history, trend, insights] = await Promise.allSettled([
      api.currentFees(),
      api.feeHistory('1h'),
      api.feeTrend(),
      api.insights(),
    ])

    return {
      current:  current.status  === 'fulfilled' ? current.value  : null,
      history:  history.status  === 'fulfilled' ? history.value  : null,
      trend:    trend.status    === 'fulfilled' ? trend.value    : null,
      insights: insights.status === 'fulfilled' ? insights.value : null,
      error:    null,
    }
  } catch (e) {
    return { current: null, history: null, trend: null, insights: null, error: String(e) }
  }
}

export default async function DashboardPage() {
  const data = await fetchDashboardData()
  return <DashboardShell initialData={data} />
}