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

pub use transformer::{FeeRecord, SpikeTransformerEvent, SpikeDetectionTransformer};

use serde::{Deserialize, Serialize};

/// Placeholder `FeeEvent` enum for use until issue #610 merges.
///
/// **This type will be superseded** by the definition introduced in #610.
/// It is kept minimal here to allow `sink.rs` to be exercised end-to-end
/// without coupling to the concurrent PR.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum FeeEvent {
    /// A new fee record was recorded for a ledger.
    NewFeeRecord {
        fee_amount: u64,
        ledger_sequence: u64,
        timestamp_ms: i64,
        transaction_hash: Option<String>,
        is_spike: bool,
    },
    /// A fee spike was detected.
    SpikeDetected {
        severity: String,
        duration_ledgers: usize,
    },
    /// A ledger was closed at the given sequence number.
    LedgerClosed(u64),
    /// The network condition description changed.
    NetworkConditionChanged(String),
    /// A pipeline processing error occurred.
    PipelineError(String),
}
