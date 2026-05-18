use serde::Serialize;

use crate::state::LiveDataSourceProvenance;

use super::{build_source_health, classify_freshness, DataFreshness, SourceHealth};

#[derive(Debug, Clone, Serialize)]
pub struct SourceSnapshot {
    pub futures_backend: String,
    pub aux_backend: String,
    pub futures_base_url: String,
    pub aux_base_url: String,
    pub freshness: DataFreshness,
    pub health: Vec<SourceHealth>,
}

pub fn build_source_snapshot(
    provenance: &LiveDataSourceProvenance,
    now: chrono::DateTime<chrono::Utc>,
) -> SourceSnapshot {
    SourceSnapshot {
        futures_backend: provenance.futures_backend.clone(),
        aux_backend: provenance.aux_backend.clone(),
        futures_base_url: provenance.futures_base_url.clone(),
        aux_base_url: provenance.aux_base_url.clone(),
        freshness: classify_freshness(provenance.fetched_at, now),
        health: vec![
            build_source_health(
                provenance.futures_backend.clone(),
                true,
                "source_registered",
            ),
            build_source_health(provenance.aux_backend.clone(), true, "source_registered"),
        ],
    }
}
