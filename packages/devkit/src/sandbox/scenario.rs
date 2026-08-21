//! Scenario DSL for composing sandbox test configurations.
//!
//! Provides a builder-pattern API for constructing [`Scenario`]s that
//! describe fixture type, duration, and optional spike injections.

use chrono::Duration;

/// Network fixture type for a scenario.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fixture {
    Normal,
    Congested,
    Spike,
}

/// A fully-resolved sandbox scenario configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct Scenario {
    pub fixture: Fixture,
    pub duration: Duration,
    pub spike_at: Option<(i64, u64)>,
}

impl Scenario {
    /// Start building a new [`Scenario`].
    pub fn builder() -> ScenarioBuilder {
        ScenarioBuilder::default()
    }
}

/// Builder for constructing a [`Scenario`] step by step.
#[derive(Debug, Clone)]
pub struct ScenarioBuilder {
    fixture: Fixture,
    duration: Duration,
    spike_at: Option<(i64, u64)>,
}

impl Default for ScenarioBuilder {
    fn default() -> Self {
        Self {
            fixture: Fixture::Normal,
            duration: Duration::hours(1),
            spike_at: None,
        }
    }
}

impl ScenarioBuilder {
    /// Set the network fixture for the scenario.
    pub fn fixture(mut self, fixture: Fixture) -> Self {
        self.fixture = fixture;
        self
    }

    /// Set the total duration of the scenario.
    pub fn duration(mut self, duration: Duration) -> Self {
        self.duration = duration;
        self
    }

    /// Inject a spike at the given offset (seconds) with the given fee (stroops).
    pub fn inject_spike_at(mut self, offset_secs: i64, fee_stroops: u64) -> Self {
        self.spike_at = Some((offset_secs, fee_stroops));
        self
    }

    /// Consume the builder and produce a [`Scenario`].
    pub fn build(self) -> Scenario {
        Scenario {
            fixture: self.fixture,
            duration: self.duration,
            spike_at: self.spike_at,
        }
    }
}
