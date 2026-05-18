use super::*;

pub(crate) fn research_verdict_shell(symbol: &str, state_dir: &str) -> Result<()> {
    ict_engine::application::release_closure::research_verdict_command(symbol, state_dir)
}

pub(crate) fn evidence_quality_breakdown_shell(
    symbol: &str,
    state_dir: &str,
    refresh: bool,
) -> Result<()> {
    ict_engine::application::release_closure::evidence_quality_breakdown_command(
        symbol, state_dir, refresh,
    )
}
