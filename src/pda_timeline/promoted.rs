use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use super::event::{PdaEvent, PdaEventKind};
use super::setups::{CanonicalSetupKind, SetupMatch};
use crate::types::Direction;

pub const PROMOTED_CANONICAL_SETUPS_CONFIG_FILE: &str = "config/promoted_canonical_setups.json";
pub const PROMOTED_CANONICAL_SETUPS_GENERATED_FILE: &str =
    "src/pda_timeline/generated_promoted_setups.rs";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PromotedCanonicalSetupManifest {
    pub version: u32,
    #[serde(default)]
    pub setups: Vec<PromotedCanonicalSetupSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromotedCanonicalSetupSpec {
    pub name: String,
    pub sequence: Vec<PdaEventKind>,
    pub direction: Option<Direction>,
    pub horizon_bars: usize,
    pub source_sweep_id: String,
    pub promoted_symbol: String,
    pub promoted_at: String,
}

pub fn embedded_promoted_canonical_setups() -> Vec<PromotedCanonicalSetupSpec> {
    serde_json::from_str::<PromotedCanonicalSetupManifest>(
        super::generated_promoted_setups::PROMOTED_CANONICAL_SETUPS_JSON,
    )
    .map(|manifest| manifest.setups)
    .unwrap_or_default()
}

pub fn match_promoted_canonical_setups(
    events: &[PdaEvent],
    horizon_bars: usize,
) -> Vec<SetupMatch> {
    let specs = embedded_promoted_canonical_setups();
    let mut out = Vec::new();
    for spec in &specs {
        if spec.sequence.len() < 2 || events.len() < spec.sequence.len() {
            continue;
        }
        let effective_horizon = spec.horizon_bars.min(horizon_bars);
        for window in events.windows(spec.sequence.len()) {
            if !window_matches_promoted_spec(window, spec, effective_horizon) {
                continue;
            }
            let direction = window
                .last()
                .map(|event| event.direction)
                .unwrap_or(Direction::Neutral);
            out.push(SetupMatch {
                kind: CanonicalSetupKind::PromotedCanonicalSequence,
                name_override: Some(spec.name.clone()),
                direction,
                anchor_bar: window
                    .first()
                    .map(|event| event.bar_index)
                    .unwrap_or_default(),
                confirm_bar: window
                    .last()
                    .map(|event| event.bar_index)
                    .unwrap_or_default(),
                event_bars: window.iter().map(|event| event.bar_index).collect(),
            });
        }
    }
    out.sort_by_key(|m| (m.confirm_bar, m.label().to_string()));
    out
}

pub fn load_promoted_canonical_setup_manifest<P: AsRef<Path>>(
    repo_root: P,
) -> Result<PromotedCanonicalSetupManifest> {
    let path = repo_root
        .as_ref()
        .join(PROMOTED_CANONICAL_SETUPS_CONFIG_FILE);
    if !path.exists() {
        return Ok(PromotedCanonicalSetupManifest {
            version: 1,
            setups: Vec::new(),
        });
    }
    let raw = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "failed to read promoted canonical setup manifest '{}'",
            path.display()
        )
    })?;
    let manifest = serde_json::from_str(&raw).with_context(|| {
        format!(
            "failed to parse promoted canonical setup manifest '{}'",
            path.display()
        )
    })?;
    Ok(manifest)
}

pub fn append_promoted_canonical_setup<P: AsRef<Path>>(
    repo_root: P,
    spec: PromotedCanonicalSetupSpec,
) -> Result<(String, String)> {
    let repo_root = repo_root.as_ref();
    let mut manifest = load_promoted_canonical_setup_manifest(repo_root)?;
    if manifest.version == 0 {
        manifest.version = 1;
    }
    if manifest
        .setups
        .iter()
        .any(|item| item.name.eq_ignore_ascii_case(&spec.name))
    {
        bail!("promoted canonical setup '{}' already exists", spec.name);
    }
    manifest.setups.push(spec);
    manifest
        .setups
        .sort_by(|left, right| left.name.cmp(&right.name));
    let config_path = repo_root.join(PROMOTED_CANONICAL_SETUPS_CONFIG_FILE);
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&config_path, serde_json::to_string_pretty(&manifest)?)?;

    let generated_path = repo_root.join(PROMOTED_CANONICAL_SETUPS_GENERATED_FILE);
    if let Some(parent) = generated_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let generated_body = format!(
        "pub const PROMOTED_CANONICAL_SETUPS_JSON: &str = r#\"{}\"#;\n",
        serde_json::to_string_pretty(&manifest)?
    );
    std::fs::write(&generated_path, generated_body)?;

    Ok((
        config_path.to_string_lossy().to_string(),
        generated_path.to_string_lossy().to_string(),
    ))
}

pub fn parse_promoted_sequence_label(label: &str) -> Result<Vec<PdaEventKind>> {
    let parts = label
        .split("->")
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .map(parse_pda_event_kind)
        .collect::<Result<Vec<_>>>()?;
    if parts.len() < 2 {
        bail!("promoted sequence must contain at least two event kinds");
    }
    Ok(parts)
}

pub fn repo_root_from_manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn build_promoted_canonical_setup_spec(
    name: &str,
    sequence_label: &str,
    direction: Option<Direction>,
    horizon_bars: usize,
    source_sweep_id: &str,
    promoted_symbol: &str,
) -> Result<PromotedCanonicalSetupSpec> {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        bail!("setup name must not be empty");
    }
    Ok(PromotedCanonicalSetupSpec {
        name: normalize_promoted_setup_name(trimmed),
        sequence: parse_promoted_sequence_label(sequence_label)?,
        direction,
        horizon_bars,
        source_sweep_id: source_sweep_id.to_string(),
        promoted_symbol: promoted_symbol.to_string(),
        promoted_at: Utc::now().to_rfc3339(),
    })
}

fn window_matches_promoted_spec(
    window: &[PdaEvent],
    spec: &PromotedCanonicalSetupSpec,
    effective_horizon: usize,
) -> bool {
    if window.len() != spec.sequence.len() {
        return false;
    }
    if window
        .iter()
        .zip(spec.sequence.iter())
        .any(|(event, expected)| event.kind != *expected)
    {
        return false;
    }
    if window
        .windows(2)
        .any(|pair| pair[0].bar_index >= pair[1].bar_index)
    {
        return false;
    }
    let span = window
        .last()
        .unwrap()
        .bar_index
        .saturating_sub(window.first().unwrap().bar_index);
    if span > effective_horizon {
        return false;
    }
    if let Some(direction) = spec.direction {
        window.last().map(|event| event.direction) == Some(direction)
    } else {
        true
    }
}

fn parse_pda_event_kind(raw: &str) -> Result<PdaEventKind> {
    let normalized = raw.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "fair_value_gap" => Ok(PdaEventKind::FairValueGap),
        "inverse_fair_value_gap" => Ok(PdaEventKind::InverseFairValueGap),
        "order_block" => Ok(PdaEventKind::OrderBlock),
        "breaker_block" => Ok(PdaEventKind::BreakerBlock),
        "mitigation_block" => Ok(PdaEventKind::MitigationBlock),
        "propulsion_block" => Ok(PdaEventKind::PropulsionBlock),
        "rejection_block" => Ok(PdaEventKind::RejectionBlock),
        "liquidity_sweep" => Ok(PdaEventKind::LiquiditySweep),
        "liquidity_void" => Ok(PdaEventKind::LiquidityVoid),
        "structure_break" => Ok(PdaEventKind::StructureBreak),
        "market_structure_shift" => Ok(PdaEventKind::MarketStructureShift),
        "cisd" => Ok(PdaEventKind::Cisd),
        "volume_imbalance" => Ok(PdaEventKind::VolumeImbalance),
        _ => Err(anyhow!("unknown pda event kind '{}'", raw)),
    }
}

fn normalize_promoted_setup_name(raw: &str) -> String {
    raw.trim()
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_promoted_sequence_label_requires_two_kinds() {
        assert!(parse_promoted_sequence_label("liquidity_sweep").is_err());
        let parsed =
            parse_promoted_sequence_label("liquidity_sweep -> market_structure_shift").unwrap();
        assert_eq!(
            parsed,
            vec![
                PdaEventKind::LiquiditySweep,
                PdaEventKind::MarketStructureShift
            ]
        );
    }

    #[test]
    fn append_promoted_canonical_setup_writes_manifest_and_generated_source() {
        let temp = tempfile::tempdir().unwrap();
        let spec = build_promoted_canonical_setup_spec(
            "Sweep Mss Continuation",
            "liquidity_sweep -> market_structure_shift",
            Some(Direction::Bull),
            30,
            "pb12:NQ:test",
            "NQ",
        )
        .unwrap();
        let (config_path, generated_path) =
            append_promoted_canonical_setup(temp.path(), spec.clone()).unwrap();
        assert!(Path::new(&config_path).exists());
        assert!(Path::new(&generated_path).exists());
        let manifest = load_promoted_canonical_setup_manifest(temp.path()).unwrap();
        assert_eq!(manifest.setups.len(), 1);
        assert_eq!(manifest.setups[0].name, "SweepMssContinuation");
        assert_eq!(manifest.setups[0].sequence, spec.sequence);
    }
}
