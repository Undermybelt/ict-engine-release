use crate::config::shell_quote;
use crate::state::{CommandRecommendations, RecommendedCommand};

#[derive(Debug, Clone)]
pub enum AnalyzeCommandSource {
    Files {
        data_htf: String,
        data_mtf: String,
        data_ltf: String,
    },
    Live {
        source: Box<crate::state::LiveDataSourceProvenance>,
    },
}

#[derive(Debug, Clone, Default)]
pub struct CommandContext {
    pub symbol: String,
    pub state_dir: String,
    pub analyze: Option<AnalyzeCommandSource>,
    pub research_data: Option<String>,
    pub paired_data: Option<String>,
    pub update_outcome: Option<String>,
    pub update_entry_signal: Option<String>,
    pub update_feedback_file: Option<String>,
    pub user_data_selection_required: bool,
}

pub fn recommended_command(
    command: String,
    ready: bool,
    missing_inputs: Vec<String>,
    rationale: impl Into<String>,
) -> RecommendedCommand {
    RecommendedCommand {
        command,
        ready,
        missing_inputs,
        rationale: rationale.into(),
        user_data_selection_required: false,
        user_data_selection_prompt: "user_data_selection_not_required".to_string(),
        recorded_data_paths: Vec::new(),
    }
}

pub fn user_data_selection_prompt(symbol: &str, data_paths: &[String]) -> String {
    let recorded = if data_paths.is_empty() {
        "recorded_paths=[]".to_string()
    } else {
        format!("recorded_paths={}", data_paths.join(", "))
    };
    format!(
        "Before using historical data for {} again, ask the user which dataset to use. {}",
        symbol, recorded
    )
}

fn demo_research_backend_suffix(recorded_data_paths: &[String]) -> &'static str {
    if recorded_data_paths.len() == 1 && recorded_data_paths[0].starts_with("examples/demo/") {
        " --backend native"
    } else {
        ""
    }
}

pub fn command_recommendations(context: &CommandContext) -> CommandRecommendations {
    let mut recorded_data_paths = Vec::new();
    if let Some(analyze) = &context.analyze {
        match analyze {
            AnalyzeCommandSource::Files {
                data_htf,
                data_mtf,
                data_ltf,
            } => {
                for path in [data_htf, data_mtf, data_ltf] {
                    if !recorded_data_paths.contains(path) {
                        recorded_data_paths.push(path.clone());
                    }
                }
            }
            AnalyzeCommandSource::Live { source } => {
                for path in [
                    source.persisted_htf_path.clone(),
                    source.persisted_mtf_path.clone(),
                    source.persisted_ltf_path.clone(),
                    source.persisted_spot_path.clone(),
                ]
                .into_iter()
                .flatten()
                {
                    if !recorded_data_paths.contains(&path) {
                        recorded_data_paths.push(path);
                    }
                }
            }
        }
    }
    if let Some(data) = &context.research_data {
        if !recorded_data_paths.contains(data) {
            recorded_data_paths.push(data.clone());
        }
    }
    if let Some(data) = &context.paired_data {
        if !recorded_data_paths.contains(data) {
            recorded_data_paths.push(data.clone());
        }
    }
    let requires_user_data_selection =
        context.user_data_selection_required && recorded_data_paths.len() > 1;
    let user_prompt = user_data_selection_prompt(&context.symbol, &recorded_data_paths);
    let research_backend_suffix = demo_research_backend_suffix(&recorded_data_paths);
    let analyze = match &context.analyze {
        Some(AnalyzeCommandSource::Files {
            data_htf,
            data_mtf,
            data_ltf,
        }) => recommended_command(
            format!(
                "ict-engine analyze --symbol {} --data-htf {} --data-mtf {} --data-ltf {} --state-dir {}",
                shell_quote(&context.symbol),
                shell_quote(data_htf),
                shell_quote(data_mtf),
                shell_quote(data_ltf),
                shell_quote(&context.state_dir)
            ),
            true,
            Vec::new(),
            "replay analyze with the same dataset",
        ),
        Some(AnalyzeCommandSource::Live { source }) => recommended_command(
            format!(
                "ict-engine analyze-live --symbol {} --futures-symbol {} --spot-symbol {} --options-symbol {} --spot-kind {} --futures-backend {} --aux-backend {} --external-http-base-url {} --crypto-public-base-url {} --state-dir {}",
                shell_quote(&context.symbol),
                shell_quote(&source.futures_symbol),
                shell_quote(&source.spot_symbol),
                shell_quote(&source.options_symbol),
                shell_quote(&source.spot_kind),
                shell_quote(&source.futures_backend),
                shell_quote(&source.aux_backend),
                shell_quote(&source.futures_base_url),
                shell_quote(&source.aux_base_url),
                shell_quote(&context.state_dir)
            ),
            true,
            Vec::new(),
            "replay live analyze with the same provider configuration",
        ),
        None => recommended_command(
            "recommended_command_unavailable".to_string(),
            false,
            vec!["analyze_input_context".to_string()],
            "analyze inputs are not available in this run context",
        ),
    };

    let mut research = if let Some(data) = &context.research_data {
        recommended_command(
            format!(
                "ict-engine factor-research --symbol {} --data {}{} --state-dir {}{}",
                shell_quote(&context.symbol),
                shell_quote(data),
                context
                    .paired_data
                    .as_ref()
                    .map(|paired| format!(" --paired-data {}", shell_quote(paired)))
                    .unwrap_or_default(),
                shell_quote(&context.state_dir),
                research_backend_suffix
            ),
            true,
            Vec::new(),
            "rerun factor research on the same dataset",
        )
    } else {
        recommended_command(
            "recommended_command_unavailable".to_string(),
            false,
            vec!["research_data_path".to_string()],
            "factor research requires a persisted data path",
        )
    };

    let mut backtest = if let Some(data) = &context.research_data {
        recommended_command(
            format!(
                "ict-engine factor-backtest --symbol {} --data {}{} --state-dir {}",
                shell_quote(&context.symbol),
                shell_quote(data),
                context
                    .paired_data
                    .as_ref()
                    .map(|paired| format!(" --paired-data {}", shell_quote(paired)))
                    .unwrap_or_default(),
                shell_quote(&context.state_dir)
            ),
            true,
            Vec::new(),
            "rerun factor backtest on the same dataset",
        )
    } else {
        recommended_command(
            "recommended_command_unavailable".to_string(),
            false,
            vec!["backtest_data_path".to_string()],
            "factor backtest requires a persisted data path",
        )
    };

    if requires_user_data_selection {
        for command in [&mut research, &mut backtest] {
            command.user_data_selection_required = true;
            command.user_data_selection_prompt = user_prompt.clone();
            command.recorded_data_paths = recorded_data_paths.clone();
            if command.ready {
                command.ready = false;
                command
                    .missing_inputs
                    .push("user_selected_historical_data".to_string());
                command.rationale = format!(
                    "{} User must explicitly choose one of the recorded historical datasets before rerun.",
                    command.rationale
                );
            }
        }
    } else {
        for command in [&mut research, &mut backtest] {
            command.recorded_data_paths = recorded_data_paths.clone();
        }
    }

    let update_command_template = format!(
        "ict-engine update --symbol {} --outcome {}{} --state-dir {}",
        shell_quote(&context.symbol),
        context
            .update_outcome
            .as_deref()
            .unwrap_or("<win|loss|breakeven>"),
        context
            .update_entry_signal
            .as_ref()
            .map(|signal| format!(" --entry-signal {}", shell_quote(signal)))
            .unwrap_or_default(),
        shell_quote(&context.state_dir)
    );

    let mut update = if let Some(feedback_file) = &context.update_feedback_file {
        let command = format!(
            "ict-engine update --symbol {} --outcome {} --state-dir {}",
            shell_quote(&context.symbol),
            context
                .update_outcome
                .as_deref()
                .unwrap_or("<win|loss|breakeven>"),
            shell_quote(&context.state_dir)
        );
        if context.update_outcome.is_some() {
            recommended_command(
                command,
                true,
                Vec::new(),
                format!(
                    "apply the persisted pending feedback artifact at {}",
                    feedback_file
                ),
            )
        } else {
            recommended_command(
                command,
                false,
                vec!["realized_outcome".to_string()],
                format!(
                    "pending update artifact exists at {} but realized outcome is still missing",
                    feedback_file
                ),
            )
        }
    } else if let Some(outcome) = &context.update_outcome {
        recommended_command(
            format!(
                "ict-engine update --symbol {} --outcome {}{} --state-dir {}",
                shell_quote(&context.symbol),
                shell_quote(outcome),
                context
                    .update_entry_signal
                    .as_ref()
                    .map(|signal| format!(" --entry-signal {}", shell_quote(signal)))
                    .unwrap_or_default(),
                shell_quote(&context.state_dir)
            ),
            true,
            Vec::new(),
            "replay the same realized outcome update",
        )
    } else {
        recommended_command(
            update_command_template,
            false,
            vec![
                "realized_outcome".to_string(),
                "pending_update_artifact_if_context_needed".to_string(),
            ],
            "update template is available, but a realized outcome is still required before it can be executed",
        )
    };

    update.recorded_data_paths = recorded_data_paths.clone();

    CommandRecommendations {
        analyze,
        research,
        backtest,
        update,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_recorded_research_path_does_not_require_user_selection() {
        let commands = command_recommendations(&CommandContext {
            symbol: "DEMO".to_string(),
            state_dir: "state".to_string(),
            analyze: Some(AnalyzeCommandSource::Files {
                data_htf: "examples/demo/demo-15m.json".to_string(),
                data_mtf: "examples/demo/demo-15m.json".to_string(),
                data_ltf: "examples/demo/demo-15m.json".to_string(),
            }),
            research_data: Some("examples/demo/demo-15m.json".to_string()),
            paired_data: None,
            update_outcome: None,
            update_entry_signal: None,
            update_feedback_file: None,
            user_data_selection_required: true,
        });

        assert!(commands.research.ready);
        assert!(!commands.research.user_data_selection_required);
        assert_eq!(
            commands.research.command,
            "ict-engine factor-research --symbol DEMO --data examples/demo/demo-15m.json --state-dir state --backend native"
        );
        assert_eq!(commands.research.recorded_data_paths.len(), 1);
        assert_eq!(
            commands.research.recorded_data_paths[0],
            "examples/demo/demo-15m.json"
        );
    }

    #[test]
    fn multiple_recorded_research_paths_still_require_user_selection() {
        let commands = command_recommendations(&CommandContext {
            symbol: "NQ".to_string(),
            state_dir: "state".to_string(),
            analyze: Some(AnalyzeCommandSource::Files {
                data_htf: "htf.json".to_string(),
                data_mtf: "mtf.json".to_string(),
                data_ltf: "ltf.json".to_string(),
            }),
            research_data: Some("ltf.json".to_string()),
            paired_data: None,
            update_outcome: None,
            update_entry_signal: None,
            update_feedback_file: None,
            user_data_selection_required: true,
        });

        assert!(!commands.research.ready);
        assert!(commands.research.user_data_selection_required);
        assert!(commands
            .research
            .missing_inputs
            .contains(&"user_selected_historical_data".to_string()));
        assert_eq!(commands.research.recorded_data_paths.len(), 3);
    }
}
