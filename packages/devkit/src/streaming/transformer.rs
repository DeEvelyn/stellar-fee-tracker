//! Spike-detection pipeline transformer.
//!
//! Consumes a [`FeeEvent::NewFeeRecord`] event and, when the fee amount
//! exceeds `baseline * threshold`, emits a [`SpikeTransformerEvent::SpikeDetected`]
//! carrying the full [`SpikeEvent`] produced by [`SpikeClassifier`].
//!
//! # Dependency note
//! Issue #610 (ibrahimmosouf-png) will introduce the canonical `FeeEvent` and
//! `FeeRecord` types in a shared streaming events module.  Until that PR is
//! merged this file carries **minimal local definitions** of `FeeRecord` and a
//! local `FeeEvent` analogue (`SpikeTransformerEvent`) to avoid a conflict.
//! Once #610 lands:
//!   1. Remove the local `FeeRecord` struct (import from the shared module).
//!   2. Replace `SpikeTransformerEvent` with the canonical `FeeEvent`.
//!   3. Adapt `SpikeDetectionTransformer::transform` accordingly.

use crate::analysis::spike_classifier::{SpikeClassifier, SpikeEvent, SpikeSeverity};

// ---------------------------------------------------------------------------
// Minimal local event types (to be replaced when #610 merges)
// ---------------------------------------------------------------------------

/// Canonical fee-record type shared by the storage layer.
///
/// The transformer uses the canonical [`crate::storage::traits::FeeRecord`]
/// (re-exported below) now that the shared streaming events landed in #610.
pub use crate::storage::traits::FeeRecord;

/// Local analogue of the canonical `FeeEvent` from issue #610.
///
/// Only the variants required by this transformer are represented.
/// **TODO (#610)**: collapse into the canonical `FeeEvent` enum.
#[derive(Debug, Clone)]
pub enum FeeEvent {
    /// A new fee record arrived from the polling loop.
    NewFeeRecord(FeeRecord),
    /// A spike was detected by the transformer and is propagated downstream.
    SpikeDetected(SpikeEvent),
    /// A ledger was closed (sequence number).
    LedgerClosed(u64),
    /// The network condition label changed.
    NetworkConditionChanged(String),
    /// An error occurred inside the pipeline.
    PipelineError(String),
}

/// Output events emitted by [`SpikeDetectionTransformer`].
///
/// This type is intentionally distinct from `FeeEvent` so it can be merged
/// cleanly alongside #610 without a name clash.
#[derive(Debug, Clone)]
pub enum SpikeTransformerEvent {
    /// A spike was detected; carries the classified [`SpikeEvent`].
    SpikeDetected(SpikeEvent),
    /// The input record did not exceed the spike threshold; passed through.
    NoSpike,
}

// ---------------------------------------------------------------------------
// Transformer
// ---------------------------------------------------------------------------

/// Synchronous pipeline transformer that detects fee spikes.
///
/// # Construction
///
/// ```rust
/// use stellar_devkit::streaming::SpikeDetectionTransformer;
///
/// // baseline = 200 stroops, threshold = 2.0× (i.e. any fee > 400 stroops triggers)
/// let transformer = SpikeDetectionTransformer::new(200, 2.0);
/// ```
///
/// # Usage
///
/// ```rust
/// use stellar_devkit::streaming::{SpikeDetectionTransformer, FeeRecord};
/// use stellar_devkit::streaming::transformer::FeeEvent;
/// use stellar_devkit::streaming::SpikeTransformerEvent;
///
/// let transformer = SpikeDetectionTransformer::new(200, 2.0);
///
/// let record = FeeRecord {
///     fee_amount: 500,
///     ledger_sequence: 42,
///     timestamp_ms: 1_700_000_000_000,
///     transaction_hash: None,
///     is_spike: false,
///     created_at: "2024-01-01T00:00:00Z".to_string(),
/// };
///
/// match transformer.transform(FeeEvent::NewFeeRecord(record)) {
///     Some(SpikeTransformerEvent::SpikeDetected(spike)) => {
///         println!("Spike! severity={:?}", spike.severity);
///     }
///     Some(SpikeTransformerEvent::NoSpike) => {}
///     None => {} // non-NewFeeRecord events are ignored
/// }
/// ```
pub struct SpikeDetectionTransformer {
    /// Baseline fee in stroops.  Used as the denominator for ratio calculation.
    baseline: u64,
    /// Multiplier threshold.  A fee must exceed `baseline * threshold` to be
    /// classified as a spike.  For example, `threshold = 2.0` means any fee
    /// more than twice the baseline triggers a spike event.
    threshold: f64,
}

impl SpikeDetectionTransformer {
    /// Create a new transformer.
    ///
    /// # Arguments
    ///
    /// * `baseline`  — the expected (normal) fee in stroops.
    /// * `threshold` — the multiplier above which a fee is considered a spike
    ///   (e.g. `2.0` = 2× the baseline).
    ///
    /// # Panics
    ///
    /// Does not panic.  A `baseline` of `0` means spikes are never emitted
    /// (division by zero is avoided inside [`SpikeClassifier`]).
    pub fn new(baseline: u64, threshold: f64) -> Self {
        Self {
            baseline,
            threshold,
        }
    }

    /// Process a single [`FeeEvent`] and optionally emit a
    /// [`SpikeTransformerEvent`].
    ///
    /// * If the event is `FeeEvent::NewFeeRecord` and the fee exceeds
    ///   `baseline * threshold`, returns
    ///   `Some(SpikeTransformerEvent::SpikeDetected(...))`.
    /// * If the record does not exceed the threshold, returns
    ///   `Some(SpikeTransformerEvent::NoSpike)`.
    /// * For all other event variants, returns `None` (pass-through /
    ///   not-applicable).
    pub fn transform(&self, event: FeeEvent) -> Option<SpikeTransformerEvent> {
        match event {
            FeeEvent::NewFeeRecord(record) => {
                let multiplier = if self.baseline == 0 {
                    return Some(SpikeTransformerEvent::NoSpike);
                } else {
                    record.fee_amount as f64 / self.baseline as f64
                };

                if multiplier > self.threshold {
                    // Delegate severity classification to SpikeClassifier.
                    let severity =
                        SpikeClassifier::classify_with_baseline(record.fee_amount, self.baseline)
                            .unwrap_or(SpikeSeverity::Low);

                    Some(SpikeTransformerEvent::SpikeDetected(SpikeEvent {
                        severity,
                        // Single-record events represent a run of 1 ledger.
                        duration_ledgers: 1,
                    }))
                } else {
                    Some(SpikeTransformerEvent::NoSpike)
                }
            }
            // Non-record events are not processed by this transformer.
            _ => None,
        }
    }

    /// Batch-transform a slice of fee `(timestamp, amount)` tuples.
    ///
    /// Delegates to [`SpikeClassifier::detect_with_threshold`] for efficient
    /// batch processing and returns one [`SpikeEvent`] per detected spike.
    ///
    /// # Arguments
    ///
    /// * `fees` — slice of `(timestamp_ms, fee_amount)` pairs.
    pub fn transform_batch(&self, fees: &[(u64, u64)]) -> Vec<SpikeEvent> {
        SpikeClassifier::detect_with_threshold(fees, self.baseline, self.threshold)
            .into_iter()
            .map(|tse| SpikeEvent {
                severity: tse.severity,
                duration_ledgers: 1,
            })
            .collect()
    }
}

// ---------------------------------------------------------------------------
// Inline tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn make_record(fee_amount: u64) -> FeeRecord {
        FeeRecord {
            fee_amount,
            ledger_sequence: 1,
            timestamp_ms: 1_700_000_000_000,
            transaction_hash: None,
            is_spike: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    // ------------------------------------------------------------------
    // Basic spike / no-spike detection
    // ------------------------------------------------------------------

    #[test]
    fn spike_detected_when_fee_exceeds_threshold() {
        let t = SpikeDetectionTransformer::new(200, 2.0);
        // 500 > 200 * 2.0 = 400 → spike
        let result = t.transform(FeeEvent::NewFeeRecord(make_record(500)));
        assert!(
            matches!(result, Some(SpikeTransformerEvent::SpikeDetected(_))),
            "Expected SpikeDetected for fee=500, baseline=200, threshold=2.0"
        );
    }

    #[test]
    fn no_spike_when_fee_below_threshold() {
        let t = SpikeDetectionTransformer::new(200, 2.0);
        // 300 < 200 * 2.0 = 400 → no spike
        let result = t.transform(FeeEvent::NewFeeRecord(make_record(300)));
        assert!(
            matches!(result, Some(SpikeTransformerEvent::NoSpike)),
            "Expected NoSpike for fee=300, baseline=200, threshold=2.0"
        );
    }

    #[test]
    fn exact_threshold_boundary_is_not_a_spike() {
        let t = SpikeDetectionTransformer::new(200, 2.0);
        // 400 == 200 * 2.0 → NOT strictly greater, so NoSpike
        let result = t.transform(FeeEvent::NewFeeRecord(make_record(400)));
        assert!(
            matches!(result, Some(SpikeTransformerEvent::NoSpike)),
            "Expected NoSpike at the exact threshold boundary"
        );
    }

    // ------------------------------------------------------------------
    // Severity classification
    // ------------------------------------------------------------------

    #[test]
    fn spike_severity_low() {
        let t = SpikeDetectionTransformer::new(100, 1.5);
        // fee=250 → ratio=2.5 → Low (2–5×)
        if let Some(SpikeTransformerEvent::SpikeDetected(spike)) =
            t.transform(FeeEvent::NewFeeRecord(make_record(250)))
        {
            assert_eq!(spike.severity, SpikeSeverity::Low);
        } else {
            panic!("Expected SpikeDetected");
        }
    }

    #[test]
    fn spike_severity_medium() {
        let t = SpikeDetectionTransformer::new(100, 1.5);
        // fee=600 → ratio=6.0 → Medium (5–10×)
        if let Some(SpikeTransformerEvent::SpikeDetected(spike)) =
            t.transform(FeeEvent::NewFeeRecord(make_record(600)))
        {
            assert_eq!(spike.severity, SpikeSeverity::Medium);
        } else {
            panic!("Expected SpikeDetected");
        }
    }

    #[test]
    fn spike_severity_high() {
        let t = SpikeDetectionTransformer::new(100, 1.5);
        // fee=1500 → ratio=15.0 → High (10–50×)
        if let Some(SpikeTransformerEvent::SpikeDetected(spike)) =
            t.transform(FeeEvent::NewFeeRecord(make_record(1500)))
        {
            assert_eq!(spike.severity, SpikeSeverity::High);
        } else {
            panic!("Expected SpikeDetected");
        }
    }

    #[test]
    fn spike_severity_critical() {
        let t = SpikeDetectionTransformer::new(100, 1.5);
        // fee=6000 → ratio=60.0 → Critical (>50×)
        if let Some(SpikeTransformerEvent::SpikeDetected(spike)) =
            t.transform(FeeEvent::NewFeeRecord(make_record(6000)))
        {
            assert_eq!(spike.severity, SpikeSeverity::Critical);
        } else {
            panic!("Expected SpikeDetected");
        }
    }

    // ------------------------------------------------------------------
    // Non-record event pass-through
    // ------------------------------------------------------------------

    #[test]
    fn non_record_events_return_none() {
        let t = SpikeDetectionTransformer::new(200, 2.0);
        assert!(t.transform(FeeEvent::LedgerClosed(42)).is_none());
        assert!(t
            .transform(FeeEvent::NetworkConditionChanged("high".to_string()))
            .is_none());
        assert!(t
            .transform(FeeEvent::PipelineError("oops".to_string()))
            .is_none());
    }

    // ------------------------------------------------------------------
    // Configurable threshold
    // ------------------------------------------------------------------

    #[test]
    fn configurable_threshold_higher() {
        // threshold=10.0 → fee must be >10× baseline
        let t = SpikeDetectionTransformer::new(100, 10.0);
        // fee=500 → 5× → NoSpike
        assert!(matches!(
            t.transform(FeeEvent::NewFeeRecord(make_record(500))),
            Some(SpikeTransformerEvent::NoSpike)
        ));
        // fee=1100 → 11× → SpikeDetected
        assert!(matches!(
            t.transform(FeeEvent::NewFeeRecord(make_record(1100))),
            Some(SpikeTransformerEvent::SpikeDetected(_))
        ));
    }

    #[test]
    fn zero_baseline_returns_no_spike() {
        let t = SpikeDetectionTransformer::new(0, 2.0);
        let result = t.transform(FeeEvent::NewFeeRecord(make_record(99999)));
        assert!(
            matches!(result, Some(SpikeTransformerEvent::NoSpike)),
            "Zero baseline must not produce spikes (avoids div-by-zero)"
        );
    }

    // ------------------------------------------------------------------
    // Batch transform
    // ------------------------------------------------------------------

    #[test]
    fn batch_transform_detects_multiple_spikes() {
        let t = SpikeDetectionTransformer::new(100, 2.0);
        let fees: Vec<(u64, u64)> = vec![
            (1_000, 100), // below threshold
            (2_000, 250), // above threshold → spike
            (3_000, 90),  // below threshold
            (4_000, 400), // above threshold → spike
        ];
        let spikes = t.transform_batch(&fees);
        assert_eq!(spikes.len(), 2, "Expected exactly 2 spikes");
    }

    #[test]
    fn batch_transform_empty_input() {
        let t = SpikeDetectionTransformer::new(100, 2.0);
        assert!(t.transform_batch(&[]).is_empty());
    }

    // ------------------------------------------------------------------
    // duration_ledgers is always 1 for single-record events
    // ------------------------------------------------------------------

    #[test]
    fn single_record_spike_has_duration_one() {
        let t = SpikeDetectionTransformer::new(200, 2.0);
        if let Some(SpikeTransformerEvent::SpikeDetected(spike)) =
            t.transform(FeeEvent::NewFeeRecord(make_record(500)))
        {
            assert_eq!(spike.duration_ledgers, 1);
        } else {
            panic!("Expected SpikeDetected");
        }
    }
}
