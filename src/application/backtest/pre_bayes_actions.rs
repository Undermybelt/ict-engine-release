use crate::state::{AgentActionItem, AgentActionPlan, ExpectedStateChange, PreBayesEvidenceFilter};

pub fn workflow_state_from_pre_bayes_filter(
    base: crate::state::WorkflowState,
    filter: &PreBayesEvidenceFilter,
) -> crate::state::WorkflowState {
    if filter
        .conflict_flags
        .iter()
        .any(|flag| flag == "pda_sequence_cluster_weak")
    {
        return crate::state::WorkflowState {
            phase: "pda_sequence_review".to_string(),
            reason: filter.rationale.join(";"),
        };
    }
    match filter.gating_status.as_str() {
        "observe_only" => crate::state::WorkflowState {
            phase: "pre_bayes_observe_only".to_string(),
            reason: filter.rationale.join(";"),
        },
        "pass_neutralized" => crate::state::WorkflowState {
            phase: "pre_bayes_neutralized_review".to_string(),
            reason: filter.rationale.join(";"),
        },
        _ => base,
    }
}

pub fn augment_action_plan_with_pre_bayes_filter(
    action_plan: &mut AgentActionPlan,
    filter: &PreBayesEvidenceFilter,
) {
    let pda_sequence_cluster_weak = filter
        .conflict_flags
        .iter()
        .any(|flag| flag == "pda_sequence_cluster_weak");
    if pda_sequence_cluster_weak {
        action_plan.items.insert(
            0,
            AgentActionItem {
                stage: "pda_sequence_review".to_string(),
                blocking: true,
                priority: "high".to_string(),
                title: pda_sequence_review_title(filter),
                rationale: format!(
                    "{}; {}",
                    pda_sequence_review_rationale(filter),
                    filter.rationale.join(";")
                ),
                expected_output:
                    "A PDA sequence review deciding whether cluster quality is too weak to reinforce the current market regime gate"
                        .to_string(),
                expected_state_changes: vec![ExpectedStateChange {
                    target: "pda_sequence_artifact".to_string(),
                    direction: "review_cluster_quality".to_string(),
                    rationale: filter.rationale.join(";"),
                }],
                suggested_files: pda_sequence_review_files(filter),
                suggested_commands: pda_sequence_review_commands(filter),
            },
        );
    }
    if filter.gating_status == "pass_hard" && filter.conflict_flags.is_empty() {
        return;
    }
    action_plan.items.insert(
        if pda_sequence_cluster_weak { 1 } else { 0 },
        AgentActionItem {
            stage: "analyze".to_string(),
            blocking: filter.gating_status == "observe_only",
            priority: "high".to_string(),
            title: "Review Pre-Bayes Evidence Gate".to_string(),
            rationale: filter.rationale.join(";"),
            expected_output:
                "A pre-Bayes gate review confirming whether evidence should pass hard, pass neutralized, or remain observe-only".to_string(),
            expected_state_changes: vec![ExpectedStateChange {
                target: "pre_bayes_evidence_filter".to_string(),
                direction: filter.gating_status.clone(),
                rationale: filter.rationale.join(";"),
            }],
            suggested_files: vec![
                "src/main.rs".to_string(),
                "src/factor_lab/engine.rs".to_string(),
                "src/bbn/trading/update.rs".to_string(),
            ],
            suggested_commands: vec![
                "ict-engine analyze --data-htf <file> --data-mtf <file> --data-ltf <file>"
                    .to_string(),
            ],
        },
    );
}

pub fn augment_action_plan_with_consumed_pre_bayes_context(
    action_plan: &mut AgentActionPlan,
    filter: &PreBayesEvidenceFilter,
    bridge: Option<&crate::state::PreBayesEntryQualityBridge>,
) {
    let bridge_diff = bridge.map(crate::application::backtest::pre_bayes_entry_quality_bridge_diff);
    action_plan.items.insert(
        0,
        AgentActionItem {
            stage: "update".to_string(),
            blocking: filter.gating_status == "observe_only" || filter.uses_soft_evidence,
            priority: "high".to_string(),
            title: "Review Consumed Pre-Bayes".to_string(),
            rationale: format!(
                "consumed_gate_status={} consumed_quality={:.3} consumed_bridge_selected_entry_quality={:?}",
                filter.gating_status,
                filter.evidence_quality_score,
                bridge_diff
                    .as_ref()
                    .and_then(|diff| diff.selected_entry_quality.clone())
            ),
            expected_output:
                "A feedback note that judges whether the realized outcome confirms or invalidates the consumed pre-bayes gate and bridge".to_string(),
            expected_state_changes: vec![ExpectedStateChange {
                target: "consumed_pre_bayes_context".to_string(),
                direction: "review_against_realized_outcome".to_string(),
                rationale: filter.rationale.join(";"),
            }],
            suggested_files: vec![
                "src/main.rs".to_string(),
                "src/bbn/trading/update.rs".to_string(),
                "src/state/types.rs".to_string(),
            ],
            suggested_commands: vec!["ict-engine update --feedback-file <file>".to_string()],
        },
    );
}

pub fn pda_sequence_review_title(filter: &PreBayesEvidenceFilter) -> String {
    if filter_has(filter, "pda_sequence_sparse_sessions") {
        "Review PDA Sequence Coverage".to_string()
    } else if filter_has(filter, "pda_sequence_low_consistency") {
        "Review PDA Sequence Consistency".to_string()
    } else if filter_has(filter, "pda_sequence_low_confidence") {
        "Review PDA Sequence Confidence".to_string()
    } else {
        "Review PDA Sequence Cluster".to_string()
    }
}

pub fn pda_sequence_review_files(filter: &PreBayesEvidenceFilter) -> Vec<String> {
    if filter_has(filter, "pda_sequence_sparse_sessions") {
        vec![
            "src/pda_sequence/emitter.rs".to_string(),
            "src/ict/pda_state.rs".to_string(),
            "src/config.rs".to_string(),
        ]
    } else if filter_has(filter, "pda_sequence_low_consistency") {
        vec![
            "src/pda_sequence/analysis.rs".to_string(),
            "src/pda_sequence/hmm_cluster.rs".to_string(),
            "src/pda_sequence/ensemble_cluster.rs".to_string(),
        ]
    } else if filter_has(filter, "pda_sequence_low_confidence") {
        vec![
            "src/pda_sequence/fcgr.rs".to_string(),
            "src/pda_sequence/cluster.rs".to_string(),
            "src/pda_sequence/kmedoids.rs".to_string(),
        ]
    } else {
        vec![
            "src/pda_sequence/analysis.rs".to_string(),
            "src/pda_sequence/fcgr.rs".to_string(),
            "src/config.rs".to_string(),
        ]
    }
}

pub fn pda_sequence_review_rationale(filter: &PreBayesEvidenceFilter) -> String {
    if filter_has(filter, "pda_sequence_sparse_sessions") {
        "PDA sequence reinforcement is unreliable because too few valid sessions were emitted"
            .to_string()
    } else if filter_has(filter, "pda_sequence_low_consistency") {
        "PDA sequence reinforcement is unreliable because DTW/HMM agreement is too low".to_string()
    } else if filter_has(filter, "pda_sequence_low_confidence") {
        "PDA sequence reinforcement is unreliable because the winning cluster confidence is too low"
            .to_string()
    } else {
        "PDA sequence reinforcement requires manual review before it can influence gating"
            .to_string()
    }
}

pub fn pda_sequence_review_commands(filter: &PreBayesEvidenceFilter) -> Vec<String> {
    if filter_has(filter, "pda_sequence_sparse_sessions") {
        vec![
            "cargo test pda_sequence::emitter -- --nocapture".to_string(),
            "ict-engine analyze --data-htf <file> --data-mtf <file> --data-ltf <file>".to_string(),
        ]
    } else if filter_has(filter, "pda_sequence_low_consistency") {
        vec![
            "cargo test pda_sequence::analysis -- --nocapture".to_string(),
            "cargo test pda_sequence::hmm_cluster -- --nocapture".to_string(),
        ]
    } else if filter_has(filter, "pda_sequence_low_confidence") {
        vec![
            "cargo test pda_sequence::fcgr -- --nocapture".to_string(),
            "cargo test pda_sequence::cluster -- --nocapture".to_string(),
        ]
    } else {
        vec!["cargo test pda_sequence -- --nocapture".to_string()]
    }
}

fn filter_has(filter: &PreBayesEvidenceFilter, flag: &str) -> bool {
    filter.conflict_flags.iter().any(|item| item == flag)
}
