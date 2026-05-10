use super::*;

fn structural_learning_evidence_line(
    credit_class: Option<&str>,
    success_credit: Option<f64>,
    observation_weight: Option<f64>,
) -> String {
    format!(
        "learning_semantics:{}",
        ict_engine::state::structural_learning_semantics_summary(
            credit_class,
            success_credit,
            observation_weight,
        )
    )
}

pub(crate) fn feedback_record_from_artifact(
    artifact: PendingUpdateArtifact,
    outcome_label: &str,
    pnl: Option<f64>,
    regime: Option<&str>,
    direction: Option<&str>,
) -> FeedbackRecord {
    let mut feedback = artifact.template_feedback;
    feedback.realized_outcome = outcome_label.to_string();
    feedback.pnl = pnl.unwrap_or_else(|| match outcome_label {
        "win" => 0.01,
        "loss" => -0.01,
        _ => 0.0,
    });
    if let Some(regime) = regime {
        feedback.regime_at_entry = normalize_regime_label(regime);
    }
    if let Some(direction) = direction {
        feedback.model_probabilities_before_trade.selected_direction =
            normalize_direction_label(direction);
    }
    feedback
}

pub(crate) fn latest_execution_candidate_for_source_run(
    state_dir: &str,
    symbol: &str,
    source_run_id: Option<&str>,
) -> Result<Option<ExecutionCandidateArtifact>> {
    let Some(source_run_id) = source_run_id else {
        return Ok(None);
    };
    Ok(load_execution_candidate_history(state_dir, symbol)?
        .into_iter()
        .rev()
        .find(|artifact| artifact.source_run_id.as_deref() == Some(source_run_id)))
}

pub(crate) fn latest_ensemble_vote_for_source_run(
    state_dir: &str,
    symbol: &str,
    source_run_id: Option<&str>,
) -> Result<Option<EnsembleVoteRecord>> {
    let Some(source_run_id) = source_run_id else {
        return Ok(None);
    };
    Ok(load_ensemble_vote_history(state_dir, symbol)?
        .into_iter()
        .rev()
        .find(|artifact| artifact.source_run_id.as_deref() == Some(source_run_id)))
}

pub(crate) fn derive_executor_scorecards_from_summaries(
    executor_summaries: &[String],
) -> Vec<EnsembleExecutorScorecard> {
    executor_summaries
        .iter()
        .map(|summary| EnsembleExecutorScorecard {
            executor: summary
                .split_whitespace()
                .find_map(|part| part.strip_prefix("executor="))
                .unwrap_or("executor_unavailable")
                .to_string(),
            latest_weight_hint: summary
                .split_whitespace()
                .find_map(|part| part.strip_prefix("weight="))
                .and_then(|value| value.parse::<f64>().ok()),
            ..EnsembleExecutorScorecard::default()
        })
        .collect()
}

pub(crate) fn load_canonical_executor_scorecards(
    state_dir: &str,
    symbol: &str,
    source_run_id: Option<&str>,
) -> Result<Vec<EnsembleExecutorScorecard>> {
    let persisted = load_ensemble_executor_scorecards(state_dir, symbol).unwrap_or_default();
    if !persisted.is_empty() {
        return Ok(persisted);
    }
    Ok(
        latest_ensemble_vote_for_source_run(state_dir, symbol, source_run_id)?
            .map(|artifact| {
                if artifact.executor_scorecards.is_empty() {
                    derive_executor_scorecards_from_summaries(&artifact.executor_summaries)
                } else {
                    artifact.executor_scorecards
                }
            })
            .unwrap_or_default(),
    )
}

pub(crate) fn apply_update_outcome_to_executor_scorecards(
    scorecards: &mut [EnsembleExecutorScorecard],
    realized_outcome: &str,
    quality_adjustment: i64,
) {
    for scorecard in scorecards {
        match realized_outcome.trim().to_ascii_lowercase().as_str() {
            "win" => scorecard.wins += 1,
            "loss" | "invalidated" => scorecard.losses += 1,
            "breakeven" | "abandoned" => scorecard.breakevens += 1,
            _ => {}
        }
        match realized_outcome.trim().to_ascii_lowercase().as_str() {
            "win" => scorecard.validated_positive += 1,
            "loss" | "invalidated" => scorecard.validated_negative += 1,
            _ => {}
        }
        scorecard.cumulative_quality_score += quality_adjustment;
        scorecard.last_outcome = Some(realized_outcome.to_string());
        scorecard.last_updated_at = Some(Utc::now());
    }
}

pub(crate) fn build_ensemble_vote_record(
    symbol: &str,
    source_phase: &str,
    source_run_id: Option<String>,
    provenance: &RunProvenance,
    dataset_comparability: &DatasetComparability,
    ensemble_vote: &ict_engine::application::orchestration::EnsembleVoteArtifact,
    compatibility_scorecards: &[EnsembleExecutorScorecard],
) -> EnsembleVoteRecord {
    EnsembleVoteRecord {
        artifact_id: format!(
            "ensemble-vote:{}:{}",
            source_phase,
            Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        ),
        generated_at: Utc::now(),
        symbol: symbol.to_string(),
        source_phase: source_phase.to_string(),
        source_run_id,
        provenance: provenance.clone(),
        dataset_comparability: dataset_comparability.clone(),
        ensemble_version: ensemble_vote.ensemble_version.clone(),
        final_action: ensemble_vote.final_action.clone(),
        recommended_command: ensemble_vote.recommended_command.clone(),
        human_next_triage: ensemble_vote.human_next_triage.clone(),
        hard_block: ensemble_vote.hard_block.clone(),
        confidence: ensemble_vote.confidence,
        consensus_strength: ensemble_vote.consensus_strength,
        disagreement_flags: ensemble_vote.disagreement_flags.clone(),
        executor_summaries: ensemble_vote.executor_summaries.clone(),
        split_explanations: ensemble_vote.split_explanations.clone(),
        executor_scorecards: compatibility_scorecards.to_vec(),
        executor_scorecards_source: Some("persisted".to_string()),
        posterior_fingerprint: ensemble_vote.posterior.fingerprint.clone(),
        posterior_normalization_status: ensemble_vote.posterior.normalization_status.clone(),
        posterior_active_regime: ensemble_vote.posterior.active_regime.clone(),
        posterior_confidence: ensemble_vote.posterior.confidence,
        posterior_probabilities: ensemble_vote.posterior.probabilities.clone(),
        posterior_evidence: ensemble_vote.posterior.evidence.clone(),
    }
}

pub(crate) fn persist_ensemble_vote_record(
    state_dir: &str,
    record: &EnsembleVoteRecord,
    canonical_scorecards: &[EnsembleExecutorScorecard],
) -> Result<()> {
    append_artifact_ledger_entry(
        state_dir,
        &record.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", record.artifact_id),
            artifact_kind: "ensemble_vote".to_string(),
            artifact_id: record.artifact_id.clone(),
            version: 1,
            generated_at: record.generated_at,
            symbol: record.symbol.clone(),
            source_phase: record.source_phase.clone(),
            source_run_id: record.source_run_id.clone(),
            path: std::path::Path::new(state_dir)
                .join(&record.symbol)
                .join(ENSEMBLE_VOTE_FILE)
                .to_string_lossy()
                .to_string(),
            status: if record.disagreement_flags.is_empty() {
                "consensus".to_string()
            } else {
                "mixed".to_string()
            },
            promote_candidate: record.confidence >= 0.60 && record.disagreement_flags.is_empty(),
            actionable: true,
            decision_hint: record.final_action.clone(),
            review_reason: record.human_next_triage.clone(),
            review_rule_version: record.ensemble_version.clone(),
            top_factor_name: None,
            top_factor_action: Some(record.final_action.clone()),
            family_scores: BTreeMap::new(),
            supersedes_artifact_id: None,
            quality_score: ((record.confidence + record.consensus_strength) * 50.0) as i32,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;
    save_ensemble_vote_artifact(state_dir, &record.symbol, record)?;
    save_ensemble_executor_scorecards(state_dir, &record.symbol, canonical_scorecards)?;
    append_ensemble_vote_history(state_dir, &record.symbol, record.clone())?;
    Ok(())
}

pub(crate) fn emit_update_output(report: &UpdateReport, ensemble: bool) -> Result<()> {
    let reflection_evidence = report
        .agent_prompts
        .prompts
        .iter()
        .map(|prompt| format!("{}:{}:{}", prompt.stage, prompt.id, prompt.objective))
        .chain(std::iter::once(structural_learning_evidence_line(
            report.structural_learning_credit_class.as_deref(),
            report.structural_learning_success_credit,
            report.structural_learning_observation_weight,
        )))
        .collect::<Vec<_>>();
    let reflection_next_candidates = report
        .recommended_next_command
        .split(';')
        .map(str::trim)
        .filter(|candidate| !candidate.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let reflection_bundle = build_reflection_bundle(ReflectionBundleInput {
        symbol: report.symbol.clone(),
        timestamp: report.provenance.data_fingerprint.clone(),
        objective: report.agent_prompts.workflow.clone(),
        expected_regime: report.workflow_state.phase.clone(),
        expected_direction: report
            .agent_action_plan
            .items
            .first()
            .map(|item| item.title.clone())
            .filter(|title| !title.is_empty())
            .unwrap_or_else(|| report.realized_outcome.clone()),
        realized_outcome: report.realized_outcome.clone(),
        evidence: reflection_evidence,
        next_candidates: reflection_next_candidates,
    });
    let mut reflection_bundle = reflection_bundle;
    if let Ok(artifact) = load_state_or_default::<ExecutionTreeArtifact, _>(
        &report.state_dir,
        &report.symbol,
        EXECUTION_TREE_TRACE_FILE,
    ) {
        reflection_bundle.execution_shap_top_k = artifact.execution_shap_top_k;
    }
    if let Ok(artifact) =
        ict_engine::pda_sequence::load_pda_sequence_analysis(&report.state_dir, &report.symbol)
    {
        ict_engine::application::reflection::apply_pda_sequence_artifact_to_reflection_bundle(
            &mut reflection_bundle,
            &artifact,
        );
    }
    let ensemble_surface = if ensemble {
        report
            .workflow_snapshot
            .latest_ensemble_vote
            .as_ref()
            .map(|vote| {
                let persisted_scorecards =
                    load_ensemble_executor_scorecards(&report.state_dir, &report.symbol)
                        .unwrap_or_default();
                let (scorecards, scorecard_source) =
                    resolved_vote_scorecards(&persisted_scorecards, vote);
                serde_json::json!({
                    "ensemble_vote": vote,
                    "executor_scorecards": scorecards,
                    "executor_scorecard_source": scorecard_source,
                })
            })
    } else {
        None
    };
    println!(
        "{}",
        serde_json::to_string_pretty(&serde_json::json!({
            "report": report,
            "reflection_bundle": reflection_bundle,
            "ensemble": ensemble_surface,
        }))?
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feedback_record_from_artifact_preserves_raw_not_followed_outcome() {
        let mut artifact = PendingUpdateArtifact::default();
        artifact.template_feedback.realized_outcome = "pending".to_string();

        let feedback = feedback_record_from_artifact(artifact, "not_followed", None, None, None);

        assert_eq!(feedback.realized_outcome, "not_followed");
        assert_eq!(feedback.pnl, 0.0);
    }

    #[test]
    fn apply_update_outcome_to_executor_scorecards_skips_not_followed_and_penalizes_invalidated() {
        let mut scorecards = vec![EnsembleExecutorScorecard::default()];

        apply_update_outcome_to_executor_scorecards(&mut scorecards, "not_followed", 0);
        assert_eq!(scorecards[0].wins, 0);
        assert_eq!(scorecards[0].losses, 0);
        assert_eq!(scorecards[0].breakevens, 0);
        assert_eq!(scorecards[0].validated_negative, 0);

        apply_update_outcome_to_executor_scorecards(&mut scorecards, "invalidated", 0);
        assert_eq!(scorecards[0].losses, 1);
        assert_eq!(scorecards[0].validated_negative, 1);
    }

    #[test]
    fn structural_learning_evidence_line_includes_fractional_semantics() {
        let line =
            structural_learning_evidence_line(Some("fractional_abandoned"), Some(0.25), Some(0.75));

        assert_eq!(
            line,
            "learning_semantics:class=fractional_abandoned success_credit=0.250 observation_weight=0.750"
        );
    }
}
