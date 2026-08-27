//! Streaming pipeline primitives for the devkit.
//!
//! ## Stability note
//!
//! `FeeEvent` is defined here as a **temporary stand-in** for the canonical
//! type being introduced by issue #610 (`ibrahimmosouf-png`).  Once #610
//! merges, this definition should be removed and the import updated to point
//! at the upstream location.
//!
//! This module also provides transformers that consume fee events and emit
//! derived events (e.g. spike detection).  Until #610 lands the transformer
//! submodule carries its own minimal local event types to stay self-contained
//! and avoid a merge conflict.

pub mod sink;

pub use sink::StdoutSink;

pub mod transformer;

pub use transformer::{FeeRecord, SpikeDetectionTransformer, SpikeTransformerEvent};

use crate::analysis::spike_classifier::SpikeEvent;

/// Events that flow through the streaming pipeline.
///
/// Each variant carries the minimal data needed for downstream processing:
///
/// - [`FeeEvent::NewFeeRecord`] — references [`FeeRecord`] directly (from
///   `storage::traits`) so the streaming layer shares the canonical storage
///   type without an extra copy struct.
/// - [`FeeEvent::SpikeDetected`] — references [`SpikeEvent`] directly (from
///   `analysis::spike_classifier`) to keep spike metadata co-located with the
///   type that produced it.
/// - [`FeeEvent::LedgerClosed`] — carries the ledger sequence number (`u64`)
///   to signal ledger-boundary events without allocating.
/// - [`FeeEvent::NetworkConditionChanged`] — carries a human-readable
///   description of the condition change as a `String`.
/// - [`FeeEvent::PipelineError`] — carries an error message string so the
///   pipeline can propagate non-fatal errors without panicking.
#[derive(Debug, Clone)]
pub enum FeeEvent {
    /// A new fee record has been observed on the network.
    NewFeeRecord(FeeRecord),
    /// A fee spike has been detected by the spike classifier.
    SpikeDetected(SpikeEvent),
    /// A ledger has closed; carries the ledger sequence number.
    LedgerClosed(u64),
    /// The network condition description has changed.
    NetworkConditionChanged(String),
    /// A non-fatal pipeline error has occurred.
    PipelineError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::spike_classifier::{SpikeEvent, SpikeSeverity};
    use crate::storage::traits::FeeRecord;

    fn sample_fee_record() -> FeeRecord {
        FeeRecord {
            fee_amount: 100,
            ledger_sequence: 42,
            timestamp_ms: 1_700_000_000_000,
            transaction_hash: Some("abc123".to_string()),
            is_spike: false,
            created_at: "2024-01-01T00:00:00Z".to_string(),
        }
    }

    fn sample_spike_event() -> SpikeEvent {
        SpikeEvent {
            severity: SpikeSeverity::High,
            duration_ledgers: 3,
        }
    }

    // ── Construction tests ──────────────────────────────────────────────────

    #[test]
    fn new_fee_record_variant_constructs() {
        let event = FeeEvent::NewFeeRecord(sample_fee_record());
        assert!(matches!(event, FeeEvent::NewFeeRecord(_)));
    }

    #[test]
    fn spike_detected_variant_constructs() {
        let event = FeeEvent::SpikeDetected(sample_spike_event());
        assert!(matches!(event, FeeEvent::SpikeDetected(_)));
    }

    #[test]
    fn ledger_closed_variant_constructs() {
        let event = FeeEvent::LedgerClosed(100);
        assert!(matches!(event, FeeEvent::LedgerClosed(100)));
    }

    #[test]
    fn network_condition_changed_variant_constructs() {
        let event = FeeEvent::NetworkConditionChanged("high congestion".to_string());
        assert!(matches!(event, FeeEvent::NetworkConditionChanged(_)));
    }

    #[test]
    fn pipeline_error_variant_constructs() {
        let event = FeeEvent::PipelineError("timeout".to_string());
        assert!(matches!(event, FeeEvent::PipelineError(_)));
    }

    // ── Debug smoke tests ───────────────────────────────────────────────────

    #[test]
    fn debug_new_fee_record_is_non_empty() {
        let event = FeeEvent::NewFeeRecord(sample_fee_record());
        assert!(!format!("{event:?}").is_empty());
    }

    #[test]
    fn debug_spike_detected_is_non_empty() {
        let event = FeeEvent::SpikeDetected(sample_spike_event());
        assert!(!format!("{event:?}").is_empty());
    }

    #[test]
    fn debug_ledger_closed_is_non_empty() {
        let event = FeeEvent::LedgerClosed(999);
        assert!(!format!("{event:?}").is_empty());
    }

    #[test]
    fn debug_network_condition_changed_is_non_empty() {
        let event = FeeEvent::NetworkConditionChanged("low throughput".to_string());
        assert!(!format!("{event:?}").is_empty());
    }

    #[test]
    fn debug_pipeline_error_is_non_empty() {
        let event = FeeEvent::PipelineError("connection reset".to_string());
        assert!(!format!("{event:?}").is_empty());
    }

    // ── Clone test ──────────────────────────────────────────────────────────

    #[test]
    fn fee_event_is_clone() {
        let original = FeeEvent::LedgerClosed(7);
        let cloned = original.clone();
        assert!(matches!(cloned, FeeEvent::LedgerClosed(7)));
    }
}
