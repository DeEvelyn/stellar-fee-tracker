# Contributing to stellar-devkit

## PR Requirements

- One logical change per PR
- PR title must follow `feat|fix|chore(devkit[/module]): short description`
- All issues closed by the PR must be listed in the body as `Closes #N`
- Branch off `main`; target `main`

## Test Expectations

- Every new module or function must have at least one test
- Integration tests go in `packages/devkit/tests/`
- Unit tests go in the same file as the code under `#[cfg(test)]`
- Run `cargo test -p stellar-devkit` before opening a PR

## Integration Tests

Everything under `packages/devkit/tests/` compiles as its own crate against
the crate's public API only, so integration tests can't reach private items
the way `#[cfg(test)]` unit tests can. New files should use the
`integration_` prefix (e.g. `integration_foo_bar.rs`) so they're easy to spot
next to the older un-prefixed files already in that folder.

### Writing a test

- One `#[test]` fn per behavior, named `test_<what>_<expected_outcome>`
  (e.g. `test_db_factory_creates_sqlite_database`).
- Build inputs directly in the test (a `Vec<FeePoint>`, a small JSON scenario,
  etc.) instead of reaching into private modules.
- Prefer `.expect("message")` over a bare `.unwrap()` on setup calls, so a
  failure names the step that broke.
- Assertions should state expected vs actual (`assert_eq!(x, y, "expected {}
  got {}", y, x)`), so a CI failure is readable without opening the file.

### Common setup patterns

- Scratch directory: `tempfile::tempdir()` returns a `TempDir` (see
  `integration_db_factory.rs`).
- Scratch file: `tempfile::NamedTempFile::new()` (see
  `integration_csv_writer.rs`).
- Mock Horizon responses: `stellar_devkit::harness::horizon_mock::HorizonMock`,
  built up with `.with_scenario_path(...)`, `.with_error_rate(...)`,
  `.with_delay_ms(...)` (see `harness_integration.rs`).
- Shared fixtures: add `mod common;` at the top of a test file to pull in the
  helpers under `tests/common/` (`TestContext` for a temp fixtures directory,
  `TestDatabase` for a scratch sqlite path).

### Test database usage

- `TestDatabase::new()` (`tests/common/setup.rs`) returns a unique sqlite path
  under the OS temp dir (`test_<uuid>.db`) plus a ready `connection_string()`
  (`sqlite:<path>`), so parallel tests never collide.
- Give each test its own `TestDatabase`; don't share one across tests.
- Never point a test at a real or shared `DATABASE_URL`, and never make a
  real network or Horizon call from a devkit test (see Boundary Rules).

### Cleanup rules

- Let `Drop` do the work: `TempDir`, `NamedTempFile`, and `TestDatabase` all
  remove their own files when they go out of scope. Don't add manual
  `std::fs::remove_*` calls on top of these.
- `TestDatabase::drop` also clears sqlite's `-journal`, `-wal`, and `-shm`
  sidecar files, so reuse it (or mirror its cleanup) whenever a test opens a
  real sqlite connection rather than just building a connection string.
- If a test mutates process-wide state (env vars, etc.), unset it at the end
  of the same test so it can't leak into whichever test runs next, since Rust
  runs tests in one process by default.
- Run `cargo test -p stellar-devkit` before opening a PR to confirm the new
  test both passes and cleans up after itself.

## Code Style

- Follow standard Rust formatting: `cargo fmt --package stellar-devkit`
- No `#[allow(dead_code)]` on public items
- All public items must have a doc comment (`///`)

## Boundary Rules

- Do **not** import from `packages/core` or any live-network crate
- Do **not** make real HTTP calls; use the mock harness
- Keep `[dependencies]` minimal — prefer `[dev-dependencies]` where possible
