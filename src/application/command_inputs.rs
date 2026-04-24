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
    pub spot_kind: Option<&'a str>,
    pub futures_backend: &'a str,
    pub aux_backend: &'a str,
    pub futures_base_url: &'a str,
    pub aux_base_url: &'a str,
    pub state_dir: &'a str,
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
            spot_kind: Some("etf"),
            futures_backend: "openalice",
            aux_backend: "openalice",
            futures_base_url: "http://futures.local",
            aux_base_url: "http://aux.local",
            state_dir: "/tmp/state",
        };

        assert_eq!(input.symbol, "CL");
        assert_eq!(input.futures_symbol, Some("CL=F"));
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
        };

        assert_eq!(input.symbol, "NQ");
        assert_eq!(input.section, Some("policy"));
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
}
