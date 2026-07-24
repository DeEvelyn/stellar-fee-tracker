use stellar_devkit::analysis::rolling_window::RollingWindow;

// ── SMA ──────────────────────────────────────────────────────────────────────

#[test]
fn sma_basic() {
    let fees = [1.0, 2.0, 3.0, 4.0, 5.0];
    let result = RollingWindow::sma(&fees, 3);
    assert_eq!(result.len(), 3);
    assert!((result[0] - 2.0).abs() < 1e-9);
    assert!((result[1] - 3.0).abs() < 1e-9);
    assert!((result[2] - 4.0).abs() < 1e-9);
}

#[test]
fn sma_window_equals_len() {
    let fees = [10.0, 20.0, 30.0];
    let result = RollingWindow::sma(&fees, 3);
    assert_eq!(result.len(), 1);
    assert!((result[0] - 20.0).abs() < 1e-9);
}

#[test]
fn sma_window_larger_than_slice_returns_empty() {
    assert!(RollingWindow::sma(&[1.0, 2.0], 5).is_empty());
}

// ── EMA ──────────────────────────────────────────────────────────────────────

#[test]
fn ema_length_matches_input() {
    let fees = [1.0, 2.0, 3.0, 4.0, 5.0];
    assert_eq!(RollingWindow::ema(&fees, 0.5).len(), fees.len());
}

#[test]
fn ema_alpha_one_equals_input() {
    let fees = [10.0, 20.0, 30.0];
    let result = RollingWindow::ema(&fees, 1.0);
    for (r, &f) in result.iter().zip(fees.iter()) {
        assert!((r - f).abs() < 1e-9);
    }
}

#[test]
fn ema_known_sequence() {
    // alpha=0.5, seed=10: 10, 0.5*20+0.5*10=15, 0.5*30+0.5*15=22.5
    let fees = [10.0, 20.0, 30.0];
    let result = RollingWindow::ema(&fees, 0.5);
    assert!((result[0] - 10.0).abs() < 1e-9);
    assert!((result[1] - 15.0).abs() < 1e-9);
    assert!((result[2] - 22.5).abs() < 1e-9);
}

#[test]
fn ema_empty_returns_empty() {
    assert!(RollingWindow::ema(&[], 0.5).is_empty());
}

// ── WMA ──────────────────────────────────────────────────────────────────────

#[test]
fn wma_basic() {
    // window=3: weights 1,2,3 / denom=6
    // [1,2,3] -> (1+4+9)/6 = 14/6 ≈ 2.333
    // [2,3,4] -> (2+6+12)/6 = 20/6 ≈ 3.333
    let fees = [1.0, 2.0, 3.0, 4.0];
    let result = RollingWindow::wma(&fees, 3);
    assert_eq!(result.len(), 2);
    assert!((result[0] - 14.0 / 6.0).abs() < 1e-9);
    assert!((result[1] - 20.0 / 6.0).abs() < 1e-9);
}

#[test]
fn wma_window_larger_than_slice_returns_empty() {
    assert!(RollingWindow::wma(&[1.0, 2.0], 5).is_empty());
}

#[test]
fn wma_most_recent_weighted_highest() {
    // With a spike at the end, WMA should be higher than SMA
    let fees = [100.0, 100.0, 1000.0];
    let wma = RollingWindow::wma(&fees, 3);
    let sma = RollingWindow::sma(&fees, 3);
    assert!(wma[0] > sma[0]);
}

// ── Issue #177: stateful RollingWindow tests ──────────────────────────────────

#[test]
fn push_returns_none_until_window_full() {
    let mut rw = RollingWindow::new(3);
    assert!(rw.push(1.0).is_none());
    assert!(rw.push(2.0).is_none());
    // Third push fills the window
    let sma = rw.push(3.0);
    assert!(sma.is_some());
    assert!((sma.unwrap() - 2.0).abs() < 1e-9);
}

#[test]
fn push_beyond_capacity_evicts_oldest() {
    let mut rw = RollingWindow::new(3);
    rw.push(10.0);
    rw.push(20.0);
    rw.push(30.0); // window full: [10, 20, 30], sma = 20
                   // Push 40 — evicts 10 → [20, 30, 40], sma = 30
    let sma = rw.push(40.0);
    assert!(sma.is_some());
    assert!((sma.unwrap() - 30.0).abs() < 1e-9);
}

#[test]
fn push_window_of_one_always_returns_that_value() {
    let mut rw = RollingWindow::new(1);
    let v1 = rw.push(5.0);
    assert!(v1.is_some());
    assert!((v1.unwrap() - 5.0).abs() < 1e-9);
    let v2 = rw.push(99.0);
    assert!(v2.is_some());
    assert!((v2.unwrap() - 99.0).abs() < 1e-9);
}

#[test]
fn sma_empty_slice_returns_empty() {
    assert!(RollingWindow::sma(&[], 3).is_empty());
}

#[test]
fn sma_sum_and_len_accuracy() {
    // sma([1,2,3,4,5], window=2): [1.5, 2.5, 3.5, 4.5]
    let fees = [1.0, 2.0, 3.0, 4.0, 5.0];
    let result = RollingWindow::sma(&fees, 2);
    assert_eq!(result.len(), 4);
    assert!((result[0] - 1.5).abs() < 1e-9);
    assert!((result[1] - 2.5).abs() < 1e-9);
    assert!((result[2] - 3.5).abs() < 1e-9);
    assert!((result[3] - 4.5).abs() < 1e-9);
}
