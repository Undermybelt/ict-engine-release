use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct BacktestCompareReport {
    pub summary: String,
    pub shrink_comparison_summary: Vec<String>,
    pub improvements: Vec<String>,
    pub regressions: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub oos_quality_delta_surface: Vec<String>,
}

pub fn compare_backtest_results(
    summary: impl Into<String>,
    shrink_comparison_summary: &[String],
    improvements: &[String],
    regressions: &[String],
    recommended_actions: &[String],
    oos_quality_delta_surface: &[String],
) -> BacktestCompareReport {
    BacktestCompareReport {
        summary: summary.into(),
        shrink_comparison_summary: shrink_comparison_summary.to_vec(),
        improvements: improvements.to_vec(),
        regressions: regressions.to_vec(),
        recommended_actions: recommended_actions.to_vec(),
        oos_quality_delta_surface: oos_quality_delta_surface.to_vec(),
    }
}

pub fn build_shrink_on_off_comparison_summary(
    shrink_enabled_quality: f64,
    shrink_disabled_quality: f64,
    shrink_enabled_return: f64,
    shrink_disabled_return: f64,
) -> Vec<String> {
    let quality_delta = shrink_enabled_quality - shrink_disabled_quality;
    let return_delta = shrink_enabled_return - shrink_disabled_return;
    let preference = if quality_delta > 0.0 {
        "shrink_on"
    } else if quality_delta < 0.0 {
        "shrink_off"
    } else {
        "neutral"
    };
    let hard_block = shrink_enabled_return > shrink_disabled_return && quality_delta < 0.0;
    let mut summary = vec![
        format!(
            "shrink_on_quality={:.3} shrink_off_quality={:.3} delta={:+.3}",
            shrink_enabled_quality, shrink_disabled_quality, quality_delta
        ),
        format!(
            "shrink_on_return={:.4} shrink_off_return={:.4} delta={:+.4}",
            shrink_enabled_return, shrink_disabled_return, return_delta
        ),
        format!("shrink_preference={preference}"),
    ];
    if hard_block {
        summary.push("hard_block=return_up_oos_down_shrink".to_string());
    }
    summary
}

pub fn build_oos_quality_delta_surface(
    baseline_oos_quality: f64,
    candidate_oos_quality: f64,
    baseline_trade_count: usize,
    candidate_trade_count: usize,
) -> Vec<String> {
    let quality_delta = candidate_oos_quality - baseline_oos_quality;
    let trade_delta = candidate_trade_count as isize - baseline_trade_count as isize;
    let direction = if quality_delta > 0.0 {
        "improved"
    } else if quality_delta < 0.0 {
        "regressed"
    } else {
        "flat"
    };
    let hard_block = trade_delta < 0 && quality_delta < 0.0;
    let mut surface = vec![
        format!(
            "oos_quality_baseline={:.3} oos_quality_candidate={:.3} oos_quality_delta={:+.3}",
            baseline_oos_quality, candidate_oos_quality, quality_delta
        ),
        format!(
            "oos_trade_count_baseline={} oos_trade_count_candidate={} oos_trade_count_delta={:+}",
            baseline_trade_count, candidate_trade_count, trade_delta
        ),
        format!("oos_quality_direction={direction}"),
    ];
    if hard_block {
        surface.push("hard_block=oos_down_shrink".to_string());
    }
    surface
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compare_report_keeps_regressions() {
        let report = compare_backtest_results(
            "summary",
            &["shrink=on".to_string()],
            &["up".to_string()],
            &["down".to_string()],
            &["next".to_string()],
            &["delta=+0.10".to_string()],
        );
        assert_eq!(report.regressions, vec!["down".to_string()]);
        assert_eq!(
            report.shrink_comparison_summary,
            vec!["shrink=on".to_string()]
        );
        assert_eq!(
            report.oos_quality_delta_surface,
            vec!["delta=+0.10".to_string()]
        );
    }

    #[test]
    fn shrink_on_off_summary_surfaces_preference_and_deltas() {
        let summary = build_shrink_on_off_comparison_summary(0.72, 0.61, 0.084, 0.065);

        assert_eq!(summary.len(), 3);
        assert!(summary[0].contains("delta=+0.110"));
        assert!(summary[1].contains("delta=+0.0190"));
        assert_eq!(summary[2], "shrink_preference=shrink_on");
    }

    #[test]
    fn shrink_on_off_summary_marks_return_up_oos_down_hard_block() {
        let summary = build_shrink_on_off_comparison_summary(0.54, 0.61, 0.084, 0.065);

        assert_eq!(summary.len(), 4);
        assert_eq!(summary[2], "shrink_preference=shrink_off");
        assert_eq!(summary[3], "hard_block=return_up_oos_down_shrink");
    }

    #[test]
    fn oos_quality_delta_surface_marks_regression() {
        let surface = build_oos_quality_delta_surface(0.58, 0.51, 18, 11);

        assert_eq!(surface.len(), 4);
        assert!(surface[0].contains("oos_quality_delta=-0.070"));
        assert!(surface[1].contains("oos_trade_count_delta=-7"));
        assert_eq!(surface[2], "oos_quality_direction=regressed");
        assert_eq!(surface[3], "hard_block=oos_down_shrink");
    }
}
