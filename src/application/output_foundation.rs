use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use crate::state::WorkflowPhaseSnapshot;

pub fn redact_local_paths(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.char_indices().peekable();
    while let Some((i, ch)) = chars.next() {
        if ch == '/' {
            let rest = &text[i..];
            let is_local = rest.starts_with("/Users/")
                || rest.starts_with("/home/")
                || rest.starts_with("/tmp/")
                || rest.starts_with("/var/")
                || rest.starts_with("/private/")
                || rest.starts_with("/Volumes/");
            if is_local {
                let mut end = text.len();
                for (relative_index, path_ch) in rest.char_indices() {
                    if path_ch.is_ascii_whitespace()
                        || matches!(path_ch, ',' | ';' | '|' | ')' | '(' | '[' | ']' | '{' | '}')
                    {
                        end = i + relative_index;
                        break;
                    }
                }
                while chars
                    .peek()
                    .is_some_and(|(next_index, _)| *next_index < end)
                {
                    chars.next();
                }
                out.push_str("<local-path>");
                continue;
            }
        }
        out.push(ch);
    }
    out
}

fn looks_like_machine_command(text: &str) -> bool {
    let Some(first_token) = text.split_whitespace().next() else {
        return false;
    };
    matches!(
        first_token,
        "ict-engine" | "cargo" | "uv" | "python" | "python3" | "bash" | "sh" | "zsh"
    ) || first_token.starts_with("./")
        || first_token.starts_with('/')
}

fn redact_local_paths_in_human_line(line: &str) -> String {
    if let Some((prefix, command)) = line.split_once("Then run: ") {
        if looks_like_machine_command(command) {
            return format!("{}Then run: {}", redact_local_paths(prefix), command);
        }
    }
    for marker in ["Next: ", "- Next: "] {
        if let Some(command) = line.strip_prefix(marker) {
            if looks_like_machine_command(command) {
                return format!("{marker}{command}");
            }
        }
    }
    redact_local_paths(line)
}

pub fn redact_local_paths_in_human_text(text: &str) -> String {
    text.split('\n')
        .map(redact_local_paths_in_human_line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn preserves_machine_command_value(key: &str) -> bool {
    matches!(
        key,
        "next_command"
            | "recommended_next_command"
            | "command"
            | "recommended_command"
            | "deferred_command"
            | "executable_command"
            | "pointer_command"
    )
}

pub fn redact_local_paths_in_value(value: &mut Value) {
    redact_local_paths_in_value_for_key(None, value);
}

fn redact_local_paths_in_value_for_key(key: Option<&str>, value: &mut Value) {
    match value {
        Value::String(text) => {
            if !key.is_some_and(preserves_machine_command_value) {
                *text = redact_local_paths(text);
            }
        }
        Value::Array(items) => {
            for item in items {
                redact_local_paths_in_value_for_key(key, item);
            }
        }
        Value::Object(map) => {
            for (child_key, item) in map.iter_mut() {
                redact_local_paths_in_value_for_key(Some(child_key.as_str()), item);
            }
        }
        _ => {}
    }
}

pub fn print_redacted_json<T: Serialize>(value: &T) -> Result<()> {
    let mut rendered = serde_json::to_value(value)?;
    redact_local_paths_in_value(&mut rendered);
    println!("{}", serde_json::to_string_pretty(&rendered)?);
    Ok(())
}

pub fn format_executor_summary_lines(executor_summaries: &[String]) -> Vec<String> {
    executor_summaries
        .iter()
        .map(|summary| summary.to_string())
        .collect()
}

pub fn short_workflow_phase_summary(phase: &WorkflowPhaseSnapshot) -> String {
    let mut parts = Vec::new();
    if let Some(direction) = &phase.selected_direction {
        parts.push(format!("direction={direction}"));
    }
    if let Some(entry) = &phase.selected_entry_quality {
        parts.push(format!("entry={entry}"));
    }
    if !phase.pre_bayes_gate_status.is_empty() {
        parts.push(format!("gate={}", phase.pre_bayes_gate_status));
    }
    if phase.pre_bayes_evidence_quality_score > 0.0 {
        parts.push(format!(
            "quality={:.3}",
            phase.pre_bayes_evidence_quality_score
        ));
    }
    if parts.is_empty() {
        phase.phase_summary.clone()
    } else {
        parts.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redaction_preserves_agent_machine_command_fields() {
        let mut value = serde_json::json!({
            "next_command": "ict-engine factor-research --state-dir /tmp/ict-agent-state",
            "recommended_next_command": "ict-engine workflow-status --state-dir /tmp/ict-agent-state",
            "recommended_command": "ict-engine workflow-status --state-dir /tmp/ict-agent-state",
            "pointer_command": "ict-engine workflow-status --state-dir /tmp/ict-agent-state --output-format json",
            "command": "ict-engine analyze --data /tmp/data.json --state-dir /tmp/ict-agent-state",
            "next_step": {
                "deferred_command": "ict-engine factor-research --data /tmp/data.json --state-dir /tmp/ict-agent-state",
                "prompt": "Ask about /tmp/data.json"
            },
            "display_command": "ict-engine factor-research --state-dir /tmp/ict-agent-state",
            "path": "/tmp/ict-agent-state/NQ/artifact.json"
        });

        redact_local_paths_in_value(&mut value);

        assert_eq!(
            value["next_command"],
            "ict-engine factor-research --state-dir /tmp/ict-agent-state"
        );
        assert_eq!(
            value["recommended_next_command"],
            "ict-engine workflow-status --state-dir /tmp/ict-agent-state"
        );
        assert_eq!(
            value["recommended_command"],
            "ict-engine workflow-status --state-dir /tmp/ict-agent-state"
        );
        assert_eq!(
            value["pointer_command"],
            "ict-engine workflow-status --state-dir /tmp/ict-agent-state --output-format json"
        );
        assert_eq!(
            value["command"],
            "ict-engine analyze --data /tmp/data.json --state-dir /tmp/ict-agent-state"
        );
        assert_eq!(
            value["next_step"]["deferred_command"],
            "ict-engine factor-research --data /tmp/data.json --state-dir /tmp/ict-agent-state"
        );
        assert_eq!(value["next_step"]["prompt"], "Ask about <local-path>");
        assert_eq!(
            value["display_command"],
            "ict-engine factor-research --state-dir <local-path>"
        );
        assert_eq!(value["path"], "<local-path>");
    }

    #[test]
    fn redaction_preserves_utf8_prompt_text_while_redacting_paths() {
        let mut value = serde_json::json!({
            "workflow": "回答五段: 基本价格结构分析, 技术面价格分析, SMT相关性分析, Regime分类结合贝叶斯分析并给推测概率, 交易计划",
            "display": "state at /tmp/ict-engine-first-run-native/report.json"
        });

        redact_local_paths_in_value(&mut value);

        let workflow = value["workflow"].as_str().unwrap();
        assert!(workflow.contains("基本价格结构分析"));
        assert!(workflow.contains("技术面价格分析"));
        assert!(workflow.contains("交易计划"));
        assert!(!workflow.contains("å"));
        assert_eq!(value["display"], "state at <local-path>");
    }

    #[test]
    fn redaction_preserves_machine_commands_inside_human_text() {
        let rendered = redact_local_paths_in_human_text(
            "State: /tmp/ict-agent-state/report.json\nNext: ict-engine factor-research --data /tmp/data.json --state-dir /tmp/ict-agent-state\n- Next: Ask the user: pick from /tmp/a.json, /tmp/b.json Then run: ict-engine factor-research --data /tmp/a.json --state-dir /tmp/ict-agent-state",
        );

        assert!(rendered.contains("State: <local-path>"));
        assert!(rendered.contains(
            "Next: ict-engine factor-research --data /tmp/data.json --state-dir /tmp/ict-agent-state"
        ));
        assert!(rendered.contains("pick from <local-path>, <local-path> Then run:"));
        assert!(rendered.contains(
            "Then run: ict-engine factor-research --data /tmp/a.json --state-dir /tmp/ict-agent-state"
        ));
    }
}
