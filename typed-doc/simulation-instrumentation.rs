/**
 * Issue #340: Add span instrumentation to simulation module
 *
 * Wraps all public simulation functions with tracing spans using
 * #[tracing::instrument] for observability of the fee simulation pipeline.
 *
 * Requirements:
 * - Each span includes: function name, input config fields, output sample count
 * - Spans visible in both JSON and human log formats
 */

// packages/devkit/src/simulation/fee_model.rs
use tracing::instrument;

/// Parameters controlling fee model behaviour.
#[derive(Debug)]
pub struct FeeModelConfig {
    pub min_fee: u64,
    pub max_fee: u64,
    pub base_reserve: u64,
    pub ledger_capacity: u64,
}

/// Output from the fee computation.
pub struct FeeEstimate {
    pub recommended: u64,
    pub low: u64,
    pub medium: u64,
    pub high: u64,
    pub sample_count: usize,
}

impl FeeModel {
    /// Compute fee estimates from the current ledger state.
    #[instrument(skip(self), fields(config = ?self.config))]
    pub fn estimate(&self, ledger: &LedgerState) -> FeeEstimate {
        let samples = self.collect_samples(ledger);
        let sorted = self.sort_samples(samples);

        let estimate = FeeEstimate {
            recommended: sorted.percentile(50),
            low: sorted.percentile(10),
            medium: sorted.percentile(25),
            high: sorted.percentile(75),
            sample_count: sorted.len(),
        };

        tracing::info!(
            sample_count = estimate.sample_count,
            recommended = estimate.recommended,
            "fee estimate computed"
        );

        estimate
    }
}

// packages/devkit/src/simulation/network_load.rs
use tracing::instrument;

impl NetworkLoad {
    /// Estimate the current network congestion level as a fraction 0.0 … 1.0.
    #[instrument(skip(self))]
    pub fn estimate_current_load(&self, recent_txs: &[Transaction]) -> f64 {
        if recent_txs.is_empty() {
            tracing::warn!("no recent transactions to estimate load");
            return 0.0;
        }

        let load = self.compute_load(recent_txs);
        let clamped = load.clamp(0.0, 1.0);

        tracing::debug!(
            raw_load = load,
            clamped_load = clamped,
            tx_count = recent_txs.len(),
            "network load estimated"
        );

        clamped
    }
}

// packages/devkit/src/simulation/congestion_predictor.rs
use tracing::instrument;

pub enum CongestionLevel {
    Low,
    Medium,
    High,
    Extreme,
}

impl CongestionPredictor {
    /// Predict short-term congestion based on recent and historical data.
    #[instrument(skip(self, recent_loads), fields(window = recent_loads.len()))]
    pub fn predict_congestion_short_term(
        &self,
        recent_loads: &[f64],
        historical_avg: f64,
    ) -> CongestionLevel {
        let current_avg = recent_loads.iter().copied().sum::<f64>()
            / recent_loads.len().max(1) as f64;

        tracing::debug!(
            current_avg = current_avg,
            historical_avg = historical_avg,
            "congestion prediction computed"
        );

        if current_avg > historical_avg * 1.5 {
            CongestionLevel::High
        } else if current_avg > historical_avg * 1.2 {
            CongestionLevel::Medium
        } else {
            CongestionLevel::Low
        }
    }

    /// Predict long-term congestion trend.
    #[instrument(skip(self))]
    pub fn predict_congestion_trend(&self, loads: &[f64]) -> f64 {
        let trend = self.compute_trend(loads);
        tracing::info!(trend = trend, "long-term congestion trend");
        trend
    }
}
