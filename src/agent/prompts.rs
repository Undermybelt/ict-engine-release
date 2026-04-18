use serde::{Deserialize, Serialize};

use crate::state::{
    DecisionThresholds, FactorIterationPrompt, FeedbackHistorySummary, PersistedFactorRanking,
    ProbabilityDiff, RankingDiffItem,
};

pub const PROMPT_PACK_VERSION: &str = "agent-prompts-v1";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPrompt {
    pub id: String,
    pub stage: String,
    pub priority: String,
    pub objective: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub success_criteria: Vec<String>,
    pub suggested_files: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentPromptPack {
    pub version: String,
    pub workflow: String,
    pub prompts: Vec<AgentPrompt>,
}

#[derive(Debug, Clone, Default)]
pub struct AgentPromptInput {
    pub id: String,
    pub stage: String,
    pub priority: String,
    pub objective: String,
    pub system_prompt: String,
    pub user_prompt: String,
    pub success_criteria: Vec<String>,
    pub suggested_files: Vec<String>,
}

impl AgentPrompt {
    pub fn new(input: AgentPromptInput) -> Self {
        Self {
            id: input.id,
            stage: input.stage,
            priority: input.priority,
            objective: input.objective,
            system_prompt: input.system_prompt,
            user_prompt: input.user_prompt,
            success_criteria: input.success_criteria,
            suggested_files: input.suggested_files,
        }
    }
}

pub fn factor_iteration_prompt_pack(
    symbol: &str,
    rankings: &[PersistedFactorRanking],
    iteration_queue: &[FactorIterationPrompt],
    feedback_summary: &FeedbackHistorySummary,
) -> AgentPromptPack {
    let top = rankings
        .iter()
        .take(3)
        .map(|ranking| {
            format!(
                "{} score={:.2} grade={} action={}",
                ranking.factor_name,
                ranking.composite_score,
                ranking.grade,
                ranking.iteration_action
            )
        })
        .collect::<Vec<_>>()
        .join("; ");
    let queue = iteration_queue
        .iter()
        .take(5)
        .map(|prompt| {
            format!(
                "{} score={:.2} action={} prompt={}",
                prompt.factor_name, prompt.composite_score, prompt.iteration_action, prompt.prompt
            )
        })
        .collect::<Vec<_>>()
        .join(" || ");

    AgentPromptPack {
        version: PROMPT_PACK_VERSION.to_string(),
        workflow: format!(
            "Use stored research state to decide factor tuning and replacement for {} without auto-generating code inside the engine. Keep internal evidence compact, but when answering a human user, translate the result into exactly five readable blocks: (1) 基本价格结构分析, (2) 技术面价格分析, (3) SMT相关性分析, (4) Regime分类结合贝叶斯分析并给推测概率, (5) 交易计划. Do not expose internal terms like pre-Bayes gate, structure_ict verdict, or market-specific fork directly unless the user explicitly asks for internals.",
            symbol
        ),
        prompts: vec![
            AgentPrompt::new(AgentPromptInput {
                id: "factor_triage".to_string(),
                stage: "factor_research".to_string(),
                priority: "high".to_string(),
                objective: "Select which factors to keep, tune, observe, or replace.".to_string(),
                system_prompt: "You are the factor-iteration agent. Use only the supplied scorecards and feedback summary to decide which factor edits are justified. Do not invent success criteria; follow the thresholds embedded in each factor prompt.".to_string(),
                user_prompt: format!(
                    "Symbol={} top_scorecards=[{}] iteration_queue=[{}] feedback_summary=records:{} wins:{} losses:{} avg_pnl:{:.6} factor_success_rates:{:?}",
                    symbol,
                    top,
                    queue,
                    feedback_summary.total_records,
                    feedback_summary.wins,
                    feedback_summary.losses,
                    feedback_summary.avg_pnl,
                    feedback_summary.factor_success_rates
                ),
                success_criteria: vec![
                    "Prioritize replace > tune > observe > keep".to_string(),
                    "Only recommend replacing a factor when its action is replace or replacement_candidate=true".to_string(),
                    "Use the factor-specific agent prompt as the acceptance rule for the next iteration".to_string(),
                ],
                suggested_files: vec![
                    "src/factor_lab/factor_definition.rs".to_string(),
                    "src/factors/registry.rs".to_string(),
                    "src/factors/weight_updater.rs".to_string(),
                ],
            }),
            AgentPrompt::new(AgentPromptInput {
                id: "feedback_review".to_string(),
                stage: "feedback_learning".to_string(),
                priority: "medium".to_string(),
                objective: "Review which factors are failing by regime and whether weight updates should be trusted.".to_string(),
                system_prompt: "You are the feedback-diagnostics agent. Compare factor success rates, grading, and weaknesses. Focus on regime-conditional failure and persistent underperformance.".to_string(),
                user_prompt: format!(
                    "Symbol={} feedback_total={} avg_pnl={:.6} weak_factors=[{}]",
                    symbol,
                    feedback_summary.total_records,
                    feedback_summary.avg_pnl,
                    rankings
                        .iter()
                        .filter(|ranking| ranking.iteration_action != "keep")
                        .map(|ranking| format!(
                            "{} weaknesses={:?} score={:.2}",
                            ranking.factor_name, ranking.weaknesses, ranking.composite_score
                        ))
                        .collect::<Vec<_>>()
                        .join("; ")
                ),
                success_criteria: vec![
                    "Flag factors with narrow regime coverage or unstable walk-forward".to_string(),
                    "Do not promote a weaker factor just because it has higher recent win rate on small sample".to_string(),
                ],
                suggested_files: vec![
                    "src/state/types.rs".to_string(),
                    "src/factors/regime_conditional.rs".to_string(),
                    "src/factors/weight_updater.rs".to_string(),
                ],
            }),
        ],
    }
}

pub fn dataset_audit_prompt(
    symbol: &str,
    data_path: &str,
    paired_data_path: Option<&str>,
    candles: usize,
    paired_candles: Option<usize>,
    source_command: &str,
) -> AgentPrompt {
    AgentPrompt::new(AgentPromptInput {
        id: "dataset_audit".to_string(),
        stage: "dataset_audit".to_string(),
        priority: "high".to_string(),
        objective: "Audit whether the dataset and command context are sufficient for trustworthy iteration.".to_string(),
        system_prompt: "You are the dataset-audit agent. Check whether the current dataset scope, paired market coverage, and command context are sufficient before making factor or model changes.".to_string(),
        user_prompt: format!(
            "Symbol={} data_path={} paired_data_path={:?} candles={} paired_candles={:?} source_command={}",
            symbol, data_path, paired_data_path, candles, paired_candles, source_command
        ),
        success_criteria: vec![
            "Flag low sample size or missing paired-market evidence before approving major factor changes".to_string(),
            "If this run is not comparable to prior runs, tell the next agent not to over-interpret score deltas".to_string(),
        ],
        suggested_files: vec![
            "src/data/loader.rs".to_string(),
            "src/main.rs".to_string(),
            "src/factor_lab/research.rs".to_string(),
        ],
    })
}

pub fn research_diff_prompt(
    symbol: &str,
    score_deltas: &[RankingDiffItem],
    generated: usize,
    applied: usize,
) -> AgentPrompt {
    AgentPrompt::new(AgentPromptInput {
        id: "research_diff_review".to_string(),
        stage: "research_diff".to_string(),
        priority: "high".to_string(),
        objective: "Review what changed in this research run compared with the previous run.".to_string(),
        system_prompt: "You are the research-diff agent. Compare factor score, weight, and action changes. Focus on whether the latest run actually improved candidate quality or just changed rankings without robust gains.".to_string(),
        user_prompt: format!(
            "Symbol={} feedback_generated={} feedback_applied={} score_deltas={:?}",
            symbol, generated, applied, score_deltas
        ),
        success_criteria: vec![
            "Flag factors whose score improved but action stayed replace or tune".to_string(),
            "Highlight score drops on previously strong factors before promoting new edits".to_string(),
        ],
        suggested_files: vec![
            "src/factor_lab/research.rs".to_string(),
            "src/state/types.rs".to_string(),
            "src/factors/weight_updater.rs".to_string(),
        ],
    })
}

pub fn update_diff_prompt(
    symbol: &str,
    probability_deltas: &[ProbabilityDiff],
    score_deltas: &[RankingDiffItem],
    duplicate_feedback_skipped: bool,
) -> AgentPrompt {
    AgentPrompt::new(AgentPromptInput {
        id: "update_diff_review".to_string(),
        stage: "update_diff".to_string(),
        priority: "high".to_string(),
        objective: "Review whether the realized update materially changed model state.".to_string(),
        system_prompt: "You are the update-diff agent. Use the trade_outcome probability deltas and factor score deltas to judge whether this realized result should change factor code, evidence mapping, or neither.".to_string(),
        user_prompt: format!(
            "Symbol={} duplicate_feedback_skipped={} trade_outcome_deltas={:?} factor_score_deltas={:?}",
            symbol, duplicate_feedback_skipped, probability_deltas, score_deltas
        ),
        success_criteria: vec![
            "If duplicate_feedback_skipped is true, recommend no model change".to_string(),
            "If outcome probabilities changed materially but factor scores did not, investigate BBN evidence mapping".to_string(),
        ],
        suggested_files: vec![
            "src/main.rs".to_string(),
            "src/bbn/trading/topology.rs".to_string(),
            "src/factors/weight_updater.rs".to_string(),
        ],
    })
}

pub fn promotion_gate_prompt(
    symbol: &str,
    rankings: &[PersistedFactorRanking],
    score_deltas: &[RankingDiffItem],
    thresholds: &DecisionThresholds,
) -> AgentPrompt {
    AgentPrompt::new(AgentPromptInput {
        id: "promotion_gate".to_string(),
        stage: "promotion_gate".to_string(),
        priority: "high".to_string(),
        objective: "Decide whether a factor iteration is strong enough to promote.".to_string(),
        system_prompt: "You are the promotion-gate agent. Only approve factor promotion when score deltas, grading, and supporting metrics clear the configured improvement thresholds. Reject cosmetic ranking changes.".to_string(),
        user_prompt: format!(
            "Symbol={} thresholds={{promotion_min_score_delta:{:.3}, promotion_min_score:{:.3}}} top_rankings={:?} score_deltas={:?}",
            symbol,
            thresholds.promotion_min_score_delta,
            thresholds.promotion_min_score,
            rankings
                .iter()
                .take(5)
                .map(|ranking| format!(
                    "{} score={:.2} grade={} action={}",
                    ranking.factor_name, ranking.composite_score, ranking.grade, ranking.iteration_action
                ))
                .collect::<Vec<_>>(),
            score_deltas
        ),
        success_criteria: vec![
            "Only promote replace/tune outcomes when score delta is material and stability is not worse".to_string(),
            "If trade_count is low or dataset coverage changed, recommend holdout validation before promotion".to_string(),
        ],
        suggested_files: vec![
            "src/state/types.rs".to_string(),
            "src/factors/weight_updater.rs".to_string(),
            "src/factor_lab/research.rs".to_string(),
        ],
    })
}

pub fn rollback_review_prompt(
    symbol: &str,
    score_deltas: &[RankingDiffItem],
    probability_deltas: &[ProbabilityDiff],
    thresholds: &DecisionThresholds,
) -> AgentPrompt {
    AgentPrompt::new(AgentPromptInput {
        id: "rollback_review".to_string(),
        stage: "rollback_review".to_string(),
        priority: "high".to_string(),
        objective: "Decide whether recent changes should be rolled back or isolated.".to_string(),
        system_prompt: "You are the rollback-review agent. Look for score degradation, action downgrades, and harmful probability shifts. Recommend rollback when the latest iteration weakened factor quality or destabilized outcome calibration.".to_string(),
        user_prompt: format!(
            "Symbol={} thresholds={{rollback_score_delta:{:.3}, rollback_probability_delta:{:.3}}} score_deltas={:?} trade_outcome_deltas={:?}",
            symbol,
            thresholds.rollback_score_delta,
            thresholds.rollback_probability_delta,
            score_deltas,
            probability_deltas
        ),
        success_criteria: vec![
            "Recommend rollback when strong factors degrade materially or outcome calibration shifts against realized performance".to_string(),
            "If only one factor family regressed, prefer targeted rollback over full revert".to_string(),
        ],
        suggested_files: vec![
            "src/main.rs".to_string(),
            "src/bbn/trading/topology.rs".to_string(),
            "src/factors/weight_updater.rs".to_string(),
        ],
    })
}
