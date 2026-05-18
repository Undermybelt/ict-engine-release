#[derive(Debug, Clone, Copy)]
pub struct AnalyzeCommandInput<'a> {
    pub symbol: &'a str,
    pub data_htf: &'a str,
    pub data_mtf: &'a str,
    pub data_ltf: &'a str,
    pub state_dir: &'a str,
    pub output_format: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct AnalyzeLiveCommandInput<'a> {
    pub symbol: &'a str,
    pub futures_symbol: Option<&'a str>,
    pub spot_symbol: Option<&'a str>,
    pub options_symbol: Option<&'a str>,
    pub options_volatility_proxy_symbol: Option<&'a str>,
    pub spot_kind: Option<&'a str>,
    pub futures_backend: &'a str,
    pub aux_backend: &'a str,
    pub futures_base_url: &'a str,
    pub aux_base_url: &'a str,
    pub state_dir: &'a str,
    pub output_format: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct WorkflowStatusCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub refresh: bool,
    pub phase: Option<&'a str>,
    pub actionable_only: bool,
    pub conflicts_only: bool,
    pub latest_promotable: bool,
    pub hard_block_only: bool,
    pub hard_block_reason: Option<&'a str>,
    pub limit: Option<usize>,
    pub output_format: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct PreBayesStatusCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub refresh: bool,
    pub section: Option<&'a str>,
    pub output_format: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct PreBayesDiffCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub refresh: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ArtifactStatusCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub artifact_id: Option<&'a str>,
    pub kind: Option<&'a str>,
    pub latest_only: bool,
    pub actionable_only: bool,
    pub rule_break_only: bool,
    pub sort_by: &'a str,
    pub descending: bool,
    pub limit: Option<usize>,
    pub recent_n: Option<usize>,
    pub consumed_only: bool,
    pub bucket_by_kind: bool,
    pub bucket_order_by: &'a str,
    pub bucket_limit: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
pub struct ArtifactDiffCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub left_artifact_id: &'a str,
    pub right_artifact_id: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct AutoQuantStatusCommandInput<'a> {
    pub state_dir: &'a str,
}

#[derive(Debug, Clone, Copy)]
pub struct AutoQuantBootstrapCommandInput<'a> {
    pub state_dir: &'a str,
    pub repo_url: Option<&'a str>,
    pub tracked_branch: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct AutoQuantUpdateCommandInput<'a> {
    pub state_dir: &'a str,
    pub repo_url: Option<&'a str>,
    pub tracked_branch: Option<&'a str>,
    pub target_ref: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct AutoQuantPdaUnitBatchCommandInput<'a> {
    pub symbol: &'a str,
    pub objective: &'a str,
    pub factors: &'a str,
    pub combination_size: usize,
    pub directions: &'a str,
    pub timeframes: &'a str,
    pub timeframe_data: &'a [String],
    pub evidence_surfaces: &'a str,
    pub indicator_list: &'a str,
    pub evidence_notes: &'a [String],
    pub max_parallel: usize,
    pub state_dir: &'a str,
    pub repo_url: Option<&'a str>,
    pub tracked_branch: Option<&'a str>,
}

#[derive(Debug, Clone)]
pub struct AutoQuantAgentMaterialBatchCommandInput<'a> {
    pub symbol: &'a str,
    pub material_paths: &'a [String],
    pub max_parallel: usize,
    pub state_dir: &'a str,
    pub repo_url: Option<&'a str>,
    pub tracked_branch: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct AutoQuantAgentMaterialDispatchCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
    pub group_indices: Option<&'a str>,
}

#[derive(Debug, Clone, Copy)]
pub struct AutoQuantAgentMaterialRankCommandInput<'a> {
    pub symbol: &'a str,
    pub state_dir: &'a str,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyze_command_input_carries_paths_and_output_format() {
        let input = AnalyzeCommandInput {
            symbol: "NQ",
            data_htf: "/tmp/htf.json",
            data_mtf: "/tmp/mtf.json",
            data_ltf: "/tmp/ltf.json",
            state_dir: "/tmp/state",
            output_format: "agent",
        };

        assert_eq!(input.symbol, "NQ");
        assert_eq!(input.data_htf, "/tmp/htf.json");
        assert_eq!(input.output_format, "agent");
    }

    #[test]
    fn analyze_live_command_input_carries_backend_and_symbols() {
        let input = AnalyzeLiveCommandInput {
            symbol: "CL",
            futures_symbol: Some("CL=F"),
            spot_symbol: Some("USO"),
            options_symbol: Some("USO"),
            options_volatility_proxy_symbol: Some("^OVX"),
            spot_kind: Some("etf"),
            futures_backend: "external_http_runtime",
            aux_backend: "external_http_runtime",
            futures_base_url: "http://futures.local",
            aux_base_url: "http://aux.local",
            state_dir: "/tmp/state",
            output_format: "human",
        };

        assert_eq!(input.symbol, "CL");
        assert_eq!(input.futures_symbol, Some("CL=F"));
        assert_eq!(input.options_volatility_proxy_symbol, Some("^OVX"));
        assert_eq!(input.spot_kind, Some("etf"));
        assert_eq!(input.futures_base_url, "http://futures.local");
    }

    #[test]
    fn workflow_status_command_input_carries_phase_and_format() {
        let input = WorkflowStatusCommandInput {
            symbol: "NQ",
            state_dir: "/tmp/state",
            refresh: false,
            phase: Some("human"),
            actionable_only: false,
            conflicts_only: false,
            latest_promotable: false,
            hard_block_only: false,
            hard_block_reason: None,
            limit: Some(3),
            output_format: "json",
        };

        assert_eq!(input.phase, Some("human"));
        assert_eq!(input.output_format, "json");
        assert_eq!(input.limit, Some(3));
    }

    #[test]
    fn pre_bayes_status_command_input_carries_section() {
        let input = PreBayesStatusCommandInput {
            symbol: "NQ",
            state_dir: "/tmp/state",
            refresh: true,
            section: Some("policy"),
            output_format: "human",
        };

        assert_eq!(input.symbol, "NQ");
        assert_eq!(input.section, Some("policy"));
        assert_eq!(input.output_format, "human");
        assert!(input.refresh);
    }

    #[test]
    fn pre_bayes_diff_command_input_carries_refresh_flag() {
        let input = PreBayesDiffCommandInput {
            symbol: "ES",
            state_dir: "/tmp/state",
            refresh: false,
        };

        assert_eq!(input.symbol, "ES");
        assert_eq!(input.state_dir, "/tmp/state");
        assert!(!input.refresh);
    }

    #[test]
    fn artifact_status_command_input_carries_filters() {
        let input = ArtifactStatusCommandInput {
            symbol: "NQ",
            state_dir: "/tmp/state",
            artifact_id: Some("artifact-1"),
            kind: Some("pending_update"),
            latest_only: true,
            actionable_only: false,
            rule_break_only: true,
            sort_by: "generated",
            descending: true,
            limit: Some(5),
            recent_n: Some(10),
            consumed_only: false,
            bucket_by_kind: true,
            bucket_order_by: "kind",
            bucket_limit: Some(2),
        };

        assert_eq!(input.artifact_id, Some("artifact-1"));
        assert_eq!(input.kind, Some("pending_update"));
        assert!(input.latest_only);
        assert!(input.rule_break_only);
        assert!(input.bucket_by_kind);
    }

    #[test]
    fn artifact_diff_command_input_carries_ids() {
        let input = ArtifactDiffCommandInput {
            symbol: "NQ",
            state_dir: "/tmp/state",
            left_artifact_id: "left-1",
            right_artifact_id: "right-1",
        };

        assert_eq!(input.symbol, "NQ");
        assert_eq!(input.left_artifact_id, "left-1");
        assert_eq!(input.right_artifact_id, "right-1");
    }

    #[test]
    fn auto_quant_status_command_input_carries_state_dir() {
        let input = AutoQuantStatusCommandInput {
            state_dir: "/tmp/state",
        };

        assert_eq!(input.state_dir, "/tmp/state");
    }

    #[test]
    fn auto_quant_bootstrap_command_input_carries_overrides() {
        let input = AutoQuantBootstrapCommandInput {
            state_dir: "/tmp/state",
            repo_url: Some("https://example.com/repo.git"),
            tracked_branch: Some("main"),
        };

        assert_eq!(input.repo_url, Some("https://example.com/repo.git"));
        assert_eq!(input.tracked_branch, Some("main"));
    }

    #[test]
    fn auto_quant_update_command_input_carries_target_ref() {
        let input = AutoQuantUpdateCommandInput {
            state_dir: "/tmp/state",
            repo_url: None,
            tracked_branch: Some("master"),
            target_ref: Some("v0.3.0"),
        };

        assert_eq!(input.tracked_branch, Some("master"));
        assert_eq!(input.target_ref, Some("v0.3.0"));
    }

    #[test]
    fn auto_quant_pda_unit_batch_command_input_carries_explicit_batch_fields() {
        let mappings = vec!["15m=/tmp/nq-15m.json".to_string()];
        let evidence_notes = vec!["need volatility regime confirmation".to_string()];
        let input = AutoQuantPdaUnitBatchCommandInput {
            symbol: "NQ",
            objective: "expansion_manipulation",
            factors: "order_block,fair_value_gap",
            combination_size: 1,
            directions: "long,short",
            timeframes: "15m",
            timeframe_data: &mappings,
            evidence_surfaces: "indicators,volatility,greeks",
            indicator_list: "rsi14,ema20,atr14",
            evidence_notes: &evidence_notes,
            max_parallel: 4,
            state_dir: "/tmp/state",
            repo_url: Some("/tmp/Auto-Quant"),
            tracked_branch: Some("master"),
        };

        assert_eq!(input.symbol, "NQ");
        assert_eq!(input.factors, "order_block,fair_value_gap");
        assert_eq!(input.max_parallel, 4);
        assert_eq!(input.timeframe_data, &mappings);
        assert_eq!(input.evidence_surfaces, "indicators,volatility,greeks");
        assert_eq!(input.repo_url, Some("/tmp/Auto-Quant"));
    }

    #[test]
    fn auto_quant_agent_material_batch_command_input_carries_material_paths() {
        let materials = vec![
            "/tmp/material-1.json".to_string(),
            "/tmp/material-2.json".to_string(),
        ];
        let input = AutoQuantAgentMaterialBatchCommandInput {
            symbol: "NQ",
            material_paths: &materials,
            max_parallel: 3,
            state_dir: "/tmp/state",
            repo_url: Some("/tmp/Auto-Quant"),
            tracked_branch: Some("master"),
        };
        assert_eq!(input.material_paths, &materials);
        assert_eq!(input.max_parallel, 3);
    }
}
