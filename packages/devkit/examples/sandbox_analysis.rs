//! Example: Running analysis on sandbox fixture data
//!
//! This example demonstrates how to use the sandbox to run percentile
//! and spike analysis on a congested fixture.

use stellar_devkit::analysis::percentile::{fee_distribution_summary, percentile_nearest_rank};
use stellar_devkit::analysis::spike_classifier::SpikeClassifier;
use stellar_devkit::sandbox::fixtures::congested_network;

fn main() {
    println!("=== Sandbox Analysis Example ===\n");

    // Generate congested fixture data
    let fees = congested_network();
    println!("Generated {} fee records", fees.len());

    let values: Vec<u64> = fees.iter().map(|(_, f)| *f).collect();

    // Compute percentiles
    let p50 = percentile_nearest_rank(&values, 50.0);
    let p95 = percentile_nearest_rank(&values, 95.0);
    let p99 = percentile_nearest_rank(&values, 99.0);

    println!("\n--- Fee Distribution ---");
    println!(
        "p50: {} stroops ({:.7} XLM)",
        p50,
        p50 as f64 / 10_000_000.0
    );
    println!(
        "p95: {} stroops ({:.7} XLM)",
        p95,
        p95 as f64 / 10_000_000.0
    );
    println!(
        "p99: {} stroops ({:.7} XLM)",
        p99,
        p99 as f64 / 10_000_000.0
    );

    // Run distribution summary
    let summary = fee_distribution_summary(&values);
    println!("\n--- Distribution Summary ---");
    println!("Mean: {:.2} stroops", summary.mean);
    println!("Std Dev: {:.2} stroops", summary.std_dev);

    // Classify spikes
    let classifier = SpikeClassifier::default();
    let spikes: Vec<_> = fees
        .iter()
        .filter(|(_, f)| {
            classifier.classify(*f)
                != stellar_devkit::analysis::spike_classifier::SpikeSeverity::Low
        })
        .collect();

    println!("\n--- Spike Analysis ---");
    println!("Detected {} spikes", spikes.len());

    println!("\n=== Analysis Complete ===");
}
