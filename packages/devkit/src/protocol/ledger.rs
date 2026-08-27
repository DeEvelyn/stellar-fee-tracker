use chrono::{DateTime, Duration, Utc};

/// A detected gap in a ledger sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerGap {
    pub from: u64,
    pub to: u64,
}

/// Validate that a sequence of ledger numbers is contiguous without gaps.
pub fn validate_ledger_sequence(sequences: &[u64]) -> Vec<LedgerGap> {
    if sequences.len() < 2 {
        return Vec::new();
    }

    let mut sorted = sequences.to_vec();
    sorted.sort_unstable();

    let mut gaps = Vec::new();
    for window in sorted.windows(2) {
        if let Some(diff) = window[1].checked_sub(window[0]) {
            if diff > 1 {
                gaps.push(LedgerGap {
                    from: window[0],
                    to: window[1],
                });
            }
        }
    }
    gaps
}

/// Estimate when a future ledger will close based on average close time.
pub fn estimate_close_time(
    current_ledger: u64,
    target_ledger: u64,
    avg_close_time_ms: u64,
) -> DateTime<Utc> {
    let ledgers_to_go = target_ledger.saturating_sub(current_ledger) as i64;
    let ms_to_go = ledgers_to_go * avg_close_time_ms as i64;
    Utc::now() + Duration::milliseconds(ms_to_go)
}
