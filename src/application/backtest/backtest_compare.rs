use crate::state::{BacktestRunRecord, ResearchRunRecord};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct BacktestCompareReport {
    pub summary: String,
    pub shrink_comparison_summary: Vec<String>,
    pub duration_sizing_delta_surface: Vec<String>,
    pub improvements: Vec<String>,
    pub regressions: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub oos_quality_delta_surface: Vec<String>,
}

pub fn compare_backtest_results(
    summary: impl Into<String>,
    shrink_comparison_summary: &[String],
    duration_sizing_delta_surface: &[String],
    improvements: &[String],
    regressions: &[String],
    recommended_actions: &[String],
    oos_quality_delta_surface: &[String],
) -> BacktestCompareReport {
    BacktestCompareReport {
        summary: summary.into(),
        shrink_comparison_summary: shrink_comparison_summary.to_vec(),
        duration_sizing_delta_surface: duration_sizing_delta_surface.to_vec(),
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

pub fn build_duration_sizing_delta_surface(
    baseline_position_size: f64,
    candidate_position_size: f64,
    baseline_kelly_fraction: f64,
    candidate_kelly_fraction: f64,
    duration_model: Option<&str>,
    remaining_expected_bars: Option<f64>,
) -> Vec<String> {
    let position_delta = candidate_position_size - baseline_position_size;
    let kelly_delta = candidate_kelly_fraction - baseline_kelly_fraction;
    let direction = if position_delta < 0.0 || kelly_delta < 0.0 {
        "scaled_down"
    } else if position_delta > 0.0 || kelly_delta > 0.0 {
        "scaled_up"
    } else {
        "unchanged"
    };
    let mut surface = vec![
        format!(
            "duration_position_size_baseline={:.4} duration_position_size_candidate={:.4} duration_position_size_delta={:+.4}",
            baseline_position_size, candidate_position_size, position_delta
        ),
        format!(
            "duration_kelly_fraction_baseline={:.4} duration_kelly_fraction_candidate={:.4} duration_kelly_fraction_delta={:+.4}",
            baseline_kelly_fraction, candidate_kelly_fraction, kelly_delta
        ),
        format!("duration_sizing_direction={direction}"),
    ];
    if let Some(model) = duration_model {
        surface.push(format!("duration_model={model}"));
    }
    if let Some(remaining) = remaining_expected_bars {
        surface.push(format!("duration_remaining_expected_bars={remaining:.3}"));
    }
    surface
}

pub fn build_backtest_compare_report(
    previous: &BacktestRunRecord,
    current: &BacktestRunRecord,
) -> Option<BacktestCompareReport> {
    if !current.dataset_comparability.comparable
        || previous.source_command != current.source_command
    {
        return None;
    }

    let coverage_delta = current.conformal_coverage_1sigma - previous.conformal_coverage_1sigma;
    let return_delta = current.total_return - previous.total_return;
    let duration_scale_previous = previous.duration_sizing_scale.unwrap_or(1.0);
    let duration_scale_current = current.duration_sizing_scale.unwrap_or(1.0);
    let duration_delta = duration_scale_current - duration_scale_previous;
    let trade_delta = current.trade_count as isize - previous.trade_count as isize;

    let comparison_summary = vec![
        format!(
            "coverage_1sigma_previous={:.3} coverage_1sigma_current={:.3} delta={:+.3}",
            previous.conformal_coverage_1sigma, current.conformal_coverage_1sigma, coverage_delta
        ),
        format!(
            "total_return_previous={:.4} total_return_current={:.4} delta={:+.4}",
            previous.total_return, current.total_return, return_delta
        ),
        format!(
            "duration_sizing_scale_previous={:.3} duration_sizing_scale_current={:.3} delta={:+.3}",
            duration_scale_previous, duration_scale_current, duration_delta
        ),
    ];

    let mut improvements = Vec::new();
    let mut regressions = Vec::new();
    if coverage_delta > 0.0 {
        improvements.push(format!("coverage_1sigma_delta={:+.3}", coverage_delta));
    } else if coverage_delta < 0.0 {
        regressions.push(format!("coverage_1sigma_delta={:+.3}", coverage_delta));
    }
    if return_delta > 0.0 {
        improvements.push(format!("total_return_delta={:+.4}", return_delta));
    } else if return_delta < 0.0 {
        regressions.push(format!("total_return_delta={:+.4}", return_delta));
    }
    if duration_delta > 0.0 {
        improvements.push(format!(
            "duration_sizing_scale_delta={:+.3}",
            duration_delta
        ));
    } else if duration_delta < 0.0 {
        regressions.push(format!(
            "duration_sizing_scale_delta={:+.3}",
            duration_delta
        ));
    }
    if trade_delta > 0 {
        improvements.push(format!("trade_count_delta=+{}", trade_delta));
    } else if trade_delta < 0 {
        regressions.push(format!("trade_count_delta={}", trade_delta));
    }

    let mut recommended_actions = Vec::new();
    if duration_scale_current < 1.0 {
        recommended_actions
            .push("inspect_duration_constraints_before_promoting_size_change".to_string());
    }
    if coverage_delta < 0.0 && duration_delta < 0.0 {
        recommended_actions
            .push("verify_duration_sizing_is_not_over-constraining_trade_capture".to_string());
    }
    if recommended_actions.is_empty() {
        recommended_actions
            .push("duration_sizing_change_is_comparable_to_previous_run".to_string());
    }

    Some(compare_backtest_results(
        format!(
            "{} compare current={} previous={} class={}",
            current.source_command,
            current.run_id,
            previous.run_id,
            current.dataset_comparability.comparison_class
        ),
        &comparison_summary,
        &build_duration_sizing_delta_surface(
            duration_scale_previous,
            duration_scale_current,
            duration_scale_previous,
            duration_scale_current,
            current
                .hybrid_duration_model
                .as_deref()
                .or(previous.hybrid_duration_model.as_deref()),
            current
                .hybrid_remaining_expected_bars
                .or(previous.hybrid_remaining_expected_bars),
        ),
        &improvements,
        &regressions,
        &recommended_actions,
        &build_oos_quality_delta_surface(
            previous.conformal_coverage_1sigma,
            current.conformal_coverage_1sigma,
            previous.trade_count,
            current.trade_count,
        ),
    ))
}

pub fn build_research_compare_report(
    previous: &ResearchRunRecord,
    current: &ResearchRunRecord,
) -> Option<BacktestCompareReport> {
    if !current.dataset_comparability.comparable
        || previous.source_command != current.source_command
    {
        return None;
    }

    let duration_scale_previous = previous.duration_sizing_scale.unwrap_or(1.0);
    let duration_scale_current = current.duration_sizing_scale.unwrap_or(1.0);
    let duration_delta = duration_scale_current - duration_scale_previous;
    let aggregate_return_delta = current.aggregate_return - previous.aggregate_return;
    let coverage_delta =
        current.backtest_conformal_coverage_1sigma - previous.backtest_conformal_coverage_1sigma;

    Some(compare_backtest_results(
        format!(
            "{} compare current={} previous={} class={}",
            current.source_command,
            current.run_id,
            previous.run_id,
            current.dataset_comparability.comparison_class
        ),
        &build_shrink_on_off_comparison_summary(
            previous.backtest_conformal_coverage_1sigma,
            current.backtest_conformal_coverage_1sigma,
            previous.aggregate_return,
            current.aggregate_return,
        ),
        &build_duration_sizing_delta_surface(
            duration_scale_previous,
            duration_scale_current,
            duration_scale_previous,
            duration_scale_current,
            current
                .hybrid_duration_model
                .as_deref()
                .or(previous.hybrid_duration_model.as_deref()),
            current
                .hybrid_remaining_expected_bars
                .or(previous.hybrid_remaining_expected_bars),
        ),
        &[
            if coverage_delta > 0.0 {
                Some(format!("coverage_1sigma_delta={:+.3}", coverage_delta))
            } else {
                None
            },
            if aggregate_return_delta > 0.0 {
                Some(format!(
                    "aggregate_return_delta={:+.4}",
                    aggregate_return_delta
                ))
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>(),
        &[
            if coverage_delta < 0.0 {
                Some(format!("coverage_1sigma_delta={:+.3}", coverage_delta))
            } else {
                None
            },
            if aggregate_return_delta < 0.0 {
                Some(format!(
                    "aggregate_return_delta={:+.4}",
                    aggregate_return_delta
                ))
            } else {
                None
            },
            if duration_delta < 0.0 {
                Some(format!(
                    "duration_sizing_scale_delta={:+.3}",
                    duration_delta
                ))
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>(),
        &if duration_scale_current < 1.0 {
            vec!["inspect_duration_constraints_before_promoting_research_candidate".to_string()]
        } else {
            vec!["research_duration_sizing_change_is_comparable_to_previous_run".to_string()]
        },
        &build_oos_quality_delta_surface(
            previous.backtest_conformal_coverage_1sigma,
            current.backtest_conformal_coverage_1sigma,
            previous.backtest_trade_count,
            current.backtest_trade_count,
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::DatasetComparability;

    #[test]
    fn compare_report_keeps_regressions() {
        let report = compare_backtest_results(
            "summary",
            &["shrink=on".to_string()],
            &["duration_sizing_direction=scaled_down".to_string()],
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
            report.duration_sizing_delta_surface,
            vec!["duration_sizing_direction=scaled_down".to_string()]
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

    #[test]
    fn duration_sizing_delta_surface_surfaces_model_remaining_and_scale() {
        let surface = build_duration_sizing_delta_surface(
            1.0,
            0.25,
            0.08,
            0.02,
            Some("negative_binomial"),
            Some(2.5),
        );

        assert_eq!(surface.len(), 5);
        assert!(surface[0].contains("duration_position_size_delta=-0.7500"));
        assert!(surface[1].contains("duration_kelly_fraction_delta=-0.0600"));
        assert_eq!(surface[2], "duration_sizing_direction=scaled_down");
        assert_eq!(surface[3], "duration_model=negative_binomial");
        assert_eq!(surface[4], "duration_remaining_expected_bars=2.500");
    }

    #[test]
    fn backtest_compare_report_surfaces_duration_delta_from_runs() {
        let previous = BacktestRunRecord {
            run_id: "backtest:prev".to_string(),
            source_command: "backtest".to_string(),
            total_return: 0.03,
            trade_count: 12,
            conformal_coverage_1sigma: 0.58,
            duration_sizing_scale: Some(1.0),
            hybrid_duration_model: Some("negative_binomial".to_string()),
            hybrid_remaining_expected_bars: Some(4.0),
            ..BacktestRunRecord::default()
        };
        let current = BacktestRunRecord {
            run_id: "backtest:curr".to_string(),
            source_command: "backtest".to_string(),
            total_return: 0.05,
            trade_count: 10,
            conformal_coverage_1sigma: 0.61,
            duration_sizing_scale: Some(0.25),
            hybrid_duration_model: Some("negative_binomial".to_string()),
            hybrid_remaining_expected_bars: Some(2.5),
            dataset_comparability: DatasetComparability {
                comparable: true,
                comparison_class: "same_data_same_config".to_string(),
                ..DatasetComparability::default()
            },
            ..BacktestRunRecord::default()
        };

        let report =
            build_backtest_compare_report(&previous, &current).expect("missing compare report");

        assert!(report.summary.contains("same_data_same_config"));
        assert!(report
            .duration_sizing_delta_surface
            .iter()
            .any(|line| line == "duration_sizing_direction=scaled_down"));
        assert!(report
            .regressions
            .iter()
            .any(|line| line == "duration_sizing_scale_delta=-0.750"));
    }

    #[test]
    fn backtest_compare_report_skips_non_comparable_runs() {
        let previous = BacktestRunRecord {
            source_command: "backtest".to_string(),
            ..BacktestRunRecord::default()
        };
        let current = BacktestRunRecord {
            source_command: "backtest".to_string(),
            dataset_comparability: DatasetComparability {
                comparable: false,
                comparison_class: "different_data_fingerprint".to_string(),
                ..DatasetComparability::default()
            },
            ..BacktestRunRecord::default()
        };

        assert!(build_backtest_compare_report(&previous, &current).is_none());
    }

    #[test]
    fn research_compare_report_surfaces_duration_delta_from_runs() {
        let previous = ResearchRunRecord {
            run_id: "research:prev".to_string(),
            source_command: "factor-research".to_string(),
            aggregate_return: 0.03,
            duration_sizing_scale: Some(1.0),
            hybrid_duration_model: Some("negative_binomial".to_string()),
            hybrid_remaining_expected_bars: Some(4.0),
            backtest_conformal_coverage_1sigma: 0.58,
            backtest_trade_count: 12,
            ..ResearchRunRecord::default()
        };
        let current = ResearchRunRecord {
            run_id: "research:curr".to_string(),
            source_command: "factor-research".to_string(),
            aggregate_return: 0.05,
            duration_sizing_scale: Some(0.25),
            hybrid_duration_model: Some("negative_binomial".to_string()),
            hybrid_remaining_expected_bars: Some(2.5),
            backtest_conformal_coverage_1sigma: 0.61,
            backtest_trade_count: 10,
            dataset_comparability: DatasetComparability {
                comparable: true,
                comparison_class: "same_data_same_config".to_string(),
                ..DatasetComparability::default()
            },
            ..ResearchRunRecord::default()
        };

        let report =
            build_research_compare_report(&previous, &current).expect("missing compare report");

        assert!(report.summary.contains("same_data_same_config"));
        assert!(report
            .duration_sizing_delta_surface
            .iter()
            .any(|line| line == "duration_sizing_direction=scaled_down"));
        assert!(report
            .regressions
            .iter()
            .any(|line| line == "duration_sizing_scale_delta=-0.750"));
    }
}
