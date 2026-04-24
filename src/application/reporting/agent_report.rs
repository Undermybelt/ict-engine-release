use serde::Serialize;

use super::compact_report::humanize_decision_hint;
use crate::application::orchestration::ExecutionTriage;
use crate::state::{recommended_next_command_meta, RecommendedNextCommandKind};

#[derive(Debug, Clone, Serialize, Default)]
pub struct AgentNextStep {
    pub action_type: String,
    pub user_input_required: bool,
    pub blocked_reason: Option<String>,
    pub prompt: Option<String>,
    pub deferred_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct AgentGuidanceReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub execution_triage: Option<ExecutionTriage>,
    pub direction: Option<String>,
    pub entry_state: Option<String>,
    pub pre_bayes_gate: Option<String>,
    pub next_command: Option<String>,
    pub decision_hint_raw: Option<String>,
    pub decision_summary: Option<String>,
    pub next_step: AgentNextStep,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
    pub recommended_next_actions: Vec<String>,
}

fn top_k(items: &[String], limit: usize) -> Vec<String> {
    items.iter().take(limit).cloned().collect()
}

fn parse_next_step(next_command: Option<&str>) -> AgentNextStep {
    let Some(next_command) = next_command
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return AgentNextStep {
            action_type: "none".to_string(),
            user_input_required: false,
            blocked_reason: None,
            prompt: None,
            deferred_command: None,
        };
    };
    let meta = recommended_next_command_meta(next_command);
    match meta.kind {
        RecommendedNextCommandKind::AskUser => AgentNextStep {
            action_type: "ask_user_choose_historical_data".to_string(),
            user_input_required: true,
            blocked_reason: Some("user_selected_historical_data_missing".to_string()),
            prompt: meta.prompt,
            deferred_command: meta.executable_command,
        },
        RecommendedNextCommandKind::Unavailable | RecommendedNextCommandKind::Unknown => {
            AgentNextStep {
                action_type: "none".to_string(),
                user_input_required: false,
                blocked_reason: None,
                prompt: None,
                deferred_command: None,
            }
        }
        _ => AgentNextStep {
            action_type: "run_command".to_string(),
            user_input_required: false,
            blocked_reason: None,
            prompt: None,
            deferred_command: meta
                .executable_command
                .or_else(|| Some(next_command.to_string())),
        },
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_agent_guidance_report(
    direction: Option<String>,
    entry_state: Option<String>,
    pre_bayes_gate: Option<String>,
    next_command: Option<String>,
    decision: Option<String>,
    evidence: &[String],
    risks: &[String],
    recommended_next_actions: &[String],
) -> AgentGuidanceReport {
    AgentGuidanceReport {
        execution_triage: None,
        direction,
        entry_state,
        pre_bayes_gate,
        next_step: parse_next_step(next_command.as_deref()),
        next_command,
        decision_summary: decision.as_ref().map(|hint| humanize_decision_hint(hint)),
        decision_hint_raw: decision,
        evidence: top_k(evidence, 3),
        risks: top_k(risks, 3),
        recommended_next_actions: top_k(recommended_next_actions, 3),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_guidance_parses_ask_user_next_step() {
        let report = build_agent_guidance_report(
            None,
            None,
            None,
            Some(
                "ask-user: Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=a.json | blocked until user_selected_historical_data | then ict-engine factor-research --symbol NQ --data a.json --state-dir state".to_string(),
            ),
            Some("observe_only_not_comparable_to_last_analyze:missing_previous".to_string()),
            &[],
            &[],
            &[],
        );

        assert_eq!(
            report.next_step.action_type,
            "ask_user_choose_historical_data"
        );
        assert!(report.next_step.user_input_required);
        assert_eq!(
            report.next_step.blocked_reason.as_deref(),
            Some("user_selected_historical_data_missing")
        );
        assert!(report
            .next_step
            .prompt
            .as_deref()
            .unwrap()
            .contains("ask the user which dataset"));
        assert_eq!(
            report.next_step.deferred_command.as_deref(),
            Some("ict-engine factor-research --symbol NQ --data a.json --state-dir state")
        );
        assert_eq!(
            report.decision_summary.as_deref(),
            Some("Observe only: current data is not comparable to the previous analyze run")
        );
    }
}
