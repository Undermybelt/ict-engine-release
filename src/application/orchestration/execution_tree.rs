use std::collections::BTreeMap;
use std::path::Path;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::application::execution::ExecutionPhysicsOverlay;
use crate::application::orchestration::AxialAttentionTrace;
use crate::domain::execution::{
    classify_execution_gate, ExecutionFeatures, DOMINANT_ENERGY_FLOOR, EXECUTION_GATE_OBSERVE,
    EXECUTION_GATE_READY, SPECTRAL_ENTROPY_CHAOS_CAP,
};
use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, save_state, ArtifactLedgerEntry,
    RunProvenance,
};
use crate::types::RegimeProbs;

pub const EXECUTION_TREE_TRACE_FILE: &str = "execution_tree_trace.json";

const PREDICTION_STRONG_THRESHOLD: f64 = 0.65;
const PREDICTION_WEAK_THRESHOLD: f64 = 0.35;
const ISING_HERD_BLOCK_THRESHOLD: f64 = 0.70;
const PYTHAGOREAN_OVERSTRETCH_WAIT_THRESHOLD: f64 = 0.70;

pub struct ExecutionTreeInput<'a> {
    pub execution_features: &'a ExecutionFeatures,
    pub physics_overlay: &'a ExecutionPhysicsOverlay,
    pub hmm_posterior: &'a RegimeProbs,
    pub mece_recovery_confidence: Option<f64>,
    pub prediction_vote_score: f64,
    /// Axial pooling trace over the MTF tensor. When `force_observe` is true
    /// the scorer downgrades an `aggressive` bias to `passive` because no
    /// timeframe is meaningfully dominant. Optional so legacy callers that
    /// do not (yet) run axial pooling keep compiling; None = neutral.
    pub axial_trace: Option<&'a AxialAttentionTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionTreeOutput {
    pub execution_score: f64,
    pub branch: String,
    pub execution_bias: String,
    pub gate_status: String,
    pub branch_probability: f64,
    pub posterior_uncertainty: f64,
    pub split_reason_lineage: Vec<String>,
    pub decision_hint: String,
    /// Top axial attention weights (feature_name, weight) carried into the
    /// trace artifact. Empty when no axial trace was provided to the scorer.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub axial_attention_trace: Vec<(String, f64)>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExecutionTreeArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub output: ExecutionTreeOutput,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub execution_shap_top_k: Vec<ExecutionShapAttribution>,
    pub provenance: RunProvenance,
}

/// SHAP-like feature attribution row for an Execution Tree branch.
/// v1 is a structural attribution (deterministic contribution function over
/// ExecutionTreeInput features), not a CatBoost/XGBoost Shapley value — the
/// trait `ExecutionShapProvider` lets a real model-SHAP implementation replace
/// the default without touching reflection_bundle consumers.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ExecutionShapAttribution {
    pub feature: String,
    pub contribution: f64,
    pub feature_value: String,
}

pub trait ExecutionShapProvider {
    /// Return SHAP attributions ordered by descending |contribution|.
    fn attributions(
        &self,
        input: &ExecutionTreeInput<'_>,
        output: &ExecutionTreeOutput,
    ) -> Vec<ExecutionShapAttribution>;
}

/// Default `ExecutionShapProvider`. Produces a structurally-consistent
/// attribution: each contribution reflects the *signed distance from its
/// decision threshold*, so "what pushed the branch" is visible even though
/// the numbers are not drawn from a trained model. Good enough to satisfy
/// `reflection_bundle.execution_shap_top_k` without faking model output.
pub struct StructuralExecutionShap {
    pub top_k: usize,
}

impl Default for StructuralExecutionShap {
    fn default() -> Self {
        Self { top_k: 5 }
    }
}

impl ExecutionShapProvider for StructuralExecutionShap {
    fn attributions(
        &self,
        input: &ExecutionTreeInput<'_>,
        output: &ExecutionTreeOutput,
    ) -> Vec<ExecutionShapAttribution> {
        let features = input.execution_features;
        let mut rows: Vec<ExecutionShapAttribution> = Vec::new();

        rows.push(ExecutionShapAttribution {
            feature: "execution_readiness".to_string(),
            contribution: features.execution_readiness - EXECUTION_GATE_READY,
            feature_value: format!("{:.4}", features.execution_readiness),
        });
        rows.push(ExecutionShapAttribution {
            feature: "prediction_vote_score".to_string(),
            contribution: input.prediction_vote_score - PREDICTION_STRONG_THRESHOLD,
            feature_value: format!("{:.4}", input.prediction_vote_score),
        });
        rows.push(ExecutionShapAttribution {
            feature: "execution_score".to_string(),
            contribution: features.execution_score - EXECUTION_GATE_OBSERVE,
            feature_value: format!("{:.4}", features.execution_score),
        });
        rows.push(ExecutionShapAttribution {
            feature: "evidence_quality".to_string(),
            contribution: features.evidence_quality - 0.5,
            feature_value: format!("{:.4}", features.evidence_quality),
        });
        if let Some(ising) = input.physics_overlay.ising.as_ref() {
            rows.push(ExecutionShapAttribution {
                feature: "ising_phase_transition_risk".to_string(),
                contribution: ISING_HERD_BLOCK_THRESHOLD - ising.phase_transition_risk,
                feature_value: format!("{:.4}", ising.phase_transition_risk),
            });
        }
        if let Some(pythagorean) = input.physics_overlay.pythagorean.as_ref() {
            rows.push(ExecutionShapAttribution {
                feature: "pythagorean_overstretch".to_string(),
                contribution: PYTHAGOREAN_OVERSTRETCH_WAIT_THRESHOLD
                    - pythagorean.normalized_overstretch,
                feature_value: format!("{:.4}", pythagorean.normalized_overstretch),
            });
        }
        // Spectral attribution rows (Round 2 §3.3). Only emitted when the
        // spectral layer actually fit — keeps top_k stable for runs where the
        // series was too short for the FFT. Sign convention:
        // - spectral_entropy: negative contribution = chaotic (pushes towards block)
        // - dominant_cycle_energy: positive = rhythmic (pushes towards fill)
        // - cycle_phase_alignment: positive = aligned to dominant mode peak
        if let Some(entropy) = features.spectral_entropy {
            rows.push(ExecutionShapAttribution {
                feature: "spectral_entropy".to_string(),
                contribution: SPECTRAL_ENTROPY_CHAOS_CAP - entropy,
                feature_value: format!("{:.4}", entropy),
            });
        }
        if let Some(energy) = features.dominant_cycle_energy {
            rows.push(ExecutionShapAttribution {
                feature: "dominant_cycle_energy".to_string(),
                contribution: energy - DOMINANT_ENERGY_FLOOR,
                feature_value: format!("{:.4}", energy),
            });
        }
        if let Some(alignment) = features.cycle_phase_alignment {
            rows.push(ExecutionShapAttribution {
                feature: "cycle_phase_alignment".to_string(),
                contribution: alignment,
                feature_value: format!("{:.4}", alignment),
            });
        }
        if let Some(confidence) = input.mece_recovery_confidence {
            rows.push(ExecutionShapAttribution {
                feature: "mece_recovery_confidence".to_string(),
                contribution: confidence - 0.95,
                feature_value: format!("{:.4}", confidence),
            });
        }
        rows.push(ExecutionShapAttribution {
            feature: "branch_probability".to_string(),
            contribution: output.branch_probability - 0.5,
            feature_value: format!("{:.4}", output.branch_probability),
        });

        rows.sort_by(|a, b| {
            b.contribution
                .abs()
                .partial_cmp(&a.contribution.abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        rows.truncate(self.top_k);
        rows
    }
}

/// Condensed "can we execute?" summary for the Execution Triage surface
/// (--execution-focus default-on view). Derived purely from
/// `ExecutionTreeOutput`, so it is additive to every report without having
/// to extend build signatures.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct ExecutionTriage {
    pub gate_status: String,
    pub branch: String,
    pub execution_bias: String,
    pub decision_hint: String,
    pub execution_score: f64,
    pub branch_probability: f64,
    pub posterior_uncertainty: f64,
    pub one_line: String,
}

pub fn build_execution_triage(output: &ExecutionTreeOutput) -> ExecutionTriage {
    let one_line = format!(
        "execution {} | branch={} | bias={} | score={:.3} | confidence={:.3} | hint={}",
        output.gate_status,
        output.branch,
        output.execution_bias,
        output.execution_score,
        output.branch_probability,
        output.decision_hint,
    );
    ExecutionTriage {
        gate_status: output.gate_status.clone(),
        branch: output.branch.clone(),
        execution_bias: output.execution_bias.clone(),
        decision_hint: output.decision_hint.clone(),
        execution_score: output.execution_score,
        branch_probability: output.branch_probability,
        posterior_uncertainty: output.posterior_uncertainty,
        one_line,
    }
}

pub trait ExecutionTreeScorer {
    fn score(&self, input: &ExecutionTreeInput<'_>) -> Result<ExecutionTreeOutput>;
}

pub struct DefaultExecutionTreeScorer;

impl ExecutionTreeScorer for DefaultExecutionTreeScorer {
    fn score(&self, input: &ExecutionTreeInput<'_>) -> Result<ExecutionTreeOutput> {
        let mut lineage: Vec<String> = Vec::new();
        let readiness = input.execution_features.execution_readiness;
        let gate_status = short_gate_status(classify_execution_gate(readiness));
        lineage.push(format!(
            "execution_readiness={:.4} → gate_status={}",
            readiness, gate_status
        ));

        let ising_risk = input
            .physics_overlay
            .ising
            .as_ref()
            .map(|state| state.phase_transition_risk)
            .unwrap_or(0.0);
        let overstretch = input
            .physics_overlay
            .pythagorean
            .as_ref()
            .map(|metrics| metrics.normalized_overstretch)
            .unwrap_or(0.0);

        let (branch, branch_probability) = if gate_status == "blocked" {
            lineage.push(format!(
                "branch=block_crowded (gate_status=blocked, readiness {:.4} < {:.2})",
                readiness, EXECUTION_GATE_OBSERVE
            ));
            (
                "block_crowded".to_string(),
                gate_distance(readiness, EXECUTION_GATE_OBSERVE),
            )
        } else if ising_risk >= ISING_HERD_BLOCK_THRESHOLD {
            lineage.push(format!(
                "branch=block_crowded (ising_phase_transition_risk={:.4} ≥ {:.2})",
                ising_risk, ISING_HERD_BLOCK_THRESHOLD
            ));
            (
                "block_crowded".to_string(),
                proximity_confidence(ising_risk, ISING_HERD_BLOCK_THRESHOLD),
            )
        } else if overstretch >= PYTHAGOREAN_OVERSTRETCH_WAIT_THRESHOLD {
            lineage.push(format!(
                "branch=wait_for_reversion (pythagorean_overstretch={:.4} ≥ {:.2})",
                overstretch, PYTHAGOREAN_OVERSTRETCH_WAIT_THRESHOLD
            ));
            (
                "wait_for_reversion".to_string(),
                proximity_confidence(overstretch, PYTHAGOREAN_OVERSTRETCH_WAIT_THRESHOLD),
            )
        } else {
            lineage.push(format!(
                "branch=fill_viable (gate_status={}, ising_risk={:.4}<{:.2}, overstretch={:.4}<{:.2})",
                gate_status, ising_risk, ISING_HERD_BLOCK_THRESHOLD, overstretch, PYTHAGOREAN_OVERSTRETCH_WAIT_THRESHOLD
            ));
            (
                "fill_viable".to_string(),
                gate_distance(readiness, EXECUTION_GATE_READY),
            )
        };

        let prediction_strength = classify_prediction_strength(input.prediction_vote_score);
        let execution_strength = classify_execution_strength(readiness);
        let (mut execution_bias, mut decision_hint) =
            execution_first_decision(prediction_strength, execution_strength);
        lineage.push(format!(
            "prediction_vote_score={:.4} ({}) × execution_readiness={:.4} ({}) → bias={}, hint={}",
            input.prediction_vote_score,
            prediction_strength,
            readiness,
            execution_strength,
            execution_bias,
            decision_hint
        ));

        // Axial pool observation gate (Round 2 §3.1). When the MTF tensor
        // has no dominant timeframe we downgrade an aggressive fill to
        // passive — the execution tree should not bet on a specific bar
        // when the weight distribution is flat. Skip handling is left to
        // the underlying execution_first_decision (weak execution already
        // blocks).
        let mut axial_attention_trace: Vec<(String, f64)> = Vec::new();
        if let Some(trace) = input.axial_trace {
            axial_attention_trace = trace.timeframe_weights.iter().take(5).cloned().collect();
            lineage.push(format!(
                "axial_timeframe_entropy={:.4} force_observe={}",
                trace.timeframe_entropy, trace.force_observe
            ));
            if trace.force_observe && execution_bias == "aggressive" {
                lineage.push(
                    "axial force_observe → bias=aggressive downgraded to passive".to_string(),
                );
                execution_bias = "passive";
                decision_hint = "execution_observe_due_to_axial_entropy";
            }
        }

        if let Some(confidence) = input.mece_recovery_confidence {
            lineage.push(format!("mece_recovery_confidence={:.4}", confidence));
        }
        lineage.push(format!(
            "hmm_posterior=(acc={:.3}, manip={:.3}, dist={:.3})",
            input.hmm_posterior.accumulation,
            input.hmm_posterior.manipulation_expansion,
            input.hmm_posterior.distribution
        ));

        Ok(ExecutionTreeOutput {
            execution_score: input.execution_features.execution_score,
            branch,
            execution_bias: execution_bias.to_string(),
            gate_status: gate_status.to_string(),
            branch_probability,
            posterior_uncertainty: (1.0 - branch_probability).clamp(0.0, 1.0),
            split_reason_lineage: lineage,
            decision_hint: decision_hint.to_string(),
            axial_attention_trace,
        })
    }
}

fn short_gate_status(raw: &str) -> &'static str {
    match raw {
        "execution_ready" => "ready",
        "execution_observe_only" => "observe",
        _ => "blocked",
    }
}

fn gate_distance(value: f64, threshold: f64) -> f64 {
    let span = (1.0 - threshold).max(f64::EPSILON);
    ((value - threshold) / span).clamp(0.0, 1.0)
}

fn proximity_confidence(value: f64, threshold: f64) -> f64 {
    let span = (1.0 - threshold).max(f64::EPSILON);
    ((value - threshold) / span).clamp(0.0, 1.0)
}

fn classify_prediction_strength(score: f64) -> &'static str {
    if score >= PREDICTION_STRONG_THRESHOLD {
        "strong"
    } else if score >= PREDICTION_WEAK_THRESHOLD {
        "medium"
    } else {
        "weak"
    }
}

fn classify_execution_strength(readiness: f64) -> &'static str {
    if readiness >= EXECUTION_GATE_READY {
        "strong"
    } else if readiness >= EXECUTION_GATE_OBSERVE {
        "medium"
    } else {
        "weak"
    }
}

/// Execution-first hard gate: regardless of prediction strength, weak
/// execution always blocks; medium/strong execution can stay actionable even
/// with weak prediction. Returns `(bias, decision_hint)`.
pub fn execution_first_decision(
    prediction_strength: &str,
    execution_strength: &str,
) -> (&'static str, &'static str) {
    match (prediction_strength, execution_strength) {
        (_, "weak") => ("skip", "execution_blocked_regardless_of_prediction"),
        ("strong", "strong") => ("aggressive", "execution_first_fill"),
        ("strong", "medium") => ("passive", "execution_observe_with_strong_prediction"),
        ("medium", "strong") => ("aggressive", "execution_first_fill_with_medium_prediction"),
        ("medium", "medium") => ("passive", "execution_observe_with_medium_prediction"),
        ("weak", "strong") => ("aggressive", "execution_first_fill_despite_weak_prediction"),
        ("weak", "medium") => ("passive", "execution_observe_despite_weak_prediction"),
        _ => ("skip", "unhandled_combination"),
    }
}

pub fn build_execution_tree_artifact(
    symbol: &str,
    output: ExecutionTreeOutput,
    execution_shap_top_k: Vec<ExecutionShapAttribution>,
    provenance: RunProvenance,
) -> ExecutionTreeArtifact {
    let generated_at = Utc::now();
    ExecutionTreeArtifact {
        artifact_id: format!(
            "execution-tree-{}-{}",
            symbol,
            generated_at.timestamp_millis()
        ),
        generated_at,
        symbol: symbol.to_string(),
        output,
        execution_shap_top_k,
        provenance,
    }
}

pub fn persist_execution_tree_artifact<P: AsRef<Path>>(
    dir: P,
    artifact: &ExecutionTreeArtifact,
    source_phase: &str,
    source_run_id: Option<String>,
) -> Result<()> {
    save_state(&dir, &artifact.symbol, EXECUTION_TREE_TRACE_FILE, artifact)?;
    let promote = artifact.output.branch == "fill_viable" && artifact.output.gate_status == "ready";
    let actionable =
        artifact.output.gate_status != "blocked" && artifact.output.branch != "block_crowded";
    let quality_score = (artifact.output.branch_probability * 100.0).round() as i32;
    append_artifact_ledger_entry(
        &dir,
        &artifact.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "execution_tree_artifact".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: 1,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: source_phase.to_string(),
            source_run_id,
            path: artifact_state_path(&dir, &artifact.symbol, EXECUTION_TREE_TRACE_FILE),
            status: artifact.output.gate_status.clone(),
            promote_candidate: promote,
            actionable,
            decision_hint: artifact.output.decision_hint.clone(),
            review_reason: format!(
                "branch={};bias={};branch_prob={:.4};uncertainty={:.4}",
                artifact.output.branch,
                artifact.output.execution_bias,
                artifact.output.branch_probability,
                artifact.output.posterior_uncertainty
            ),
            review_rule_version: "execution-tree-artifact-v1".to_string(),
            top_factor_name: None,
            top_factor_action: None,
            family_scores: BTreeMap::from([
                (
                    "execution_score".to_string(),
                    artifact.output.execution_score,
                ),
                (
                    "branch_probability".to_string(),
                    artifact.output.branch_probability,
                ),
            ]),
            supersedes_artifact_id: None,
            quality_score,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::execution::ExecutionPhysicsOverlay;
    use crate::domain::execution::ExecutionFeatures;
    use crate::domain::regime::IsingState;
    use crate::ict::PythagoreanExtensionMetrics;
    use crate::types::RegimeProbs;
    use std::fs;
    use tempfile::TempDir;

    fn baseline_features(readiness: f64) -> ExecutionFeatures {
        ExecutionFeatures {
            execution_readiness: readiness,
            execution_score: readiness,
            evidence_quality: 0.6,
            ..Default::default()
        }
    }

    fn flat_overlay() -> ExecutionPhysicsOverlay {
        ExecutionPhysicsOverlay {
            ou: None,
            ising: Some(IsingState {
                magnetization: 0.0,
                coupling_strength: 0.2,
                phase_transition_risk: 0.2,
                herding_bias: 0.1,
            }),
            pythagorean: Some(PythagoreanExtensionMetrics {
                trendline_distance: 0.0,
                orthogonal_extension: 0.0,
                normalized_overstretch: 0.1,
            }),
            spectral: None,
        }
    }

    fn neutral_posterior() -> RegimeProbs {
        RegimeProbs {
            accumulation: 0.34,
            manipulation_expansion: 0.33,
            distribution: 0.33,
        }
    }

    #[test]
    fn ready_low_overlay_yields_fill_viable() {
        let features = baseline_features(0.85);
        let overlay = flat_overlay();
        let posterior = neutral_posterior();
        let input = ExecutionTreeInput {
            execution_features: &features,
            physics_overlay: &overlay,
            hmm_posterior: &posterior,
            mece_recovery_confidence: Some(0.97),
            prediction_vote_score: 0.7,
            axial_trace: None,
        };
        let output = DefaultExecutionTreeScorer.score(&input).unwrap();
        assert_eq!(output.branch, "fill_viable");
        assert_eq!(output.gate_status, "ready");
        assert!(output.branch_probability > 0.0);
        assert!(!output.split_reason_lineage.is_empty());
    }

    #[test]
    fn high_ising_risk_blocks_even_when_ready() {
        let features = baseline_features(0.85);
        let mut overlay = flat_overlay();
        if let Some(ising) = overlay.ising.as_mut() {
            ising.phase_transition_risk = 0.9;
        }
        let posterior = neutral_posterior();
        let input = ExecutionTreeInput {
            execution_features: &features,
            physics_overlay: &overlay,
            hmm_posterior: &posterior,
            mece_recovery_confidence: None,
            prediction_vote_score: 0.7,
            axial_trace: None,
        };
        let output = DefaultExecutionTreeScorer.score(&input).unwrap();
        assert_eq!(output.branch, "block_crowded");
    }

    #[test]
    fn high_overstretch_routes_to_wait_for_reversion() {
        let features = baseline_features(0.85);
        let mut overlay = flat_overlay();
        if let Some(p) = overlay.pythagorean.as_mut() {
            p.normalized_overstretch = 0.85;
        }
        let posterior = neutral_posterior();
        let input = ExecutionTreeInput {
            execution_features: &features,
            physics_overlay: &overlay,
            hmm_posterior: &posterior,
            mece_recovery_confidence: None,
            prediction_vote_score: 0.7,
            axial_trace: None,
        };
        let output = DefaultExecutionTreeScorer.score(&input).unwrap();
        assert_eq!(output.branch, "wait_for_reversion");
    }

    #[test]
    fn weak_execution_blocks_even_with_strong_prediction() {
        let features = baseline_features(0.30);
        let overlay = flat_overlay();
        let posterior = neutral_posterior();
        let input = ExecutionTreeInput {
            execution_features: &features,
            physics_overlay: &overlay,
            hmm_posterior: &posterior,
            mece_recovery_confidence: Some(0.97),
            prediction_vote_score: 0.95,
            axial_trace: None,
        };
        let output = DefaultExecutionTreeScorer.score(&input).unwrap();
        assert_eq!(output.gate_status, "blocked");
        assert_eq!(output.execution_bias, "skip");
        assert_eq!(
            output.decision_hint,
            "execution_blocked_regardless_of_prediction"
        );
    }

    #[test]
    fn persists_artifact_and_ledger_entry() {
        let features = baseline_features(0.80);
        let overlay = flat_overlay();
        let posterior = neutral_posterior();
        let output = DefaultExecutionTreeScorer
            .score(&ExecutionTreeInput {
                execution_features: &features,
                physics_overlay: &overlay,
                hmm_posterior: &posterior,
                mece_recovery_confidence: Some(0.97),
                prediction_vote_score: 0.7,
                axial_trace: None,
            })
            .unwrap();
        let artifact =
            build_execution_tree_artifact("NQ", output, Vec::new(), RunProvenance::default());
        let dir = TempDir::new().unwrap();
        persist_execution_tree_artifact(dir.path(), &artifact, "analyze", None).unwrap();

        let trace_path = dir.path().join("NQ").join(EXECUTION_TREE_TRACE_FILE);
        assert!(trace_path.exists());
        let raw = fs::read_to_string(&trace_path).unwrap();
        assert!(raw.contains("\"branch\""));
        assert!(raw.contains("\"split_reason_lineage\""));

        let ledger_path = dir
            .path()
            .join("NQ")
            .join(crate::state::ARTIFACT_LEDGER_FILE);
        let ledger = fs::read_to_string(&ledger_path).unwrap();
        assert!(ledger.contains("\"execution_tree_artifact\""));
        assert!(ledger.contains("\"execution-tree-artifact-v1\""));
        let entries: Vec<ArtifactLedgerEntry> = serde_json::from_str(&ledger).unwrap();
        assert_eq!(
            entries[0].path,
            trace_path.to_string_lossy(),
            "ledger path must point at the selected state_dir artifact"
        );
    }

    #[test]
    fn structural_shap_is_deterministic_and_bounded() {
        let overlay = flat_overlay();
        let features = baseline_features(0.82);
        let posterior = neutral_posterior();
        let input = ExecutionTreeInput {
            execution_features: &features,
            physics_overlay: &overlay,
            hmm_posterior: &posterior,
            mece_recovery_confidence: Some(0.97),
            prediction_vote_score: 0.72,
            axial_trace: None,
        };
        let output = DefaultExecutionTreeScorer.score(&input).unwrap();
        let provider = StructuralExecutionShap::default();
        let first = provider.attributions(&input, &output);
        let second = provider.attributions(&input, &output);
        assert_eq!(first, second, "structural SHAP must be deterministic");
        assert!(first.len() <= 5, "top_k default must clamp to 5");
        // Contributions are ordered by descending |contribution|.
        for window in first.windows(2) {
            assert!(
                window[0].contribution.abs() >= window[1].contribution.abs(),
                "contributions must be sorted by |contribution| desc"
            );
        }
    }

    #[test]
    fn triage_one_line_covers_core_fields() {
        let output = ExecutionTreeOutput {
            execution_score: 0.82,
            branch: "fill_viable".to_string(),
            execution_bias: "aggressive".to_string(),
            gate_status: "ready".to_string(),
            branch_probability: 0.6,
            posterior_uncertainty: 0.4,
            split_reason_lineage: vec![],
            decision_hint: "execution_first_fill".to_string(),
            axial_attention_trace: Vec::new(),
        };
        let triage = build_execution_triage(&output);
        assert!(triage.one_line.contains("ready"));
        assert!(triage.one_line.contains("fill_viable"));
        assert!(triage.one_line.contains("aggressive"));
        assert!(triage.one_line.contains("execution_first_fill"));
        assert_eq!(triage.gate_status, "ready");
        assert_eq!(triage.branch, "fill_viable");
    }
}
