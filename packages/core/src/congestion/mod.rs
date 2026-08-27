mod scoring;
pub mod wait_advisory;

#[allow(unused_imports)]
pub use scoring::{
    congestion_label, congestion_score, CongestionInput, CongestionLevel, TrendDirection,
};
#[allow(unused_imports)]
pub use wait_advisory::{compute_wait_advisory, WaitAdvisory, WaitRecommendation};

#[cfg(test)]
mod tests;
