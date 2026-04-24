use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use crate::state::WorkflowPhaseSnapshot;

pub fn redact_local_paths(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'/' {
            let rest = &text[i..];
            let is_local = rest.starts_with("/Users/")
                || rest.starts_with("/home/")
                || rest.starts_with("/tmp/")
                || rest.starts_with("/var/")
                || rest.starts_with("/private/")
                || rest.starts_with("/Volumes/");
            if is_local {
                let mut j = i;
                while j < bytes.len() {
                    let ch = bytes[j];
                    if ch.is_ascii_whitespace()
                        || matches!(
                            ch,
                            b',' | b';' | b'|' | b')' | b'(' | b'[' | b']' | b'{' | b'}'
                        )
                    {
                        break;
                    }
                    j += 1;
                }
                out.push_str("<local-path>");
                i = j;
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

fn preserves_machine_command_value(key: &str) -> bool {
    matches!(
        key,
        "next_command"
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
            "recommended_command": "ict-engine workflow-status --state-dir /tmp/ict-agent-state",
            "pointer_command": "ict-engine workflow-status --state-dir /tmp/ict-agent-state --output-format json",
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
            value["recommended_command"],
            "ict-engine workflow-status --state-dir /tmp/ict-agent-state"
        );
        assert_eq!(
            value["pointer_command"],
            "ict-engine workflow-status --state-dir /tmp/ict-agent-state --output-format json"
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
}
