use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct SourceHealth {
    pub backend: String,
    pub healthy: bool,
    pub reason: String,
}

pub fn build_source_health(
    backend: impl Into<String>,
    healthy: bool,
    reason: impl Into<String>,
) -> SourceHealth {
    SourceHealth {
        backend: backend.into(),
        healthy,
        reason: reason.into(),
    }
}
