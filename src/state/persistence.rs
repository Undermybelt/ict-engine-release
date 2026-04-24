use anyhow::{Context, Result};
use chrono::Utc;
use serde::{de::DeserializeOwned, Serialize};
use std::path::Path;

use crate::state::types::{
    AnalyzeRunRecord, ArtifactLedgerEntry, BacktestRunRecord, EnsembleExecutorScorecard,
    EnsembleVoteRecord, ExecutionCandidateArtifact, FactorAutoresearchAttempt,
    FactorAutoresearchLiveSnapshot, FactorAutoresearchSession, FactorMutationRunRecord,
    LearningState, PendingUpdateArtifact, PreBayesPolicyRecord, ResearchRunRecord, TrainRunRecord,
    UpdateRunRecord, ANALYZE_RUNS_FILE, ARTIFACT_LEDGER_FILE, BACKTEST_RUNS_FILE,
    ENSEMBLE_EXECUTOR_SCORECARDS_FILE, ENSEMBLE_VOTE_FILE, ENSEMBLE_VOTE_HISTORY_FILE,
    EXECUTION_CANDIDATE_FILE, EXECUTION_CANDIDATE_HISTORY_FILE, FACTOR_AUTORESEARCH_ATTEMPTS_FILE,
    FACTOR_AUTORESEARCH_FINAL_FILE, FACTOR_AUTORESEARCH_LIVE_FILE,
    FACTOR_AUTORESEARCH_SESSIONS_FILE, FACTOR_MUTATION_RUNS_FILE, LEARNING_STATE_FILE,
    PENDING_UPDATE_ARTIFACT_FILE, PENDING_UPDATE_HISTORY_FILE, PRE_BAYES_POLICY_HISTORY_FILE,
    RESEARCH_RUNS_FILE, TRADE_HISTORY_FILE, TRAIN_RUNS_FILE, UPDATE_RUNS_FILE,
    WORKFLOW_SNAPSHOT_FILE,
};

pub fn artifact_state_path<P: AsRef<Path>>(dir: P, symbol: &str, filename: &str) -> String {
    dir.as_ref()
        .join(symbol)
        .join(filename)
        .to_string_lossy()
        .to_string()
}

/// Load state from JSON file
pub fn load_state<T: DeserializeOwned, P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    filename: &str,
) -> Result<T> {
    let path = dir.as_ref().join(symbol).join(filename);
    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read state file: {:?}", path))?;
    let data: T = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse state file: {:?}", path))?;
    Ok(data)
}

pub fn load_state_or_default<T, P: AsRef<Path>>(dir: P, symbol: &str, filename: &str) -> Result<T>
where
    T: DeserializeOwned + Default,
{
    if state_exists(&dir, symbol, filename) {
        load_state(dir, symbol, filename)
    } else {
        Ok(T::default())
    }
}

/// Save state to JSON file
pub fn save_state<T: Serialize + ?Sized, P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    filename: &str,
    data: &T,
) -> Result<()> {
    let dir_path = dir.as_ref().join(symbol);
    std::fs::create_dir_all(&dir_path)
        .with_context(|| format!("Failed to create directory: {:?}", dir_path))?;

    let path = dir_path.join(filename);
    let json = serde_json::to_string_pretty(data).context("Failed to serialize state")?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write state file: {:?}", path))?;

    Ok(())
}

pub fn save_text_state<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    filename: &str,
    content: &str,
) -> Result<()> {
    let dir_path = dir.as_ref().join(symbol);
    std::fs::create_dir_all(&dir_path)
        .with_context(|| format!("Failed to create directory: {:?}", dir_path))?;

    let path = dir_path.join(filename);
    std::fs::write(&path, content)
        .with_context(|| format!("Failed to write state file: {:?}", path))?;

    Ok(())
}

pub fn load_learning_state<P: AsRef<Path>>(dir: P, symbol: &str) -> Result<LearningState> {
    load_state_or_default(dir, symbol, LEARNING_STATE_FILE)
}

pub fn save_learning_state<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    learning_state: &LearningState,
) -> Result<()> {
    let mut learning_state = learning_state.clone();
    learning_state.last_updated = Some(Utc::now());
    save_state(dir, symbol, LEARNING_STATE_FILE, &learning_state)
}

pub fn append_learning_feedback<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    feedback: crate::state::types::FeedbackRecord,
) -> Result<LearningState> {
    let mut learning_state = load_learning_state(&dir, symbol)?;
    learning_state.merge_feedback_records(&[feedback]);
    save_learning_state(&dir, symbol, &learning_state)?;
    Ok(learning_state)
}

pub fn append_learning_feedback_batch<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    feedback: &[crate::state::types::FeedbackRecord],
) -> Result<LearningState> {
    let mut learning_state = load_learning_state(&dir, symbol)?;
    learning_state.merge_feedback_records(feedback);
    save_learning_state(&dir, symbol, &learning_state)?;
    Ok(learning_state)
}

pub fn append_trade_history<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    trades: &[crate::types::TradeRecord],
) -> Result<Vec<crate::types::TradeRecord>> {
    let mut history: Vec<crate::types::TradeRecord> =
        load_state_or_default(&dir, symbol, TRADE_HISTORY_FILE)?;
    history.extend(trades.iter().cloned());
    save_state(&dir, symbol, TRADE_HISTORY_FILE, &history)?;
    Ok(history)
}

pub fn append_research_run<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    record: ResearchRunRecord,
) -> Result<Vec<ResearchRunRecord>> {
    let mut history: Vec<ResearchRunRecord> =
        load_state_or_default(&dir, symbol, RESEARCH_RUNS_FILE)?;
    history.push(record);
    save_state(&dir, symbol, RESEARCH_RUNS_FILE, &history)?;
    Ok(history)
}

pub fn append_train_run<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    record: TrainRunRecord,
) -> Result<Vec<TrainRunRecord>> {
    let mut history: Vec<TrainRunRecord> = load_state_or_default(&dir, symbol, TRAIN_RUNS_FILE)?;
    history.push(record);
    save_state(&dir, symbol, TRAIN_RUNS_FILE, &history)?;
    Ok(history)
}

pub fn append_factor_mutation_run<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    record: FactorMutationRunRecord,
) -> Result<Vec<FactorMutationRunRecord>> {
    let mut history: Vec<FactorMutationRunRecord> =
        load_state_or_default(&dir, symbol, FACTOR_MUTATION_RUNS_FILE)?;
    history.push(record);
    save_state(&dir, symbol, FACTOR_MUTATION_RUNS_FILE, &history)?;
    Ok(history)
}

pub fn load_factor_autoresearch_sessions<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Vec<FactorAutoresearchSession>> {
    load_state_or_default(dir, symbol, FACTOR_AUTORESEARCH_SESSIONS_FILE)
}

pub fn save_factor_autoresearch_sessions<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    sessions: &[FactorAutoresearchSession],
) -> Result<()> {
    save_state(dir, symbol, FACTOR_AUTORESEARCH_SESSIONS_FILE, sessions)
}

pub fn append_factor_autoresearch_attempt<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    attempt: FactorAutoresearchAttempt,
) -> Result<Vec<FactorAutoresearchAttempt>> {
    let mut history: Vec<FactorAutoresearchAttempt> =
        load_state_or_default(&dir, symbol, FACTOR_AUTORESEARCH_ATTEMPTS_FILE)?;
    history.push(attempt);
    save_state(&dir, symbol, FACTOR_AUTORESEARCH_ATTEMPTS_FILE, &history)?;
    Ok(history)
}

pub fn load_factor_autoresearch_attempts<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Vec<FactorAutoresearchAttempt>> {
    load_state_or_default(dir, symbol, FACTOR_AUTORESEARCH_ATTEMPTS_FILE)
}

pub fn load_factor_autoresearch_live_snapshot<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<FactorAutoresearchLiveSnapshot> {
    load_state_or_default(dir, symbol, FACTOR_AUTORESEARCH_LIVE_FILE)
}

pub fn save_factor_autoresearch_live_snapshot<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    snapshot: &FactorAutoresearchLiveSnapshot,
) -> Result<()> {
    save_state(dir, symbol, FACTOR_AUTORESEARCH_LIVE_FILE, snapshot)
}

pub fn save_factor_autoresearch_final_summary<P: AsRef<Path>, T: Serialize + ?Sized>(
    dir: P,
    symbol: &str,
    summary: &T,
) -> Result<()> {
    save_state(dir, symbol, FACTOR_AUTORESEARCH_FINAL_FILE, summary)
}

pub fn append_analyze_run<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    record: AnalyzeRunRecord,
) -> Result<Vec<AnalyzeRunRecord>> {
    let mut history: Vec<AnalyzeRunRecord> =
        load_state_or_default(&dir, symbol, ANALYZE_RUNS_FILE)?;
    history.push(record);
    save_state(&dir, symbol, ANALYZE_RUNS_FILE, &history)?;
    Ok(history)
}

pub fn append_pre_bayes_policy_history<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    record: PreBayesPolicyRecord,
) -> Result<Vec<PreBayesPolicyRecord>> {
    let mut history: Vec<PreBayesPolicyRecord> =
        load_state_or_default(&dir, symbol, PRE_BAYES_POLICY_HISTORY_FILE)?;
    history.push(record);
    save_state(&dir, symbol, PRE_BAYES_POLICY_HISTORY_FILE, &history)?;
    Ok(history)
}

pub fn load_pre_bayes_policy_history<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Vec<PreBayesPolicyRecord>> {
    load_state_or_default(dir, symbol, PRE_BAYES_POLICY_HISTORY_FILE)
}

pub fn append_update_run<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    record: UpdateRunRecord,
) -> Result<Vec<UpdateRunRecord>> {
    let mut history: Vec<UpdateRunRecord> = load_state_or_default(&dir, symbol, UPDATE_RUNS_FILE)?;
    history.push(record);
    save_state(&dir, symbol, UPDATE_RUNS_FILE, &history)?;
    Ok(history)
}

pub fn append_backtest_run<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    record: BacktestRunRecord,
) -> Result<Vec<BacktestRunRecord>> {
    let mut history: Vec<BacktestRunRecord> =
        load_state_or_default(&dir, symbol, BACKTEST_RUNS_FILE)?;
    history.push(record);
    save_state(&dir, symbol, BACKTEST_RUNS_FILE, &history)?;
    Ok(history)
}

pub fn load_workflow_snapshot<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<crate::state::types::WorkflowSnapshot> {
    load_state_or_default(dir, symbol, WORKFLOW_SNAPSHOT_FILE)
}

pub fn save_workflow_snapshot<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    snapshot: &crate::state::types::WorkflowSnapshot,
) -> Result<()> {
    save_state(dir, symbol, WORKFLOW_SNAPSHOT_FILE, snapshot)
}

pub fn load_pending_update_artifact<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<PendingUpdateArtifact> {
    load_state_or_default(dir, symbol, PENDING_UPDATE_ARTIFACT_FILE)
}

pub fn save_pending_update_artifact<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    artifact: &PendingUpdateArtifact,
) -> Result<()> {
    save_state(dir, symbol, PENDING_UPDATE_ARTIFACT_FILE, artifact)
}

pub fn append_pending_update_artifact_history<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    artifact: PendingUpdateArtifact,
) -> Result<Vec<PendingUpdateArtifact>> {
    let mut history: Vec<PendingUpdateArtifact> =
        load_state_or_default(&dir, symbol, PENDING_UPDATE_HISTORY_FILE)?;
    history.push(artifact);
    save_state(&dir, symbol, PENDING_UPDATE_HISTORY_FILE, &history)?;
    Ok(history)
}

pub fn load_pending_update_history<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Vec<PendingUpdateArtifact>> {
    load_state_or_default(dir, symbol, PENDING_UPDATE_HISTORY_FILE)
}

pub fn load_execution_candidate_artifact<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<ExecutionCandidateArtifact> {
    load_state_or_default(dir, symbol, EXECUTION_CANDIDATE_FILE)
}

pub fn save_execution_candidate_artifact<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    artifact: &ExecutionCandidateArtifact,
) -> Result<()> {
    save_state(dir, symbol, EXECUTION_CANDIDATE_FILE, artifact)
}

pub fn append_execution_candidate_history<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    artifact: ExecutionCandidateArtifact,
) -> Result<Vec<ExecutionCandidateArtifact>> {
    let mut history: Vec<ExecutionCandidateArtifact> =
        load_state_or_default(&dir, symbol, EXECUTION_CANDIDATE_HISTORY_FILE)?;
    history.push(artifact);
    save_state(&dir, symbol, EXECUTION_CANDIDATE_HISTORY_FILE, &history)?;
    Ok(history)
}

pub fn load_execution_candidate_history<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Vec<ExecutionCandidateArtifact>> {
    load_state_or_default(dir, symbol, EXECUTION_CANDIDATE_HISTORY_FILE)
}

pub fn load_ensemble_vote_artifact<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<EnsembleVoteRecord> {
    load_state_or_default(dir, symbol, ENSEMBLE_VOTE_FILE)
}

pub fn save_ensemble_vote_artifact<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    artifact: &EnsembleVoteRecord,
) -> Result<()> {
    save_state(dir, symbol, ENSEMBLE_VOTE_FILE, artifact)
}

pub fn append_ensemble_vote_history<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    artifact: EnsembleVoteRecord,
) -> Result<Vec<EnsembleVoteRecord>> {
    let mut history: Vec<EnsembleVoteRecord> =
        load_state_or_default(&dir, symbol, ENSEMBLE_VOTE_HISTORY_FILE)?;
    history.push(artifact);
    save_state(&dir, symbol, ENSEMBLE_VOTE_HISTORY_FILE, &history)?;
    Ok(history)
}

pub fn load_ensemble_vote_history<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Vec<EnsembleVoteRecord>> {
    load_state_or_default(dir, symbol, ENSEMBLE_VOTE_HISTORY_FILE)
}

pub fn load_ensemble_executor_scorecards<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Vec<EnsembleExecutorScorecard>> {
    load_state_or_default(dir, symbol, ENSEMBLE_EXECUTOR_SCORECARDS_FILE)
}

pub fn save_ensemble_executor_scorecards<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    scorecards: &[EnsembleExecutorScorecard],
) -> Result<()> {
    save_state(dir, symbol, ENSEMBLE_EXECUTOR_SCORECARDS_FILE, &scorecards)
}

pub fn migrate_ensemble_executor_scorecards<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Vec<EnsembleExecutorScorecard>> {
    let persisted: Vec<EnsembleExecutorScorecard> =
        load_state_or_default(&dir, symbol, ENSEMBLE_EXECUTOR_SCORECARDS_FILE)?;
    if !persisted.is_empty() {
        return Ok(persisted);
    }
    let history: Vec<EnsembleVoteRecord> =
        load_state_or_default(&dir, symbol, ENSEMBLE_VOTE_HISTORY_FILE)?;
    let derived = history
        .into_iter()
        .rev()
        .find_map(|record| {
            if !record.executor_scorecards.is_empty() {
                Some(record.executor_scorecards)
            } else if !record.executor_summaries.is_empty() {
                Some(
                    record
                        .executor_summaries
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
                        .collect(),
                )
            } else {
                None
            }
        })
        .unwrap_or_default();
    if !derived.is_empty() {
        save_ensemble_executor_scorecards(&dir, symbol, &derived)?;
    }
    Ok(derived)
}

pub fn append_artifact_ledger_entry<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    entry: ArtifactLedgerEntry,
) -> Result<Vec<ArtifactLedgerEntry>> {
    let mut history: Vec<ArtifactLedgerEntry> =
        load_state_or_default(&dir, symbol, ARTIFACT_LEDGER_FILE)?;
    history.push(entry);
    save_state(&dir, symbol, ARTIFACT_LEDGER_FILE, &history)?;
    Ok(history)
}

pub fn load_artifact_ledger<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
) -> Result<Vec<ArtifactLedgerEntry>> {
    load_state_or_default(dir, symbol, ARTIFACT_LEDGER_FILE)
}

pub fn mark_artifact_consumed<P: AsRef<Path>>(
    dir: P,
    symbol: &str,
    artifact_id: &str,
    update_run_id: &str,
    realized_outcome: &str,
    pnl: f64,
) -> Result<Vec<ArtifactLedgerEntry>> {
    let mut history: Vec<ArtifactLedgerEntry> =
        load_state_or_default(&dir, symbol, ARTIFACT_LEDGER_FILE)?;
    let now = Utc::now();
    for entry in &mut history {
        if entry.artifact_id == artifact_id {
            entry.consumed_by_update_run_id = Some(update_run_id.to_string());
            entry.consumed_at = Some(now);
            entry.consumed_outcome = Some(realized_outcome.to_string());
            entry.regraded_at = Some(now);
            let (regrade_status, regrade_reason, quality_adjustment) = match realized_outcome {
                "win" if pnl > 0.0 => ("validated_positive", "consumed_with_positive_pnl", 20),
                "win" => ("validated_positive", "consumed_with_win_outcome", 10),
                "loss" if pnl < 0.0 => ("validated_negative", "consumed_with_negative_pnl", -20),
                "loss" => ("validated_negative", "consumed_with_loss_outcome", -10),
                _ => ("validated_neutral", "consumed_with_breakeven_outcome", 0),
            };
            entry.consumption_regrade_status = Some(regrade_status.to_string());
            entry.consumption_regrade_reason = Some(regrade_reason.to_string());
            entry.quality_score += quality_adjustment;
            entry.actionable = false;
            entry.promote_candidate = false;
        }
    }
    save_state(&dir, symbol, ARTIFACT_LEDGER_FILE, &history)?;
    Ok(history)
}

/// Check if state file exists
pub fn state_exists<P: AsRef<Path>>(dir: P, symbol: &str, filename: &str) -> bool {
    dir.as_ref().join(symbol).join(filename).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bbn::trading::topology::build_trading_network;
    use crate::state::types::{
        AnalyzeRunRecord, BacktestRunRecord, FeedbackFactorUsage, FeedbackRecord,
        ModelProbabilitySnapshot, ResearchRunRecord, TRADE_HISTORY_FILE,
    };
    use crate::types::{Direction, Regime, TradeRecord};
    use chrono::Utc;
    use std::collections::HashMap;

    #[test]
    fn test_migrate_ensemble_executor_scorecards_from_vote_history() {
        let temp = tempfile::tempdir().unwrap();
        let record = EnsembleVoteRecord {
            artifact_id: "ensemble-vote:test".to_string(),
            generated_at: Utc::now(),
            symbol: "NQ".to_string(),
            source_phase: "analyze".to_string(),
            source_run_id: Some("run-1".to_string()),
            provenance: crate::state::types::RunProvenance::default(),
            dataset_comparability: crate::state::types::DatasetComparability::default(),
            ensemble_version: "ensemble-audit-v2-weighted".to_string(),
            final_action: "observe".to_string(),
            recommended_command: "ict-engine workflow-status --symbol NQ --phase human-next"
                .to_string(),
            human_next_triage: "hard_blocked=false ensemble_action=observe".to_string(),
            hard_block: crate::application::orchestration::EnsembleHardBlockArtifact::default(),
            confidence: 0.5,
            consensus_strength: 0.5,
            disagreement_flags: Vec::new(),
            executor_summaries: vec![
                "executor=catboost_stub action=observe confidence=0.500 weight=0.55".to_string(),
            ],
            split_explanations: vec!["active_regime=research".to_string()],
            executor_scorecards: Vec::new(),
            executor_scorecards_source: Some("fallback".to_string()),
            posterior_fingerprint: "fp-test".to_string(),
            posterior_normalization_status: "normalized".to_string(),
            posterior_active_regime: "research".to_string(),
            posterior_confidence: Some(0.5),
            posterior_probabilities: std::collections::BTreeMap::new(),
            posterior_evidence: vec!["mtf=test".to_string()],
        };
        append_ensemble_vote_history(temp.path(), "NQ", record).unwrap();

        let migrated = migrate_ensemble_executor_scorecards(temp.path(), "NQ").unwrap();
        assert_eq!(migrated[0].executor, "catboost_stub");
        assert_eq!(migrated[0].latest_weight_hint, Some(0.55));
    }

    #[test]
    fn test_save_and_load_bayesian_network_state() {
        let temp = tempfile::tempdir().unwrap();
        let network = build_trading_network().unwrap();

        save_state(temp.path(), "NQ", "bbn_network.json", &network).unwrap();
        let restored =
            load_state::<crate::bbn::BayesianNetwork, _>(temp.path(), "NQ", "bbn_network.json")
                .unwrap();

        assert_eq!(network.nodes.len(), restored.nodes.len());
        assert_eq!(network.edges.len(), restored.edges.len());
        assert_eq!(network.topological_order, restored.topological_order);
    }

    #[test]
    fn test_feedback_persistence_round_trip() {
        let temp = tempfile::tempdir().unwrap();
        let feedback = FeedbackRecord {
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            source: "backtest".to_string(),
            run_id: None,
            trade_id: None,
            prompt_version: None,
            factor_version: None,
            data_fingerprint: None,
            factors_used: vec![FeedbackFactorUsage {
                factor_name: "trend_momentum".to_string(),
                category: "trend_momentum".to_string(),
                direction: Direction::Bull,
                value: 0.8,
                confidence: 0.7,
                weight: 0.2,
                long_support: 0.5,
                short_support: 0.0,
                uncertainty_contribution: 0.1,
            }],
            model_probabilities_before_trade: ModelProbabilitySnapshot {
                selected_direction: Direction::Bull,
                selected_probability: 0.61,
                long_score: 0.61,
                short_score: 0.34,
                win_prob_long: 0.58,
                win_prob_short: 0.41,
                uncertainty: 0.2,
            },
            realized_outcome: "win".to_string(),
            pnl: 0.02,
            regime_at_entry: Regime::ManipulationExpansion,
        };

        let learning_state = append_learning_feedback(temp.path(), "NQ", feedback).unwrap();
        assert_eq!(learning_state.feedback_history.len(), 1);

        let restored = load_learning_state(temp.path(), "NQ").unwrap();
        assert_eq!(restored.feedback_history.len(), 1);
        assert_eq!(restored.summary().wins, 1);
    }

    #[test]
    fn test_load_state_or_default_works_for_trade_history() {
        let temp = tempfile::tempdir().unwrap();
        let restored: Vec<TradeRecord> =
            load_state_or_default(temp.path(), "NQ", TRADE_HISTORY_FILE).unwrap();
        assert!(restored.is_empty());

        let trade = TradeRecord {
            timestamp: Utc::now(),
            symbol: crate::types::Symbol::NQ,
            direction: Direction::Bull,
            entry_price: 100.0,
            exit_price: 101.0,
            pnl: 0.01,
            exit_reason: None,
            regime_at_entry: Regime::ManipulationExpansion,
            cascade_max_layer: crate::types::CascadeLayer::L1,
            cascade_direction: Direction::Bull,
            factor_values: HashMap::new(),
        };
        save_state(temp.path(), "NQ", TRADE_HISTORY_FILE, &vec![trade]).unwrap();
        let restored: Vec<TradeRecord> =
            load_state_or_default(temp.path(), "NQ", TRADE_HISTORY_FILE).unwrap();
        assert_eq!(restored.len(), 1);
    }

    #[test]
    fn test_append_trade_history_appends_instead_of_overwriting() {
        let temp = tempfile::tempdir().unwrap();
        let trade_one = TradeRecord {
            timestamp: Utc::now(),
            symbol: crate::types::Symbol::NQ,
            direction: Direction::Bull,
            entry_price: 100.0,
            exit_price: 101.0,
            pnl: 0.01,
            exit_reason: None,
            regime_at_entry: Regime::ManipulationExpansion,
            cascade_max_layer: crate::types::CascadeLayer::L1,
            cascade_direction: Direction::Bull,
            factor_values: HashMap::new(),
        };
        let trade_two = TradeRecord {
            timestamp: Utc::now(),
            symbol: crate::types::Symbol::NQ,
            direction: Direction::Bear,
            entry_price: 102.0,
            exit_price: 100.0,
            pnl: 0.02,
            exit_reason: None,
            regime_at_entry: Regime::Distribution,
            cascade_max_layer: crate::types::CascadeLayer::L1,
            cascade_direction: Direction::Bear,
            factor_values: HashMap::new(),
        };

        append_trade_history(temp.path(), "NQ", &[trade_one]).unwrap();
        let history = append_trade_history(temp.path(), "NQ", &[trade_two]).unwrap();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_append_research_run_persists_history() {
        let temp = tempfile::tempdir().unwrap();
        let record = ResearchRunRecord {
            run_id: "research:test".to_string(),
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            research_objective: String::new(),
            provenance: crate::state::RunProvenance::default(),
            decision_thresholds: crate::state::DecisionThresholds::default(),
            dataset_comparability: crate::state::DatasetComparability::default(),
            promotion_decision: crate::state::PromotionDecision::default(),
            rollback_recommendation: crate::state::RollbackRecommendation::default(),
            family_history_window: 5,
            data_path: "sample.json".to_string(),
            paired_data_path: None,
            candles: 140,
            paired_candles: None,
            config_name: "default".to_string(),
            source_command: "factor-research".to_string(),
            factor_count: 5,
            best_factor: Some("trend_momentum".to_string()),
            aggregate_return: 0.02,
            feedback_records_generated: 10,
            feedback_records_applied: 8,
            factor_score_deltas: Vec::new(),
            factor_family_decisions: Vec::new(),
            factor_family_outcomes: Vec::new(),
            factor_family_diffs: Vec::new(),
            factor_family_history: Vec::new(),
            decision_history_summary: crate::state::DecisionHistorySummary::default(),
            workflow_state: crate::state::WorkflowState::default(),
            agent_action_plan: crate::state::AgentActionPlan::default(),
            recommended_commands: crate::state::CommandRecommendations::default(),
            recommended_next_command: String::new(),
            recommended_next_command_meta: crate::state::types::recommended_next_command_meta(""),
            agent_context_bundle: crate::state::AgentContextBundle::default(),
            agent_context_bundle_minimal: crate::state::AgentContextBundleMinimal::default(),
            feedback_history_summary: crate::state::FeedbackHistorySummary::default(),
            multi_timeframe_summary: Vec::new(),
            execution_artifact_id: None,
            execution_edge_share: None,
            prediction_edge_share: None,
            execution_readiness: None,
            execution_gate_status: None,
            pda_cluster_label: None,
            artifact_action_summary: Vec::new(),
            duration_sizing_scale: None,
            hybrid_duration_model: None,
            hybrid_remaining_expected_bars: None,
            backtest_conformal_coverage_1sigma: 0.0,
            backtest_trade_count: 0,
            artifact_decision_summary: crate::state::ArtifactDecisionSummary::default(),
            artifact_decision_section: crate::state::ArtifactDecisionSection::default(),
            agent_prompts: crate::agent::AgentPromptPack::default(),
            prompt_workflow: "workflow".to_string(),
            factor_mutation_evaluation: None,
        };

        let history = append_research_run(temp.path(), "NQ", record).unwrap();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_append_train_run_persists_history() {
        let temp = tempfile::tempdir().unwrap();
        let record = TrainRunRecord {
            run_id: "train:test".to_string(),
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            provenance: crate::state::RunProvenance::default(),
            dataset_comparability: crate::state::DatasetComparability::default(),
            source_command: "train".to_string(),
            data_path: "sample.json".to_string(),
            epochs: 20,
            candles: 240,
            observations: 180,
            final_state: "Bull".to_string(),
            log_likelihood: -12.0,
            viterbi_log_likelihood: -11.0,
            workflow_state: crate::state::WorkflowState::default(),
            agent_action_plan: crate::state::AgentActionPlan::default(),
            recommended_commands: crate::state::CommandRecommendations::default(),
            recommended_next_command: String::new(),
            recommended_next_command_meta: crate::state::types::recommended_next_command_meta(""),
            agent_context_bundle: crate::state::AgentContextBundle::default(),
            agent_context_bundle_minimal: crate::state::AgentContextBundleMinimal::default(),
            agent_prompts: crate::agent::AgentPromptPack::default(),
            prompt_workflow: "workflow".to_string(),
            multi_timeframe_summary: Vec::new(),
        };

        let history = append_train_run(temp.path(), "NQ", record).unwrap();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_append_backtest_run_persists_history() {
        let temp = tempfile::tempdir().unwrap();
        let record = BacktestRunRecord {
            run_id: "backtest:test".to_string(),
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            provenance: crate::state::RunProvenance::default(),
            decision_thresholds: crate::state::DecisionThresholds::default(),
            dataset_comparability: crate::state::DatasetComparability::default(),
            promotion_decision: crate::state::PromotionDecision::default(),
            rollback_recommendation: crate::state::RollbackRecommendation::default(),
            family_history_window: 5,
            data_path: "sample.json".to_string(),
            paired_data_path: None,
            candles: 140,
            paired_candles: None,
            warmup_bars: 60,
            hold_bars: 10,
            online_learning: true,
            source_command: "backtest".to_string(),
            total_return: 0.05,
            trade_count: 12,
            conformal_coverage_1sigma: 0.75,
            conformal_miscoverage_1sigma: 0.25,
            mean_prediction_interval_half_width: 0.02,
            worst_window_miscoverage: 0.03,
            regime_break_penalty: 0.10,
            structural_break_score: 1.25,
            structural_break_index: Some(4),
            structural_break_detected: true,
            signal_structural_break_score: 1.10,
            signal_structural_break_index: Some(4),
            signal_structural_break_detected: true,
            residual_structural_break_score: 0.80,
            residual_structural_break_index: Some(5),
            residual_structural_break_detected: false,
            rolling_ic_structural_break_score: 1.35,
            rolling_ic_structural_break_index: Some(3),
            rolling_ic_structural_break_detected: true,
            factor_score_deltas: Vec::new(),
            trade_outcome_deltas: Vec::new(),
            factor_family_decisions: Vec::new(),
            factor_family_outcomes: Vec::new(),
            factor_family_diffs: Vec::new(),
            factor_family_history: Vec::new(),
            decision_history_summary: crate::state::DecisionHistorySummary::default(),
            workflow_state: crate::state::WorkflowState::default(),
            agent_action_plan: crate::state::AgentActionPlan::default(),
            recommended_commands: crate::state::CommandRecommendations::default(),
            recommended_next_command: String::new(),
            recommended_next_command_meta: crate::state::types::recommended_next_command_meta(""),
            agent_context_bundle: crate::state::AgentContextBundle::default(),
            agent_context_bundle_minimal: crate::state::AgentContextBundleMinimal::default(),
            feedback_history_summary: crate::state::FeedbackHistorySummary::default(),
            multi_timeframe_summary: Vec::new(),
            objective_market_credibility_shrink: None,
            artifact_action_summary: Vec::new(),
            duration_sizing_scale: None,
            hybrid_duration_model: None,
            hybrid_remaining_expected_bars: None,
            execution_artifact_id: None,
            execution_edge_share: None,
            prediction_edge_share: None,
            execution_readiness: None,
            execution_gate_status: None,
            artifact_decision_summary: crate::state::ArtifactDecisionSummary::default(),
            artifact_decision_section: crate::state::ArtifactDecisionSection::default(),
            agent_prompts: crate::agent::AgentPromptPack::default(),
            prompt_workflow: "workflow".to_string(),
        };

        let history = append_backtest_run(temp.path(), "NQ", record).unwrap();
        assert_eq!(history.len(), 1);
    }

    #[test]
    fn test_append_analyze_run_persists_history() {
        let temp = tempfile::tempdir().unwrap();
        let record = AnalyzeRunRecord {
            run_id: "analyze:test".to_string(),
            timestamp: Utc::now(),
            symbol: "NQ".to_string(),
            provenance: crate::state::RunProvenance::default(),
            decision_thresholds: crate::state::DecisionThresholds::default(),
            dataset_comparability: crate::state::DatasetComparability::default(),
            promotion_decision: crate::state::PromotionDecision::default(),
            rollback_recommendation: crate::state::RollbackRecommendation::default(),
            family_history_window: 5,
            source_command: "analyze".to_string(),
            data_htf_path: Some("htf.json".to_string()),
            data_mtf_path: Some("mtf.json".to_string()),
            data_ltf_path: Some("ltf.json".to_string()),
            live_data_source: None,
            htf_bars: 120,
            mtf_bars: 140,
            ltf_bars: 160,
            selected_direction: crate::types::Direction::Bull,
            selected_entry_quality: "high".to_string(),
            decision_hint: "observe".to_string(),
            hybrid_regime_label: None,
            hybrid_regime_age_bars: None,
            hybrid_duration_model: None,
            hybrid_remaining_expected_bars: None,
            pre_bayes_evidence_filter: crate::state::PreBayesEvidenceFilter::default(),
            pre_bayes_entry_quality_bridge: crate::state::PreBayesEntryQualityBridge::default(),
            factor_family_decisions: Vec::new(),
            factor_family_outcomes: Vec::new(),
            factor_family_diffs: Vec::new(),
            factor_family_history: Vec::new(),
            decision_history_summary: crate::state::DecisionHistorySummary::default(),
            workflow_state: crate::state::WorkflowState::default(),
            agent_action_plan: crate::state::AgentActionPlan::default(),
            recommended_commands: crate::state::CommandRecommendations::default(),
            recommended_next_command: String::new(),
            recommended_next_command_meta: crate::state::types::recommended_next_command_meta(""),
            agent_context_bundle: crate::state::AgentContextBundle::default(),
            agent_context_bundle_minimal: crate::state::AgentContextBundleMinimal::default(),
            feedback_history_summary: crate::state::FeedbackHistorySummary::default(),
            multi_timeframe_summary: Vec::new(),
            execution_artifact_id: None,
            execution_edge_share: None,
            prediction_edge_share: None,
            execution_readiness: None,
            execution_gate_status: None,
            artifact_action_summary: Vec::new(),
            artifact_decision_summary: crate::state::ArtifactDecisionSummary::default(),
            artifact_decision_section: crate::state::ArtifactDecisionSection::default(),
            agent_prompts: crate::agent::AgentPromptPack::default(),
            prompt_workflow: "workflow".to_string(),
        };

        let history = append_analyze_run(temp.path(), "NQ", record).unwrap();
        assert_eq!(history.len(), 1);
    }
}
