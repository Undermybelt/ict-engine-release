use super::*;

pub(crate) struct EnsembleVoteOverlayInput<'a> {
    pub(crate) recent_ensemble_votes: Vec<EnsembleVoteRecord>,
    pub(crate) analyze_runs: &'a [AnalyzeRunRecord],
    pub(crate) research_runs: &'a [ResearchRunRecord],
    pub(crate) backtest_runs: &'a [BacktestRunRecord],
    pub(crate) update_runs: &'a [UpdateRunRecord],
    pub(crate) latest_analyze: Option<&'a AnalyzeRunRecord>,
    pub(crate) latest_research: Option<&'a ResearchRunRecord>,
    pub(crate) latest_backtest: Option<&'a BacktestRunRecord>,
    pub(crate) latest_update: Option<&'a UpdateRunRecord>,
}

pub(crate) struct BuildWorkflowSnapshotInput<'a> {
    pub(crate) state_dir: &'a str,
    pub(crate) symbol: &'a str,
    pub(crate) analyze_runs: &'a [AnalyzeRunRecord],
    pub(crate) research_runs: &'a [ResearchRunRecord],
    pub(crate) backtest_runs: &'a [BacktestRunRecord],
    pub(crate) update_runs: &'a [UpdateRunRecord],
    pub(crate) latest_train: Option<&'a TrainRunRecord>,
    pub(crate) latest_analyze: Option<&'a AnalyzeRunRecord>,
    pub(crate) latest_research: Option<&'a ResearchRunRecord>,
    pub(crate) latest_backtest: Option<&'a BacktestRunRecord>,
    pub(crate) latest_update: Option<&'a UpdateRunRecord>,
    pub(crate) pre_bayes_policy_history: &'a [PreBayesPolicyRecord],
    pub(crate) pending_update_history: &'a [PendingUpdateArtifact],
    pub(crate) execution_candidate_history: &'a [ExecutionCandidateArtifact],
    pub(crate) artifact_ledger: &'a [ArtifactLedgerEntry],
}

pub(crate) struct SyntheticPhaseEnsembleVoteInput<'a> {
    pub(crate) symbol: &'a str,
    pub(crate) timestamp: chrono::DateTime<Utc>,
    pub(crate) source_phase: &'a str,
    pub(crate) run_id: &'a str,
    pub(crate) provenance: &'a RunProvenance,
    pub(crate) dataset_comparability: &'a DatasetComparability,
    pub(crate) recommended_next_command: &'a str,
    pub(crate) canonical: &'a ict_engine::state::CanonicalStructuralRegimePosterior,
}

pub(crate) fn overlay_or_synthesize_phase_ensemble_votes(
    input: EnsembleVoteOverlayInput<'_>,
) -> Vec<EnsembleVoteRecord> {
    let EnsembleVoteOverlayInput {
        mut recent_ensemble_votes,
        analyze_runs,
        research_runs,
        backtest_runs,
        update_runs,
        latest_analyze,
        latest_research,
        latest_backtest,
        latest_update,
    } = input;
    if analyze_runs.is_empty()
        && research_runs.is_empty()
        && backtest_runs.is_empty()
        && update_runs.is_empty()
        && latest_analyze.is_none()
    {
        return recent_ensemble_votes;
    }

    let analyze_by_run_id = analyze_runs
        .iter()
        .filter_map(|run| {
            run.canonical_structural_regime_posterior
                .as_ref()
                .map(|canonical| (run.run_id.as_str(), canonical))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let research_by_run_id = research_runs
        .iter()
        .filter_map(|run| {
            run.canonical_structural_regime_posterior
                .as_ref()
                .map(|canonical| (run.run_id.as_str(), canonical))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let backtest_by_run_id = backtest_runs
        .iter()
        .filter_map(|run| {
            run.canonical_structural_regime_posterior
                .as_ref()
                .map(|canonical| (run.run_id.as_str(), canonical))
        })
        .collect::<std::collections::BTreeMap<_, _>>();
    let update_by_run_id = update_runs
        .iter()
        .filter_map(|run| {
            run.consumed_canonical_structural_regime_posterior
                .as_ref()
                .map(|canonical| (run.run_id.as_str(), canonical))
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    if recent_ensemble_votes.is_empty() {
        let latest_synthetic = [
            latest_analyze.and_then(|analyze| {
                analyze
                    .canonical_structural_regime_posterior
                    .as_ref()
                    .map(|canonical| {
                        (
                            analyze.timestamp,
                            SyntheticPhaseEnsembleVoteInput {
                                symbol: &analyze.symbol,
                                timestamp: analyze.timestamp,
                                source_phase: "analyze",
                                run_id: &analyze.run_id,
                                provenance: &analyze.provenance,
                                dataset_comparability: &analyze.dataset_comparability,
                                recommended_next_command: &analyze.recommended_next_command,
                                canonical,
                            },
                        )
                    })
            }),
            latest_research.and_then(|research| {
                research
                    .canonical_structural_regime_posterior
                    .as_ref()
                    .map(|canonical| {
                        (
                            research.timestamp,
                            SyntheticPhaseEnsembleVoteInput {
                                symbol: &research.symbol,
                                timestamp: research.timestamp,
                                source_phase: "research",
                                run_id: &research.run_id,
                                provenance: &research.provenance,
                                dataset_comparability: &research.dataset_comparability,
                                recommended_next_command: &research.recommended_next_command,
                                canonical,
                            },
                        )
                    })
            }),
            latest_backtest.and_then(|backtest| {
                backtest
                    .canonical_structural_regime_posterior
                    .as_ref()
                    .map(|canonical| {
                        (
                            backtest.timestamp,
                            SyntheticPhaseEnsembleVoteInput {
                                symbol: &backtest.symbol,
                                timestamp: backtest.timestamp,
                                source_phase: "backtest",
                                run_id: &backtest.run_id,
                                provenance: &backtest.provenance,
                                dataset_comparability: &backtest.dataset_comparability,
                                recommended_next_command: &backtest.recommended_next_command,
                                canonical,
                            },
                        )
                    })
            }),
            latest_update.and_then(|update| {
                update
                    .consumed_canonical_structural_regime_posterior
                    .as_ref()
                    .map(|canonical| {
                        (
                            update.timestamp,
                            SyntheticPhaseEnsembleVoteInput {
                                symbol: &update.symbol,
                                timestamp: update.timestamp,
                                source_phase: "update",
                                run_id: &update.run_id,
                                provenance: &update.provenance,
                                dataset_comparability: &update.dataset_comparability,
                                recommended_next_command: &update.recommended_next_command,
                                canonical,
                            },
                        )
                    })
            }),
        ]
        .into_iter()
        .flatten()
        .max_by(|left, right| {
            left.0
                .cmp(&right.0)
                .then_with(|| left.1.run_id.cmp(right.1.run_id))
        });

        if let Some((_, input)) = latest_synthetic {
            if let Some(synthetic) = synthetic_phase_ensemble_vote(input) {
                recent_ensemble_votes.push(synthetic);
            }
        }
        return recent_ensemble_votes;
    }

    for vote in &mut recent_ensemble_votes {
        if let Some(run_id) = vote.source_run_id.as_deref() {
            let canonical = match vote.source_phase.as_str() {
                "analyze" => analyze_by_run_id.get(run_id).copied(),
                "factor-research" | "research" => research_by_run_id.get(run_id).copied(),
                "factor-backtest" | "backtest" => backtest_by_run_id.get(run_id).copied(),
                "update" => update_by_run_id.get(run_id).copied(),
                _ => None,
            };
            if let Some(canonical) = canonical {
                overlay_analyze_canonical_regime_on_ensemble_vote(vote, canonical);
            }
        }
    }
    recent_ensemble_votes
}

pub(crate) fn refresh_workflow_snapshot(state_dir: &str, symbol: &str) -> Result<WorkflowSnapshot> {
    let analyze_runs: Vec<AnalyzeRunRecord> =
        load_state_or_default(state_dir, symbol, ANALYZE_RUNS_FILE)?;
    let train_runs: Vec<TrainRunRecord> =
        load_state_or_default(state_dir, symbol, TRAIN_RUNS_FILE)?;
    let research_runs: Vec<ResearchRunRecord> =
        load_state_or_default(state_dir, symbol, RESEARCH_RUNS_FILE)?;
    let backtest_runs: Vec<BacktestRunRecord> =
        load_state_or_default(state_dir, symbol, BACKTEST_RUNS_FILE)?;
    let update_runs: Vec<UpdateRunRecord> =
        load_state_or_default(state_dir, symbol, UPDATE_RUNS_FILE)?;
    let pre_bayes_policy_history = load_pre_bayes_policy_history(state_dir, symbol)?;
    let pending_update_history = load_pending_update_history(state_dir, symbol)?;
    let execution_candidate_history = load_execution_candidate_history(state_dir, symbol)?;
    let artifact_ledger = load_artifact_ledger(state_dir, symbol)?;

    let snapshot = build_workflow_snapshot(BuildWorkflowSnapshotInput {
        state_dir,
        symbol,
        analyze_runs: &analyze_runs,
        research_runs: &research_runs,
        backtest_runs: &backtest_runs,
        update_runs: &update_runs,
        latest_train: train_runs.last(),
        latest_analyze: analyze_runs.last(),
        latest_research: research_runs.last(),
        latest_backtest: backtest_runs.last(),
        latest_update: update_runs.last(),
        pre_bayes_policy_history: &pre_bayes_policy_history,
        pending_update_history: &pending_update_history,
        execution_candidate_history: &execution_candidate_history,
        artifact_ledger: &artifact_ledger,
    });
    save_workflow_snapshot(state_dir, symbol, &snapshot)?;
    Ok(snapshot)
}

pub(crate) fn build_workflow_snapshot(input: BuildWorkflowSnapshotInput<'_>) -> WorkflowSnapshot {
    let BuildWorkflowSnapshotInput {
        state_dir,
        symbol,
        analyze_runs,
        research_runs,
        backtest_runs,
        update_runs,
        latest_train,
        latest_analyze,
        latest_research,
        latest_backtest,
        latest_update,
        pre_bayes_policy_history,
        pending_update_history,
        execution_candidate_history,
        artifact_ledger,
    } = input;

    let train = latest_train.map(workflow_phase_snapshot_from_train_run);
    let analyze = latest_analyze.map(workflow_phase_snapshot_from_analyze_run);
    let research = latest_research.map(workflow_phase_snapshot_from_research_run);
    let backtest = latest_backtest.map(workflow_phase_snapshot_from_backtest_run);
    let update = latest_update.map(workflow_phase_snapshot_from_update_run);
    let field_diffs = workflow_field_diffs(&analyze, &research, &backtest, &update);
    let disagreements = workflow_disagreements(&analyze, &research, &backtest, &update);
    let recent_pending_updates = pending_update_history
        .iter()
        .rev()
        .take(5)
        .map(|artifact| pending_update_summary(state_dir, symbol, artifact))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let recent_execution_candidates = execution_candidate_history
        .iter()
        .rev()
        .take(5)
        .map(|artifact| execution_candidate_summary(state_dir, symbol, artifact))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let mut recent_ensemble_votes = load_ensemble_vote_history(state_dir, symbol)
        .unwrap_or_default()
        .into_iter()
        .rev()
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    recent_ensemble_votes = overlay_or_synthesize_phase_ensemble_votes(EnsembleVoteOverlayInput {
        recent_ensemble_votes,
        analyze_runs,
        research_runs,
        backtest_runs,
        update_runs,
        latest_analyze,
        latest_research,
        latest_backtest,
        latest_update,
    });
    let recent_artifacts = artifact_ledger
        .iter()
        .rev()
        .take(10)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let actionable_artifacts = artifact_ledger
        .iter()
        .filter(|entry| entry.actionable && entry.consumed_by_update_run_id.is_none())
        .cloned()
        .collect::<Vec<_>>();
    let latest_promotable_artifact = artifact_ledger
        .iter()
        .filter(|entry| entry.promote_candidate && entry.consumed_by_update_run_id.is_none())
        .max_by_key(|entry| artifact_generated_recency_key(entry))
        .cloned();
    let artifact_history_summary = build_artifact_history_summary(artifact_ledger);
    let artifact_factor_trends =
        build_artifact_factor_trends(artifact_ledger, &research, &backtest, &update);
    let artifact_family_trends =
        build_artifact_family_trends(artifact_ledger, &research, &backtest, &update);
    let review_rules = artifact_review_rules();
    let review_rule_sources = artifact_review_rule_sources();
    let artifact_lineage_summaries = build_artifact_lineage_summaries_with_embedded_snapshots(
        artifact_ledger,
        pending_update_history,
        execution_candidate_history,
    );
    let artifact_consumed_impact_summary = build_artifact_consumed_impact_summary(artifact_ledger);
    let artifact_decision_summary = artifact_decision_summary_from_trends(
        &actionable_artifacts,
        latest_promotable_artifact.as_ref(),
        &artifact_lineage_summaries,
        &artifact_factor_trends,
        &artifact_family_trends,
        &artifact_consumed_impact_summary,
    );
    let latest_pre_bayes_policy =
        latest_analyze.map(|run| run.pre_bayes_evidence_filter.policy.clone());
    let latest_pre_bayes_entry_quality_bridge =
        latest_analyze.map(|run| run.pre_bayes_entry_quality_bridge.clone());
    let latest_pre_bayes_entry_quality_bridge_diff = latest_analyze
        .map(|run| pre_bayes_entry_quality_bridge_diff(&run.pre_bayes_entry_quality_bridge));
    let recent_pre_bayes_policies = pre_bayes_policy_history
        .iter()
        .rev()
        .take(5)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let latest_pre_bayes_policy_diff = recent_pre_bayes_policies
        .last()
        .map(|record| record.diff_from_previous.clone());
    let latest_pre_bayes_policy_lineage = Some(pre_bayes_policy_lineage_summary(
        &recent_pre_bayes_policies,
        latest_analyze
            .map(|run| run.pre_bayes_evidence_filter.gating_status.as_str())
            .unwrap_or(""),
    ));
    let latest_pre_bayes_soft_evidence_diff = latest_analyze
        .map(|run| pre_bayes_soft_evidence_diff(&run.pre_bayes_evidence_filter))
        .unwrap_or_default();
    let artifact_rule_break_effects = build_artifact_rule_break_effects(artifact_ledger);
    let artifact_factor_rule_break_impacts =
        build_artifact_factor_rule_break_impacts(artifact_ledger, &artifact_rule_break_effects);
    let artifact_family_rule_break_impacts =
        build_artifact_family_rule_break_impacts(artifact_ledger, &artifact_rule_break_effects);
    let mut phases = [
        train.clone(),
        analyze.clone(),
        research.clone(),
        backtest.clone(),
        update.clone(),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>();
    phases.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
    let current = phases.last().cloned();
    let blocking_truth = workflow_blocking_truth(
        symbol,
        state_dir,
        current.as_ref(),
        latest_analyze,
        &artifact_decision_summary,
    );

    let mut risk_flags = std::collections::BTreeSet::new();
    for phase in [
        train.as_ref(),
        analyze.as_ref(),
        research.as_ref(),
        backtest.as_ref(),
        update.as_ref(),
    ]
    .into_iter()
    .flatten()
    {
        for flag in &phase.risk_flags {
            risk_flags.insert(format!("{}:{}", phase.phase, flag));
        }
    }

    let current_recommended_next_command = current
        .as_ref()
        .map(|phase| phase.recommended_next_command.clone())
        .unwrap_or_default();
    let current_recommended_next_command_meta = current
        .as_ref()
        .map(|phase| {
            if phase.recommended_next_command_meta.kind
                == ict_engine::state::RecommendedNextCommandKind::Unknown
                && !phase.recommended_next_command.is_empty()
            {
                recommended_next_command_meta(&phase.recommended_next_command)
            } else {
                phase.recommended_next_command_meta.clone()
            }
        })
        .unwrap_or_else(|| recommended_next_command_meta(&current_recommended_next_command));

    WorkflowSnapshot {
        symbol: symbol.to_string(),
        generated_at: Utc::now(),
        current_focus_phase: current
            .as_ref()
            .map(|phase| phase.phase.clone())
            .unwrap_or_default(),
        current_focus_reason: current
            .as_ref()
            .map(|phase| phase.workflow_reason.clone())
            .unwrap_or_default(),
        blocking_truth,
        recommended_next_command: current_recommended_next_command,
        recommended_next_command_meta: current_recommended_next_command_meta,
        pending_actions: current.map(|phase| phase.top_actions).unwrap_or_default(),
        risk_flags: risk_flags
            .into_iter()
            .chain(
                disagreements
                    .iter()
                    .map(|item| format!("{}:{}", item.severity, item.id)),
            )
            .collect(),
        latest_train: train,
        latest_analyze: analyze,
        latest_research: research,
        latest_backtest: backtest,
        latest_update: update,
        latest_pre_bayes_policy,
        latest_pre_bayes_entry_quality_bridge,
        latest_pre_bayes_entry_quality_bridge_diff,
        latest_pre_bayes_policy_diff,
        latest_pre_bayes_policy_lineage,
        latest_pre_bayes_soft_evidence_diff,
        recent_pre_bayes_policies,
        latest_pending_update: recent_pending_updates.last().cloned(),
        recent_pending_updates,
        latest_execution_candidate: recent_execution_candidates.last().cloned(),
        recent_execution_candidates,
        latest_ensemble_vote: recent_ensemble_votes.last().cloned(),
        recent_ensemble_votes,
        recent_artifacts,
        actionable_artifacts,
        latest_promotable_artifact,
        artifact_history_summary,
        artifact_factor_trends,
        artifact_family_trends,
        artifact_decision_summary,
        artifact_review_rules: review_rules,
        artifact_review_rule_sources: review_rule_sources,
        artifact_lineage_summaries,
        artifact_rule_break_effects,
        artifact_factor_rule_break_impacts,
        artifact_family_rule_break_impacts,
        artifact_consumed_impact_summary,
        field_diffs,
        disagreements,
    }
}

pub(crate) fn gate_aware_recommended_next_command(
    stored: &str,
    commands: &CommandRecommendations,
) -> String {
    for command in [&commands.research, &commands.backtest] {
        if command.user_data_selection_required {
            return render_recommended_command(command);
        }
    }
    stored.to_string()
}

pub(crate) fn workflow_phase_snapshot_from_analyze_run(
    run: &AnalyzeRunRecord,
) -> WorkflowPhaseSnapshot {
    let bridge_diff = pre_bayes_entry_quality_bridge_diff(&run.pre_bayes_entry_quality_bridge);
    let mut filtered_assignments = run.pre_bayes_evidence_filter.evidence_assignments.clone();
    filtered_assignments.insert(
        "__policy_version".to_string(),
        run.pre_bayes_evidence_filter.policy.version.clone(),
    );
    if let Some(canonical) = run.canonical_structural_regime_posterior.as_ref() {
        let regime_bundle_applied = run
            .pre_bayes_evidence_filter
            .evidence_assignments
            .get("regime_bundle_bbn_application_status")
            .map(|status| status == "applied")
            .unwrap_or(false);
        if !regime_bundle_applied {
            let regime_label = canonical.active_regime.clone().or_else(|| {
                canonical
                    .probabilities
                    .iter()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(label, _)| label.clone())
            });
            if let Some(regime_label) = regime_label {
                filtered_assignments.insert("market_regime".to_string(), regime_label);
            }
        }
    }
    let mut pre_bayes_soft_evidence = BTreeMap::from([
        (
            "market_regime".to_string(),
            run.pre_bayes_evidence_filter
                .soft_market_regime_distribution
                .clone(),
        ),
        (
            "liquidity_context".to_string(),
            run.pre_bayes_evidence_filter
                .soft_liquidity_context_distribution
                .clone(),
        ),
        (
            "factor_alignment".to_string(),
            run.pre_bayes_evidence_filter
                .soft_factor_alignment_distribution
                .clone(),
        ),
        (
            "factor_uncertainty".to_string(),
            run.pre_bayes_evidence_filter
                .soft_factor_uncertainty_distribution
                .clone(),
        ),
        (
            "multi_timeframe_resonance".to_string(),
            run.pre_bayes_evidence_filter
                .soft_multi_timeframe_resonance_distribution
                .clone(),
        ),
    ]);
    if let Some(canonical) = run.canonical_structural_regime_posterior.as_ref() {
        let regime_bundle_applied = run
            .pre_bayes_evidence_filter
            .evidence_assignments
            .get("regime_bundle_bbn_application_status")
            .map(|status| status == "applied")
            .unwrap_or(false);
        if !regime_bundle_applied && !canonical.probabilities.is_empty() {
            pre_bayes_soft_evidence
                .insert("market_regime".to_string(), canonical.probabilities.clone());
        }
    }
    let regime_bundle_fragment = run
        .pre_bayes_evidence_filter
        .evidence_assignments
        .get("regime_bundle_bbn_application_status")
        .zip(
            run.pre_bayes_evidence_filter
                .evidence_assignments
                .get("regime_bundle_bbn_market_regime"),
        )
        .map(|(status, regime)| format!(" regime_bundle_bbn={status}:{regime}"))
        .unwrap_or_default();
    let duration_fragment = if let (Some(model), Some(remaining)) = (
        run.hybrid_duration_model.as_deref(),
        run.hybrid_remaining_expected_bars,
    ) {
        format!(
            " hybrid_duration_model={} hybrid_remaining_expected_bars={:.3}",
            model, remaining
        )
    } else {
        String::new()
    };
    let mut phase = WorkflowPhaseSnapshot {
        phase: "analyze".to_string(),
        source_command: run.source_command.clone(),
        run_id: run.run_id.clone(),
        timestamp: run.timestamp,
        workflow_phase: run.workflow_state.phase.clone(),
        workflow_reason: run.workflow_state.reason.clone(),
        promotion_status: run.promotion_decision.status.clone(),
        rollback_scope: run.rollback_recommendation.scope.clone(),
        comparable_to_previous: run.dataset_comparability.comparable,
        comparison_class: run.dataset_comparability.comparison_class.clone(),
        recommended_next_command: gate_aware_recommended_next_command(
            &run.recommended_next_command,
            &run.recommended_commands,
        ),
        recommended_next_command_meta: recommended_next_command_meta(
            &gate_aware_recommended_next_command(
                &run.recommended_next_command,
                &run.recommended_commands,
            ),
        ),
        phase_summary: format!(
            "selected_direction={:?} selected_entry_quality={} pre_bayes_status={} pre_bayes_quality={:.3} decision_hint={}{}{} {}",
            run.selected_direction,
            run.selected_entry_quality,
            run.pre_bayes_evidence_filter.gating_status,
            run.pre_bayes_evidence_filter.evidence_quality_score,
            run.decision_hint,
            duration_fragment,
            regime_bundle_fragment,
            multi_timeframe_phase_hint(&run.multi_timeframe_summary)
        ),
        top_actions: workflow_top_actions(&run.agent_action_plan),
        risk_flags: workflow_phase_risk_flags(
            &run.dataset_comparability,
            &run.promotion_decision,
            &run.rollback_recommendation,
        )
        .into_iter()
        .chain(
            run.pre_bayes_evidence_filter
                .conflict_flags
                .iter()
                .map(|flag| format!("pre_bayes:{}", flag)),
        )
        .collect(),
        selected_direction: Some(format!("{:?}", run.selected_direction)),
        selected_entry_quality: Some(run.selected_entry_quality.clone()),
        pre_bayes_gate_status: run.pre_bayes_evidence_filter.gating_status.clone(),
        pre_bayes_uses_soft_evidence: run.pre_bayes_evidence_filter.uses_soft_evidence,
        pre_bayes_policy_version: run.pre_bayes_evidence_filter.policy.version.clone(),
        pre_bayes_evidence_quality_score: run.pre_bayes_evidence_filter.evidence_quality_score,
        pre_bayes_conflict_flags: run.pre_bayes_evidence_filter.conflict_flags.clone(),
        pre_bayes_filtered_assignments: filtered_assignments,
        pre_bayes_soft_evidence,
        market_state_evidence: run.market_state_evidence.clone(),
        canonical_structural_active_regime: run
            .canonical_structural_regime_posterior
            .as_ref()
            .and_then(|posterior| posterior.active_regime.clone()),
        canonical_structural_confidence: run
            .canonical_structural_regime_posterior
            .as_ref()
            .and_then(|posterior| posterior.confidence),
        canonical_structural_probabilities: run
            .canonical_structural_regime_posterior
            .as_ref()
            .map(|posterior| posterior.probabilities.clone())
            .unwrap_or_default(),
        pre_bayes_long_signal_probability: Some(
            run.pre_bayes_entry_quality_bridge.long_signal_probability,
        ),
        pre_bayes_short_signal_probability: Some(
            run.pre_bayes_entry_quality_bridge.short_signal_probability,
        ),
        pre_bayes_selected_entry_quality_probability: run
            .pre_bayes_entry_quality_bridge
            .selected_entry_quality
            .values()
            .copied()
            .fold(None, |acc, value| {
                Some(acc.map(|current| current.max(value)).unwrap_or(value))
            }),
        pre_bayes_bridge_selected_entry_quality: bridge_diff.selected_entry_quality.clone(),
        pre_bayes_bridge_probability_gap: Some(bridge_diff.long_short_signal_probability_gap),
        pre_bayes_bridge_rationale_summary: bridge_diff.rationale_summary,
        pre_bayes_multi_timeframe_direction_bias: run
            .pre_bayes_evidence_filter
            .filtered_multi_timeframe_direction_bias
            .clone(),
        pre_bayes_multi_timeframe_alignment_score: run
            .pre_bayes_evidence_filter
            .filtered_multi_timeframe_alignment_score,
        pre_bayes_multi_timeframe_entry_alignment_score: run
            .pre_bayes_evidence_filter
            .filtered_multi_timeframe_entry_alignment_score,
        pda_cluster_label: run.agent_context_bundle_minimal.pda_cluster_label.clone(),
        hybrid_duration_model: run.hybrid_duration_model.clone(),
        hybrid_remaining_expected_bars: run.hybrid_remaining_expected_bars,
        spectral_entropy: None,
        sparsity_ratio: None,
        segments_gate: None,
        realized_outcome: None,
        structural_learning_credit_class: None,
        structural_learning_success_credit: None,
        structural_learning_observation_weight: None,
        family_states: run
            .factor_family_outcomes
            .iter()
            .map(|item| {
                format!(
                    "{}:{}:{}",
                    item.family, item.promotion_decision.status, item.rollback_recommendation.scope
                )
            })
            .collect(),
        factor_actions: run.agent_context_bundle.top_factor_actions.clone(),
        multi_timeframe_summary: run.multi_timeframe_summary.clone(),
        structural_feedback: None,
        family_score_map: run
            .factor_family_decisions
            .iter()
            .map(|family| (family.family.clone(), family.avg_score))
            .collect(),
        factor_score_map: BTreeMap::new(),
        objective_market_credibility_shrink: None,
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        execution_gate_status: None,
    };
    apply_analyze_run_execution_fields(&mut phase, run);
    phase.phase_summary = format!(
        "{}{}",
        phase.phase_summary,
        execution_phase_summary_suffix(&phase)
    );
    phase
}

pub(crate) fn workflow_phase_snapshot_from_train_run(
    run: &TrainRunRecord,
) -> WorkflowPhaseSnapshot {
    WorkflowPhaseSnapshot {
        phase: "train".to_string(),
        source_command: run.source_command.clone(),
        run_id: run.run_id.clone(),
        timestamp: run.timestamp,
        workflow_phase: run.workflow_state.phase.clone(),
        workflow_reason: run.workflow_state.reason.clone(),
        promotion_status: "promotion_status_unavailable".to_string(),
        rollback_scope: "rollback_scope_unavailable".to_string(),
        comparable_to_previous: run.dataset_comparability.comparable,
        comparison_class: run.dataset_comparability.comparison_class.clone(),
        recommended_next_command: gate_aware_recommended_next_command(
            &run.recommended_next_command,
            &run.recommended_commands,
        ),
        recommended_next_command_meta: recommended_next_command_meta(
            &gate_aware_recommended_next_command(
                &run.recommended_next_command,
                &run.recommended_commands,
            ),
        ),
        phase_summary: format!(
            "final_state={} observations={} epochs={} log_likelihood={:.4} {}",
            run.final_state,
            run.observations,
            run.epochs,
            run.log_likelihood,
            multi_timeframe_phase_hint(&run.multi_timeframe_summary)
        ),
        top_actions: workflow_top_actions(&run.agent_action_plan),
        risk_flags: if run.dataset_comparability.comparable {
            Vec::new()
        } else {
            vec![format!(
                "not_comparable:{}",
                run.dataset_comparability.comparison_class
            )]
        },
        selected_direction: None,
        selected_entry_quality: None,
        pre_bayes_gate_status: "pre_bayes_gate_unavailable".to_string(),
        pre_bayes_uses_soft_evidence: false,
        pre_bayes_policy_version: "policy_version_unavailable".to_string(),
        pre_bayes_evidence_quality_score: 0.0,
        pre_bayes_conflict_flags: Vec::new(),
        pre_bayes_filtered_assignments: BTreeMap::new(),
        pre_bayes_soft_evidence: BTreeMap::new(),
        market_state_evidence: Vec::new(),
        canonical_structural_active_regime: None,
        canonical_structural_confidence: None,
        canonical_structural_probabilities: BTreeMap::new(),
        pre_bayes_long_signal_probability: None,
        pre_bayes_short_signal_probability: None,
        pre_bayes_selected_entry_quality_probability: None,
        pre_bayes_bridge_selected_entry_quality: None,
        pre_bayes_bridge_probability_gap: None,
        pre_bayes_bridge_rationale_summary: Vec::new(),
        pre_bayes_multi_timeframe_direction_bias: "direction_bias_unavailable".to_string(),
        pre_bayes_multi_timeframe_alignment_score: None,
        pre_bayes_multi_timeframe_entry_alignment_score: None,
        pda_cluster_label: run.agent_context_bundle_minimal.pda_cluster_label.clone(),
        hybrid_duration_model: None,
        hybrid_remaining_expected_bars: None,
        spectral_entropy: None,
        sparsity_ratio: None,
        segments_gate: None,
        realized_outcome: None,
        structural_learning_credit_class: None,
        structural_learning_success_credit: None,
        structural_learning_observation_weight: None,
        family_states: Vec::new(),
        factor_actions: Vec::new(),
        multi_timeframe_summary: run.multi_timeframe_summary.clone(),
        structural_feedback: None,
        family_score_map: BTreeMap::new(),
        factor_score_map: BTreeMap::new(),
        objective_market_credibility_shrink: None,
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        execution_gate_status: None,
    }
}

pub(crate) fn workflow_phase_snapshot_from_research_run(
    run: &ResearchRunRecord,
) -> WorkflowPhaseSnapshot {
    let mut phase = WorkflowPhaseSnapshot {
        phase: "research".to_string(),
        source_command: run.source_command.clone(),
        run_id: run.run_id.clone(),
        timestamp: run.timestamp,
        workflow_phase: run.workflow_state.phase.clone(),
        workflow_reason: run.workflow_state.reason.clone(),
        promotion_status: run.promotion_decision.status.clone(),
        rollback_scope: run.rollback_recommendation.scope.clone(),
        comparable_to_previous: run.dataset_comparability.comparable,
        comparison_class: run.dataset_comparability.comparison_class.clone(),
        recommended_next_command: gate_aware_recommended_next_command(
            &run.recommended_next_command,
            &run.recommended_commands,
        ),
        recommended_next_command_meta: recommended_next_command_meta(
            &gate_aware_recommended_next_command(
                &run.recommended_next_command,
                &run.recommended_commands,
            ),
        ),
        phase_summary: format!(
            "objective={} best_factor={:?} aggregate_return={:.4} feedback_applied={} credibility={} {}",
            if run.research_objective.is_empty() {
                "generic"
            } else {
                run.research_objective.as_str()
            },
            run.best_factor,
            run.aggregate_return,
            run.feedback_records_applied,
            run.artifact_action_summary
                .iter()
                .find(|item| item.starts_with("conformal_credibility:"))
                .cloned()
                .unwrap_or_else(|| "conformal_credibility:unavailable".to_string()),
            multi_timeframe_phase_hint(&run.multi_timeframe_summary)
        ),
        top_actions: workflow_top_actions(&run.agent_action_plan),
        risk_flags: workflow_phase_risk_flags(
            &run.dataset_comparability,
            &run.promotion_decision,
            &run.rollback_recommendation,
        ),
        selected_direction: None,
        selected_entry_quality: None,
        pre_bayes_gate_status: "pre_bayes_gate_unavailable".to_string(),
        pre_bayes_uses_soft_evidence: false,
        pre_bayes_policy_version: "policy_version_unavailable".to_string(),
        pre_bayes_evidence_quality_score: 0.0,
        pre_bayes_conflict_flags: Vec::new(),
        pre_bayes_filtered_assignments: BTreeMap::new(),
        pre_bayes_soft_evidence: BTreeMap::new(),
        market_state_evidence: Vec::new(),
        canonical_structural_active_regime: run
            .canonical_structural_regime_posterior
            .as_ref()
            .and_then(|posterior| posterior.active_regime.clone()),
        canonical_structural_confidence: run
            .canonical_structural_regime_posterior
            .as_ref()
            .and_then(|posterior| posterior.confidence),
        canonical_structural_probabilities: run
            .canonical_structural_regime_posterior
            .as_ref()
            .map(|posterior| posterior.probabilities.clone())
            .unwrap_or_default(),
        pre_bayes_long_signal_probability: None,
        pre_bayes_short_signal_probability: None,
        pre_bayes_selected_entry_quality_probability: None,
        pre_bayes_bridge_selected_entry_quality: None,
        pre_bayes_bridge_probability_gap: None,
        pre_bayes_bridge_rationale_summary: Vec::new(),
        pre_bayes_multi_timeframe_direction_bias: "direction_bias_unavailable".to_string(),
        pre_bayes_multi_timeframe_alignment_score: None,
        pre_bayes_multi_timeframe_entry_alignment_score: None,
        hybrid_duration_model: None,
        hybrid_remaining_expected_bars: None,
        spectral_entropy: None,
        sparsity_ratio: None,
        segments_gate: None,
        realized_outcome: None,
        structural_learning_credit_class: None,
        structural_learning_success_credit: None,
        structural_learning_observation_weight: None,
        family_states: run
            .factor_family_outcomes
            .iter()
            .map(|item| {
                format!(
                    "{}:{}:{}",
                    item.family, item.promotion_decision.status, item.rollback_recommendation.scope
                )
            })
            .collect(),
        factor_actions: run.agent_context_bundle.top_factor_actions.clone(),
        multi_timeframe_summary: run.multi_timeframe_summary.clone(),
        structural_feedback: None,
        family_score_map: run
            .factor_family_decisions
            .iter()
            .map(|family| (family.family.clone(), family.avg_score))
            .collect(),
        factor_score_map: run
            .factor_score_deltas
            .iter()
            .map(|item| (item.factor_name.clone(), item.new_score))
            .collect(),
        objective_market_credibility_shrink: None,
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        execution_gate_status: None,
        pda_cluster_label: run.agent_context_bundle_minimal.pda_cluster_label.clone(),
    };
    ict_engine::application::execution::apply_research_run_execution_fields(&mut phase, run);
    phase.phase_summary = format!(
        "{}{}",
        phase.phase_summary,
        execution_phase_summary_suffix(&phase)
    );
    phase
}

pub(crate) fn workflow_phase_snapshot_from_backtest_run(
    run: &BacktestRunRecord,
) -> WorkflowPhaseSnapshot {
    let objective_market_shrink_summary = run
        .objective_market_credibility_shrink
        .as_ref()
        .map(|item| {
            format!(
                " objective_market_shrink={:.3} objective_market_credibility={:.3} objective_market_shrink_triggered={}",
                item.shrink_weight, item.credibility_score, item.shrink_triggered
            )
        })
        .unwrap_or_default();
    let mut phase = WorkflowPhaseSnapshot {
        phase: "backtest".to_string(),
        source_command: run.source_command.clone(),
        run_id: run.run_id.clone(),
        timestamp: run.timestamp,
        workflow_phase: run.workflow_state.phase.clone(),
        workflow_reason: run.workflow_state.reason.clone(),
        promotion_status: run.promotion_decision.status.clone(),
        rollback_scope: run.rollback_recommendation.scope.clone(),
        comparable_to_previous: run.dataset_comparability.comparable,
        comparison_class: run.dataset_comparability.comparison_class.clone(),
        recommended_next_command: gate_aware_recommended_next_command(
            &run.recommended_next_command,
            &run.recommended_commands,
        ),
        recommended_next_command_meta: recommended_next_command_meta(
            &gate_aware_recommended_next_command(
                &run.recommended_next_command,
                &run.recommended_commands,
            ),
        ),
        phase_summary: format!(
            "total_return={:.4} trade_count={} source={} coverage_1sigma={:.3} break_penalty={:.3} structural_break_detected={} structural_break_score={:.3} structural_break_index={:?}{} {}",
            run.total_return,
            run.trade_count,
            run.source_command,
            run.conformal_coverage_1sigma,
            run.regime_break_penalty,
            run.structural_break_detected,
            run.structural_break_score,
            run.structural_break_index,
            objective_market_shrink_summary,
            multi_timeframe_phase_hint(&run.multi_timeframe_summary)
        ),
        top_actions: workflow_top_actions(&run.agent_action_plan),
        risk_flags: workflow_phase_risk_flags(
            &run.dataset_comparability,
            &run.promotion_decision,
            &run.rollback_recommendation,
        ),
        selected_direction: None,
        selected_entry_quality: None,
        pre_bayes_gate_status: "pre_bayes_gate_unavailable".to_string(),
        pre_bayes_uses_soft_evidence: false,
        pre_bayes_policy_version: "policy_version_unavailable".to_string(),
        pre_bayes_evidence_quality_score: 0.0,
        pre_bayes_conflict_flags: Vec::new(),
        pre_bayes_filtered_assignments: BTreeMap::new(),
        pre_bayes_soft_evidence: BTreeMap::new(),
        market_state_evidence: Vec::new(),
        canonical_structural_active_regime: run
            .canonical_structural_regime_posterior
            .as_ref()
            .and_then(|posterior| posterior.active_regime.clone()),
        canonical_structural_confidence: run
            .canonical_structural_regime_posterior
            .as_ref()
            .and_then(|posterior| posterior.confidence),
        canonical_structural_probabilities: run
            .canonical_structural_regime_posterior
            .as_ref()
            .map(|posterior| posterior.probabilities.clone())
            .unwrap_or_default(),
        pre_bayes_long_signal_probability: None,
        pre_bayes_short_signal_probability: None,
        pre_bayes_selected_entry_quality_probability: None,
        pre_bayes_bridge_selected_entry_quality: None,
        pre_bayes_bridge_probability_gap: None,
        pre_bayes_bridge_rationale_summary: Vec::new(),
        pre_bayes_multi_timeframe_direction_bias: "direction_bias_unavailable".to_string(),
        pre_bayes_multi_timeframe_alignment_score: None,
        pre_bayes_multi_timeframe_entry_alignment_score: None,
        hybrid_duration_model: None,
        hybrid_remaining_expected_bars: None,
        spectral_entropy: None,
        sparsity_ratio: None,
        segments_gate: None,
        realized_outcome: None,
        structural_learning_credit_class: None,
        structural_learning_success_credit: None,
        structural_learning_observation_weight: None,
        family_states: run
            .factor_family_outcomes
            .iter()
            .map(|item| {
                format!(
                    "{}:{}:{}",
                    item.family, item.promotion_decision.status, item.rollback_recommendation.scope
                )
            })
            .collect(),
        factor_actions: run.agent_context_bundle.top_factor_actions.clone(),
        multi_timeframe_summary: run.multi_timeframe_summary.clone(),
        structural_feedback: None,
        family_score_map: run
            .factor_family_decisions
            .iter()
            .map(|family| (family.family.clone(), family.avg_score))
            .collect(),
        factor_score_map: run
            .factor_score_deltas
            .iter()
            .map(|item| (item.factor_name.clone(), item.new_score))
            .collect(),
        objective_market_credibility_shrink: run.objective_market_credibility_shrink.clone(),
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        execution_gate_status: None,
        pda_cluster_label: run.agent_context_bundle_minimal.pda_cluster_label.clone(),
    };
    ict_engine::application::execution::apply_backtest_run_execution_fields(&mut phase, run);
    phase.phase_summary = format!(
        "{}{}",
        phase.phase_summary,
        execution_phase_summary_suffix(&phase)
    );
    phase
}

pub(crate) fn workflow_phase_snapshot_from_update_run(
    run: &UpdateRunRecord,
) -> WorkflowPhaseSnapshot {
    let consumed_bridge_diff = run
        .consumed_pre_bayes_entry_quality_bridge
        .as_ref()
        .map(pre_bayes_entry_quality_bridge_diff);
    let mut pre_bayes_filtered_assignments = run
        .consumed_pre_bayes_evidence_filter
        .as_ref()
        .map(|filter| filter.evidence_assignments.clone())
        .unwrap_or_default();
    let mut pre_bayes_soft_evidence = run
        .consumed_pre_bayes_evidence_filter
        .as_ref()
        .map(|filter| {
            BTreeMap::from([
                (
                    "market_regime".to_string(),
                    filter.soft_market_regime_distribution.clone(),
                ),
                (
                    "liquidity_context".to_string(),
                    filter.soft_liquidity_context_distribution.clone(),
                ),
                (
                    "factor_alignment".to_string(),
                    filter.soft_factor_alignment_distribution.clone(),
                ),
                (
                    "factor_uncertainty".to_string(),
                    filter.soft_factor_uncertainty_distribution.clone(),
                ),
                (
                    "multi_timeframe_resonance".to_string(),
                    filter.soft_multi_timeframe_resonance_distribution.clone(),
                ),
            ])
        })
        .unwrap_or_default();
    if let Some(canonical) = run.consumed_canonical_structural_regime_posterior.as_ref() {
        let regime_label = canonical.active_regime.clone().or_else(|| {
            canonical
                .probabilities
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
                .map(|(label, _)| label.clone())
        });
        if let Some(regime_label) = regime_label {
            pre_bayes_filtered_assignments.insert("market_regime".to_string(), regime_label);
        }
        if !canonical.probabilities.is_empty() {
            pre_bayes_soft_evidence
                .insert("market_regime".to_string(), canonical.probabilities.clone());
        }
    }
    let mut phase = WorkflowPhaseSnapshot {
        phase: "update".to_string(),
        source_command: run.source_command.clone(),
        run_id: run.run_id.clone(),
        timestamp: run.timestamp,
        workflow_phase: run.workflow_state.phase.clone(),
        workflow_reason: run.workflow_state.reason.clone(),
        promotion_status: run.promotion_decision.status.clone(),
        rollback_scope: run.rollback_recommendation.scope.clone(),
        comparable_to_previous: run.dataset_comparability.comparable,
        comparison_class: run.dataset_comparability.comparison_class.clone(),
        recommended_next_command: gate_aware_recommended_next_command(
            &run.recommended_next_command,
            &run.recommended_commands,
        ),
        recommended_next_command_meta: recommended_next_command_meta(
            &gate_aware_recommended_next_command(
                &run.recommended_next_command,
                &run.recommended_commands,
            ),
        ),
        phase_summary: format!(
            "realized_outcome={} learning_credit_class={} success_credit={:.3} observation_weight={:.3} feedback_applied={} duplicate_feedback_skipped={} consumed_pre_bayes_gate_status={} {}",
            run.realized_outcome,
            run.structural_learning_credit_class
                .as_deref()
                .unwrap_or("unavailable"),
            run.structural_learning_success_credit.unwrap_or_default(),
            run.structural_learning_observation_weight.unwrap_or_default(),
            run.feedback_records_applied,
            run.duplicate_feedback_skipped,
            run.consumed_pre_bayes_evidence_filter
                .as_ref()
                .map(|filter| filter.gating_status.clone())
                .unwrap_or_default(),
            multi_timeframe_phase_hint(&run.consumed_multi_timeframe_summary)
        ),
        top_actions: workflow_top_actions(&run.agent_action_plan),
        risk_flags: workflow_phase_risk_flags(
            &run.dataset_comparability,
            &run.promotion_decision,
            &run.rollback_recommendation,
        ),
        selected_direction: None,
        selected_entry_quality: Some(run.normalized_entry_quality.clone()),
        pre_bayes_gate_status: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.gating_status.clone())
            .unwrap_or_default(),
        pre_bayes_uses_soft_evidence: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.uses_soft_evidence)
            .unwrap_or(false),
        pre_bayes_policy_version: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.policy.version.clone())
            .unwrap_or_default(),
        pre_bayes_evidence_quality_score: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.evidence_quality_score)
            .unwrap_or_default(),
        pre_bayes_conflict_flags: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.conflict_flags.clone())
            .unwrap_or_default(),
        pre_bayes_filtered_assignments,
        pre_bayes_soft_evidence,
        market_state_evidence: Vec::new(),
        canonical_structural_active_regime: run
            .consumed_canonical_structural_regime_posterior
            .as_ref()
            .and_then(|posterior| posterior.active_regime.clone()),
        canonical_structural_confidence: run
            .consumed_canonical_structural_regime_posterior
            .as_ref()
            .and_then(|posterior| posterior.confidence),
        canonical_structural_probabilities: run
            .consumed_canonical_structural_regime_posterior
            .as_ref()
            .map(|posterior| posterior.probabilities.clone())
            .unwrap_or_default(),
        pre_bayes_long_signal_probability: None,
        pre_bayes_short_signal_probability: None,
        pre_bayes_selected_entry_quality_probability: None,
        pre_bayes_bridge_selected_entry_quality: consumed_bridge_diff
            .as_ref()
            .and_then(|bridge| bridge.selected_entry_quality.clone()),
        pre_bayes_bridge_probability_gap: consumed_bridge_diff
            .as_ref()
            .map(|bridge| bridge.long_short_signal_probability_gap),
        pre_bayes_bridge_rationale_summary: consumed_bridge_diff
            .as_ref()
            .map(|bridge| bridge.rationale_summary.clone())
            .unwrap_or_default(),
        pre_bayes_multi_timeframe_direction_bias: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .map(|filter| filter.filtered_multi_timeframe_direction_bias.clone())
            .unwrap_or_else(|| "direction_bias_unavailable".to_string()),
        pre_bayes_multi_timeframe_alignment_score: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .and_then(|filter| filter.filtered_multi_timeframe_alignment_score),
        pre_bayes_multi_timeframe_entry_alignment_score: run
            .consumed_pre_bayes_evidence_filter
            .as_ref()
            .and_then(|filter| filter.filtered_multi_timeframe_entry_alignment_score),
        hybrid_duration_model: None,
        hybrid_remaining_expected_bars: None,
        spectral_entropy: None,
        sparsity_ratio: None,
        segments_gate: None,
        realized_outcome: Some(run.realized_outcome.clone()),
        structural_learning_credit_class: run.structural_learning_credit_class.clone(),
        structural_learning_success_credit: run.structural_learning_success_credit,
        structural_learning_observation_weight: run.structural_learning_observation_weight,
        family_states: Vec::new(),
        factor_actions: Vec::new(),
        multi_timeframe_summary: run.consumed_multi_timeframe_summary.clone(),
        structural_feedback: run.structural_feedback.clone(),
        family_score_map: BTreeMap::new(),
        factor_score_map: BTreeMap::new(),
        objective_market_credibility_shrink: None,
        execution_edge_share: None,
        prediction_edge_share: None,
        execution_readiness: None,
        execution_gate_status: None,
        pda_cluster_label: run.agent_context_bundle_minimal.pda_cluster_label.clone(),
    };
    ict_engine::application::execution::apply_update_run_execution_fields(&mut phase, run);
    phase.phase_summary = format!(
        "{}{}",
        phase.phase_summary,
        execution_phase_summary_suffix(&phase)
    );
    phase
}

pub(crate) fn synthetic_phase_ensemble_vote(
    input: SyntheticPhaseEnsembleVoteInput<'_>,
) -> Option<EnsembleVoteRecord> {
    let SyntheticPhaseEnsembleVoteInput {
        symbol,
        timestamp,
        source_phase,
        run_id,
        provenance,
        dataset_comparability,
        recommended_next_command,
        canonical,
    } = input;
    let active_regime = canonical.active_regime.clone().or_else(|| {
        canonical
            .probabilities
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(label, _)| label.clone())
    })?;
    let confidence = canonical.confidence.unwrap_or_else(|| {
        canonical
            .probabilities
            .get(&active_regime)
            .copied()
            .unwrap_or(0.5)
    });

    Some(EnsembleVoteRecord {
        artifact_id: format!("ensemble-vote:synthetic:{}", run_id),
        generated_at: timestamp,
        symbol: symbol.to_string(),
        source_phase: source_phase.to_string(),
        source_run_id: Some(run_id.to_string()),
        provenance: provenance.clone(),
        dataset_comparability: dataset_comparability.clone(),
        ensemble_version: "ensemble-audit-v2-synthetic-phase".to_string(),
        final_action: "observe".to_string(),
        recommended_command: recommended_next_command.to_string(),
        human_next_triage: "synthetic_from_phase_canonical_structural_posterior".to_string(),
        hard_block: ict_engine::application::orchestration::EnsembleHardBlockArtifact::default(),
        confidence,
        consensus_strength: confidence,
        disagreement_flags: Vec::new(),
        executor_summaries: vec!["synthetic_from_phase_canonical_structural_posterior".to_string()],
        split_explanations: vec![format!("active_regime={active_regime}")],
        executor_scorecards: Vec::new(),
        executor_scorecards_source: None,
        posterior_fingerprint: format!("synthetic:{}", run_id),
        posterior_normalization_status: "canonical_structural_regime_posterior".to_string(),
        posterior_active_regime: active_regime,
        posterior_confidence: Some(confidence),
        posterior_probabilities: canonical.probabilities.clone(),
        posterior_evidence: canonical.evidence.clone(),
    })
}

pub(crate) fn overlay_analyze_canonical_regime_on_ensemble_vote(
    vote: &mut EnsembleVoteRecord,
    canonical: &ict_engine::state::CanonicalStructuralRegimePosterior,
) {
    let active_regime = canonical.active_regime.clone().or_else(|| {
        canonical
            .probabilities
            .iter()
            .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(label, _)| label.clone())
    });
    if let Some(active_regime) = active_regime {
        vote.posterior_active_regime = active_regime;
    }
    vote.posterior_probabilities = canonical.probabilities.clone();
    vote.posterior_confidence = canonical.confidence;
    vote.confidence = canonical
        .confidence
        .unwrap_or_else(|| vote.posterior_confidence.unwrap_or(vote.confidence));
    vote.consensus_strength = vote.confidence;
    vote.posterior_normalization_status = "canonical_structural_regime_posterior".to_string();
    vote.posterior_evidence = canonical.evidence.clone();
}

pub(crate) fn compact_canonical_structural_regime_summary(
    posterior: Option<&ict_engine::state::CanonicalStructuralRegimePosterior>,
) -> String {
    let Some(posterior) = posterior else {
        return "unavailable".to_string();
    };
    let active = posterior.active_regime.as_deref().unwrap_or("unavailable");
    let confidence = posterior.confidence.unwrap_or_default();
    let probabilities = if posterior.probabilities.is_empty() {
        "unavailable".to_string()
    } else {
        posterior
            .probabilities
            .iter()
            .map(|(label, probability)| format!("{label}={probability:.3}"))
            .collect::<Vec<_>>()
            .join(",")
    };
    format!(
        "active={} confidence={:.3} probs={} evidence_count={}",
        active,
        confidence,
        probabilities,
        posterior.evidence.len()
    )
}

pub(crate) fn workflow_top_actions(plan: &AgentActionPlan) -> Vec<String> {
    plan.items
        .iter()
        .take(3)
        .map(|item| format!("{}:{}", item.stage, item.title))
        .collect()
}

pub(crate) fn workflow_blocking_truth(
    symbol: &str,
    state_dir: &str,
    current_phase: Option<&WorkflowPhaseSnapshot>,
    pre_bayes_filter: Option<&AnalyzeRunRecord>,
    artifact_decision_summary: &ict_engine::state::ArtifactDecisionSummary,
) -> WorkflowBlockingTruth {
    let current_recommended_command = current_phase
        .map(|phase| phase.recommended_next_command.clone())
        .unwrap_or_default();
    if current_recommended_command.contains("user_selected_historical_data") {
        return WorkflowBlockingTruth {
            stage: current_phase
                .map(|phase| phase.phase.clone())
                .unwrap_or_else(|| "data_selection".to_string()),
            status: "blocked".to_string(),
            reason: "user_selected_historical_data_missing".to_string(),
            evidence: vec![
                "historical data reuse requires explicit user path selection".to_string(),
                current_recommended_command.clone(),
            ],
            next_command: current_recommended_command,
        };
    }
    if let Some(analyze) = pre_bayes_filter {
        let gate_status = analyze.pre_bayes_evidence_filter.gating_status.clone();
        let bridge_diff =
            pre_bayes_entry_quality_bridge_diff(&analyze.pre_bayes_entry_quality_bridge);
        let bridge_gap = bridge_diff.long_short_signal_probability_gap;
        let hard_pass = pre_bayes_gate_is_hard_pass(&gate_status);
        let bridge_gap_clear_threshold = env_f64("ICT_ENGINE_BRIDGE_GAP_CLEAR_THRESHOLD", 0.12);
        if !hard_pass || bridge_gap < bridge_gap_clear_threshold {
            let mut evidence = vec![
                format!("pre_bayes_gate_status={gate_status}"),
                format!("bridge_probability_gap={bridge_gap:.3}"),
                format!(
                    "selected_entry_quality={}",
                    bridge_diff
                        .selected_entry_quality
                        .unwrap_or_else(|| "entry_quality_unavailable".to_string())
                ),
            ];
            evidence.extend(
                analyze
                    .pre_bayes_evidence_filter
                    .rationale
                    .iter()
                    .take(3)
                    .cloned(),
            );
            return WorkflowBlockingTruth {
                stage: "analyze".to_string(),
                status: if hard_pass {
                    "bridge_needs_confirmation".to_string()
                } else {
                    gate_status.clone()
                },
                reason: if hard_pass {
                    format!(
                        "pre_bayes passed but bridge gap {:.3} is below confirmation threshold",
                        bridge_gap
                    )
                } else {
                    analyze
                        .pre_bayes_evidence_filter
                        .rationale
                        .first()
                        .cloned()
                        .unwrap_or_else(|| {
                            "pre-bayes gate still blocks downstream chain".to_string()
                        })
                },
                evidence,
                next_command: if current_recommended_command.is_empty() {
                    format!(
                        "ict-engine pre-bayes-status --symbol {} --state-dir {}",
                        shell_quote(symbol),
                        shell_quote(state_dir)
                    )
                } else {
                    current_recommended_command
                },
            };
        }
    }
    if artifact_decision_summary.consumed_trend_status == "validated_regressing" {
        return WorkflowBlockingTruth {
            stage: "artifact_consumption".to_string(),
            status: artifact_decision_summary.consumed_trend_status.clone(),
            reason: artifact_decision_summary.consumed_trend_reason.clone(),
            evidence: artifact_decision_summary.consumed_target_kinds.clone(),
            next_command: format!(
                "ict-engine workflow-status --symbol {} --state-dir {} --phase artifact-consumed-gate",
                shell_quote(symbol),
                shell_quote(state_dir)
            ),
        };
    }
    if let Some(phase) = current_phase {
        if let Some(credibility_block) = phase.risk_flags.iter().find(|flag| {
            flag.contains("conformal_coverage_low")
                || flag.contains("regime_break_penalty_high")
                || flag.contains("structural_break_detected")
        }) {
            return WorkflowBlockingTruth {
                stage: phase.phase.clone(),
                status: "credibility_gate_blocked".to_string(),
                reason: format!(
                    "workflow credibility gate blocked next step because {}",
                    credibility_block
                ),
                evidence: phase.risk_flags.clone(),
                next_command: format!(
                    "ict-engine workflow-status --symbol {} --state-dir {} --phase human-next",
                    shell_quote(symbol),
                    shell_quote(state_dir)
                ),
            };
        }
    }
    if let Some(phase) = current_phase {
        return WorkflowBlockingTruth {
            stage: phase.phase.clone(),
            status: "follow_current_focus".to_string(),
            reason: phase.workflow_reason.clone(),
            evidence: phase.top_actions.clone(),
            next_command: phase.recommended_next_command.clone(),
        };
    }
    WorkflowBlockingTruth {
        stage: "stage_unavailable".to_string(),
        status: "insufficient_state".to_string(),
        reason: "no workflow phase snapshots available".to_string(),
        evidence: Vec::new(),
        next_command: "next_command_unavailable".to_string(),
    }
}

pub(crate) fn workflow_phase_risk_flags(
    comparability: &DatasetComparability,
    promotion: &PromotionDecision,
    rollback: &RollbackRecommendation,
) -> Vec<String> {
    let mut flags = Vec::new();
    if !comparability.comparable {
        flags.push(format!("not_comparable:{}", comparability.comparison_class));
    }
    if rollback.should_rollback {
        flags.push(format!("rollback:{}", rollback.reason));
    }
    if !promotion.approved && !promotion.status.is_empty() && promotion.status != "observe" {
        flags.push(format!("promotion_blocked:{}", promotion.reason));
    }
    flags
}

pub(crate) fn workflow_field_diffs(
    analyze: &Option<WorkflowPhaseSnapshot>,
    research: &Option<WorkflowPhaseSnapshot>,
    backtest: &Option<WorkflowPhaseSnapshot>,
    update: &Option<WorkflowPhaseSnapshot>,
) -> Vec<WorkflowFieldDiff> {
    let mut diffs = Vec::new();
    for (left, right) in [
        (research.as_ref(), backtest.as_ref()),
        (analyze.as_ref(), update.as_ref()),
        (research.as_ref(), update.as_ref()),
    ] {
        if let (Some(left), Some(right)) = (left, right) {
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "promotion_status",
                &left.promotion_status,
                &right.promotion_status,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "rollback_scope",
                &left.rollback_scope,
                &right.rollback_scope,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "workflow_phase",
                &left.workflow_phase,
                &right.workflow_phase,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "comparison_class",
                &left.comparison_class,
                &right.comparison_class,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_gate_status",
                &left.pre_bayes_gate_status,
                &right.pre_bayes_gate_status,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_policy_version",
                &left.pre_bayes_policy_version,
                &right.pre_bayes_policy_version,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_uses_soft_evidence",
                if left.pre_bayes_uses_soft_evidence {
                    "true"
                } else {
                    "false"
                },
                if right.pre_bayes_uses_soft_evidence {
                    "true"
                } else {
                    "false"
                },
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_soft_market_regime",
                &workflow_market_regime_diff_value(left),
                &workflow_market_regime_diff_value(right),
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_bridge_selected_entry_quality",
                &left
                    .pre_bayes_bridge_selected_entry_quality
                    .clone()
                    .unwrap_or_default(),
                &right
                    .pre_bayes_bridge_selected_entry_quality
                    .clone()
                    .unwrap_or_default(),
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_bridge_probability_gap",
                &left
                    .pre_bayes_bridge_probability_gap
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
                &right
                    .pre_bayes_bridge_probability_gap
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_multi_timeframe_direction_bias",
                &left.pre_bayes_multi_timeframe_direction_bias,
                &right.pre_bayes_multi_timeframe_direction_bias,
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_multi_timeframe_alignment_score",
                &left
                    .pre_bayes_multi_timeframe_alignment_score
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
                &right
                    .pre_bayes_multi_timeframe_alignment_score
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
            );
            push_workflow_field_diff(
                &mut diffs,
                left,
                right,
                "pre_bayes_multi_timeframe_entry_alignment_score",
                &left
                    .pre_bayes_multi_timeframe_entry_alignment_score
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
                &right
                    .pre_bayes_multi_timeframe_entry_alignment_score
                    .map(|value| format!("{value:.4}"))
                    .unwrap_or_default(),
            );
        }
    }
    diffs
}

fn workflow_market_regime_diff_value(snapshot: &WorkflowPhaseSnapshot) -> String {
    if !snapshot.canonical_structural_probabilities.is_empty() {
        format!("{:?}", snapshot.canonical_structural_probabilities)
    } else {
        format!(
            "{:?}",
            snapshot.pre_bayes_soft_evidence.get("market_regime")
        )
    }
}

fn push_workflow_field_diff(
    diffs: &mut Vec<WorkflowFieldDiff>,
    left: &WorkflowPhaseSnapshot,
    right: &WorkflowPhaseSnapshot,
    field: &str,
    left_value: &str,
    right_value: &str,
) {
    if left_value != right_value {
        diffs.push(WorkflowFieldDiff {
            left_phase: left.phase.clone(),
            right_phase: right.phase.clone(),
            field: field.to_string(),
            left_value: left_value.to_string(),
            right_value: right_value.to_string(),
            severity: if field == "promotion_status" || field == "rollback_scope" {
                "high".to_string()
            } else {
                "medium".to_string()
            },
        });
    }
}

pub(crate) fn workflow_disagreements(
    analyze: &Option<WorkflowPhaseSnapshot>,
    research: &Option<WorkflowPhaseSnapshot>,
    backtest: &Option<WorkflowPhaseSnapshot>,
    update: &Option<WorkflowPhaseSnapshot>,
) -> Vec<WorkflowDisagreement> {
    let mut disagreements = Vec::new();

    if let (Some(analyze), Some(update)) = (analyze, update) {
        if analyze
            .selected_direction
            .as_deref()
            .map(|direction| direction == "Bull" || direction == "Bear")
            .unwrap_or(false)
            && update.rollback_scope != "none"
        {
            disagreements.push(WorkflowDisagreement {
                id: "analyze_direction_vs_update_rollback".to_string(),
                severity: "high".to_string(),
                summary: "analyze directional bias conflicts with the latest update rollback state"
                    .to_string(),
                phases: vec![analyze.phase.clone(), update.phase.clone()],
                recommended_action: "review realized feedback against the current directional evidence before trusting deployment decisions".to_string(),
                evidence: vec![
                    format!(
                        "analyze.selected_direction={}",
                        analyze.selected_direction.clone().unwrap_or_default()
                    ),
                    format!("update.rollback_scope={}", update.rollback_scope),
                    format!(
                        "update.realized_outcome={}",
                        update.realized_outcome.clone().unwrap_or_default()
                    ),
                ],
                sources: Vec::new(),
            });
        }
    }

    if let (Some(research), Some(backtest)) = (research, backtest) {
        if research.promotion_status != backtest.promotion_status {
            disagreements.push(WorkflowDisagreement {
                id: "research_vs_backtest_promotion_status".to_string(),
                severity: "high".to_string(),
                summary: "research and backtest disagree on promotion status".to_string(),
                phases: vec![research.phase.clone(), backtest.phase.clone()],
                recommended_action:
                    "compare score deltas with backtest returns before promoting factor changes"
                        .to_string(),
                evidence: vec![
                    format!("research.promotion_status={}", research.promotion_status),
                    format!("backtest.promotion_status={}", backtest.promotion_status),
                ],
                sources: family_conflict_sources(research, backtest)
                    .into_iter()
                    .chain(factor_conflict_sources(research, backtest))
                    .collect(),
            });
        }
    }

    if let Some(analyze) = analyze {
        for downstream in [research.as_ref(), backtest.as_ref(), update.as_ref()]
            .into_iter()
            .flatten()
        {
            if analyze.pre_bayes_gate_status == "observe_only"
                && downstream.promotion_status == "promote"
            {
                let soft_divergences = pre_bayes_soft_divergence_evidence(analyze);
                disagreements.push(WorkflowDisagreement {
                    id: format!("analyze_pre_bayes_observe_only_vs_{}_promote", downstream.phase),
                    severity: "high".to_string(),
                    summary:
                        "analyze pre-bayes gate is observe-only but a downstream phase still promotes"
                            .to_string(),
                    phases: vec![analyze.phase.clone(), downstream.phase.clone()],
                    recommended_action:
                        "resolve pre-bayes evidence quality before trusting downstream promotion"
                            .to_string(),
                    evidence: vec![
                        format!(
                            "analyze.pre_bayes_gate_status={}",
                            analyze.pre_bayes_gate_status
                        ),
                        format!(
                            "analyze.pre_bayes_quality={:.3}",
                            analyze.pre_bayes_evidence_quality_score
                        ),
                        format!(
                            "analyze.pre_bayes_policy_version={}",
                            analyze.pre_bayes_policy_version
                        ),
                        format!(
                            "analyze.pre_bayes_uses_soft_evidence={}",
                            analyze.pre_bayes_uses_soft_evidence
                        ),
                        format!(
                            "analyze.pre_bayes_long_signal_probability={:.3}",
                            analyze.pre_bayes_long_signal_probability.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_short_signal_probability={:.3}",
                            analyze.pre_bayes_short_signal_probability.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_selected_entry_quality_probability={:.3}",
                            analyze
                                .pre_bayes_selected_entry_quality_probability
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_bridge_selected_entry_quality={}",
                            analyze
                                .pre_bayes_bridge_selected_entry_quality
                                .clone()
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_bridge_probability_gap={:.3}",
                            analyze.pre_bayes_bridge_probability_gap.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_direction_bias={}",
                            analyze.pre_bayes_multi_timeframe_direction_bias
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_alignment_score={:.3}",
                            analyze
                                .pre_bayes_multi_timeframe_alignment_score
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_entry_alignment_score={:.3}",
                            analyze
                                .pre_bayes_multi_timeframe_entry_alignment_score
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_soft_divergences={}",
                            if soft_divergences.is_empty() {
                                "none".to_string()
                            } else {
                                soft_divergences.join("|")
                            }
                        ),
                        format!(
                            "{}.promotion_status={}",
                            downstream.phase, downstream.promotion_status
                        ),
                    ],
                    sources: vec![WorkflowConflictSource {
                        scope: "pre_bayes_bridge".to_string(),
                        subject: "policy_version_and_selected_entry_quality".to_string(),
                        left_phase: analyze.phase.clone(),
                        left_value: format!(
                            "{}:{}",
                            analyze.pre_bayes_policy_version,
                            analyze
                                .pre_bayes_bridge_selected_entry_quality
                                .clone()
                                .unwrap_or_default()
                        ),
                        right_phase: downstream.phase.clone(),
                        right_value: downstream.promotion_status.clone(),
                        evidence: vec![
                            "observe_only gate conflicts with downstream promote".to_string(),
                            format!(
                                "uses_soft_evidence={}",
                                analyze.pre_bayes_uses_soft_evidence
                            ),
                            format!(
                                "long_short_signal_probability_gap={:.3}",
                                analyze.pre_bayes_bridge_probability_gap.unwrap_or_default()
                            ),
                            format!(
                                "multi_timeframe_direction_bias={}",
                                analyze.pre_bayes_multi_timeframe_direction_bias
                            ),
                            format!(
                                "soft_divergences={}",
                                if soft_divergences.is_empty() {
                                    "none".to_string()
                                } else {
                                    soft_divergences.join("|")
                                }
                            ),
                        ],
                    }],
                });
            }
            if analyze.pre_bayes_gate_status == "pass_neutralized"
                && downstream.promotion_status == "promote"
            {
                let soft_divergences = pre_bayes_soft_divergence_evidence(analyze);
                disagreements.push(WorkflowDisagreement {
                    id: format!(
                        "analyze_pre_bayes_neutralized_vs_{}_promote",
                        downstream.phase
                    ),
                    severity: "medium".to_string(),
                    summary:
                        "analyze pre-bayes gate is neutralized while a downstream phase still promotes"
                            .to_string(),
                    phases: vec![analyze.phase.clone(), downstream.phase.clone()],
                    recommended_action:
                        "review whether neutralized evidence is strong enough to justify promotion"
                            .to_string(),
                    evidence: vec![
                        format!(
                            "analyze.pre_bayes_gate_status={}",
                            analyze.pre_bayes_gate_status
                        ),
                        format!(
                            "analyze.pre_bayes_quality={:.3}",
                            analyze.pre_bayes_evidence_quality_score
                        ),
                        format!(
                            "analyze.pre_bayes_policy_version={}",
                            analyze.pre_bayes_policy_version
                        ),
                        format!(
                            "analyze.pre_bayes_uses_soft_evidence={}",
                            analyze.pre_bayes_uses_soft_evidence
                        ),
                        format!(
                            "analyze.pre_bayes_long_signal_probability={:.3}",
                            analyze.pre_bayes_long_signal_probability.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_short_signal_probability={:.3}",
                            analyze.pre_bayes_short_signal_probability.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_selected_entry_quality_probability={:.3}",
                            analyze
                                .pre_bayes_selected_entry_quality_probability
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_bridge_selected_entry_quality={}",
                            analyze
                                .pre_bayes_bridge_selected_entry_quality
                                .clone()
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_bridge_probability_gap={:.3}",
                            analyze.pre_bayes_bridge_probability_gap.unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_direction_bias={}",
                            analyze.pre_bayes_multi_timeframe_direction_bias
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_alignment_score={:.3}",
                            analyze
                                .pre_bayes_multi_timeframe_alignment_score
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_multi_timeframe_entry_alignment_score={:.3}",
                            analyze
                                .pre_bayes_multi_timeframe_entry_alignment_score
                                .unwrap_or_default()
                        ),
                        format!(
                            "analyze.pre_bayes_soft_divergences={}",
                            if soft_divergences.is_empty() {
                                "none".to_string()
                            } else {
                                soft_divergences.join("|")
                            }
                        ),
                        format!(
                            "{}.promotion_status={}",
                            downstream.phase, downstream.promotion_status
                        ),
                    ],
                    sources: vec![WorkflowConflictSource {
                        scope: "pre_bayes_bridge".to_string(),
                        subject: "policy_version_and_selected_entry_quality".to_string(),
                        left_phase: analyze.phase.clone(),
                        left_value: format!(
                            "{}:{}",
                            analyze.pre_bayes_policy_version,
                            analyze
                                .pre_bayes_bridge_selected_entry_quality
                                .clone()
                                .unwrap_or_default()
                        ),
                        right_phase: downstream.phase.clone(),
                        right_value: downstream.promotion_status.clone(),
                        evidence: vec![
                            "neutralized gate conflicts with downstream promote".to_string(),
                            format!(
                                "long_short_signal_probability_gap={:.3}",
                                analyze.pre_bayes_bridge_probability_gap.unwrap_or_default()
                            ),
                            format!(
                                "multi_timeframe_direction_bias={}",
                                analyze.pre_bayes_multi_timeframe_direction_bias
                            ),
                            format!(
                                "soft_divergences={}",
                                if soft_divergences.is_empty() {
                                    "none".to_string()
                                } else {
                                    soft_divergences.join("|")
                                }
                            ),
                        ],
                    }],
                });
            }
        }
    }

    for (left, right) in [
        (research.as_ref(), update.as_ref()),
        (backtest.as_ref(), update.as_ref()),
        (research.as_ref(), backtest.as_ref()),
    ] {
        if let (Some(left), Some(right)) = (left, right) {
            let score_promotes = left.promotion_status == "promote"
                && right.workflow_phase == "artifact_rollback_review";
            let reverse_score_promotes = right.promotion_status == "promote"
                && left.workflow_phase == "artifact_rollback_review";
            if score_promotes || reverse_score_promotes {
                let (promote_phase, artifact_phase) = if score_promotes {
                    (left, right)
                } else {
                    (right, left)
                };
                disagreements.push(WorkflowDisagreement {
                    id: format!(
                        "{}_vs_{}_artifact_consumption_gate",
                        promote_phase.phase, artifact_phase.phase
                    ),
                    severity: "high".to_string(),
                    summary:
                        "score-based promotion conflicts with an artifact consumption rollback gate"
                            .to_string(),
                    phases: vec![promote_phase.phase.clone(), artifact_phase.phase.clone()],
                    recommended_action:
                        "resolve artifact consumption regression before trusting score-based promotion"
                            .to_string(),
                    evidence: vec![
                        format!(
                            "{}.promotion_status={}",
                            promote_phase.phase, promote_phase.promotion_status
                        ),
                        format!(
                            "{}.workflow_phase={}",
                            artifact_phase.phase, artifact_phase.workflow_phase
                        ),
                        format!(
                            "{}.rollback_scope={}",
                            artifact_phase.phase, artifact_phase.rollback_scope
                        ),
                    ],
                    sources: family_conflict_sources(promote_phase, artifact_phase)
                        .into_iter()
                        .chain(factor_conflict_sources(promote_phase, artifact_phase))
                        .collect(),
                });
            }
        }
    }

    if let (Some(backtest), Some(update)) = (backtest, update) {
        if backtest.rollback_scope == "none" && update.rollback_scope != "none" {
            disagreements.push(WorkflowDisagreement {
                id: "backtest_stable_vs_update_rollback".to_string(),
                severity: "medium".to_string(),
                summary: "backtest stayed stable but the latest realized update recommends rollback".to_string(),
                phases: vec![backtest.phase.clone(), update.phase.clone()],
                recommended_action: "inspect live execution drift and feedback provenance before keeping or rolling back changes".to_string(),
                evidence: vec![
                    format!("backtest.rollback_scope={}", backtest.rollback_scope),
                    format!("update.rollback_scope={}", update.rollback_scope),
                ],
                sources: family_conflict_sources(backtest, update)
                    .into_iter()
                    .chain(factor_conflict_sources(backtest, update))
                    .collect(),
            });
        }
    }

    if let (Some(research), Some(backtest)) = (research, backtest) {
        let sources = family_conflict_sources(research, backtest);
        if !sources.is_empty() {
            disagreements.push(WorkflowDisagreement {
                id: "research_backtest_family_conflicts".to_string(),
                severity: "medium".to_string(),
                summary: "research and backtest disagree on family-level decisions".to_string(),
                phases: vec![research.phase.clone(), backtest.phase.clone()],
                recommended_action: "inspect family score deltas and rollback scopes before acting on a single phase".to_string(),
                evidence: sources
                    .iter()
                    .map(|source| {
                        format!(
                            "family:{} {}={} {}={}",
                            source.subject,
                            source.left_phase,
                            source.left_value,
                            source.right_phase,
                            source.right_value
                        )
                    })
                    .collect(),
                sources,
            });
        }
        let sources = factor_conflict_sources(research, backtest);
        if !sources.is_empty() {
            disagreements.push(WorkflowDisagreement {
                id: "research_backtest_factor_conflicts".to_string(),
                severity: "medium".to_string(),
                summary: "research and backtest disagree on factor-level actions".to_string(),
                phases: vec![research.phase.clone(), backtest.phase.clone()],
                recommended_action: "check factor scorecards and iteration queue ordering before selecting the next factor edit".to_string(),
                evidence: sources
                    .iter()
                    .map(|source| {
                        format!(
                            "factor:{} {}={} {}={}",
                            source.subject,
                            source.left_phase,
                            source.left_value,
                            source.right_phase,
                            source.right_value
                        )
                    })
                    .collect(),
                sources,
            });
        }
    }

    disagreements
}

fn pre_bayes_soft_divergence_evidence(snapshot: &WorkflowPhaseSnapshot) -> Vec<String> {
    snapshot
        .pre_bayes_soft_evidence
        .iter()
        .filter_map(|(node, distribution)| {
            let dominant = distribution
                .iter()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal))?;
            let filtered = snapshot.pre_bayes_filtered_assignments.get(node)?;
            (dominant.0 != filtered).then(|| {
                format!(
                    "{}:{}->{:.3}:filtered={}",
                    node, dominant.0, dominant.1, filtered
                )
            })
        })
        .collect()
}

pub(crate) fn family_conflict_sources(
    left: &WorkflowPhaseSnapshot,
    right: &WorkflowPhaseSnapshot,
) -> Vec<WorkflowConflictSource> {
    let left_map = left
        .family_states
        .iter()
        .filter_map(|item| {
            let mut parts = item.splitn(3, ':');
            Some((
                parts.next()?.to_string(),
                format!("{}:{}", parts.next()?, parts.next()?),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    let right_map = right
        .family_states
        .iter()
        .filter_map(|item| {
            let mut parts = item.splitn(3, ':');
            Some((
                parts.next()?.to_string(),
                format!("{}:{}", parts.next()?, parts.next()?),
            ))
        })
        .collect::<BTreeMap<_, _>>();
    left_map
        .iter()
        .filter_map(|(family, left_value)| {
            let right_value = right_map.get(family)?;
            (left_value != right_value).then(|| WorkflowConflictSource {
                scope: "family".to_string(),
                subject: family.clone(),
                left_phase: left.phase.clone(),
                left_value: left_value.clone(),
                right_phase: right.phase.clone(),
                right_value: right_value.clone(),
                evidence: workflow_numeric_family_evidence(left, right, family),
            })
        })
        .collect()
}

pub(crate) fn factor_conflict_sources(
    left: &WorkflowPhaseSnapshot,
    right: &WorkflowPhaseSnapshot,
) -> Vec<WorkflowConflictSource> {
    let left_map = left
        .factor_actions
        .iter()
        .filter_map(|item| {
            let mut parts = item.splitn(3, ':');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect::<BTreeMap<_, _>>();
    let right_map = right
        .factor_actions
        .iter()
        .filter_map(|item| {
            let mut parts = item.splitn(3, ':');
            Some((parts.next()?.to_string(), parts.next()?.to_string()))
        })
        .collect::<BTreeMap<_, _>>();
    left_map
        .iter()
        .filter_map(|(factor, left_value)| {
            let right_value = right_map.get(factor)?;
            (left_value != right_value).then(|| WorkflowConflictSource {
                scope: "factor".to_string(),
                subject: factor.clone(),
                left_phase: left.phase.clone(),
                left_value: left_value.clone(),
                right_phase: right.phase.clone(),
                right_value: right_value.clone(),
                evidence: workflow_numeric_factor_evidence(left, right, factor),
            })
        })
        .collect()
}

fn workflow_numeric_family_evidence(
    left: &WorkflowPhaseSnapshot,
    right: &WorkflowPhaseSnapshot,
    family: &str,
) -> Vec<String> {
    let left_score = left.family_score_map.get(family).copied();
    let right_score = right.family_score_map.get(family).copied();
    match (left_score, right_score) {
        (Some(left_score), Some(right_score)) => vec![
            format!("left_avg_score={:.4}", left_score),
            format!("right_avg_score={:.4}", right_score),
            format!("avg_score_delta={:.4}", right_score - left_score),
        ],
        _ => Vec::new(),
    }
}

fn workflow_numeric_factor_evidence(
    left: &WorkflowPhaseSnapshot,
    right: &WorkflowPhaseSnapshot,
    factor: &str,
) -> Vec<String> {
    let left_score = left.factor_score_map.get(factor).copied();
    let right_score = right.factor_score_map.get(factor).copied();
    match (left_score, right_score) {
        (Some(left_score), Some(right_score)) => vec![
            format!("left_composite_score={:.4}", left_score),
            format!("right_composite_score={:.4}", right_score),
            format!("composite_score_delta={:.4}", right_score - left_score),
        ],
        _ => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn analyze_snapshot_keeps_applied_regime_bundle_bbn_evidence_visible() {
        let snapshot = workflow_phase_snapshot_from_analyze_run(&AnalyzeRunRecord {
            run_id: "analyze:regime-bundle".to_string(),
            source_command: "analyze --regime-consumer-bundle bundle.json --apply-regime-bundle-bbn-soft-evidence".to_string(),
            pre_bayes_evidence_filter: PreBayesEvidenceFilter {
                uses_soft_evidence: true,
                gating_status: "pass_hard".to_string(),
                evidence_assignments: BTreeMap::from([
                    ("market_regime".to_string(), "bull".to_string()),
                    (
                        "regime_bundle_bbn_application_status".to_string(),
                        "applied".to_string(),
                    ),
                    (
                        "regime_bundle_bbn_market_regime".to_string(),
                        "bull".to_string(),
                    ),
                ]),
                soft_market_regime_distribution: BTreeMap::from([
                    ("bull".to_string(), 0.9),
                    ("bear".to_string(), 0.05),
                    ("range".to_string(), 0.05),
                ]),
                ..PreBayesEvidenceFilter::default()
            },
            canonical_structural_regime_posterior: Some(
                ict_engine::state::CanonicalStructuralRegimePosterior {
                    active_regime: Some("trend".to_string()),
                    confidence: Some(0.78),
                    probabilities: BTreeMap::from([
                        ("trend".to_string(), 0.78),
                        ("range".to_string(), 0.14),
                        ("transition".to_string(), 0.08),
                    ]),
                    evidence: vec!["duration_persistence_prior=0.900".to_string()],
                },
            ),
            ..AnalyzeRunRecord::default()
        });

        assert_eq!(
            snapshot.pre_bayes_filtered_assignments["market_regime"],
            "bull"
        );
        assert_eq!(
            snapshot.pre_bayes_filtered_assignments["regime_bundle_bbn_application_status"],
            "applied"
        );
        assert_eq!(
            snapshot.pre_bayes_soft_evidence["market_regime"]["bull"],
            0.9
        );
        assert!(snapshot
            .phase_summary
            .contains("regime_bundle_bbn=applied:bull"));
    }
}
