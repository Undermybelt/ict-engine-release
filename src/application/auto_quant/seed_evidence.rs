use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::handoff::{
    auto_quant_active_strategy_count, auto_quant_run_command, AutoQuantWorkspaceConfig,
};
use super::strategy_materials::{discover_strategy_materials, AutoQuantStrategyMaterialSummary};
use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, save_state, ArtifactLedgerEntry,
};

pub const AUTO_QUANT_SEED_MATERIAL_EVIDENCE_REVIEW_RULE_VERSION: &str =
    "auto-quant-seed-material-evidence-v1";
pub const AUTO_QUANT_SEED_MATERIAL_EVIDENCE_DEFAULT_LIMIT: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoQuantSeedMaterialPacket {
    pub summary: AutoQuantStrategyMaterialSummary,
    pub evidence_strength_score: f64,
    pub inferred_tags: Vec<String>,
    pub seed_strategy_name: String,
    pub seed_strategy_path: String,
    pub authoring_focus: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub strategy_excerpt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_excerpt: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub materialized_strategy_path: Option<String>,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub materialization_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoQuantSeedMaterialEvidenceArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub state_dir: String,
    pub strategy_material_root: String,
    pub workspace_repo_root: String,
    pub strategies_dir: String,
    pub strategy_template_path: String,
    pub artifact_path: String,
    pub active_strategy_count: usize,
    pub selected_materials: Vec<AutoQuantSeedMaterialPacket>,
    pub suggested_next_steps: Vec<String>,
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub materialized_strategy_paths: Vec<String>,
}

pub fn persist_auto_quant_seed_material_evidence(
    symbol: &str,
    state_dir: &str,
    strategy_material_root: Option<&str>,
    workspace: &AutoQuantWorkspaceConfig,
    limit: usize,
) -> Result<Option<AutoQuantSeedMaterialEvidenceArtifact>> {
    let Some(root) = strategy_material_root
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let selected_materials = discover_strategy_materials(Some(root), limit);
    if selected_materials.is_empty() {
        return Ok(None);
    }

    let generated_at = Utc::now();
    let artifact_id = format!(
        "auto-quant-seed-material-evidence:{}:{}",
        symbol,
        generated_at.format("%Y%m%dT%H%M%S%.3fZ")
    );
    let filename = format!(
        "auto_quant_seed_material_evidence.{}.json",
        generated_at.format("%Y%m%dT%H%M%S%.3fZ")
    );
    let artifact_path = artifact_state_path(state_dir, symbol, &filename);
    let root_path = Path::new(root);
    let active_strategy_count = auto_quant_active_strategy_count(workspace);
    let strategy_template_path = PathBuf::from(&workspace.strategies_dir)
        .join("_template.py.example")
        .to_string_lossy()
        .to_string();
    let mut packets = selected_materials
        .into_iter()
        .enumerate()
        .map(|(index, material)| build_seed_material_packet(root_path, workspace, &material, index))
        .collect::<Vec<_>>();
    let materialized_strategy_paths =
        materialize_seed_strategy_files(workspace, &artifact_id, &generated_at, root, &mut packets);
    let suggested_next_steps = build_suggested_next_steps(workspace, active_strategy_count);
    let notes = build_notes(root, active_strategy_count);
    let artifact = AutoQuantSeedMaterialEvidenceArtifact {
        artifact_id: artifact_id.clone(),
        generated_at,
        symbol: symbol.to_string(),
        state_dir: state_dir.to_string(),
        strategy_material_root: root.to_string(),
        workspace_repo_root: workspace.repo_root.clone(),
        strategies_dir: workspace.strategies_dir.clone(),
        strategy_template_path,
        artifact_path: artifact_path.clone(),
        active_strategy_count,
        selected_materials: packets,
        suggested_next_steps,
        notes,
        materialized_strategy_paths,
    };

    save_state(state_dir, symbol, &filename, &artifact)?;
    append_artifact_ledger_entry(
        state_dir,
        symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "auto_quant_seed_material_evidence".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: 1,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: "auto_quant_seed_materials".to_string(),
            source_run_id: None,
            path: artifact_path,
            status: "seed_materials_ready".to_string(),
            promote_candidate: false,
            actionable: true,
            decision_hint: "author_auto_quant_seed_strategies".to_string(),
            review_reason: artifact.suggested_next_steps.join(" | "),
            review_rule_version: AUTO_QUANT_SEED_MATERIAL_EVIDENCE_REVIEW_RULE_VERSION.to_string(),
            top_factor_name: artifact
                .selected_materials
                .first()
                .map(|material| material.summary.name.clone()),
            top_factor_action: Some("seed".to_string()),
            family_scores: artifact
                .selected_materials
                .iter()
                .map(|material| {
                    (
                        material.seed_strategy_name.clone(),
                        material.evidence_strength_score,
                    )
                })
                .collect::<BTreeMap<_, _>>(),
            supersedes_artifact_id: None,
            quality_score: artifact_quality_score(&artifact.selected_materials),
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;

    Ok(Some(artifact))
}

fn build_seed_material_packet(
    root: &Path,
    workspace: &AutoQuantWorkspaceConfig,
    material: &AutoQuantStrategyMaterialSummary,
    index: usize,
) -> AutoQuantSeedMaterialPacket {
    let seed_strategy_name = suggested_seed_strategy_name(&material.name, index);
    let seed_strategy_path = PathBuf::from(&workspace.strategies_dir)
        .join(format!("{seed_strategy_name}.py"))
        .to_string_lossy()
        .to_string();
    let strategy_excerpt = read_excerpt(root, &material.strategy_path, 28, 1800);
    let evidence_excerpt = material
        .evidence_csv_path
        .as_deref()
        .and_then(|path| read_excerpt(root, path, 12, 1200));
    let inferred_tags = infer_material_tags(&material.name, strategy_excerpt.as_deref());
    let authoring_focus = build_authoring_focus(material, &inferred_tags);

    AutoQuantSeedMaterialPacket {
        summary: material.clone(),
        evidence_strength_score: material_evidence_strength_score(material),
        inferred_tags,
        seed_strategy_name,
        seed_strategy_path,
        authoring_focus,
        strategy_excerpt,
        evidence_excerpt,
        materialized_strategy_path: None,
        materialization_status: String::new(),
    }
}

fn build_suggested_next_steps(
    workspace: &AutoQuantWorkspaceConfig,
    active_strategy_count: usize,
) -> Vec<String> {
    if active_strategy_count == 0 {
        vec![
            format!(
                "read {} and the strategy template before authoring any seed strategy descendants",
                workspace.program_md
            ),
            format!(
                "author 2-3 active non-underscore strategy files under {} from these seed packets before any run.py execution",
                workspace.strategies_dir
            ),
            format!(
                "run {} only after the seed descendants are written and reviewed",
                auto_quant_run_command(workspace)
            ),
        ]
    } else {
        vec![
            "use these seed packets as evidence for the next strategy fork or replacement wave rather than as direct runtime dependencies".to_string(),
            format!(
                "keep or discard descendants only from measured Auto-Quant results, then re-run {}",
                auto_quant_run_command(workspace)
            ),
        ]
    }
}

fn build_notes(root: &str, active_strategy_count: usize) -> Vec<String> {
    let mut notes = vec![
        format!("external_seed_material_root={root}"),
        "seed_materials_are_read_only_reference_inputs".to_string(),
        "do_not_execute_or_copy_external_scripts_directly_into_the_managed_workspace".to_string(),
        "re-express_external_logic_as_auto_quant_native_strategy_code_before_any_run".to_string(),
    ];
    if active_strategy_count == 0 {
        notes.push(
            "workspace_currently_has_no_active_strategies_so_seed_authoring_is_required"
                .to_string(),
        );
    }
    notes
}

fn material_evidence_strength_score(material: &AutoQuantStrategyMaterialSummary) -> f64 {
    let mut score: f64 = 10.0;
    if material.evidence_csv_path.is_some() {
        score += 25.0;
    }
    score += match material.trade_rows {
        0 => 0.0,
        1..=99 => 10.0,
        _ => 20.0,
    };
    if material.total_net_pnl.unwrap_or_default() > 0.0 {
        score += 15.0;
    }
    if material.average_score.is_some() {
        score += 15.0;
    }
    if material.tp_count > material.sl_count {
        score += 10.0;
    }
    score.min(95.0)
}

fn artifact_quality_score(materials: &[AutoQuantSeedMaterialPacket]) -> i32 {
    if materials.is_empty() {
        return 10;
    }
    let average = materials
        .iter()
        .map(|material| material.evidence_strength_score)
        .sum::<f64>()
        / materials.len() as f64;
    average.round() as i32
}

fn build_authoring_focus(
    material: &AutoQuantStrategyMaterialSummary,
    inferred_tags: &[String],
) -> Vec<String> {
    let mut focus = vec![
        "re-express only the underlying trading idea as Auto-Quant-native strategy logic; never copy maintainer-local paths or execute the external script directly".to_string(),
    ];
    if !inferred_tags.is_empty() {
        focus.push(format!(
            "preserve the strongest visible ideas from this material: {}",
            inferred_tags.join(", ")
        ));
    }
    if material.trade_rows > 0 {
        let mut evidence = format!(
            "measured evidence exists with {} trade rows",
            material.trade_rows
        );
        if let Some(total_net_pnl) = material.total_net_pnl {
            evidence.push_str(&format!(", total net pnl {total_net_pnl:.2}"));
        }
        focus.push(evidence);
    } else {
        focus.push(
            "no matched csv evidence was found, so treat this as weak inspiration and validate aggressively"
                .to_string(),
        );
    }
    if let Some(average_score) = material.average_score {
        focus.push(format!(
            "csv evidence includes score-like signal with average {:.2}; keep only the structural idea if backtests confirm it",
            average_score
        ));
    }
    if material.be_count > 0 {
        focus.push(
            "the external evidence shows explicit break-even management; keep it only if it survives Auto-Quant backtests"
                .to_string(),
        );
    }
    focus
}

fn infer_material_tags(name: &str, strategy_excerpt: Option<&str>) -> Vec<String> {
    let mut haystack = name.to_ascii_lowercase();
    if let Some(excerpt) = strategy_excerpt {
        haystack.push(' ');
        haystack.push_str(&excerpt.to_ascii_lowercase());
    }
    let mut tags = Vec::new();
    for (needle, tag) in [
        ("ict", "ict-structure"),
        ("breakout", "breakout"),
        ("trend", "trend-following"),
        ("momentum", "momentum"),
        ("vol", "volatility"),
        ("squeeze", "volatility-squeeze"),
        ("bb", "bollinger-bands"),
        ("ema", "ema-reclaim"),
        ("rsi", "rsi-gating"),
        ("kill zone", "session-timing"),
        ("killzone", "session-timing"),
        ("be", "break-even-management"),
        ("bos", "structure-break"),
    ] {
        if haystack.contains(needle) && !tags.iter().any(|existing| existing == tag) {
            tags.push(tag.to_string());
        }
    }
    tags
}

fn suggested_seed_strategy_name(name: &str, index: usize) -> String {
    let mut parts = name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .take(6)
        .map(to_seed_token)
        .collect::<Vec<_>>();
    if parts.is_empty() {
        parts.push(format!("Material{}", index + 1));
    }
    format!("Tomac{}Seed", parts.join(""))
}

fn to_seed_token(part: &str) -> String {
    let lower = part.to_ascii_lowercase();
    if lower.chars().all(|ch| ch.is_ascii_digit()) {
        return format!("N{lower}");
    }
    let mut chars = lower.chars();
    let Some(first) = chars.next() else {
        return String::new();
    };
    let mut token = String::new();
    if first.is_ascii_digit() {
        token.push('N');
    }
    token.push(first.to_ascii_uppercase());
    token.push_str(chars.as_str());
    token
}

fn read_excerpt(
    root: &Path,
    relative_path: &str,
    max_lines: usize,
    max_chars: usize,
) -> Option<String> {
    let content = std::fs::read_to_string(root.join(relative_path)).ok()?;
    let excerpt = content
        .lines()
        .take(max_lines)
        .collect::<Vec<_>>()
        .join("\n");
    if excerpt.is_empty() {
        return None;
    }
    Some(truncate_chars(&excerpt, max_chars))
}

fn truncate_chars(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }
    value.chars().take(max_chars).collect::<String>()
}

fn materialize_seed_strategy_files(
    workspace: &AutoQuantWorkspaceConfig,
    artifact_id: &str,
    generated_at: &DateTime<Utc>,
    strategy_material_root: &str,
    packets: &mut [AutoQuantSeedMaterialPacket],
) -> Vec<String> {
    let strategies_dir = Path::new(&workspace.strategies_dir);
    if let Err(_err) = std::fs::create_dir_all(strategies_dir) {
        for packet in packets.iter_mut() {
            packet.materialization_status = "skipped_no_strategies_dir".to_string();
        }
        return Vec::new();
    }
    let mut written = Vec::new();
    for packet in packets.iter_mut() {
        let target_path = strategies_dir.join(format!("{}.py", packet.seed_strategy_name));
        let target_path_str = target_path.to_string_lossy().to_string();
        if target_path.exists() {
            packet.materialized_strategy_path = Some(target_path_str.clone());
            packet.materialization_status = "skipped_existing".to_string();
            continue;
        }
        let body = render_seed_strategy_scaffold(
            artifact_id,
            generated_at,
            strategy_material_root,
            packet,
        );
        match std::fs::write(&target_path, body) {
            Ok(()) => {
                packet.materialized_strategy_path = Some(target_path_str.clone());
                packet.materialization_status = "materialized".to_string();
                written.push(target_path_str);
            }
            Err(_err) => {
                packet.materialization_status = "skipped_write_failed".to_string();
            }
        }
    }
    written
}

fn render_seed_strategy_scaffold(
    artifact_id: &str,
    generated_at: &DateTime<Utc>,
    strategy_material_root: &str,
    packet: &AutoQuantSeedMaterialPacket,
) -> String {
    let class_name = packet.seed_strategy_name.as_str();
    let source_material = packet.summary.strategy_path.as_str();
    let evidence_csv = packet
        .summary
        .evidence_csv_path
        .as_deref()
        .unwrap_or("(none)");
    let inferred_tags = if packet.inferred_tags.is_empty() {
        "(none-inferred)".to_string()
    } else {
        packet.inferred_tags.join(", ")
    };
    let timestamp = generated_at.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let mut focus_block = String::new();
    for line in &packet.authoring_focus {
        focus_block.push_str("# - ");
        focus_block.push_str(line);
        focus_block.push('\n');
    }
    let strategy_excerpt = sanitize_docstring_excerpt(packet.strategy_excerpt.as_deref());
    let evidence_excerpt = sanitize_docstring_excerpt(packet.evidence_excerpt.as_deref());

    let template = r#"# AUTO-GENERATED SEED SCAFFOLD - DO NOT REMOVE THIS HEADER
# auto_quant_seed_material_evidence_artifact_id: {ARTIFACT_ID}
# source_material: {SOURCE_MATERIAL}
# source_evidence_csv: {EVIDENCE_CSV}
# source_root: {SOURCE_ROOT}
# generated_at: {GENERATED_AT}
# inferred_tags: {INFERRED_TAGS}
# iteration_focus:
{FOCUS_BLOCK}#
# This file is a derived Auto-Quant-native scaffold. It does NOT import or execute
# any external Tomac code; the embedded excerpts below are reference docstrings
# bundled solely so Auto-Quant's iteration loop can re-derive measured signals
# without ever leaving the managed workspace.
from __future__ import annotations

import operator
from functools import reduce

import talib.abstract as ta
from freqtrade.strategy import BooleanParameter, IStrategy, IntParameter
from pandas import DataFrame


class {CLASS_NAME}(IStrategy):
    """Hyperopt-ready seed scaffold derived from external strategy material.

    Auto-Quant iteration is expected to mutate the parameters declared below and
    backtest the result; the docstring excerpts are provenance-only reference
    bundled into the file and never imported or executed.

    --- source_strategy_excerpt ---
    {STRATEGY_EXCERPT}
    --- end source_strategy_excerpt ---

    --- source_evidence_excerpt ---
    {EVIDENCE_EXCERPT}
    --- end source_evidence_excerpt ---
    """

    INTERFACE_VERSION = 3
    timeframe = "5m"
    can_short = False
    process_only_new_candles = True
    use_exit_signal = True
    exit_profit_only = False
    ignore_roi_if_entry_signal = False

    minimal_roi = {"0": 0.05}
    stoploss = -0.05
    trailing_stop = False
    startup_candle_count = 200

    # --- Auto-Quant iteration surface (hyperopt-able) ---
    ema_fast_period = IntParameter(8, 50, default=21, space="buy", optimize=True)
    ema_slow_period = IntParameter(40, 200, default=89, space="buy", optimize=True)
    rsi_period = IntParameter(7, 21, default=14, space="buy", optimize=True)
    rsi_buy_threshold = IntParameter(20, 60, default=35, space="buy", optimize=True)
    rsi_sell_threshold = IntParameter(50, 90, default=70, space="sell", optimize=True)
    atr_period = IntParameter(7, 30, default=14, space="buy", optimize=True)
    use_ema_trend_filter = BooleanParameter(default=True, space="buy", optimize=True)
    use_rsi_entry_filter = BooleanParameter(default=True, space="buy", optimize=True)
    use_rsi_exit_filter = BooleanParameter(default=True, space="sell", optimize=True)

    def populate_indicators(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        dataframe["ema_fast"] = ta.EMA(dataframe, timeperiod=self.ema_fast_period.value)
        dataframe["ema_slow"] = ta.EMA(dataframe, timeperiod=self.ema_slow_period.value)
        dataframe["rsi"] = ta.RSI(dataframe, timeperiod=self.rsi_period.value)
        dataframe["atr"] = ta.ATR(dataframe, timeperiod=self.atr_period.value)
        return dataframe

    def populate_entry_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        conditions = []
        if self.use_ema_trend_filter.value:
            conditions.append(dataframe["ema_fast"] > dataframe["ema_slow"])
        if self.use_rsi_entry_filter.value:
            conditions.append(dataframe["rsi"] > self.rsi_buy_threshold.value)
        if conditions:
            dataframe.loc[reduce(operator.and_, conditions), "enter_long"] = 1
        else:
            dataframe.loc[:, "enter_long"] = 0
        return dataframe

    def populate_exit_trend(self, dataframe: DataFrame, metadata: dict) -> DataFrame:
        if self.use_rsi_exit_filter.value:
            dataframe.loc[
                dataframe["rsi"] > self.rsi_sell_threshold.value, "exit_long"
            ] = 1
        else:
            dataframe.loc[:, "exit_long"] = 0
        return dataframe
"#;
    template
        .replace("{ARTIFACT_ID}", artifact_id)
        .replace("{SOURCE_MATERIAL}", source_material)
        .replace("{EVIDENCE_CSV}", evidence_csv)
        .replace("{SOURCE_ROOT}", strategy_material_root)
        .replace("{GENERATED_AT}", &timestamp)
        .replace("{INFERRED_TAGS}", &inferred_tags)
        .replace("{FOCUS_BLOCK}", &focus_block)
        .replace("{STRATEGY_EXCERPT}", &strategy_excerpt)
        .replace("{EVIDENCE_EXCERPT}", &evidence_excerpt)
        .replace("{CLASS_NAME}", class_name)
}

fn sanitize_docstring_excerpt(excerpt: Option<&str>) -> String {
    let raw = excerpt.unwrap_or("(no excerpt available)");
    raw.replace("\"\"\"", "'''")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::auto_quant::handoff::auto_quant_workspace_config;
    use crate::state::ARTIFACT_LEDGER_FILE;
    use std::path::Path;

    fn init_workspace(path: &Path) {
        std::fs::create_dir_all(path.join("user_data/strategies")).unwrap();
        std::fs::write(
            path.join("user_data/strategies/_template.py.example"),
            "class Template: pass",
        )
        .unwrap();
        std::fs::write(path.join("program.md"), "program").unwrap();
        std::fs::write(path.join("run.py"), "print('run')").unwrap();
    }

    #[test]
    fn persist_seed_material_evidence_writes_artifact_and_ledger_entry() {
        let state = tempfile::tempdir().unwrap();
        let workspace_dir = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        init_workspace(workspace_dir.path());
        std::fs::write(
            root.path().join("ultimate_ict_strategy.py"),
            "# ULTIMATE ICT\nEMA = 21\n",
        )
        .unwrap();
        std::fs::write(
            root.path().join("ultimate_ict_results.csv"),
            "Time,Net PnL,Result,Score\n2024-01-01,10,TP,5\n2024-01-02,-2,SL,3\n",
        )
        .unwrap();

        let workspace = auto_quant_workspace_config(workspace_dir.path().to_str().unwrap());
        let artifact = persist_auto_quant_seed_material_evidence(
            "NQ",
            state.path().to_str().unwrap(),
            Some(root.path().to_str().unwrap()),
            &workspace,
            3,
        )
        .unwrap()
        .unwrap();

        assert!(Path::new(&artifact.artifact_path).exists());
        assert_eq!(artifact.selected_materials.len(), 1);
        assert!(artifact
            .selected_materials
            .first()
            .unwrap()
            .seed_strategy_path
            .ends_with(".py"));
        assert!(artifact
            .selected_materials
            .first()
            .unwrap()
            .strategy_excerpt
            .as_deref()
            .unwrap()
            .contains("ULTIMATE ICT"));
        assert!(artifact.notes.iter().any(|note| {
            note == "do_not_execute_or_copy_external_scripts_directly_into_the_managed_workspace"
        }));

        let ledger =
            std::fs::read_to_string(state.path().join("NQ").join(ARTIFACT_LEDGER_FILE)).unwrap();
        assert!(ledger.contains("auto_quant_seed_material_evidence"));
        assert!(ledger.contains(AUTO_QUANT_SEED_MATERIAL_EVIDENCE_REVIEW_RULE_VERSION));

        let packet = artifact.selected_materials.first().unwrap();
        let materialized_path = packet
            .materialized_strategy_path
            .as_deref()
            .expect("materialized strategy path");
        assert_eq!(packet.materialization_status, "materialized");
        assert!(artifact
            .materialized_strategy_paths
            .iter()
            .any(|path| path == materialized_path));
        let scaffold = std::fs::read_to_string(materialized_path).unwrap();
        assert!(scaffold.contains("AUTO-GENERATED SEED SCAFFOLD"));
        assert!(scaffold.contains(&format!(
            "auto_quant_seed_material_evidence_artifact_id: {}",
            artifact.artifact_id
        )));
        assert!(scaffold.contains("from freqtrade.strategy import"));
        assert!(scaffold.contains("IStrategy"));
        assert!(scaffold.contains(&format!("class {}(IStrategy)", packet.seed_strategy_name)));
        assert!(scaffold.contains("IntParameter("));
        assert!(scaffold.contains("BooleanParameter("));
        assert!(scaffold.contains("ema_fast_period"));
        assert!(scaffold.contains("rsi_buy_threshold"));
        assert!(scaffold.contains("\"enter_long\"] = 1"));
        assert!(scaffold.contains("ULTIMATE ICT"));
        assert!(!scaffold.contains("import tomac"));
        assert!(!scaffold.contains("from tomac"));
    }

    #[test]
    fn persist_seed_material_evidence_is_idempotent_for_existing_scaffolds() {
        let state = tempfile::tempdir().unwrap();
        let workspace_dir = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        init_workspace(workspace_dir.path());
        std::fs::write(
            root.path().join("ultimate_ict_strategy.py"),
            "# ULTIMATE ICT\nEMA = 21\n",
        )
        .unwrap();

        let workspace = auto_quant_workspace_config(workspace_dir.path().to_str().unwrap());
        let first = persist_auto_quant_seed_material_evidence(
            "NQ",
            state.path().to_str().unwrap(),
            Some(root.path().to_str().unwrap()),
            &workspace,
            3,
        )
        .unwrap()
        .unwrap();
        let scaffold_path = first
            .selected_materials
            .first()
            .unwrap()
            .materialized_strategy_path
            .clone()
            .unwrap();
        std::fs::write(&scaffold_path, "# user-edited body\n").unwrap();

        let second = persist_auto_quant_seed_material_evidence(
            "NQ",
            state.path().to_str().unwrap(),
            Some(root.path().to_str().unwrap()),
            &workspace,
            3,
        )
        .unwrap()
        .unwrap();
        let packet = second.selected_materials.first().unwrap();
        assert_eq!(packet.materialization_status, "skipped_existing");
        assert!(second.materialized_strategy_paths.is_empty());
        let preserved = std::fs::read_to_string(&scaffold_path).unwrap();
        assert_eq!(preserved, "# user-edited body\n");
    }

    #[test]
    fn persist_seed_material_evidence_returns_none_when_no_materials_exist() {
        let state = tempfile::tempdir().unwrap();
        let workspace_dir = tempfile::tempdir().unwrap();
        let root = tempfile::tempdir().unwrap();
        init_workspace(workspace_dir.path());

        let workspace = auto_quant_workspace_config(workspace_dir.path().to_str().unwrap());
        let artifact = persist_auto_quant_seed_material_evidence(
            "NQ",
            state.path().to_str().unwrap(),
            Some(root.path().to_str().unwrap()),
            &workspace,
            3,
        )
        .unwrap();

        assert!(artifact.is_none());
    }
}
