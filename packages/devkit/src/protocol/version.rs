use crate::error::DevkitError;

pub struct ProtocolVersion {
    pub version: String,
}

impl ProtocolVersion {
    pub fn from_headers(headers: &[(String, String)]) -> Result<Self, DevkitError> {
        let version = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("x-soroban-version"))
            .map(|(_, v)| v.clone())
            .unwrap_or_else(|| "unknown".into());
        Ok(Self { version })
    }

    pub fn is_compatible(&self) -> bool {
        !self.version.is_empty() && self.version != "unknown"
    }
}
