use crate::state::{FactorPipelineLabelSource, PreBayesEvidenceFilter};

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LogicFamilyTag {
    pub entry_logic_id: Option<String>,
    pub logic_family: Option<String>,
}

fn from_decision_hint(decision_hint: &str) -> LogicFamilyTag {
    let hint = decision_hint.to_ascii_lowercase();
    if hint.contains("ote") {
        return LogicFamilyTag {
            entry_logic_id: Some("ict_ote_liquidity_sweep".to_string()),
            logic_family: Some("ict_ote_fvg_ob".to_string()),
        };
    }
    if hint.contains("sfp") || hint.contains("sweep") {
        return LogicFamilyTag {
            entry_logic_id: Some("ict_bull_bear_sweep".to_string()),
            logic_family: Some("ict_sniper".to_string()),
        };
    }
    if hint.contains("divergence") || hint.contains("div") {
        return LogicFamilyTag {
            entry_logic_id: Some("long_short_divergence_sweep".to_string()),
            logic_family: Some("ict_divergence".to_string()),
        };
    }
    LogicFamilyTag::default()
}

fn from_trace(
    market: &FactorPipelineLabelSource,
    liquidity: &FactorPipelineLabelSource,
    resonance: &FactorPipelineLabelSource,
) -> LogicFamilyTag {
    let joined = [
        market.label.as_str(),
        market.derivation.as_str(),
        &market.evidence.join(" "),
        liquidity.label.as_str(),
        liquidity.derivation.as_str(),
        &liquidity.evidence.join(" "),
        resonance.label.as_str(),
        resonance.derivation.as_str(),
        &resonance.evidence.join(" "),
    ]
    .join(" ")
    .to_ascii_lowercase();

    if joined.contains("ote") || joined.contains("fvg") || joined.contains("ob") {
        return LogicFamilyTag {
            entry_logic_id: Some("ict_ote_liquidity_sweep".to_string()),
            logic_family: Some("ict_ote_fvg_ob".to_string()),
        };
    }
    if joined.contains("perfect") || joined.contains("ict zone") {
        return LogicFamilyTag {
            entry_logic_id: Some("perfect_ict_zone_reversal".to_string()),
            logic_family: Some("ict_zone_reversal".to_string()),
        };
    }
    if joined.contains("purified") || joined.contains("wpr") {
        return LogicFamilyTag {
            entry_logic_id: Some("purified_wpr_sweep".to_string()),
            logic_family: Some("purified_sweep".to_string()),
        };
    }
    if joined.contains("sfp") {
        return LogicFamilyTag {
            entry_logic_id: Some("ict_sfp_bull_bear".to_string()),
            logic_family: Some("ict_sfp".to_string()),
        };
    }
    LogicFamilyTag::default()
}

pub fn infer_logic_family_tag(
    decision_hint: &str,
    market: &FactorPipelineLabelSource,
    liquidity: &FactorPipelineLabelSource,
    resonance: &FactorPipelineLabelSource,
) -> LogicFamilyTag {
    let direct = from_decision_hint(decision_hint);
    if direct.logic_family.is_some() {
        return direct;
    }
    from_trace(market, liquidity, resonance)
}

pub fn apply_logic_family_tag(filter: &mut PreBayesEvidenceFilter, tag: &LogicFamilyTag) {
    if filter.entry_logic_id.is_none() {
        filter.entry_logic_id = tag.entry_logic_id.clone();
    }
    if filter.logic_family.is_none() {
        filter.logic_family = tag.logic_family.clone();
    }
    if let Some(value) = &filter.entry_logic_id {
        filter.rationale.push(format!("entry_logic_id={value}"));
    }
    if let Some(value) = &filter.logic_family {
        filter.rationale.push(format!("logic_family={value}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infers_sfp_from_decision_hint() {
        let empty = FactorPipelineLabelSource::default();
        let tag = infer_logic_family_tag("SFP bearish sweep", &empty, &empty, &empty);
        assert_eq!(tag.logic_family.as_deref(), Some("ict_sniper"));
    }
}
