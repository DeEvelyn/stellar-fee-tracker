use stellar_devkit::protocol::{estimate_close_time, validate_ledger_sequence, LedgerGap};

#[test]
fn detects_gap_in_sequence() {
    let seqs = vec![1, 2, 3, 5, 6, 10];
    let gaps = validate_ledger_sequence(&seqs);
    assert_eq!(gaps.len(), 2);
    assert_eq!(gaps[0], LedgerGap { from: 3, to: 5 });
    assert_eq!(gaps[1], LedgerGap { from: 6, to: 10 });
}

#[test]
fn no_gaps_in_contiguous_sequence() {
    let seqs = vec![100, 101, 102, 103, 104];
    let gaps = validate_ledger_sequence(&seqs);
    assert!(gaps.is_empty());
}

#[test]
fn empty_sequence_returns_no_gaps() {
    let gaps = validate_ledger_sequence(&[]);
    assert!(gaps.is_empty());
}

#[test]
fn single_element_returns_no_gaps() {
    let gaps = validate_ledger_sequence(&[42]);
    assert!(gaps.is_empty());
}

#[test]
fn unsorted_input_is_handled() {
    let seqs = vec![5, 1, 3, 2, 4];
    let gaps = validate_ledger_sequence(&seqs);
    assert!(gaps.is_empty());
}

#[test]
fn estimate_close_time_returns_future() {
    let estimate = estimate_close_time(100, 200, 5000);
    assert!(estimate > chrono::Utc::now());
}

#[test]
fn estimate_close_time_proportional_to_distance() {
    let short = estimate_close_time(100, 110, 5000);
    let long = estimate_close_time(100, 120, 5000);
    assert!(long > short);
}
