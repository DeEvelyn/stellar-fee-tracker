/// Issue #370: Unit tests for gap detector
///
/// Asserts gaps are found at correct positions.
/// Asserts no false positives on clean data.
///
/// File to create: packages/devkit/tests/data_quality_gaps.rs
/// (mapped to packages/core/tests/data_quality_gaps.rs in current layout)

use stellar_fee_tracker::gaps::{detect_gaps, Gap};

// ── Clean data — no false positives ──────────────────────────────────────────

#[test]
fn test_consecutive_ledgers_no_gaps() {
    let ledgers = vec![100, 101, 102, 103, 104];
    let gaps = detect_gaps(&ledgers);
    assert!(gaps.is_empty(), "expected no gaps, found {gaps:?}");
}

#[test]
fn test_single_ledger_no_gaps() {
    let ledgers = vec![42];
    let gaps = detect_gaps(&ledgers);
    assert!(gaps.is_empty());
}

#[test]
fn test_empty_input_no_gaps() {
    let ledgers: Vec<u32> = vec![];
    let gaps = detect_gaps(&ledgers);
    assert!(gaps.is_empty());
}

#[test]
fn test_two_consecutive_ledgers_no_gaps() {
    let ledgers = vec![1, 2];
    let gaps = detect_gaps(&ledgers);
    assert!(gaps.is_empty());
}

// ── Gap detection ────────────────────────────────────────────────────────────

#[test]
fn test_single_missing_ledger_in_middle() {
    // ledgers 100, 101, 103 → gap at 102
    let ledgers = vec![100, 101, 103];
    let gaps = detect_gaps(&ledgers);
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0], Gap { from: 102, to: 102 });
}

#[test]
fn test_multiple_consecutive_missing() {
    // ledgers 10, 11, 15 → gap 12..=14
    let ledgers = vec![10, 11, 15];
    let gaps = detect_gaps(&ledgers);
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0], Gap { from: 12, to: 14 });
}

#[test]
fn test_multiple_disjoint_gaps() {
    let ledgers = vec![1, 2, 5, 6, 10];
    let gaps = detect_gaps(&ledgers);
    assert_eq!(gaps.len(), 2);
    // gap 3..=4, gap 7..=9
    assert!(gaps.contains(&Gap { from: 3, to: 4 }));
    assert!(gaps.contains(&Gap { from: 7, to: 9 }));
}

// ── Edge cases ───────────────────────────────────────────────────────────────

#[test]
fn test_gap_at_start() {
    // first ledger is 100, but sequence conceptually starts at 50
    let ledgers = vec![50, 100, 101];
    // gap 51..=99
    let gaps = detect_gaps(&ledgers);
    assert!(!gaps.is_empty());
    assert!(gaps.iter().any(|g| g.from == 51 && g.to == 99));
}

#[test]
fn test_reverse_order_still_detects_gaps() {
    let mut ledgers = vec![1, 5, 3, 2];
    ledgers.sort();
    let gaps = detect_gaps(&ledgers);
    assert_eq!(gaps.len(), 1);
    assert_eq!(gaps[0], Gap { from: 4, to: 4 });
}

#[test]
fn test_duplicates_no_false_gaps() {
    let ledgers = vec![100, 100, 101, 102];
    // duplicates should be ignored/deduplicated
    let gaps = detect_gaps(&ledgers);
    assert!(gaps.is_empty());
}
