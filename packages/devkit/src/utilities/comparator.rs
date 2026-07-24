//! Fee distribution comparator.
//!
//! Produces a structured diff between two slices of fee data, including
//! p50/p95/mean deltas and a volatility verdict.

use crate::simulation::fee_model::FeePoint;

/// The result of comparing two fee datasets side-by-side.
#[derive(Debug, Clone)]
pub struct ComparisonResult {
    pub label_baseline: String,
    pub label_target: String,
    pub count_baseline: usize,
    pub count_target: usize,
    pub mean_baseline: f64,
    pub mean_target: f64,
    /// Absolute mean delta (baseline - target).
    pub mean_delta: f64,
    /// Percentage mean delta relative to the baseline.
    pub mean_delta_pct: f64,
    pub p50_baseline: u64,
    pub p50_target: u64,
    pub p50_delta: i64,
    pub p95_baseline: u64,
    pub p95_target: u64,
    pub p95_delta: i64,
    pub spikes_baseline: usize,
    pub spikes_target: usize,
    /// Whether the target distribution is more volatile than the baseline.
    pub target_more_volatile: bool,
}

impl ComparisonResult {
    /// Render the comparison as a human-readable report.
    pub fn display(&self) -> String {
        format!(
            "Distribution Comparison: {} vs {}\n\
             ========================================\n\
             count      baseline={:>8}  target={:>8}\n\
             mean       baseline={:>8.2}  target={:>8.2}  delta={:+.2} ({:+.1}%)\n\
             p50        baseline={:>8}  target={:>8}  delta={:+}\n\
             p95        baseline={:>8}  target={:>8}  delta={:+}\n\
             spikes     baseline={:>8}  target={:>8}\n\
             volatility {}\n",
            self.label_baseline, self.label_target,
            self.count_baseline, self.count_target,
            self.mean_baseline, self.mean_target, self.mean_delta, self.mean_delta_pct,
            self.p50_baseline, self.p50_target, self.p50_delta,
            self.p95_baseline, self.p95_target, self.p95_delta,
            self.spikes_baseline, self.spikes_target,
            if self.target_more_volatile { "target is more volatile" } else { "target is less volatile" },
        )
    }

    /// Render the comparison as a JSON object.
    pub fn to_json(&self) -> String {
        format!(
            concat!(
                r#"{{"label_baseline":"{lb}","label_target":"{lt}","#,
                r#""count_baseline":{cb},"count_target":{ct},"#,
                r#""mean_baseline":{mb:.2},"mean_target":{mt:.2},"#,
                r#""mean_delta":{md:.2},"mean_delta_pct":{mdp:.2},"#,
                r#""p50_baseline":{p50b},"p50_target":{p50t},"p50_delta":{p50d},"#,
                r#""p95_baseline":{p95b},"p95_target":{p95t},"p95_delta":{p95d},"#,
                r#""spikes_baseline":{sb},"spikes_target":{st},"#,
                r#""target_more_volatile":{tmv}}}"#,
            ),
            lb = self.label_baseline, lt = self.label_target,
            cb = self.count_baseline, ct = self.count_target,
            mb = self.mean_baseline, mt = self.mean_target,
            md = self.mean_delta, mdp = self.mean_delta_pct,
            p50b = self.p50_baseline, p50t = self.p50_target, p50d = self.p50_delta,
            p95b = self.p95_baseline, p95t = self.p95_target, p95d = self.p95_delta,
            sb = self.spikes_baseline, st = self.spikes_target,
            tmv = self.target_more_volatile,
        )
    }

    /// Plain-English verdict.
    pub fn verdict(&self) -> &'static str {
        if self.mean_delta_pct.abs() < 5.0 {
            "distributions are similar (< 5% mean difference)"
        } else if self.mean_delta > 0.0 {
            "baseline has meaningfully higher fees than target"
        } else {
            "target has meaningfully higher fees than baseline"
        }
    }
}

/// Compares two fee datasets and produces a [`ComparisonResult`].
pub struct FeeComparator;

impl FeeComparator {
    /// Compare `baseline` and `target` slices with the given labels.
    pub fn compare(
        baseline: &[FeePoint],
        target: &[FeePoint],
        label_baseline: &str,
        label_target: &str,
    ) -> ComparisonResult {
        let mean_b = mean_fee(baseline);
        let mean_t = mean_fee(target);
        let mean_delta = mean_b - mean_t;
        let mean_delta_pct = if mean_b != 0.0 { (mean_delta / mean_b) * 100.0 } else { 0.0 };

        let p50_b = sorted_percentile(baseline, 50);
        let p50_t = sorted_percentile(target, 50);
        let p95_b = sorted_percentile(baseline, 95);
        let p95_t = sorted_percentile(target, 95);

        ComparisonResult {
            label_baseline: label_baseline.to_string(),
            label_target: label_target.to_string(),
            count_baseline: baseline.len(),
            count_target: target.len(),
            mean_baseline: mean_b,
            mean_target: mean_t,
            mean_delta,
            mean_delta_pct,
            p50_baseline: p50_b,
            p50_target: p50_t,
            p50_delta: p50_b as i64 - p50_t as i64,
            p95_baseline: p95_b,
            p95_target: p95_t,
            p95_delta: p95_b as i64 - p95_t as i64,
            spikes_baseline: baseline.iter().filter(|p| p.is_spike).count(),
            spikes_target: target.iter().filter(|p| p.is_spike).count(),
            target_more_volatile: std_dev(target) > std_dev(baseline),
        }
    }
}

fn mean_fee(points: &[FeePoint]) -> f64 {
    if points.is_empty() { return 0.0; }
    points.iter().map(|p| p.fee as f64).sum::<f64>() / points.len() as f64
}

fn std_dev(points: &[FeePoint]) -> f64 {
    if points.is_empty() { return 0.0; }
    let mean = mean_fee(points);
    let var = points.iter().map(|p| (p.fee as f64 - mean).powi(2)).sum::<f64>() / points.len() as f64;
    var.sqrt()
}

fn sorted_percentile(points: &[FeePoint], p: u8) -> u64 {
    if points.is_empty() { return 0; }
    let mut fees: Vec<u64> = points.iter().map(|p| p.fee).collect();
    fees.sort_unstable();
    let idx = ((p as f64 / 100.0 * fees.len() as f64).ceil() as usize).saturating_sub(1);
    fees[idx.min(fees.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::simulation::fee_model::FeePoint;

    fn low() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0, fee: 100, ledger: 1, is_spike: false },
            FeePoint { timestamp: 100, fee: 120, ledger: 2, is_spike: false },
        ]
    }

    fn high() -> Vec<FeePoint> {
        vec![
            FeePoint { timestamp: 0, fee: 400, ledger: 1, is_spike: false },
            FeePoint { timestamp: 100, fee: 800, ledger: 2, is_spike: true },
        ]
    }

    #[test]
    fn compare_fields_populated() {
        let r = FeeComparator::compare(&low(), &high(), "jan", "feb");
        assert_eq!(r.label_baseline, "jan");
        assert_eq!(r.count_baseline, 2);
    }

    #[test]
    fn mean_delta_negative_when_target_higher() {
        let r = FeeComparator::compare(&low(), &high(), "a", "b");
        assert!(r.mean_delta < 0.0);
    }

    #[test]
    fn spikes_counted() {
        let r = FeeComparator::compare(&low(), &high(), "a", "b");
        assert_eq!(r.spikes_baseline, 0);
        assert_eq!(r.spikes_target, 1);
    }

    #[test]
    fn identical_datasets_are_similar() {
        let r = FeeComparator::compare(&low(), &low(), "a", "b");
        assert!(r.verdict().contains("similar"));
    }

    #[test]
    fn to_json_contains_keys() {
        let r = FeeComparator::compare(&low(), &high(), "a", "b");
        let json = r.to_json();
        assert!(json.contains("p50_delta"));
        assert!(json.contains("p95_delta"));
    }

    #[test]
    fn empty_baseline_does_not_panic() {
        let r = FeeComparator::compare(&[], &high(), "empty", "b");
        assert_eq!(r.count_baseline, 0);
    }
}
