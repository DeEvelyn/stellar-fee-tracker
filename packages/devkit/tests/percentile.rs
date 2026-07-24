use stellar_devkit::analysis::percentile::Percentile;

// ── Issue #139: nearest-rank percentile tests ────────────────────────────────

#[test]
fn nearest_rank_p10_p50_p90_p95_p99() {
    let data: Vec<u64> = (1..=100).collect();
    assert_eq!(Percentile::nearest_rank(&data, 10), 10);
    assert_eq!(Percentile::nearest_rank(&data, 50), 50);
    assert_eq!(Percentile::nearest_rank(&data, 90), 90);
    assert_eq!(Percentile::nearest_rank(&data, 95), 95);
    assert_eq!(Percentile::nearest_rank(&data, 99), 99);
}

#[test]
fn nearest_rank_single_element() {
    assert_eq!(Percentile::nearest_rank(&[42], 50), 42);
    assert_eq!(Percentile::nearest_rank(&[42], 1), 42);
    assert_eq!(Percentile::nearest_rank(&[42], 100), 42);
}

#[test]
fn nearest_rank_empty_returns_zero() {
    assert_eq!(Percentile::nearest_rank(&[], 50), 0);
}

#[test]
fn nearest_rank_small_dataset() {
    let data = [10u64, 20, 30, 40, 50];
    assert_eq!(Percentile::nearest_rank(&data, 1), 10);
    assert_eq!(Percentile::nearest_rank(&data, 50), 30);
    assert_eq!(Percentile::nearest_rank(&data, 100), 50);
}

// ── Issue #140: interpolation percentile tests ───────────────────────────────

#[test]
fn interpolation_p10_p50_p90_p95_p99() {
    let data: Vec<u64> = (1..=100).collect();
    assert_eq!(Percentile::linear_interpolation(&data, 10), 10);
    assert_eq!(Percentile::linear_interpolation(&data, 50), 50);
    assert_eq!(Percentile::linear_interpolation(&data, 90), 90);
    assert_eq!(Percentile::linear_interpolation(&data, 95), 95);
    assert_eq!(Percentile::linear_interpolation(&data, 99), 99);
}

#[test]
fn interpolation_boundaries() {
    let data = [10u64, 20, 30, 40, 50];
    assert_eq!(Percentile::linear_interpolation(&data, 0), 10);
    assert_eq!(Percentile::linear_interpolation(&data, 100), 50);
}

#[test]
fn interpolation_midpoint_is_average() {
    // p50 of [10, 20] should interpolate to 15
    let data = [10u64, 20];
    assert_eq!(Percentile::linear_interpolation(&data, 50), 15);
}

#[test]
fn interpolation_single_element() {
    assert_eq!(Percentile::linear_interpolation(&[7], 50), 7);
}

#[test]
fn interpolation_empty_returns_zero() {
    assert_eq!(Percentile::linear_interpolation(&[], 50), 0);
}

/// Nearest-rank and interpolation agree on exact-rank positions.
#[test]
fn nearest_rank_vs_interpolation_same_on_exact_ranks() {
    let data: Vec<u64> = (1..=10).collect();
    for p in [10usize, 20, 30, 40, 50, 60, 70, 80, 90, 100] {
        let nr = Percentile::nearest_rank(&data, p);
        let li = Percentile::linear_interpolation(&data, p);
        // They may differ by at most 1 due to rounding; document the difference.
        assert!(
            nr.abs_diff(li) <= 1,
            "p{p}: nearest_rank={nr}, linear_interpolation={li}, diff > 1"
        );
    }
}

// ── Issue #177: p0/p25/p50/p75/p100 and single-element coverage ───────────────

#[test]
fn nearest_rank_p0_p25_p50_p75_p100_known_slice() {
    // Sorted slice [10, 20, 30, 40, 50, 60, 70, 80]
    let data = [10u64, 20, 30, 40, 50, 60, 70, 80];
    assert_eq!(Percentile::nearest_rank(&data, 0), 10);
    assert_eq!(Percentile::nearest_rank(&data, 25), 20);
    assert_eq!(Percentile::nearest_rank(&data, 50), 40);
    assert_eq!(Percentile::nearest_rank(&data, 75), 60);
    assert_eq!(Percentile::nearest_rank(&data, 100), 80);
}

#[test]
fn interpolation_p0_p25_p50_p75_p100_known_slice() {
    let data = [10u64, 20, 30, 40, 50, 60, 70, 80];
    assert_eq!(Percentile::linear_interpolation(&data, 0), 10);
    assert_eq!(Percentile::linear_interpolation(&data, 100), 80);
    // p50 should be between 40 and 50
    let p50 = Percentile::linear_interpolation(&data, 50);
    assert!((40..=50).contains(&p50), "p50={p50} not in [40,50]");
}

#[test]
fn nearest_rank_single_element_all_percentiles() {
    assert_eq!(Percentile::nearest_rank(&[99], 0), 99);
    assert_eq!(Percentile::nearest_rank(&[99], 25), 99);
    assert_eq!(Percentile::nearest_rank(&[99], 75), 99);
    assert_eq!(Percentile::nearest_rank(&[99], 100), 99);
}

#[test]
fn interpolation_single_element_all_percentiles() {
    assert_eq!(Percentile::linear_interpolation(&[42], 0), 42);
    assert_eq!(Percentile::linear_interpolation(&[42], 25), 42);
    assert_eq!(Percentile::linear_interpolation(&[42], 75), 42);
    assert_eq!(Percentile::linear_interpolation(&[42], 100), 42);
}
