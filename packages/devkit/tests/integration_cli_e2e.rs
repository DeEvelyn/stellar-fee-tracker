//! End-to-end integration test for the `devkit inspect` subcommand (issue #498).
//!
//! Unlike `cli_inspect.rs`, `cli_compare.rs`, etc. (which reimplement the
//! inspect/percentile logic inline in-process), this test writes a fee CSV
//! to disk, spawns the actual compiled `devkit` binary against it, and
//! asserts on the real stdout: that it contains a percentile table and that
//! the reported p50 matches the value computed by hand for the fixture data.
//!
//! # BLOCKED — see PR description for issue #498
//!
//! This test targets the CLI surface as documented (`devkit inspect --file
//! <path>`), but two pre-existing problems in this crate mean it will not
//! compile/run yet:
//!
//! 1. `packages/devkit/src/cli/mod.rs` has an unresolved merge conflict:
//!    `InspectArgs`, `ValidateArgs`, and several `Commands` variants
//!    (`Validate`, `Repair`, `Compare`, `Inspect`) are each defined twice,
//!    and the file does not currently compile.
//! 2. `packages/devkit/Cargo.toml` has no `[[bin]]` target, and there is no
//!    `main.rs` anywhere that wires `cli::Cli::parse()` up to a runnable
//!    entry point. There is currently no `devkit` binary for
//!    `env!("CARGO_BIN_EXE_devkit")` to resolve, so `cargo test` will fail
//!    to build this file until a bin target exists.
//!
//! Per issue #498's stated file list, this PR intentionally touches only
//! this test file; the fixes above are flagged as a follow-up rather than
//! bundled in here.

use std::io::Write;
use std::process::Command;

/// Fee CSV fixture written to a temp file before invoking the CLI.
/// Header matches what `csv_reader::parse_csv_row` expects:
/// `timestamp,fee_amount,ledger_sequence,is_spike`.
const FEE_CSV: &str = "\
timestamp,fee_amount,ledger_sequence,is_spike
0,100,1,false
5,120,2,false
10,110,3,false
15,300,4,true
20,130,5,false
25,140,6,false
30,125,7,false
35,150,8,false
40,600,9,true
45,135,10,false
";

/// Expected p50 for the fee column above, computed the same way
/// `percentile_table::percentile` does (nearest-rank method):
///
/// sorted fees: 100 110 120 125 130 135 140 150 300 600  (n = 10)
/// idx = ceil(50 / 100 * 10) - 1 = 4  ->  sorted[4] = 130
const EXPECTED_P50: u64 = 130;

#[test]
fn inspect_cli_prints_percentile_table_with_correct_p50() {
    // Path to the compiled `devkit` binary. This macro is resolved at
    // compile time by cargo based on the crate's `[[bin]]` targets; it will
    // fail to compile until one named `devkit` exists (see module docs).
    let bin = env!("CARGO_BIN_EXE_devkit");

    let mut csv_path = std::env::temp_dir();
    csv_path.push(format!("devkit_e2e_fees_{}.csv", std::process::id()));
    {
        let mut f = std::fs::File::create(&csv_path).expect("create temp fee csv");
        f.write_all(FEE_CSV.as_bytes())
            .expect("write temp fee csv");
    }

    let output = Command::new(bin)
        .arg("inspect")
        .arg("--file")
        .arg(&csv_path)
        .output()
        .expect("failed to run `devkit inspect`");

    let _ = std::fs::remove_file(&csv_path);

    assert!(
        output.status.success(),
        "devkit inspect exited with failure. stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("Percentile Table"),
        "expected a percentile table in stdout, got:\n{stdout}"
    );

    let p50_line = stdout
        .lines()
        .find(|line| line.trim_start().starts_with("p50:"))
        .unwrap_or_else(|| panic!("no p50 line found in stdout:\n{stdout}"));

    assert!(
        p50_line.contains(&EXPECTED_P50.to_string()),
        "expected p50 of {EXPECTED_P50} stroops, got line: {p50_line:?}"
    );
}
