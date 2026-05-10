use super::*;
use ict_engine::application::regime::HMM_STATE_FILE;

pub(crate) fn train_command(
    symbol: &str,
    data: &str,
    epochs: usize,
    state_dir: &str,
) -> Result<()> {
    let (observations, multi_timeframe_summary, candles_total) =
        build_multi_timeframe_training_observations(data)?;
    let initial_params = load_or_init_hmm_params(symbol, state_dir);
    let trained_params = BaumWelch::fit(&observations, &initial_params, epochs, 1e-4);
    let (_, log_likelihood) = ForwardBackward::forward(&observations, &trained_params);
    let (states, viterbi_log_likelihood) = Viterbi::decode(&observations, &trained_params);
    let learning_state = load_learning_state(state_dir, symbol)?;
    let previous_runs: Vec<TrainRunRecord> =
        load_state_or_default(state_dir, symbol, TRAIN_RUNS_FILE)?;
    let provenance = run_provenance(
        &learning_state,
        &["train", data, &epochs.to_string()],
        compute_hash(&["train", symbol, data, &epochs.to_string()]),
    );
    let dataset_comparability = dataset_comparability(
        previous_runs.last().map(|run| run.run_id.clone()),
        previous_runs.last().map(|run| &run.provenance),
        &provenance,
    );
    let workflow_state = WorkflowState {
        phase: "train_review_ready".to_string(),
        reason: "multi_timeframe_hmm_training_completed".to_string(),
    };
    let mut agent_action_plan = AgentActionPlan {
        summary: "review multi-timeframe HMM training outcome".to_string(),
        items: vec![AgentActionItem {
            stage: "train".to_string(),
            blocking: false,
            priority: "medium".to_string(),
            title: "Review Train Run".to_string(),
            rationale: format!(
                "epochs={} observations={} final_state={}",
                epochs,
                observations.len(),
                states.last().copied().map(state_name).unwrap_or("Unknown")
            ),
            expected_output: "A training review confirming whether the latest HMM state should feed the next analyze/research cycle".to_string(),
            expected_state_changes: vec![ExpectedStateChange {
                target: "hmm_params".to_string(),
                direction: "trained_multi_timeframe".to_string(),
                rationale: "multi_timeframe_hmm_training_completed".to_string(),
            }],
            suggested_files: vec!["src/main.rs".to_string(), "src/hmm/baum_welch.rs".to_string()],
            suggested_commands: vec!["ict-engine analyze --data-htf <file> --data-mtf <file> --data-ltf <file>".to_string()],
        }],
    };
    let recommended_commands = command_recommendations(&CommandContext {
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        analyze: Some(AnalyzeCommandSource::Files {
            data_htf: data.to_string(),
            data_mtf: data.to_string(),
            data_ltf: data.to_string(),
        }),
        research_data: Some(data.to_string()),
        paired_data: None,
        update_outcome: None,
        update_entry_signal: None,
        update_feedback_file: pending_update_artifact_path(state_dir, symbol),
        user_data_selection_required: true,
    });
    concretize_action_plan_commands(&mut agent_action_plan, &recommended_commands);
    let recommended_next_command =
        recommended_next_command(&agent_action_plan, &recommended_commands);
    let mut agent_context_bundle = build_agent_context_bundle(BuildAgentContextBundleInput {
        symbol,
        state_dir,
        workflow_state: &workflow_state,
        decision_hint: "train_review_ready",
        recommended_next_command: &recommended_next_command,
        recommended_commands: &recommended_commands,
        dataset_comparability: &dataset_comparability,
        factor_iteration_queue: &[],
        family_outcomes: &[],
        pre_bayes_evidence_filter: None,
        pre_bayes_entry_quality_bridge: None,
        pda_sequence_summary: None,
        factor_mutation_evaluation: None,
        artifact_decision_summary: None,
    });
    agent_context_bundle.multi_timeframe_summary = multi_timeframe_summary.clone();
    let agent_context_bundle_minimal = build_agent_context_bundle_minimal(&agent_context_bundle);
    let mut agent_prompts = AgentPromptPack {
        version: PROMPT_PACK_VERSION.to_string(),
        workflow: format!(
            "Review the latest multi-timeframe HMM training result for {} before the next analyze/research cycle.",
            symbol
        ),
        prompts: vec![dataset_audit_prompt(symbol, data, None, candles_total, None, "train")],
    };
    agent_prompts.prompts.push(AgentPrompt::new(AgentPromptInput {
        id: "train_review".to_string(),
        stage: "train".to_string(),
        priority: "high".to_string(),
        objective: "Review whether the latest multi-timeframe HMM training result is usable.".to_string(),
        system_prompt: "You are the train-review agent. Use the training observations, likelihoods, and multi-timeframe summary to decide whether the latest HMM training result should feed the next analysis cycle or be treated cautiously.".to_string(),
        user_prompt: format!(
            "Symbol={} epochs={} observations={} final_state={} log_likelihood={:.4} viterbi_log_likelihood={:.4} multi_timeframe_summary={:?}",
            symbol,
            epochs,
            observations.len(),
            states.last().copied().map(state_name).unwrap_or("Unknown"),
            log_likelihood,
            viterbi_log_likelihood,
            multi_timeframe_summary
        ),
        success_criteria: vec![
            "Prefer using the trained HMM only when likelihoods are finite and multi-timeframe coverage is present".to_string(),
            "If multi-timeframe coverage is missing, downgrade confidence in the next analyze cycle".to_string(),
        ],
        suggested_files: vec!["src/main.rs".to_string(), "src/hmm/baum_welch.rs".to_string()],
    }));

    save_state(state_dir, symbol, HMM_STATE_FILE, &trained_params)?;
    append_train_run(
        state_dir,
        symbol,
        TrainRunRecord {
            run_id: format!(
                "train:{}:{}",
                symbol,
                Utc::now().format("%Y%m%dT%H%M%S%.3fZ")
            ),
            timestamp: Utc::now(),
            symbol: symbol.to_string(),
            provenance,
            dataset_comparability,
            source_command: "train".to_string(),
            data_path: data.to_string(),
            epochs,
            candles: candles_total,
            observations: observations.len(),
            final_state: states
                .last()
                .copied()
                .map(state_name)
                .unwrap_or("Unknown")
                .to_string(),
            log_likelihood,
            viterbi_log_likelihood,
            workflow_state,
            agent_action_plan,
            recommended_commands,
            recommended_next_command_meta: recommended_next_command_meta(&recommended_next_command),
            recommended_next_command,
            agent_context_bundle,
            agent_context_bundle_minimal,
            agent_prompts: agent_prompts.clone(),
            prompt_workflow: agent_prompts.workflow.clone(),
            multi_timeframe_summary: multi_timeframe_summary.clone(),
        },
    )?;
    let workflow_snapshot = refresh_workflow_snapshot(state_dir, symbol)?;

    println!(
        "train symbol={} state_dir={} epochs={} candles={} observations={} final_state={} log_likelihood={:.4} viterbi_log_likelihood={:.4} multi_timeframe_summary={:?} workflow_phase={} saved={}/{}",
        symbol,
        state_dir,
        epochs,
        candles_total,
        observations.len(),
        states.last().copied().map(state_name).unwrap_or("Unknown"),
        log_likelihood,
        viterbi_log_likelihood,
        multi_timeframe_summary,
        workflow_snapshot.current_focus_phase,
        symbol,
        HMM_STATE_FILE,
    );
    Ok(())
}
