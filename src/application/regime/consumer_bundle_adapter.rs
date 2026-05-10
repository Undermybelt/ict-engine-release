use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, path::Path};

use crate::state::PreBayesEvidenceFilter;

const EXPECTED_SCHEMA_VERSION: &str = "regime-consumer-bundle/v1";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BundleStatus {
    Disabled,
    Loaded,
    Missing,
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionTreeHint {
    AcceptRegime,
    TransitionGuardrail,
    UnknownAbstain,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegimeBbnEvidenceStrength {
    Strong,
    Moderate,
    Neutral,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegimeBbnEvidenceApplicationStatus {
    Applied,
    Skipped,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RegimeReadOnlyBbnSoftEvidence {
    pub strength: RegimeBbnEvidenceStrength,
    pub weight: f64,
    pub decision_state: String,
    pub trade_usable: Option<bool>,
    pub label: Option<String>,
    pub label_set: Vec<String>,
    pub transition_hazard: Option<f64>,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegimeDecisionSummary {
    #[serde(default)]
    pub timestamp: String,
    #[serde(default)]
    pub decision_state: String,
    #[serde(default)]
    pub trade_usable: bool,
    #[serde(default)]
    pub final_label: String,
    #[serde(default)]
    pub label_set: Vec<String>,
    #[serde(default)]
    pub abstain_reasons: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RegimeConsumerHints {
    #[serde(default)]
    pub execution_tree_hint: String,
    #[serde(default)]
    pub bbn_evidence_hint: Value,
    #[serde(default)]
    pub path_ranker_context: Value,
    #[serde(default)]
    pub user_vrp_nq_context: Value,
    #[serde(default)]
    pub trade_usable: bool,
}

#[derive(Debug, Clone)]
pub struct RegimeConsumerBundleAdapter {
    pub status: BundleStatus,
    pub latest_decision: Option<RegimeDecisionSummary>,
    pub consumer_hints: Option<RegimeConsumerHints>,
    pub error: Option<String>,
}

impl RegimeConsumerBundleAdapter {
    pub fn disabled() -> Self {
        Self {
            status: BundleStatus::Disabled,
            latest_decision: None,
            consumer_hints: None,
            error: None,
        }
    }

    pub fn load_optional(path: Option<&Path>, strict: bool) -> Result<Self> {
        let Some(path) = path else {
            return Ok(Self::disabled());
        };
        if !path.exists() {
            let message = format!("regime consumer bundle missing: {}", path.display());
            if strict {
                return Err(anyhow!(message));
            }
            return Ok(Self::neutral(BundleStatus::Missing, message));
        }

        let raw = match std::fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(err) => {
                let message = format!("failed to read regime consumer bundle: {err}");
                if strict {
                    return Err(anyhow!(message));
                }
                return Ok(Self::neutral(BundleStatus::Invalid, message));
            }
        };
        let payload: Value = match serde_json::from_str(&raw) {
            Ok(payload) => payload,
            Err(err) => {
                let message = format!("invalid regime consumer bundle json: {err}");
                if strict {
                    return Err(anyhow!(message));
                }
                return Ok(Self::neutral(BundleStatus::Invalid, message));
            }
        };
        let schema = payload
            .get("schema_version")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if schema != EXPECTED_SCHEMA_VERSION {
            let message = format!("invalid regime consumer bundle schema: {schema}");
            if strict {
                return Err(anyhow!(message));
            }
            return Ok(Self::neutral(BundleStatus::Invalid, message));
        }

        let latest_decision = payload
            .get("latest_decision")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|err| anyhow!("invalid latest_decision: {err}"))?;
        let consumer_hints = payload
            .get("consumer_hints")
            .cloned()
            .map(serde_json::from_value)
            .transpose()
            .map_err(|err| anyhow!("invalid consumer_hints: {err}"))?;
        if latest_decision.is_none() || consumer_hints.is_none() {
            let message =
                "invalid regime consumer bundle: missing latest_decision or consumer_hints"
                    .to_string();
            if strict {
                return Err(anyhow!(message));
            }
            return Ok(Self::neutral(BundleStatus::Invalid, message));
        }

        Ok(Self {
            status: BundleStatus::Loaded,
            latest_decision,
            consumer_hints,
            error: None,
        })
    }

    pub fn is_loaded(&self) -> bool {
        self.status == BundleStatus::Loaded
    }

    pub fn execution_tree_hint(&self) -> ExecutionTreeHint {
        let raw = self
            .consumer_hints
            .as_ref()
            .map(|hints| hints.execution_tree_hint.as_str())
            .unwrap_or_default();
        match raw {
            "accept_regime" => ExecutionTreeHint::AcceptRegime,
            "transition_guardrail" => ExecutionTreeHint::TransitionGuardrail,
            _ => ExecutionTreeHint::UnknownAbstain,
        }
    }

    pub fn bbn_evidence_hint(&self) -> Option<&Value> {
        self.consumer_hints
            .as_ref()
            .map(|hints| &hints.bbn_evidence_hint)
            .filter(|value| !value.is_null())
    }

    pub fn to_read_only_bbn_soft_evidence(&self) -> RegimeReadOnlyBbnSoftEvidence {
        let hint = self.bbn_evidence_hint();
        let decision_state = hint
            .and_then(|value| value.get("regime_decision_state"))
            .and_then(Value::as_str)
            .or_else(|| {
                self.latest_decision
                    .as_ref()
                    .map(|decision| decision.decision_state.as_str())
            })
            .unwrap_or_default()
            .to_string();
        let trade_usable = hint
            .and_then(|value| value.get("regime_trade_usable"))
            .and_then(Value::as_bool)
            .or_else(|| {
                self.latest_decision
                    .as_ref()
                    .map(|decision| decision.trade_usable)
            });
        let label = hint
            .and_then(|value| value.get("regime_label"))
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .or_else(|| {
                self.latest_decision
                    .as_ref()
                    .map(|decision| decision.final_label.as_str())
                    .filter(|value| !value.is_empty())
                    .map(ToString::to_string)
            });
        let label_set = hint
            .and_then(|value| value.get("regime_label_set"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|items| !items.is_empty())
            .or_else(|| {
                self.latest_decision
                    .as_ref()
                    .map(|decision| decision.label_set.clone())
            })
            .unwrap_or_default();
        let transition_hazard = hint
            .and_then(|value| value.get("regime_transition_hazard"))
            .and_then(Value::as_f64);
        let reasons = hint
            .and_then(|value| value.get("regime_decision_reasons"))
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
            })
            .filter(|items| !items.is_empty())
            .or_else(|| {
                self.latest_decision
                    .as_ref()
                    .map(|decision| decision.abstain_reasons.clone())
            })
            .unwrap_or_default();
        let (strength, weight) = match (self.is_loaded(), decision_state.as_str(), trade_usable) {
            (true, "single_label_99", Some(true)) => (RegimeBbnEvidenceStrength::Strong, 0.9),
            (true, "single_label_95", Some(true)) | (true, "accepted", Some(true)) => {
                (RegimeBbnEvidenceStrength::Moderate, 0.65)
            }
            _ => (RegimeBbnEvidenceStrength::Neutral, 0.0),
        };

        RegimeReadOnlyBbnSoftEvidence {
            strength,
            weight,
            decision_state,
            trade_usable,
            label,
            label_set,
            transition_hazard,
            reasons,
        }
    }

    pub fn trace_entries(&self, path: Option<&Path>) -> Vec<String> {
        let mut entries = vec![format!(
            "regime_bundle_status={}",
            self.status.as_trace_value()
        )];
        if let Some(path) = path {
            entries.push(format!("regime_bundle_path={}", path.display()));
        }
        if let Some(error) = self.error.as_ref() {
            entries.push(format!(
                "regime_bundle_error={}",
                compact_trace_value(error)
            ));
        }
        if let Some(decision) = self.latest_decision.as_ref() {
            entries.push(format!(
                "regime_decision_state={}",
                compact_trace_value(&decision.decision_state)
            ));
            entries.push(format!("regime_trade_usable={}", decision.trade_usable));
            if !decision.final_label.is_empty() {
                entries.push(format!(
                    "regime_final_label={}",
                    compact_trace_value(&decision.final_label)
                ));
            }
        }
        entries.push(format!(
            "regime_execution_tree_hint={}",
            self.execution_tree_hint().as_trace_value()
        ));
        entries
    }

    pub fn bbn_soft_evidence_trace_entries(&self) -> Vec<String> {
        self.to_read_only_bbn_soft_evidence().trace_entries()
    }

    pub fn append_read_only_bbn_diagnostics(
        &self,
        artifact_action_summary: &mut Vec<String>,
        pre_bayes_filter: &mut PreBayesEvidenceFilter,
    ) {
        let bbn_trace_entries = self.bbn_soft_evidence_trace_entries();
        artifact_action_summary.push(format!(
            "regime_bbn_soft_evidence_trace:{}",
            bbn_trace_entries.join("|")
        ));
        artifact_action_summary.extend(bbn_trace_entries.iter().cloned());
        pre_bayes_filter.rationale.extend(
            bbn_trace_entries
                .iter()
                .map(|entry| format!("read_only_{entry}")),
        );
        for entry in bbn_trace_entries {
            if let Some((key, value)) = entry.split_once('=') {
                pre_bayes_filter
                    .evidence_assignments
                    .insert(format!("read_only_{key}"), value.to_string());
            }
        }
    }

    pub fn apply_bbn_soft_evidence_to_pre_bayes_filter(
        &self,
        filter: &mut PreBayesEvidenceFilter,
        opt_in: bool,
    ) -> RegimeBbnEvidenceApplicationStatus {
        if !opt_in {
            filter
                .rationale
                .push("regime_bundle_bbn_evidence_skipped=flag_disabled".to_string());
            return RegimeBbnEvidenceApplicationStatus::Skipped;
        }

        let evidence = self.to_read_only_bbn_soft_evidence();
        let Some(bbn_label) = evidence.bbn_market_regime_label() else {
            filter
                .rationale
                .push("regime_bundle_bbn_evidence_skipped=no_supported_label".to_string());
            return RegimeBbnEvidenceApplicationStatus::Skipped;
        };
        if evidence.strength == RegimeBbnEvidenceStrength::Neutral || evidence.weight <= 0.0 {
            filter.rationale.push(format!(
                "regime_bundle_bbn_evidence_skipped=strength:{}",
                evidence.strength.as_trace_value()
            ));
            return RegimeBbnEvidenceApplicationStatus::Skipped;
        }

        filter.uses_soft_evidence = true;
        filter.filtered_market_regime_label = bbn_label.to_string();
        filter.soft_market_regime_distribution =
            market_regime_distribution(bbn_label, evidence.weight);
        filter.evidence_assignments.insert(
            "regime_bundle_bbn_evidence_application".to_string(),
            "applied".to_string(),
        );
        filter.evidence_assignments.insert(
            "regime_bundle_bbn_market_regime".to_string(),
            bbn_label.to_string(),
        );
        filter.evidence_assignments.insert(
            "regime_bundle_bbn_evidence_weight".to_string(),
            format!("{:.3}", evidence.weight),
        );
        filter.rationale.push(format!(
            "regime_bundle_bbn_evidence_applied=strength:{} label:{}",
            evidence.strength.as_trace_value(),
            evidence.label.as_deref().unwrap_or("unknown")
        ));
        RegimeBbnEvidenceApplicationStatus::Applied
    }

    fn neutral(status: BundleStatus, error: String) -> Self {
        Self {
            status,
            latest_decision: None,
            consumer_hints: None,
            error: Some(error),
        }
    }
}

impl BundleStatus {
    fn as_trace_value(&self) -> &'static str {
        match self {
            BundleStatus::Disabled => "disabled",
            BundleStatus::Loaded => "loaded",
            BundleStatus::Missing => "missing",
            BundleStatus::Invalid => "invalid",
        }
    }
}

impl ExecutionTreeHint {
    fn as_trace_value(&self) -> &'static str {
        match self {
            ExecutionTreeHint::AcceptRegime => "accept_regime",
            ExecutionTreeHint::TransitionGuardrail => "transition_guardrail",
            ExecutionTreeHint::UnknownAbstain => "unknown_abstain",
        }
    }
}

impl RegimeBbnEvidenceStrength {
    fn as_trace_value(&self) -> &'static str {
        match self {
            RegimeBbnEvidenceStrength::Strong => "strong",
            RegimeBbnEvidenceStrength::Moderate => "moderate",
            RegimeBbnEvidenceStrength::Neutral => "neutral",
        }
    }
}

impl RegimeReadOnlyBbnSoftEvidence {
    fn trace_entries(&self) -> Vec<String> {
        let mut entries = vec![
            format!(
                "regime_bbn_soft_evidence_strength={}",
                self.strength.as_trace_value()
            ),
            format!("regime_bbn_soft_evidence_weight={:.3}", self.weight),
            format!(
                "regime_bbn_decision_state={}",
                compact_trace_value(&self.decision_state)
            ),
        ];
        if let Some(trade_usable) = self.trade_usable {
            entries.push(format!("regime_bbn_trade_usable={trade_usable}"));
        }
        if let Some(label) = self.label.as_ref() {
            entries.push(format!("regime_bbn_label={}", compact_trace_value(label)));
        }
        if !self.label_set.is_empty() {
            entries.push(format!(
                "regime_bbn_label_set={}",
                self.label_set
                    .iter()
                    .map(|label| compact_trace_value(label))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        if let Some(transition_hazard) = self.transition_hazard {
            entries.push(format!(
                "regime_bbn_transition_hazard={transition_hazard:.3}"
            ));
        }
        if !self.reasons.is_empty() {
            entries.push(format!(
                "regime_bbn_reasons={}",
                self.reasons
                    .iter()
                    .map(|reason| compact_trace_value(reason))
                    .collect::<Vec<_>>()
                    .join(",")
            ));
        }
        entries
    }

    fn bbn_market_regime_label(&self) -> Option<&'static str> {
        self.label
            .as_deref()
            .and_then(regime_bundle_label_to_bbn_market_regime)
    }
}

fn regime_bundle_label_to_bbn_market_regime(label: &str) -> Option<&'static str> {
    let primary = label.split('/').next().unwrap_or(label);
    match primary {
        "primary::TrendExpansion" | "TrendExpansion" => Some("bull"),
        "primary::RangeConsolidation" | "RangeConsolidation" => Some("range"),
        "primary::ExtremeStress" | "ExtremeStress" => Some("range"),
        "primary::ReversalBrewing" | "ReversalBrewing" => Some("range"),
        _ => None,
    }
}

fn market_regime_distribution(selected: &str, weight: f64) -> BTreeMap<String, f64> {
    let clamped = weight.clamp(0.0, 1.0);
    let remainder = (1.0 - clamped) / 2.0;
    ["bull", "bear", "range"]
        .into_iter()
        .map(|state| {
            let probability = if state == selected {
                clamped
            } else {
                remainder
            };
            (state.to_string(), probability)
        })
        .collect()
}

fn compact_trace_value(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join("_")
}
