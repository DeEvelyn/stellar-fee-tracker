pub mod benchmark;
pub mod completions;
pub mod config;
pub mod export;
pub mod replay;
pub mod version;

use clap::{Parser, Subcommand};

pub use completions::{CompletionsArgs, ShellType};
pub use config::ConfigArgs;
pub use version::VersionArgs;

/// Arguments for the `simulate` subcommand.
pub struct SimulateArgs {
    /// Base fee floor in stroops.
    pub base_fee: u64,
    /// Probability of a fee spike on any given ledger [0.0, 1.0].
    pub spike_prob: f64,
    /// Number of ledgers to simulate.
    pub duration: u64,
}

impl Default for SimulateArgs {
    fn default() -> Self {
        Self { base_fee: 100, spike_prob: 0.05, duration: 1_000 }
    }
}

impl SimulateArgs {
    /// Run the fee simulation and stream results to stdout.
    pub fn run(&self) {
        println!(
            "Simulating {} ledgers | base_fee={} stroops | spike_prob={:.2}",
            self.duration, self.base_fee, self.spike_prob
        );
    }
}

/// Arguments for the `mock` subcommand.
pub struct MockArgs {
    /// Scenario to load (e.g. "normal", "congested", "spike").
    pub scenario: String,
    /// Port the mock Horizon server listens on.
    pub port: u16,
}

impl Default for MockArgs {
    fn default() -> Self {
        Self { scenario: String::from("normal"), port: 8090 }
    }
}

impl MockArgs {
    /// Start the mock Horizon server with the configured scenario.
    pub fn run(&self) {
        println!(
            "Starting mock Horizon server on :{} with scenario '{}'",
            self.port, self.scenario
        );
    }
}

// ---------------------------------------------------------------------------
// #403: Piped-input helpers for validate and inspect subcommands
// ---------------------------------------------------------------------------

/// Input source for subcommands that accept fee data (file or stdin pipe).
#[derive(Debug, Clone)]
pub enum InputSource {
    /// Read from a file at the given path.
    File(std::path::PathBuf),
    /// Read from stdin (piped input).
    Stdin,
}

impl Default for InputSource {
    fn default() -> Self {
        Self::Stdin
    }
}

impl InputSource {
    /// Detect whether data is being piped into the process.
    ///
    /// Returns `true` when stdin is not a terminal (i.e., it has been redirected
    /// or data is being piped into the process).
    pub fn is_piped() -> bool {
        // atty is not a dependency; use a simple heuristic: check if
        // stdin has data by testing the fd type via std libc metadata.
        // Safer portable approach: fall back to checking an env hint.
        // In practice callers pass `InputSource::Stdin` explicitly when piping.
        !std::io::IsTerminal::is_terminal(&std::io::stdin())
    }

    /// Load fee points using the CSV reader.
    pub fn load_csv(
        &self,
    ) -> Result<crate::utilities::csv_reader::CsvReadResult, std::io::Error> {
        match self {
            Self::File(path) => crate::utilities::csv_reader::read_csv_file(path),
            Self::Stdin => Ok(crate::utilities::csv_reader::read_csv_stdin()),
        }
    }

    /// Load fee points using the JSON reader.
    pub fn load_json(
        &self,
    ) -> Result<Vec<crate::simulation::fee_model::FeePoint>, crate::utilities::json_reader::JsonReadError>
    {
        match self {
            Self::File(path) => crate::utilities::json_reader::read_json_file(path),
            Self::Stdin => crate::utilities::json_reader::read_json_stdin(),
        }
    }
}

/// Arguments for the `validate` subcommand.
///
/// Supports piped input: `cat fees.csv | devkit validate --format csv`
pub struct ValidateArgs {
    /// Input source (file path or stdin).
    pub input: InputSource,
    /// Format of the input data: "csv" or "json".
    pub format: String,
    /// Output format for the report: "text" or "json".
    pub output_format: String,
// ── validate (issue #391) ────────────────────────────────────────────────────

/// Arguments for the `validate` subcommand.
///
/// Runs data quality checks on a fee CSV or JSON file and prints a quality
/// report. Output format is controlled by `--output-format`.
///
/// # Usage
/// ```text
/// devkit validate --file fees.csv --format csv
/// devkit validate --file fees.json --format json --output-format json
/// ```
pub struct ValidateArgs {
    /// Path to the fee data file.
    pub file: std::path::PathBuf,
    /// Input format: "csv" or "json".
    pub format: String,
    /// Output format: "text" (default) or "json".
    pub output_format: String,
    /// Suppress all output except errors and the final result.
    pub quiet: bool,
}

impl Default for ValidateArgs {
    fn default() -> Self {
        Self {
            input: InputSource::Stdin,
            format: "csv".to_string(),
            output_format: "text".to_string(),
            file: std::path::PathBuf::from("fees.csv"),
            format: "csv".into(),
            output_format: "text".into(),
            quiet: false,
        }
    }
}

impl ValidateArgs {
    /// Run the validate subcommand, printing a data quality report.
    pub fn run(&self) {
        let points = match self.format.as_str() {
            "csv" => {
                match self.input.load_csv() {
                    Ok(result) => {
                        if result.skipped > 0 {
                            eprintln!("Warning: {} malformed rows were skipped", result.skipped);
                        }
                        result.points
                    }
                    Err(e) => {
                        eprintln!("Error reading CSV: {}", e);
                        std::process::exit(1);
                    }
                }
            }
            "json" => match self.input.load_json() {
                Ok(pts) => pts,
                Err(e) => {
                    eprintln!("Error reading JSON: {}", e);
                    std::process::exit(1);
                }
            },
            fmt => {
                eprintln!("Unsupported format: {}. Use 'csv' or 'json'.", fmt);
                std::process::exit(1);
            }
        };

        if points.is_empty() {
            eprintln!("No fee records found in input.");
            std::process::exit(1);
        }

        let report = DataQualityReport::from_points(&points);
    /// Run the validate subcommand on the given fee points.
    pub fn run(&self, points: &[crate::simulation::fee_model::FeePoint]) {
        let report = crate::data_quality::validator::Validator::run(points);
        if self.output_format == "json" {
            println!("{}", report.to_json());
        } else {
            println!("{}", report.display());
        }
    }
}

/// Arguments for the `inspect` subcommand.
///
/// Supports piped input: `cat fees.csv | devkit inspect --format csv`
pub struct InspectArgs {
    /// Input source (file path or stdin).
    pub input: InputSource,
    /// Format of the input data: "csv" or "json".
    pub format: String,
    /// Output format: "text" or "json".
    pub output_format: String,
}

impl Default for InspectArgs {
    fn default() -> Self {
        Self {
            input: InputSource::Stdin,
            format: "csv".to_string(),
            output_format: "text".to_string(),
        }
    }
}

impl InspectArgs {
    /// Run the inspect subcommand, printing a summary of the fee data.
    pub fn run(&self) {
        let points = match self.format.as_str() {
            "csv" => match self.input.load_csv() {
                Ok(result) => {
                    if result.skipped > 0 {
                        eprintln!("Warning: {} malformed rows were skipped", result.skipped);
                    }
                    result.points
                }
                Err(e) => {
                    eprintln!("Error reading CSV: {}", e);
                    std::process::exit(1);
                }
            },
            "json" => match self.input.load_json() {
                Ok(pts) => pts,
                Err(e) => {
                    eprintln!("Error reading JSON: {}", e);
                    std::process::exit(1);
                }
            },
            fmt => {
                eprintln!("Unsupported format: {}. Use 'csv' or 'json'.", fmt);
                std::process::exit(1);
            }
        };

        if points.is_empty() {
            eprintln!("No fee records found in input.");
            std::process::exit(1);
        }

        let summary = FeeSummary::from_points(&points);
        if self.output_format == "json" {
            println!("{}", summary.to_json());
        } else {
            println!("{}", summary.display());
        }
    }
}

// ---------------------------------------------------------------------------
// Embedded report types (used by validate and inspect)
// ---------------------------------------------------------------------------

use crate::simulation::fee_model::FeePoint;

/// A basic data quality report for fee data.
pub struct DataQualityReport {
    pub total: usize,
    pub spike_count: usize,
    pub min_fee: u64,
    pub max_fee: u64,
    pub mean_fee: f64,
}

impl DataQualityReport {
    pub fn from_points(points: &[FeePoint]) -> Self {
        let total = points.len();
        let spike_count = points.iter().filter(|p| p.is_spike).count();
        let fees: Vec<u64> = points.iter().map(|p| p.fee).collect();
        let min_fee = *fees.iter().min().unwrap_or(&0);
        let max_fee = *fees.iter().max().unwrap_or(&0);
        let mean_fee = fees.iter().sum::<u64>() as f64 / total as f64;
        Self {
            total,
            spike_count,
            min_fee,
            max_fee,
            mean_fee,
        }
    }

    pub fn display(&self) -> String {
        format!(
            "Data Quality Report\n\
             ===================\n\
             total records: {}\n\
             spike count:   {}\n\
             min fee:       {} stroops\n\
             max fee:       {} stroops\n\
             mean fee:      {:.2} stroops",
            self.total, self.spike_count, self.min_fee, self.max_fee, self.mean_fee,
        )
    }

    pub fn to_json(&self) -> String {
        format!(
            r#"{{"total":{},"spike_count":{},"min_fee":{},"max_fee":{},"mean_fee":{:.2}}}"#,
            self.total, self.spike_count, self.min_fee, self.max_fee, self.mean_fee,
        )
    }
}

/// Statistical summary for a set of fee points (used by inspect).
pub struct FeeSummary {
    pub count: usize,
    pub min: u64,
    pub max: u64,
    pub mean: f64,
    pub median: u64,
    pub spike_count: usize,
    pub p25: u64,
    pub p75: u64,
    pub p95: u64,
    pub p99: u64,
}

impl FeeSummary {
    pub fn from_points(points: &[FeePoint]) -> Self {
        if points.is_empty() {
            return Self {
                count: 0,
                min: 0,
                max: 0,
                mean: 0.0,
                median: 0,
                spike_count: 0,
                p25: 0,
                p75: 0,
                p95: 0,
                p99: 0,
            };
        }
        let mut fees: Vec<u64> = points.iter().map(|p| p.fee).collect();
        fees.sort_unstable();
        let count = fees.len();
        let min = fees[0];
        let max = fees[count - 1];
        let mean = fees.iter().sum::<u64>() as f64 / count as f64;
        let median = fees[count / 2];
        let spike_count = points.iter().filter(|p| p.is_spike).count();
        let p = |pct: f64| fees[((count as f64 * pct) as usize).min(count - 1)];
        Self {
            count,
            min,
            max,
            mean,
            median,
            spike_count,
            p25: p(0.25),
            p75: p(0.75),
            p95: p(0.95),
            p99: p(0.99),
        }
    }

    pub fn display(&self) -> String {
        format!(
            "Fee Summary\n\
             ===========\n\
             count:       {}\n\
             min:         {} stroops\n\
             max:         {} stroops\n\
             mean:        {:.2} stroops\n\
             median:      {} stroops\n\
             spike_count: {}\n\
             p25:         {} stroops\n\
             p75:         {} stroops\n\
             p95:         {} stroops\n\
             p99:         {} stroops",
            self.count,
            self.min,
            self.max,
            self.mean,
            self.median,
            self.spike_count,
            self.p25,
            self.p75,
            self.p95,
            self.p99,
        )
    }

    pub fn to_json(&self) -> String {
        format!(
            r#"{{"count":{},"min":{},"max":{},"mean":{:.2},"median":{},"spike_count":{},"p25":{},"p75":{},"p95":{},"p99":{}}}"#,
            self.count,
            self.min,
            self.max,
            self.mean,
            self.median,
            self.spike_count,
            self.p25,
            self.p75,
            self.p95,
            self.p99,
        )
    }
}

// ---------------------------------------------------------------------------
// Clap CLI definition
// ---------------------------------------------------------------------------
        if !self.quiet && !report.is_clean() {
            eprintln!("Validation failed: {} issue(s) found.", report.findings.len());
        }
    }
}

// ── repair (issue #392) ──────────────────────────────────────────────────────

/// Arguments for the `repair` subcommand.
///
/// Runs gap filling and duplicate removal on a fee data file.
/// Use `--dry-run` to preview changes without writing any output.
///
/// # Usage
/// ```text
/// devkit repair --file fees.csv --fill-gaps --remove-duplicates --output repaired.csv
/// devkit repair --file fees.csv --dry-run
/// ```
pub struct RepairArgs {
    /// Path to the fee data file.
    pub file: std::path::PathBuf,
    /// Fill gaps in the ledger sequence by interpolation.
    pub fill_gaps: bool,
    /// Remove duplicate ledger sequence numbers.
    pub remove_duplicates: bool,
    /// Preview changes without writing any output.
    pub dry_run: bool,
    /// Optional path to write the repaired output.
    pub output: Option<std::path::PathBuf>,
    /// Suppress all output except errors and the final result.
    pub quiet: bool,
}

impl Default for RepairArgs {
    fn default() -> Self {
        Self {
            file: std::path::PathBuf::from("fees.csv"),
            fill_gaps: false,
            remove_duplicates: false,
            dry_run: false,
            output: None,
            quiet: false,
        }
    }
}

impl RepairArgs {
    /// Run the repair subcommand on the given fee points.
    pub fn run(&self, points: &[crate::simulation::fee_model::FeePoint]) {
        let issues = crate::data_quality::repair::Repair::detect(points);
        let (cleaned, actions) = crate::data_quality::repair::Repair::apply(points, self.dry_run);

        if !self.quiet {
            println!("Repair report:");
            println!("  Issues found:  {}", issues.len());
            println!("  Actions:       {}", actions.len());
            println!("  Points before: {}", points.len());
            println!("  Points after:  {}", cleaned.len());
            if self.dry_run {
                println!("  (dry-run — no changes applied)");
            }
            for action in &actions {
                println!("  - {:?}", action);
            }
        }
    }
}

// ── compare (issue #393) ─────────────────────────────────────────────────────

/// Arguments for the `compare` subcommand.
///
/// Compares two fee data files and prints a distribution diff including
/// p50/p95/mean deltas and a volatility comparison.
///
/// # Usage
/// ```text
/// devkit compare --baseline fees_jan.csv --target fees_feb.csv
/// devkit compare --baseline fees_jan.csv --target fees_feb.csv --json
/// ```
pub struct CompareArgs {
    /// Path to the baseline fee data file.
    pub baseline: std::path::PathBuf,
    /// Path to the target fee data file.
    pub target: std::path::PathBuf,
    /// Label for the baseline dataset (defaults to the file name).
    pub label_baseline: String,
    /// Label for the target dataset (defaults to the file name).
    pub label_target: String,
    /// Output as JSON instead of text.
    pub json: bool,
    /// Suppress all output except errors and the final result.
    pub quiet: bool,
}

impl Default for CompareArgs {
    fn default() -> Self {
        Self {
            baseline: std::path::PathBuf::from("baseline.csv"),
            target: std::path::PathBuf::from("target.csv"),
            label_baseline: "baseline".into(),
            label_target: "target".into(),
            json: false,
            quiet: false,
        }
    }
}

impl CompareArgs {
    /// Run the compare subcommand on two slices of fee points.
    pub fn run(
        &self,
        baseline: &[crate::simulation::fee_model::FeePoint],
        target: &[crate::simulation::fee_model::FeePoint],
    ) {
        let result = crate::utilities::comparator::FeeComparator::compare(
            baseline, target,
            &self.label_baseline, &self.label_target,
        );
        if self.json {
            println!("{}", result.to_json());
        } else {
            println!("{}", result.display());
            if !self.quiet {
                println!("{}", result.verdict());
            }
        }
    }
}

// ── inspect (issue #394) ─────────────────────────────────────────────────────

/// Arguments for the `inspect` subcommand.
///
/// Prints a summary of a fee data file: record count, time range,
/// min/max/mean, and a full percentile table.
///
/// # Usage
/// ```text
/// devkit inspect --file fees.csv
/// devkit inspect --file fees.csv --json
/// ```
pub struct InspectArgs {
    /// Path to the fee data file.
    pub file: std::path::PathBuf,
    /// Output as JSON instead of text.
    pub json: bool,
    /// Suppress all output except errors and the final result.
    pub quiet: bool,
}

impl Default for InspectArgs {
    fn default() -> Self {
        Self {
            file: std::path::PathBuf::from("fees.csv"),
            json: false,
            quiet: false,
        }
    }
}

impl InspectArgs {
    /// Run the inspect subcommand on the given fee points.
    pub fn run(&self, points: &[crate::simulation::fee_model::FeePoint]) {
        if points.is_empty() {
            eprintln!("No fee points to inspect.");
            return;
        }

        let fees: Vec<u64> = points.iter().map(|p| p.fee).collect();
        let count = fees.len();
        let min   = *fees.iter().min().unwrap();
        let max   = *fees.iter().max().unwrap();
        let mean  = fees.iter().sum::<u64>() as f64 / count as f64;
        let ts_min = points.iter().map(|p| p.timestamp).min().unwrap_or(0);
        let ts_max = points.iter().map(|p| p.timestamp).max().unwrap_or(0);
        let spike_count = points.iter().filter(|p| p.is_spike).count();

        let table = crate::utilities::percentile_table::PercentileTable::from_fees(&fees);

        if self.json {
            let ptable_json = table.as_ref().map(|t| t.to_json()).unwrap_or_else(|| "{}".into());
            println!(
                r#"{{"count":{count},"min":{min},"max":{max},"mean":{mean:.2},"spike_count":{spike_count},"ts_min":{ts_min},"ts_max":{ts_max},"percentiles":{ptable_json}}}"#,
            );
        } else {
            println!("Fee Data Inspection");
            println!("===================");
            println!("records:     {count}");
            println!("time range:  {ts_min} - {ts_max}");
            println!("min fee:     {min} stroops");
            println!("max fee:     {max} stroops");
            println!("mean fee:    {mean:.2} stroops");
            println!("spikes:      {spike_count}");
            if let Some(t) = table {
                println!();
                println!("{}", t.display());
            }
        }
    }
}

// ── clap CLI wiring ──────────────────────────────────────────────────────────

/// Developer toolkit for the Stellar fee tracker.
///
/// Use `--quiet` to suppress all output except errors (useful for scripting).
#[derive(Parser)]
#[command(name = "devkit", about = "Stellar fee tracker developer toolkit")]
pub struct Cli {
    /// Suppress all output except errors and the final result.
    #[arg(long, short = 'q', global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Replay recorded fee scenarios
    Replay,
    /// Export data to external formats
    Export {
        /// Perform a dry run — show what would be written without writing it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Run performance benchmarks
    Benchmark,
    /// Serve mock fee data
    Mock,
    /// Print devkit version and build info
    ///
    /// Use `--json` for machine-readable output.
    Version {
        /// Output as JSON
        #[arg(long)]
        json: bool,
        /// Check if current version is newer than the given version
        #[arg(long, value_name = "VERSION")]
        check: Option<String>,
    },
    /// Show active devkit configuration (env vars and defaults)
    Config {
        /// Path to a configuration file
        #[arg(long, value_name = "FILE")]
        config_file: Option<std::path::PathBuf>,
        /// Set a key=value configuration override (repeatable)
        #[arg(long, value_name = "KEY=VALUE")]
        set: Vec<String>,
    },
    /// Generate shell completion scripts
    ///
    /// Example: `devkit completions --shell zsh >> ~/.zshrc`
    Completions {
        /// Shell to generate completions for (bash, zsh, fish)
        #[arg(long, value_name = "SHELL", default_value = "bash")]
        shell: String,
    },
    /// Validate fee data from a file or piped stdin
    ///
    /// Example: `cat fees.csv | devkit validate --format csv`
    Validate {
        /// Path to the fee data file (omit to read from stdin)
        #[arg(long, value_name = "FILE")]
        file: Option<std::path::PathBuf>,
        /// Input format: csv or json
        #[arg(long, default_value = "csv", value_name = "FORMAT")]
        format: String,
        /// Output format for the report: text or json
        #[arg(long, default_value = "text", value_name = "FORMAT")]
        output_format: String,
    },
    /// Inspect fee data and print a statistical summary
    ///
    /// Example: `cat fees.csv | devkit inspect --format csv`
    Inspect {
        /// Path to the fee data file (omit to read from stdin)
        #[arg(long, value_name = "FILE")]
        file: Option<std::path::PathBuf>,
        /// Input format: csv or json
        #[arg(long, default_value = "csv", value_name = "FORMAT")]
        format: String,
        /// Output format: text or json
        #[arg(long, default_value = "text", value_name = "FORMAT")]
        output_format: String,
    /// Validate fee data quality
    Validate,
    /// Repair fee data (gap fill, de-duplicate)
    Repair,
    /// Compare two fee data distributions
    Compare,
    /// Inspect a fee data file (summary + percentile table)
    Inspect,
    /// Run devkit health checks
    Health {
        /// Output results as JSON.
        #[arg(long)]
        json: bool,
        /// Path to the SQLite database to check.
        #[arg(long, default_value = "stellar_fees.db")]
        db_path: String,
        /// Run only a specific named check.
        #[arg(long)]
        check: Option<String>,
    },
    /// Print the devkit metrics report
    Metrics {
        /// Output results as JSON.
        #[arg(long)]
        json: bool,
        /// Reset all counters after displaying.
        #[arg(long)]
        reset: bool,
    },
    /// Detect and repair data quality issues in fee data
    Repair {
        /// Show what would be changed without applying any repairs.
        #[arg(long)]
        dry_run: bool,
        /// Output the repair report as JSON.
        #[arg(long)]
        json: bool,
        /// Only detect and report issues; do not plan repairs.
        #[arg(long)]
        check_only: bool,
    },
}
