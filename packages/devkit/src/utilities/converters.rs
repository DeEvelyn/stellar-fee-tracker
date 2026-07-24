/// Number of stroops in one XLM.
pub const STROOPS_PER_XLM: u64 = 10_000_000;

/// Convert stroops to XLM as f64.
pub fn stroop_to_xlm(stroops: u64) -> f64 {
    stroops as f64 / STROOPS_PER_XLM as f64
}

/// Convert XLM (f64) to stroops. Returns an error for negative values.
pub fn xlm_to_stroop(xlm: f64) -> Result<u64, ConversionError> {
    if xlm < 0.0 {
        return Err(ConversionError::NegativeXlm);
    }
    Ok((xlm * STROOPS_PER_XLM as f64).round() as u64)
}

/// Errors during stroop/XLM conversion.
#[derive(Debug, PartialEq)]
pub enum ConversionError {
    NegativeXlm,
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConversionError::NegativeXlm => write!(f, "XLM value must be non-negative"),
        }
    }
}

impl std::error::Error for ConversionError {}
