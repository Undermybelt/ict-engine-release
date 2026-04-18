use super::glossary_map::humanize_term;

#[derive(Debug, Clone, Default)]
pub struct HumanAnalyzeReport {
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
        sections.extend([
            format!(
                "Basic price structure\n{}",
                self.basic_price_structure_analysis
            ),
            format!("Technical price\n{}", self.technical_price_analysis),
            format!("SMT correlation\n{}", self.smt_correlation_analysis),
            format!("Regime + Bayesian view\n{}", self.regime_bayes_analysis),
            format!("Trade plan\n{}", self.trade_plan),
        ]);
        sections.join("\n\n")
    }
}

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
    fn human_report_renders_five_sections() {
        let report = build_human_analyze_report(None, None, None, None, "a", "b", "c", "d", "e");
        let rendered = report.render();
        assert!(rendered.contains("Basic price structure"));
        assert!(rendered.contains("Trade plan"));
    }

    #[test]
    fn human_report_renders_summary_lines_first() {
        let report = build_human_analyze_report(
            Some(
                "NQ | Bull bias | Entry: medium | Gate: observe_only | Quality: 0.244".to_string(),
            ),
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
                "NQ | Bull bias | Entry: medium | Gate: observe_only | Quality: 0.244\n\nDecision: Observe only\n\nAction: TUNE structure_ict\n\nNext: wait"
            )
        );
    }
}
