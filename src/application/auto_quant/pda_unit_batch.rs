use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::str::FromStr;

use crate::state::{
    append_artifact_ledger_entry, artifact_state_path, save_state, ArtifactLedgerEntry,
};

use super::handoff::{
    build_factor_research_handoff_payload, AutoQuantIterationUnitContext,
    BuildFactorResearchHandoffPayloadInput,
};
use super::persistence::persist_handoff_payload;
use super::types::AutoQuantDependencyStatus;

pub const AUTO_QUANT_PDA_UNIT_BATCH_RULE_VERSION: &str = "auto-quant-pda-unit-batch-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum AutoQuantPdaPrimitiveKind {
    OrderBlock,
    FairValueGap,
    InverseFvg,
    BreakerBlock,
    MitigationBlock,
    RejectionBlock,
    PropulsionBlock,
    LiquidityVoid,
    VolumeImbalance,
    MarketStructureShift,
    Cisd,
}

impl AutoQuantPdaPrimitiveKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OrderBlock => "order_block",
            Self::FairValueGap => "fair_value_gap",
            Self::InverseFvg => "inverse_fvg",
            Self::BreakerBlock => "breaker_block",
            Self::MitigationBlock => "mitigation_block",
            Self::RejectionBlock => "rejection_block",
            Self::PropulsionBlock => "propulsion_block",
            Self::LiquidityVoid => "liquidity_void",
            Self::VolumeImbalance => "volume_imbalance",
            Self::MarketStructureShift => "market_structure_shift",
            Self::Cisd => "cisd",
        }
    }

    pub fn detector_summary(self) -> &'static str {
        match self {
            Self::OrderBlock => {
                "Last opposite-direction candle followed by at least one ATR of displacement."
            }
            Self::FairValueGap => "Three-bar non-overlap (BISI or SIBI).",
            Self::InverseFvg => {
                "Prior fair value gap gets fully traded through, then re-broken in the opposite direction."
            }
            Self::BreakerBlock => "Order block failure that gets revisited as structure.",
            Self::MitigationBlock => {
                "Failed swing revisit without taking the previous extreme."
            }
            Self::RejectionBlock => "Single-bar wick-dominant rejection at key price.",
            Self::PropulsionBlock => {
                "Single-bar body-dominant expansion with range and volume confirmation."
            }
            Self::LiquidityVoid => "ATR-normalized imbalance gap / delivery vacuum.",
            Self::VolumeImbalance => "Rolling-window volume shock beyond the configured sigma threshold.",
            Self::MarketStructureShift => {
                "Swing-pivot structure break confirmed by close."
            }
            Self::Cisd => "Three-bar change in state of delivery against the prior trend.",
        }
    }
}

impl FromStr for AutoQuantPdaPrimitiveKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let normalized = value.trim().to_ascii_lowercase().replace('-', "_");
        match normalized.as_str() {
            "order_block" | "ob" => Ok(Self::OrderBlock),
            "fair_value_gap" | "fvg" => Ok(Self::FairValueGap),
            "inverse_fvg" | "ifvg" | "inverse_fair_value_gap" => Ok(Self::InverseFvg),
            "breaker_block" | "bb" => Ok(Self::BreakerBlock),
            "mitigation_block" | "mb" => Ok(Self::MitigationBlock),
            "rejection_block" | "rb" => Ok(Self::RejectionBlock),
            "propulsion_block" | "pb" => Ok(Self::PropulsionBlock),
            "liquidity_void" | "lv" => Ok(Self::LiquidityVoid),
            "volume_imbalance" | "vi" => Ok(Self::VolumeImbalance),
            "market_structure_shift" | "mss" => Ok(Self::MarketStructureShift),
            "cisd" => Ok(Self::Cisd),
            _ => bail!("unknown PDA primitive '{}'", value),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum AutoQuantUnitDirection {
    Long,
    Short,
}

impl AutoQuantUnitDirection {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Long => "long",
            Self::Short => "short",
        }
    }

    pub fn user_narrative(self) -> &'static str {
        match self {
            Self::Long => "Trade long only. Do not add short entries to this unit.",
            Self::Short => "Trade short only. Do not add long entries to this unit.",
        }
    }
}

impl FromStr for AutoQuantUnitDirection {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "long" | "bull" => Ok(Self::Long),
            "short" | "bear" => Ok(Self::Short),
            _ => bail!("unknown unit direction '{}'", value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoQuantPdaUnitScope {
    pub symbol: String,
    pub timeframe: String,
    pub direction: String,
    pub data_path: String,
    pub primitive_sequence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AutoQuantConsumerEvidenceProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regime_profit_branch_path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub main_regime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_regime: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_sub_regime_or_profit_factor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profit_factor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_surfaces: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_indicators: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub notes: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub provider_guidance: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoQuantPdaUnitBrief {
    pub thesis: String,
    pub execution_rules: Vec<String>,
    pub evaluation_priority: Vec<String>,
    #[serde(default)]
    pub consumer_evidence_profile: AutoQuantConsumerEvidenceProfile,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoQuantPdaUnitJob {
    pub unit_id: String,
    pub unit_label: String,
    pub isolated_state_dir: String,
    pub scope: AutoQuantPdaUnitScope,
    pub brief: AutoQuantPdaUnitBrief,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub handoff_artifact_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoQuantPdaUnitDispatchGroup {
    pub group_index: usize,
    pub unit_ids: Vec<String>,
    pub handoff_artifact_paths: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AutoQuantPdaUnitBatchArtifact {
    pub artifact_id: String,
    pub generated_at: DateTime<Utc>,
    pub symbol: String,
    pub objective: String,
    pub combination_size: usize,
    pub max_parallel: usize,
    pub shared_workspace_root: String,
    pub selected_timeframes: Vec<String>,
    pub selected_primitives: Vec<String>,
    #[serde(default)]
    pub consumer_evidence_profile: AutoQuantConsumerEvidenceProfile,
    pub unit_jobs: Vec<AutoQuantPdaUnitJob>,
    pub dispatch_groups: Vec<AutoQuantPdaUnitDispatchGroup>,
    pub notes: Vec<String>,
}

pub struct AutoQuantPdaUnitBatchBuildInput<'a> {
    pub symbol: &'a str,
    pub objective: &'a str,
    pub factors: &'a str,
    pub combination_size: usize,
    pub directions: &'a str,
    pub timeframes: &'a str,
    pub timeframe_data_entries: &'a [String],
    pub evidence_surfaces: &'a str,
    pub indicator_list: &'a str,
    pub evidence_notes: &'a [String],
    pub max_parallel: usize,
    pub state_dir: &'a str,
    pub dependency_status: AutoQuantDependencyStatus,
}

pub struct AutoQuantPdaUnitBatchArtifactInput {
    pub symbol: String,
    pub objective: String,
    pub combination_size: usize,
    pub max_parallel: usize,
    pub shared_workspace_root: String,
    pub selected_primitives: Vec<AutoQuantPdaPrimitiveKind>,
    pub selected_timeframes: Vec<String>,
    pub consumer_evidence_profile: AutoQuantConsumerEvidenceProfile,
    pub unit_jobs: Vec<AutoQuantPdaUnitJob>,
}

pub fn parse_pda_primitive_csv(value: &str) -> Result<Vec<AutoQuantPdaPrimitiveKind>> {
    let mut out = Vec::new();
    for item in value.split(',') {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.push(AutoQuantPdaPrimitiveKind::from_str(trimmed)?);
    }
    if out.is_empty() {
        bail!("at least one PDA primitive is required");
    }
    Ok(out)
}

pub fn parse_unit_direction_csv(value: &str) -> Result<Vec<AutoQuantUnitDirection>> {
    let mut out = Vec::new();
    for item in value.split(',') {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            continue;
        }
        out.push(AutoQuantUnitDirection::from_str(trimmed)?);
    }
    if out.is_empty() {
        bail!("at least one unit direction is required");
    }
    Ok(out)
}

pub fn parse_timeframe_data_mappings(entries: &[String]) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for entry in entries {
        let Some((timeframe, path)) = entry.split_once('=') else {
            bail!(
                "timeframe-data entry '{}' must look like <timeframe>=<path>",
                entry
            );
        };
        let timeframe = timeframe.trim();
        let path = path.trim();
        if timeframe.is_empty() || path.is_empty() {
            bail!(
                "timeframe-data entry '{}' must include both timeframe and path",
                entry
            );
        }
        if out
            .insert(timeframe.to_string(), path.to_string())
            .is_some()
        {
            bail!("duplicate timeframe-data mapping for '{}'", timeframe);
        }
    }
    if out.is_empty() {
        bail!("at least one --timeframe-data entry is required");
    }
    Ok(out)
}

pub fn parse_consumer_evidence_profile(
    evidence_surfaces_csv: &str,
    indicator_list_csv: &str,
    notes: &[String],
) -> AutoQuantConsumerEvidenceProfile {
    let required_surfaces = parse_normalized_csv(evidence_surfaces_csv);
    let required_indicators = parse_normalized_csv(indicator_list_csv);
    let notes = notes
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let provider_guidance = build_provider_guidance(&required_surfaces, &required_indicators);
    AutoQuantConsumerEvidenceProfile {
        required_surfaces,
        required_indicators,
        notes,
        provider_guidance,
        ..Default::default()
    }
}

fn parse_normalized_csv(value: &str) -> Vec<String> {
    let mut out = Vec::<String>::new();
    for item in value.split(',') {
        let normalized = item.trim().to_ascii_lowercase().replace('-', "_");
        if normalized.is_empty() {
            continue;
        }
        if !out.iter().any(|existing| existing == &normalized) {
            out.push(normalized);
        }
    }
    out
}

fn build_provider_guidance(
    required_surfaces: &[String],
    required_indicators: &[String],
) -> Vec<String> {
    let mut guidance = Vec::new();
    if required_surfaces
        .iter()
        .any(|surface| surface == "indicators" || surface == "volatility")
    {
        guidance.push(
            "Indicators and volatility evidence should be computed directly from the declared candle series, not approximated through a separate local model."
                .to_string(),
        );
    }
    if required_surfaces.iter().any(|surface| {
        matches!(
            surface.as_str(),
            "greeks" | "open_interest" | "implied_volatility" | "options_chain"
        )
    }) {
        guidance.push(
            "This unit requires an options-capable provider/runtime for Greeks, OI, IV, or chain context. If the provider cannot supply those fields, ask the user for a better provider/runtime instead of fabricating them locally."
                .to_string(),
        );
    }
    if required_surfaces
        .iter()
        .any(|surface| surface == "cross_market")
    {
        guidance.push(
            "This unit requires explicit paired/cross-market reference data. Ask the user or provider layer for the paired symbol/data source instead of inferring it silently."
                .to_string(),
        );
    }
    if required_surfaces
        .iter()
        .any(|surface| surface == "session_context")
    {
        guidance.push(
            "This unit requires explicit session segmentation (for example Asia/London/NY windows) as part of the strategy truth."
                .to_string(),
        );
    }
    if !required_indicators.is_empty() {
        guidance.push(format!(
            "Required indicators to preserve exactly in the strategy implementation: {}.",
            required_indicators.join(", ")
        ));
    }
    guidance
}

pub fn select_timeframe_data(
    requested_timeframes_csv: &str,
    timeframe_data: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, String>> {
    let mut out = BTreeMap::new();
    for timeframe in requested_timeframes_csv
        .split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let Some(path) = timeframe_data.get(timeframe) else {
            bail!(
                "requested timeframe '{}' has no matching --timeframe-data mapping",
                timeframe
            );
        };
        out.insert(timeframe.to_string(), path.clone());
    }
    if out.is_empty() {
        bail!("at least one requested timeframe is required");
    }
    Ok(out)
}

pub fn build_unit_sequences(
    primitives: &[AutoQuantPdaPrimitiveKind],
    combination_size: usize,
) -> Result<Vec<Vec<AutoQuantPdaPrimitiveKind>>> {
    if primitives.is_empty() {
        bail!("cannot build unit sequences from an empty primitive set");
    }
    if combination_size == 0 {
        bail!("combination_size must be at least 1");
    }
    if combination_size > primitives.len() {
        bail!(
            "combination_size={} exceeds selected primitive count={}",
            combination_size,
            primitives.len()
        );
    }
    let mut out = Vec::new();
    let mut current = Vec::new();
    let mut used = vec![false; primitives.len()];
    build_sequences_recursive(
        primitives,
        combination_size,
        &mut used,
        &mut current,
        &mut out,
    );
    Ok(out)
}

fn build_sequences_recursive(
    primitives: &[AutoQuantPdaPrimitiveKind],
    combination_size: usize,
    used: &mut [bool],
    current: &mut Vec<AutoQuantPdaPrimitiveKind>,
    out: &mut Vec<Vec<AutoQuantPdaPrimitiveKind>>,
) {
    if current.len() == combination_size {
        out.push(current.clone());
        return;
    }
    for (index, primitive) in primitives.iter().enumerate() {
        if used[index] {
            continue;
        }
        used[index] = true;
        current.push(*primitive);
        build_sequences_recursive(primitives, combination_size, used, current, out);
        current.pop();
        used[index] = false;
    }
}

pub fn build_unit_jobs(
    symbol: &str,
    sequences: &[Vec<AutoQuantPdaPrimitiveKind>],
    directions: &[AutoQuantUnitDirection],
    timeframe_data: &BTreeMap<String, String>,
    consumer_evidence_profile: &AutoQuantConsumerEvidenceProfile,
    state_dir: &str,
) -> Vec<AutoQuantPdaUnitJob> {
    let mut jobs = Vec::new();
    for sequence in sequences {
        for direction in directions {
            for (timeframe, data_path) in timeframe_data {
                let unit_label = format!(
                    "{}:{}:{}:{}",
                    symbol,
                    timeframe,
                    direction.as_str(),
                    sequence
                        .iter()
                        .map(|item| item.as_str())
                        .collect::<Vec<_>>()
                        .join("->")
                );
                let unit_id = unit_label
                    .replace(':', "__")
                    .replace("->", "__")
                    .replace('/', "_");
                let isolated_state_dir = PathBuf::from(state_dir)
                    .join("units")
                    .join(&unit_id)
                    .to_string_lossy()
                    .to_string();
                jobs.push(AutoQuantPdaUnitJob {
                    unit_id,
                    unit_label,
                    isolated_state_dir,
                    scope: AutoQuantPdaUnitScope {
                        symbol: symbol.to_string(),
                        timeframe: timeframe.clone(),
                        direction: direction.as_str().to_string(),
                        data_path: data_path.clone(),
                        primitive_sequence: sequence
                            .iter()
                            .map(|item| item.as_str().to_string())
                            .collect(),
                    },
                    brief: build_unit_brief(
                        symbol,
                        timeframe,
                        *direction,
                        sequence,
                        consumer_evidence_profile,
                    ),
                    handoff_artifact_path: None,
                });
            }
        }
    }
    jobs
}

fn build_unit_brief(
    symbol: &str,
    timeframe: &str,
    direction: AutoQuantUnitDirection,
    sequence: &[AutoQuantPdaPrimitiveKind],
    consumer_evidence_profile: &AutoQuantConsumerEvidenceProfile,
) -> AutoQuantPdaUnitBrief {
    let sequence_label = sequence
        .iter()
        .map(|item| item.as_str())
        .collect::<Vec<_>>()
        .join(" -> ");
    let primitive_details = sequence
        .iter()
        .map(|item| format!("{}: {}", item.as_str(), item.detector_summary()))
        .collect::<Vec<_>>();
    AutoQuantPdaUnitBrief {
        thesis: format!(
            "Iterate one Auto-Quant strategy unit around the ordered PDA sequence [{}] on {} {} bars for {}. Treat repo PDA code only as reference; the strategy implementation may differ internally, but it must preserve this ordered event narrative and optimize win rate first, Sharpe second, return third.",
            sequence_label, symbol, timeframe, direction.as_str()
        ),
        execution_rules: {
            let mut rules = vec![
            direction.user_narrative().to_string(),
            format!(
                "One unit only: setup sequence [{}], symbol {}, timeframe {}.",
                sequence_label, symbol, timeframe
            ),
            format!(
                "Primitive reference details: {}",
                primitive_details.join(" | ")
            ),
            "Do not widen the strategy into unrelated setups just because repo structure_ict aggregates more than this unit.".to_string(),
            ];
            if !consumer_evidence_profile.required_surfaces.is_empty() {
                rules.push(format!(
                    "Consumer-required evidence surfaces: {}.",
                    consumer_evidence_profile.required_surfaces.join(", ")
                ));
            }
            if !consumer_evidence_profile.required_indicators.is_empty() {
                rules.push(format!(
                    "Consumer-required indicators: {}.",
                    consumer_evidence_profile.required_indicators.join(", ")
                ));
            }
            rules.extend(consumer_evidence_profile.provider_guidance.iter().cloned());
            rules.extend(consumer_evidence_profile.notes.iter().cloned());
            rules
        },
        evaluation_priority: vec![
            "win_rate".to_string(),
            "sharpe".to_string(),
            "return".to_string(),
        ],
        consumer_evidence_profile: consumer_evidence_profile.clone(),
    }
}

pub fn build_dispatch_groups(
    jobs: &[AutoQuantPdaUnitJob],
    max_parallel: usize,
) -> Vec<AutoQuantPdaUnitDispatchGroup> {
    let width = max_parallel.max(1);
    jobs.chunks(width)
        .enumerate()
        .map(|(index, chunk)| AutoQuantPdaUnitDispatchGroup {
            group_index: index,
            unit_ids: chunk.iter().map(|job| job.unit_id.clone()).collect(),
            handoff_artifact_paths: chunk
                .iter()
                .filter_map(|job| job.handoff_artifact_path.clone())
                .collect(),
        })
        .collect()
}

pub fn build_batch_artifact(
    input: AutoQuantPdaUnitBatchArtifactInput,
) -> AutoQuantPdaUnitBatchArtifact {
    let generated_at = Utc::now();
    let dispatch_groups = build_dispatch_groups(&input.unit_jobs, input.max_parallel);
    AutoQuantPdaUnitBatchArtifact {
        artifact_id: format!(
            "auto-quant-pda-unit-batch:{}:{}",
            input.symbol,
            generated_at.format("%Y%m%dT%H%M%S%.3fZ")
        ),
        generated_at,
        symbol: input.symbol,
        objective: input.objective,
        combination_size: input.combination_size,
        max_parallel: input.max_parallel,
        shared_workspace_root: input.shared_workspace_root,
        selected_timeframes: input.selected_timeframes,
        selected_primitives: input
            .selected_primitives
            .iter()
            .map(|item| item.as_str().to_string())
            .collect(),
        consumer_evidence_profile: input.consumer_evidence_profile,
        unit_jobs: input.unit_jobs,
        dispatch_groups,
        notes: vec![
            "shared_auto_quant_workspace_is_read_only_between_parallel_units".to_string(),
            "each_unit_uses_an_isolated_state_dir".to_string(),
            "evaluation_priority=win_rate>sharpe>return".to_string(),
        ],
    }
}

pub fn persist_auto_quant_pda_unit_batch(
    input: AutoQuantPdaUnitBatchBuildInput<'_>,
) -> Result<AutoQuantPdaUnitBatchArtifact> {
    let parsed_primitives = parse_pda_primitive_csv(input.factors)?;
    let parsed_directions = parse_unit_direction_csv(input.directions)?;
    let timeframe_data = parse_timeframe_data_mappings(input.timeframe_data_entries)?;
    let selected_timeframe_data = select_timeframe_data(input.timeframes, &timeframe_data)?;
    let consumer_evidence_profile = parse_consumer_evidence_profile(
        input.evidence_surfaces,
        input.indicator_list,
        input.evidence_notes,
    );
    let sequences = build_unit_sequences(&parsed_primitives, input.combination_size)?;
    let mut jobs = build_unit_jobs(
        input.symbol,
        &sequences,
        &parsed_directions,
        &selected_timeframe_data,
        &consumer_evidence_profile,
        input.state_dir,
    );

    for job in &mut jobs {
        let mut payload =
            build_factor_research_handoff_payload(BuildFactorResearchHandoffPayloadInput {
                symbol: input.symbol,
                data: &job.scope.data_path,
                objective: input.objective,
                provider_profile_selector: None,
                paired_data: None,
                auxiliary_evidence_path: None,
                mutation_spec_path: None,
                strategy_material_root: None,
                state_dir: &job.isolated_state_dir,
                dependency_status: input.dependency_status.clone(),
            });
        payload.iteration_unit = Some(AutoQuantIterationUnitContext {
            unit_label: job.unit_label.clone(),
            primitive_sequence: job.scope.primitive_sequence.clone(),
            timeframe: job.scope.timeframe.clone(),
            direction: job.scope.direction.clone(),
            strategy_brief: job.brief.thesis.clone(),
            evaluation_priority: job.brief.evaluation_priority.clone(),
            consumer_evidence_profile: Some(job.brief.consumer_evidence_profile.clone()),
        });
        payload
            .notes
            .push("auto_quant_pda_unit_batch_dispatch".to_string());
        payload.suggested_next_steps.insert(
            0,
            format!(
                "treat this as one isolated unit only: sequence={} symbol={} timeframe={} direction={}",
                job.scope.primitive_sequence.join("->"),
                job.scope.symbol,
                job.scope.timeframe,
                job.scope.direction
            ),
        );
        payload.agent_prompt = format!(
            "{} Unit strategy brief: {} Execution rules: {}",
            payload.agent_prompt,
            job.brief.thesis,
            job.brief.execution_rules.join(" | ")
        );
        let handoff_path = persist_handoff_payload(&job.isolated_state_dir, &payload)?;
        job.handoff_artifact_path = Some(handoff_path);
    }

    let selected_timeframes = selected_timeframe_data.keys().cloned().collect::<Vec<_>>();
    let workspace_root = input.dependency_status.managed_dir.clone();
    let artifact = build_batch_artifact(AutoQuantPdaUnitBatchArtifactInput {
        symbol: input.symbol.to_string(),
        objective: input.objective.to_string(),
        combination_size: input.combination_size,
        max_parallel: input.max_parallel,
        shared_workspace_root: workspace_root,
        selected_primitives: parsed_primitives,
        selected_timeframes,
        consumer_evidence_profile,
        unit_jobs: jobs,
    });
    persist_batch_artifact(input.state_dir, &artifact)?;
    Ok(artifact)
}

fn persist_batch_artifact(state_dir: &str, artifact: &AutoQuantPdaUnitBatchArtifact) -> Result<()> {
    let filename = format!(
        "auto_quant_pda_unit_batch.{}.json",
        artifact.generated_at.format("%Y%m%dT%H%M%S%.3fZ")
    );
    save_state(state_dir, &artifact.symbol, &filename, artifact)?;
    let path = artifact_state_path(state_dir, &artifact.symbol, &filename);
    append_artifact_ledger_entry(
        state_dir,
        &artifact.symbol,
        ArtifactLedgerEntry {
            entry_id: format!("ledger:{}", artifact.artifact_id),
            artifact_kind: "auto_quant_pda_unit_batch".to_string(),
            artifact_id: artifact.artifact_id.clone(),
            version: 1,
            generated_at: artifact.generated_at,
            symbol: artifact.symbol.clone(),
            source_phase: "auto_quant_pda_unit_batch".to_string(),
            source_run_id: None,
            path,
            status: "batch_ready_for_dispatch".to_string(),
            promote_candidate: false,
            actionable: true,
            decision_hint: "auto_quant_parallel_dispatch".to_string(),
            review_reason: format!(
                "units={} max_parallel={} combination_size={}",
                artifact.unit_jobs.len(),
                artifact.max_parallel,
                artifact.combination_size
            ),
            review_rule_version: AUTO_QUANT_PDA_UNIT_BATCH_RULE_VERSION.to_string(),
            top_factor_name: artifact.selected_primitives.first().cloned(),
            top_factor_action: Some("dispatch".to_string()),
            family_scores: BTreeMap::new(),
            supersedes_artifact_id: None,
            quality_score: artifact.unit_jobs.len().min(i32::MAX as usize) as i32,
            consumed_by_update_run_id: None,
            consumed_at: None,
            consumed_outcome: None,
            regraded_at: None,
            consumption_regrade_status: None,
            consumption_regrade_reason: None,
        },
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_pda_primitive_kind_accepts_user_facing_names() {
        let parsed = parse_pda_primitive_csv("order_block,fvg,mss,cisd").unwrap();
        assert_eq!(
            parsed,
            vec![
                AutoQuantPdaPrimitiveKind::OrderBlock,
                AutoQuantPdaPrimitiveKind::FairValueGap,
                AutoQuantPdaPrimitiveKind::MarketStructureShift,
                AutoQuantPdaPrimitiveKind::Cisd,
            ]
        );
    }

    #[test]
    fn build_unit_sequences_uses_ordered_permutations_for_combination_size_two() {
        let sequences = build_unit_sequences(
            &[
                AutoQuantPdaPrimitiveKind::OrderBlock,
                AutoQuantPdaPrimitiveKind::FairValueGap,
                AutoQuantPdaPrimitiveKind::Cisd,
            ],
            2,
        )
        .unwrap();

        let labels = sequences
            .iter()
            .map(|seq| {
                seq.iter()
                    .map(|item| item.as_str())
                    .collect::<Vec<_>>()
                    .join("->")
            })
            .collect::<Vec<_>>();

        assert_eq!(
            labels,
            vec![
                "order_block->fair_value_gap",
                "order_block->cisd",
                "fair_value_gap->order_block",
                "fair_value_gap->cisd",
                "cisd->order_block",
                "cisd->fair_value_gap",
            ]
        );
    }

    #[test]
    fn build_unit_jobs_expands_direction_timeframe_cross_product() {
        let sequences = build_unit_sequences(&[AutoQuantPdaPrimitiveKind::OrderBlock], 1).unwrap();
        let directions = vec![AutoQuantUnitDirection::Long, AutoQuantUnitDirection::Short];
        let timeframe_data = BTreeMap::from([
            ("15m".to_string(), "/tmp/nq-15m.json".to_string()),
            ("1h".to_string(), "/tmp/nq-1h.json".to_string()),
        ]);
        let profile = parse_consumer_evidence_profile(
            "indicators,volatility,greeks",
            "rsi14,ema20,atr14",
            &["consumer wants session-aware volatility".to_string()],
        );

        let jobs = build_unit_jobs(
            "NQ",
            &sequences,
            &directions,
            &timeframe_data,
            &profile,
            "/tmp/state",
        );
        assert_eq!(jobs.len(), 4);
        assert!(jobs.iter().any(|job| {
            job.scope.timeframe == "15m"
                && job.scope.direction == "long"
                && job.scope.primitive_sequence == vec!["order_block".to_string()]
        }));
        assert!(jobs.iter().any(|job| {
            job.scope.timeframe == "1h"
                && job.scope.direction == "short"
                && job.brief.evaluation_priority
                    == vec![
                        "win_rate".to_string(),
                        "sharpe".to_string(),
                        "return".to_string(),
                    ]
        }));
        assert!(jobs.iter().all(|job| {
            job.brief
                .consumer_evidence_profile
                .required_surfaces
                .iter()
                .any(|surface| surface == "greeks")
        }));
    }

    #[test]
    fn dispatch_groups_chunk_by_max_parallel() {
        let sequences = build_unit_sequences(&[AutoQuantPdaPrimitiveKind::OrderBlock], 1).unwrap();
        let directions = vec![AutoQuantUnitDirection::Long, AutoQuantUnitDirection::Short];
        let timeframe_data = BTreeMap::from([
            ("15m".to_string(), "/tmp/nq-15m.json".to_string()),
            ("1h".to_string(), "/tmp/nq-1h.json".to_string()),
        ]);
        let jobs = build_unit_jobs(
            "NQ",
            &sequences,
            &directions,
            &timeframe_data,
            &AutoQuantConsumerEvidenceProfile::default(),
            "/tmp/state",
        );
        let groups = build_dispatch_groups(&jobs, 3);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].unit_ids.len(), 3);
        assert_eq!(groups[1].unit_ids.len(), 1);
    }

    #[test]
    fn parse_consumer_evidence_profile_builds_provider_guidance() {
        let profile = parse_consumer_evidence_profile(
            "indicators,volatility,greeks,open_interest,cross_market",
            "rsi14,ema20,atr14",
            &["consumer wants NY session context".to_string()],
        );
        assert!(profile
            .required_surfaces
            .iter()
            .any(|item| item == "greeks"));
        assert!(profile
            .provider_guidance
            .iter()
            .any(|line| line.contains("options-capable provider")));
        assert!(profile
            .provider_guidance
            .iter()
            .any(|line| line.contains("Required indicators")));
        assert_eq!(
            profile.notes,
            vec!["consumer wants NY session context".to_string()]
        );
    }

    #[test]
    fn persist_auto_quant_pda_unit_batch_writes_batch_and_unit_handoffs() {
        let temp = tempfile::tempdir().unwrap();
        let managed_dir = temp.path().join("managed-auto-quant");
        let strategies_dir = managed_dir.join("user_data/strategies");
        std::fs::create_dir_all(&strategies_dir).unwrap();
        std::fs::write(managed_dir.join("program.md"), "program").unwrap();
        std::fs::write(managed_dir.join("prepare.py"), "print('prepare')").unwrap();
        std::fs::write(managed_dir.join("run.py"), "print('run')").unwrap();
        std::fs::write(
            strategies_dir.join("_template.py.example"),
            "class Template: pass",
        )
        .unwrap();

        let artifact = persist_auto_quant_pda_unit_batch(AutoQuantPdaUnitBatchBuildInput {
            symbol: "NQ",
            objective: "expansion_manipulation",
            factors: "order_block,fair_value_gap",
            combination_size: 1,
            directions: "long,short",
            timeframes: "15m",
            timeframe_data_entries: &["15m=/tmp/nq-15m.json".to_string()],
            evidence_surfaces: "indicators,greeks,implied_volatility",
            indicator_list: "rsi14,ema20,atr14",
            evidence_notes: &["consumer needs explicit volatility context".to_string()],
            max_parallel: 2,
            state_dir: temp.path().to_str().unwrap(),
            dependency_status: AutoQuantDependencyStatus {
                repo_url: "repo".to_string(),
                managed_dir: managed_dir.to_string_lossy().to_string(),
                tracked_branch: "master".to_string(),
                pinned_ref: None,
                current_commit: None,
                upstream_commit: None,
                bootstrap_needed: false,
                config_present: true,
                managed_repo_present: true,
                healthy: true,
                update_available: false,
                required_files: Vec::new(),
                notes: Vec::new(),
                adapter_version: "v1".to_string(),
                last_sync: None,
            },
        })
        .unwrap();

        assert_eq!(artifact.unit_jobs.len(), 4);
        assert_eq!(artifact.dispatch_groups.len(), 2);
        assert!(artifact
            .consumer_evidence_profile
            .required_surfaces
            .iter()
            .any(|item| item == "greeks"));
        assert!(artifact
            .unit_jobs
            .iter()
            .all(|job| job.handoff_artifact_path.is_some()));
        assert!(artifact.unit_jobs.iter().all(|job| {
            job.brief
                .consumer_evidence_profile
                .provider_guidance
                .iter()
                .any(|line| line.contains("options-capable provider"))
        }));
    }
}
