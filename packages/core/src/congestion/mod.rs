mod scoring;
pub mod wait_advisory;

pub use scoring::{
    congestion_label, congestion_score, CongestionInput, CongestionLevel, TrendDirection,
};
pub use wait_advisory::{compute_wait_advisory, WaitAdvisory, WaitRecommendation};

#[cfg(test)]
mod tests;
