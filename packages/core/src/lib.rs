// Library root \u2014 exposes internal modules for integration tests in .
// Production entry point remains .

pub mod alerts;
pub mod api;
pub mod cache;
pub mod db;
pub mod error;
pub mod insights;
pub mod metrics;
pub mod middleware;
pub mod recommendation;
pub mod repository;
pub mod scheduler;
pub mod services;
pub mod store;

// These modules are only needed by the binary.
// Declared pub so integration tests can reach them if needed, but they
// contain no logic of interest to tests.
pub mod cli;
pub mod config;
pub mod logging;

