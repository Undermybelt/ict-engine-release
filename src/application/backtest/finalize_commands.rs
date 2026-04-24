use crate::state::{AgentActionPlan, CommandRecommendations, RecommendedCommand};

pub fn render_recommended_command(command: &RecommendedCommand) -> String {
    if command.user_data_selection_required {
        let rendered_command = if command.command.is_empty() {
            "choose historical dataset with user before running command".to_string()
        } else {
            command.command.clone()
        };
        let prompt = if command.user_data_selection_prompt.is_empty() {
            "ask user which historical dataset to use".to_string()
        } else {
            command.user_data_selection_prompt.clone()
        };
        return format!(
            "ask-user: {} | blocked until user_selected_historical_data | then {}",
            prompt, rendered_command
        );
    }
    if command.ready {
        command.command.clone()
    } else if !command.command.is_empty() {
        format!(
            "blocked: {} missing={}",
            command.command,
            command.missing_inputs.join(",")
        )
    } else if !command.rationale.is_empty() {
        format!(
            "blocked: {} missing={}",
            command.rationale,
            command.missing_inputs.join(",")
        )
    } else {
        "blocked".to_string()
    }
}

pub fn recommended_next_command(
    action_plan: &AgentActionPlan,
    commands: &CommandRecommendations,
) -> String {
    action_plan
        .items
        .iter()
        .max_by(|a, b| {
            action_priority(a.stage.as_str(), a.blocking, &a.priority).cmp(&action_priority(
                b.stage.as_str(),
                b.blocking,
                &b.priority,
            ))
        })
        .and_then(|item| {
            item.suggested_commands
                .iter()
                .find(|command| {
                    !(command.is_empty()
                        || command.starts_with("blocked:")
                        || command.contains('<') && command.contains('>'))
                })
                .cloned()
                .or_else(|| {
                    let command = command_for_stage(&item.stage, commands);
                    if command.user_data_selection_required {
                        Some(render_recommended_command(command))
                    } else {
                        command.ready.then(|| command.command.clone())
                    }
                })
        })
        .or_else(|| {
            [
                &commands.analyze,
                &commands.research,
                &commands.backtest,
                &commands.update,
            ]
            .into_iter()
            .find_map(|command| {
                if command.user_data_selection_required {
                    Some(render_recommended_command(command))
                } else if command.ready {
                    Some(command.command.clone())
                } else {
                    None
                }
            })
        })
        .unwrap_or_default()
}

pub fn concretize_action_plan_commands(
    action_plan: &mut AgentActionPlan,
    recommended_commands: &CommandRecommendations,
) {
    for item in &mut action_plan.items {
        let rendered =
            render_recommended_command(command_for_stage(&item.stage, recommended_commands));
        let has_template = item
            .suggested_commands
            .iter()
            .any(|command| command.contains('<') && command.contains('>'));
        if has_template {
            item.suggested_commands
                .retain(|command| !command.contains('<') || !command.contains('>'));
        }
        if !rendered.is_empty()
            && !item
                .suggested_commands
                .iter()
                .any(|command| command == &rendered)
        {
            item.suggested_commands.insert(0, rendered);
        }
    }
}

fn command_for_stage<'a>(
    stage: &str,
    commands: &'a CommandRecommendations,
) -> &'a RecommendedCommand {
    match stage {
        "analyze" | "market_analysis" | "pda_sequence_review" => &commands.analyze,
        "promotion" | "family_review" => &commands.research,
        "iteration" => &commands.backtest,
        "artifact_consumption" => {
            if commands.update.ready {
                &commands.update
            } else if commands.research.ready {
                &commands.research
            } else {
                &commands.backtest
            }
        }
        "rollback" => {
            if commands.update.ready {
                &commands.update
            } else if commands.research.ready {
                &commands.research
            } else {
                &commands.update
            }
        }
        _ => &commands.analyze,
    }
}

fn action_priority(stage: &str, blocking: bool, priority: &str) -> (u8, u8) {
    let stage_score = match stage {
        "artifact_consumption" => 5,
        "rollback" => 4,
        "promotion" => 3,
        "iteration" => 2,
        "family_review" => 1,
        _ => 0,
    };
    let blocking_score = if blocking { 10 } else { 0 };
    let priority_score = match priority {
        "high" => 3,
        "medium" => 2,
        "low" => 1,
        _ => 0,
    };
    (stage_score + blocking_score, priority_score)
}
