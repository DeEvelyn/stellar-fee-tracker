## Fee Recommendation Engine

This section explains how `FeeRecommendationEngine` turns recent network activity into a concrete fee recommendation, and the assumptions and limitations behind it.

### What "inclusion probability" means

Inclusion probability is the estimated chance that a transaction paying a given fee gets included within a target number of ledgers.

It's computed from the recent sample of observed fees (see below). For a single ledger, the probability that a candidate fee `f` would have been included is simply the fraction of recently observed fees that were less than or equal to `f`:

```
p_one = (count of recent fees <= f) / (total recent fees)
```

For a target window of `N` ledgers, the engine treats each ledger as an independent opportunity for inclusion and computes the probability of being included in *at least one* of them:

```
p_n = 1 - (1 - p_one) ^ N
```

This is why widening the target ledger window (e.g. from 1 to 5) increases the reported confidence for the same fee — you're not waiting for one shot, you're getting multiple chances.

### How historical fee distribution is sampled

The engine pulls every fee data point recorded in the **last hour** from the fee history store. If that window has fewer than **50 samples**, it's treated as a cold start (see [Limitations](#limitations)) and the engine skips statistical estimation entirely in favor of a fixed fallback.

When enough samples exist, they're sorted once and percentiles are read off the sorted array by rank (e.g. p50, p90, p99), which is also what backs the urgency-based base fee:

| Urgency | Percentile used as base fee |
|---|---|
| Low | p30 |
| Medium | p60 |
| High | p80 |
| Urgent | p95 |

### How binary search finds the minimum fee for a target confidence

Given a target confidence (e.g. 0.90) and a target ledger window, the engine binary-searches over the **sorted list of recently observed fees** — not over the raw fee space — to find the cheapest sampled fee that meets the confidence bar:

1. Set `lo = 0`, `hi = last index` of the sorted fee array.
2. At each step, check the inclusion probability of the fee at the midpoint.
3. If that probability already meets or exceeds the target, search the lower half (cheaper fees); otherwise search the upper half.
4. Converge to the smallest sampled fee whose probability clears the target.

If even the highest sampled fee (p99) can't reach the target confidence, the engine extrapolates: it takes p99 and multiplies it by 1.1, then reports that fee along with whatever probability it actually achieves.

### How network condition adjustments are applied

After a base fee is chosen from the percentile table above, it's adjusted based on the current network trend before being returned:

| Network condition | Adjustment |
|---|---|
| Congested | × 1.30 |
| Rising | × 1.15 |
| Normal | unchanged |
| Declining | × 0.95 (floored at 100 stroops) |
| Unknown (no insights engine configured) | unchanged |

The condition itself comes from a separate insights/trend component, when one is wired in; if it isn't, the engine reports the condition as `"unknown"` and applies no adjustment.

### The three alternative tiers

Every recommendation response includes three alternative options alongside the primary recommendation, each computed via the same binary-search-for-confidence process described above, with fixed parameters:

| Tier | Target confidence | Target ledger window |
|---|---|---|
| `economy` | 0.70 | 5 ledgers |
| `standard` | 0.90 | 2 ledgers |
| `fast` | 0.99 | 1 ledger |

These are not adjustable per-request — they're fixed reference points so callers can present a consistent "cheap / normal / fast" choice regardless of current network conditions.

### Limitations

- **Cold start (fewer than 50 samples):** When the last hour of fee history has fewer than `min_sample_count` (50) data points, the engine does not attempt statistical estimation. It falls back to fixed fees by urgency level (100 / 500 / 1,000 / 5,000 stroops for low/medium/high/urgent), a flat confidence of 0.85, and reports network condition as `"unknown"`. Treat fallback responses as rough defaults, not data-backed estimates.
- **Rapid network changes:** The sampling window is a trailing hour, and recommendation results are cached briefly (10 seconds by default). A sudden shift in network congestion — a surge starting mid-window, for instance — won't be fully reflected until enough new samples accumulate and the cache expires. Recommendations during volatile periods should be treated as lagging indicators, not real-time guarantees.
- **Sample independence assumption:** The multi-ledger probability formula (`1 - (1 - p_one)^N`) assumes each ledger's inclusion chance is independent. In practice, consecutive ledgers under sustained congestion are correlated, so confidence for larger ledger windows may be optimistic during prolonged surges.