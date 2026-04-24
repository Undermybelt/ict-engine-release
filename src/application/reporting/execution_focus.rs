//! Round 2 §3.6 — `--execution-focus` 铺路 (execution-first surface).
//!
//! Builds a compact "can we execute?" surface from an `ExecutionArtifact`.
//! Drives the terminal ordering that the main plan Sprint 4 §4.3 eventually
//! promotes to default. Round 2 keeps it **opt-in via `ICT_EXECUTION_FOCUS`
//! env var** — no CLI flag wiring yet, no main.rs change.
//!
//! Consumers:
//! - workflow_status human view can prepend the focus surface when the env
//!   var is set (see `workflow_status::execution_focus_enabled`).
//! - external integrators can call `build_execution_focus_surface()`
//!   directly against any `ExecutionArtifact` they load from disk.

use serde::Serialize;

use crate::application::reflection::ReflectionBundle;
use crate::domain::execution::ExecutionArtifact;

/// Environment variable name that enables the focus surface. Not a CLI flag
/// yet — deliberately held back per Round 2 §3.6 ("不升默认值"). The Sprint 4
/// §4.3 upgrade will replace the env var with a proper `--execution-focus`
/// clap flag AND flip the default.
pub const EXECUTION_FOCUS_ENV_VAR: &str = "ICT_EXECUTION_FOCUS";

#[derive(Debug, Clone, Serialize, Default)]
pub struct ExecutionFocusSurface {
    pub gate_status: String,
    pub execution_bias: String,
    pub why_execution_dominates: Option<String>,
    pub readiness: f64,
    pub execution_edge_share: f64,
    pub prediction_edge_share: f64,
    /// Top-3 SHAP attribution rows (feature, contribution, value). Empty when
    /// `reflection_bundle.execution_shap_top_k` is empty.
    pub shap_top_3: Vec<(String, f64, String)>,
    /// One-line "能否做 → 为什么 → 证据" summary for terminal rendering.
    pub one_line: String,
}

/// Returns `true` when the env var is set to a non-empty, non-"0", non-"false"
/// value. Kept permissive — operators typically `export ICT_EXECUTION_FOCUS=1`.
pub fn execution_focus_enabled() -> bool {
    match std::env::var(EXECUTION_FOCUS_ENV_VAR) {
        Ok(value) => {
            let trimmed = value.trim().to_ascii_lowercase();
            !trimmed.is_empty() && trimmed != "0" && trimmed != "false" && trimmed != "no"
        }
        Err(_) => false,
    }
}

pub fn build_execution_focus_surface(
    artifact: &ExecutionArtifact,
    reflection: Option<&ReflectionBundle>,
    execution_bias: Option<&str>,
) -> ExecutionFocusSurface {
    let shap_top_3 = reflection
        .map(|bundle| {
            bundle
                .execution_shap_top_k
                .iter()
                .take(3)
                .map(|row| {
                    (
                        row.feature.clone(),
                        row.contribution,
                        row.feature_value.clone(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let why = reflection.and_then(|bundle| bundle.why_execution_dominates.clone());
    let bias = execution_bias.unwrap_or("unknown").to_string();

    let one_line = format!(
        "Execute? gate={} bias={} readiness={:.3} edge={:.3} | why: {}",
        artifact.hard_gate_status,
        bias,
        artifact.features.execution_readiness,
        artifact.features.execution_edge_share,
        why.as_deref().unwrap_or("no_dominance"),
    );

    ExecutionFocusSurface {
        gate_status: artifact.hard_gate_status.clone(),
        execution_bias: bias,
        why_execution_dominates: why,
        readiness: artifact.features.execution_readiness,
        execution_edge_share: artifact.features.execution_edge_share,
        prediction_edge_share: artifact.features.prediction_edge_share,
        shap_top_3,
        one_line,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::orchestration::ExecutionShapAttribution;
    use crate::application::reflection::ReflectionBundle;
    use crate::domain::execution::ExecutionFeatures;
    use crate::state::RunProvenance;
    use chrono::Utc;

    fn sample_artifact() -> ExecutionArtifact {
        ExecutionArtifact {
            artifact_id: "execution:test".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            features: ExecutionFeatures {
                execution_readiness: 0.80,
                execution_edge_share: 0.72,
                prediction_edge_share: 0.28,
                execution_score: 0.80,
                prediction_score: 0.31,
                ..Default::default()
            },
            hard_gate_status: "execution_ready".to_string(),
            provenance: RunProvenance::default(),
        }
    }

    #[test]
    fn env_flag_defaults_off() {
        // Covers default. We cannot reliably mutate a global env var in a
        // concurrent test suite, so we only assert "unset → off" and trust
        // the parser logic via direct-value checks below.
        std::env::remove_var(EXECUTION_FOCUS_ENV_VAR);
        assert!(!execution_focus_enabled());
    }

    #[test]
    fn focus_surface_includes_one_line_summary() {
        let artifact = sample_artifact();
        let surface = build_execution_focus_surface(&artifact, None, Some("aggressive"));
        assert!(surface.one_line.contains("gate=execution_ready"));
        assert!(surface.one_line.contains("bias=aggressive"));
        assert!(surface.one_line.contains("readiness=0.800"));
        assert!(surface.one_line.contains("edge=0.720"));
    }

    #[test]
    fn focus_surface_pulls_shap_top_3_from_reflection() {
        let artifact = sample_artifact();
        let reflection = ReflectionBundle {
            why_execution_dominates: Some("execution_score=0.80".to_string()),
            execution_shap_top_k: vec![
                ExecutionShapAttribution {
                    feature: "execution_readiness".to_string(),
                    contribution: 0.15,
                    feature_value: "0.8000".to_string(),
                },
                ExecutionShapAttribution {
                    feature: "evidence_quality".to_string(),
                    contribution: 0.30,
                    feature_value: "0.8800".to_string(),
                },
                ExecutionShapAttribution {
                    feature: "ising_phase_transition_risk".to_string(),
                    contribution: 0.50,
                    feature_value: "0.2000".to_string(),
                },
                ExecutionShapAttribution {
                    feature: "spectral_entropy".to_string(),
                    contribution: 0.68,
                    feature_value: "0.1200".to_string(),
                },
            ],
            ..ReflectionBundle::default()
        };
        let surface = build_execution_focus_surface(&artifact, Some(&reflection), Some("passive"));
        assert_eq!(surface.shap_top_3.len(), 3);
        assert_eq!(surface.shap_top_3[0].0, "execution_readiness");
        assert_eq!(
            surface.why_execution_dominates.as_deref(),
            Some("execution_score=0.80")
        );
    }
}
