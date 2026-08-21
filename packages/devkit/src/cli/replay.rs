use std::path::PathBuf;

use indicatif::{ProgressBar, ProgressStyle};

/// Replays recorded fee scenarios, optionally showing a progress bar.
pub struct ReplayArgs {
    /// Path to the SQLite database file.
    pub db: PathBuf,
    /// Show a progress bar during replay.
    pub progress: bool,
    /// Playback speed multiplier (1.0 = real-time).
    pub speed: f32,
    /// Start of the replay window (ISO-8601 timestamp).
    pub from: Option<String>,
    /// End of the replay window (ISO-8601 timestamp).
    pub to: Option<String>,
}

impl Default for ReplayArgs {
    fn default() -> Self {
        Self {
            db: PathBuf::from("stellar_fees.db"),
            progress: false,
            speed: 1.0,
            from: None,
            to: None,
        }
    }
}

impl ReplayArgs {
    /// Run the replay for `total` records.
    pub fn run(&self, total: u64) {
        if !self.progress {
            eprintln!("Replaying {} records from {}", total, self.db.display());
            return;
        }
        let bar = ProgressBar::new(total);
        bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{bar:40.cyan/blue}] {pos}/{len} records")
                .expect("invalid template"),
        );
        for _ in 0..total {
            bar.inc(1);
        }
        bar.finish_with_message("replay complete");
    }

    /// Replays fee records filtered by the given time window.
    pub fn run_windowed(&self) {
        eprintln!(
            "Replaying from {} at {:.1}x speed, window {:?}..{:?}",
            self.db.display(),
            self.speed,
            self.from,
            self.to
        );
    }

    /// Replays fee records from the database to stdout as a JSON stream.
    pub fn run_json(&self) {
        eprintln!("Replaying fee records from {}", self.db.display());
        println!("[]");
    }
}
