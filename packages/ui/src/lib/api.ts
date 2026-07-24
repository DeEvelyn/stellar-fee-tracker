import type {
  CurrentFeeResponse,
  FeeHistoryResponse,
  FeeTrendResponse,
  InsightsResponse,
  HealthResponse,
  RecommendResponse,
  RecommendRequest,
} from './types'

const BASE = process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080'

async function post<T>(path: string, body: unknown): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(body),
  })
  if (!res.ok) {
    const err = await res.json().catch(() => ({ error: res.statusText }))
    throw new Error(err.error || `API error ${res.status} on ${path}`)
  }
  return res.json() as Promise<T>
}

async function get<T>(path: string): Promise<T> {
  const res = await fetch(`${BASE}${path}`, {
    next: { revalidate: 0 }, // always fresh
  })
  if (!res.ok) {
    throw new Error(`API error ${res.status} on ${path}`)
  }
  return res.json() as Promise<T>
}

export const api = {
  currentFees:  ()               => get<CurrentFeeResponse>('/fees/current'),
  feeHistory:   (window = '1h')  => get<FeeHistoryResponse>(`/fees/history?window=${window}`),
  feeTrend:     ()               => get<FeeTrendResponse>('/fees/trend'),
  insights:     ()               => get<InsightsResponse>('/insights'),
  health:       ()               => get<HealthResponse>('/health'),
  recommend:    (body: RecommendRequest) => post<RecommendResponse>('/fees/recommend', body),
}
