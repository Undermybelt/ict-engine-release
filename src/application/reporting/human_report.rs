use super::glossary_map::humanize_term;

pub fn humanize_next_step_line(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "recommended_command_unavailable" {
        return "(no next step)".to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("ask-user: ") {
        let prompt = rest
            .split(" | blocked until ")
            .next()
            .unwrap_or(rest)
            .trim()
            .trim_end_matches('.')
            .to_string();
        let deferred = rest
            .split(" | then ")
            .nth(1)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        return match deferred {
            Some(cmd) => format!("Ask the user: {prompt}. Then run: {cmd}"),
            None => format!("Ask the user: {prompt}"),
        };
    }
    trimmed.to_string()
}

#[derive(Debug, Clone, Default)]
pub struct HumanAnalyzeReport {
    pub execution_triage_line: Option<String>,
    pub summary_line: Option<String>,
    pub decision_line: Option<String>,
    pub action_line: Option<String>,
    pub next_action_line: Option<String>,
    pub basic_price_structure_analysis: String,
    pub technical_price_analysis: String,
    pub smt_correlation_analysis: String,
    pub regime_bayes_analysis: String,
    pub trade_plan: String,
}

impl HumanAnalyzeReport {
    pub fn render(&self) -> String {
        let mut sections = Vec::new();
        if let Some(triage_line) = &self.execution_triage_line {
            sections.push(triage_line.clone());
        }
        if let Some(summary_line) = &self.summary_line {
            sections.push(summary_line.clone());
        }
        if let Some(decision_line) = &self.decision_line {
            sections.push(decision_line.clone());
        }
        if let Some(action_line) = &self.action_line {
            sections.push(action_line.clone());
        }
        if let Some(next_action_line) = &self.next_action_line {
            sections.push(next_action_line.clone());
        }
        push_labeled_line(
            &mut sections,
            "Structure",
            &self.basic_price_structure_analysis,
        );
        push_labeled_line(&mut sections, "Technicals", &self.technical_price_analysis);
        push_labeled_line(&mut sections, "SMT", &self.smt_correlation_analysis);
        push_labeled_line(&mut sections, "Regime", &self.regime_bayes_analysis);
        push_labeled_line(&mut sections, "Plan", &self.trade_plan);
        sections.join("\n")
    }
}

fn push_labeled_line(lines: &mut Vec<String>, label: &str, value: &str) {
    let trimmed = value.trim();
    if !trimmed.is_empty() {
        lines.push(format!("{label}: {trimmed}"));
    }
}

#[allow(clippy::too_many_arguments)]
pub fn build_human_analyze_report(
    summary_line: Option<String>,
    decision_line: Option<String>,
    action_line: Option<String>,
    next_action_line: Option<String>,
    basic_price_structure_analysis: impl Into<String>,
    technical_price_analysis: impl Into<String>,
    smt_correlation_analysis: impl Into<String>,
    regime_bayes_analysis: impl Into<String>,
    trade_plan: impl Into<String>,
) -> HumanAnalyzeReport {
    HumanAnalyzeReport {
        execution_triage_line: None,
        summary_line: summary_line.map(|line| humanize_term(&line)),
        decision_line: decision_line.map(|line| humanize_term(&line)),
        action_line: action_line.map(|line| humanize_term(&line)),
        next_action_line: next_action_line.map(|line| humanize_term(&line)),
        basic_price_structure_analysis: humanize_term(&basic_price_structure_analysis.into()),
        technical_price_analysis: humanize_term(&technical_price_analysis.into()),
        smt_correlation_analysis: humanize_term(&smt_correlation_analysis.into()),
        regime_bayes_analysis: humanize_term(&regime_bayes_analysis.into()),
        trade_plan: humanize_term(&trade_plan.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn humanize_next_step_line_converts_ask_user_to_natural_language() {
        let raw = "ask-user: Before using historical data for NQ again, ask the user which dataset to use. recorded_paths=/tmp/a.json, /tmp/b.json | blocked until user_selected_historical_data | then ict-engine factor-research --symbol NQ --data /tmp/a.json --state-dir state";
        let rendered = humanize_next_step_line(raw);
        assert!(!rendered.contains("ask-user:"));
        assert!(rendered.contains("Ask the user"));
        assert!(rendered.contains("Then run:"));
        assert!(rendered.contains("ict-engine factor-research"));
    }

    #[test]
    fn humanize_next_step_line_passes_through_ict_engine_command() {
        let raw = "ict-engine analyze --symbol NQ --state-dir state";
        assert_eq!(humanize_next_step_line(raw), raw);
    }

    #[test]
    fn humanize_next_step_line_handles_empty_and_unavailable() {
        assert_eq!(humanize_next_step_line(""), "(no next step)");
        assert_eq!(
            humanize_next_step_line("recommended_command_unavailable"),
            "(no next step)"
        );
    }

    #[test]
    fn human_report_renders_five_sections() {
        let report = build_human_analyze_report(None, None, None, None, "a", "b", "c", "d", "e");
        let rendered = report.render();
        assert!(rendered.contains("Structure: a"));
        assert!(rendered.contains("Plan: e"));
        assert!(!rendered.contains("Basic price structure"));
    }

    #[test]
    fn human_report_renders_summary_lines_first() {
        let report = build_human_analyze_report(
            Some("NQ | Bull bias | entry=medium | gate=observe_only | quality=0.244".to_string()),
            Some("Decision: Observe only".to_string()),
            Some("Action: TUNE structure_ict".to_string()),
            Some("Next: wait".to_string()),
            "a",
            "b",
            "c",
            "d",
            "e",
        );
        let rendered = report.render();
        assert!(
            rendered.starts_with(
                "NQ | Bull bias | entry=medium | gate=observe_only | quality=0.244\nDecision: Observe only\nAction: TUNE structure_ict\nNext: wait"
            )
        );
    }
}
