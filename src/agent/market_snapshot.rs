use chrono_tz::America::New_York;
use serde::{Deserialize, Serialize};

use crate::data::loader::format_ny;
use crate::state::{
    FootprintCompactStatus, PriorCompactStatus, ReflectionMismatchTag, ReflectionStatus,
    RegimeCompactStatus,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMarketSnapshot {
    pub timezone: String,
    pub as_of_ny: Option<String>,
    pub regime_status: RegimeCompactStatus,
    pub footprint_status: FootprintCompactStatus,
    pub prior_status: PriorCompactStatus,
    pub reflection_status: ReflectionStatus,
}

impl AgentMarketSnapshot {
    pub fn empty() -> Self {
        Self {
            timezone: "America/New_York".to_string(),
            as_of_ny: None,
            regime_status: RegimeCompactStatus::default(),
            footprint_status: FootprintCompactStatus::default(),
            prior_status: PriorCompactStatus::default(),
            reflection_status: ReflectionStatus::default(),
        }
    }

    pub fn new(
        regime_status: RegimeCompactStatus,
        footprint_status: FootprintCompactStatus,
        prior_status: PriorCompactStatus,
        reflection_status: ReflectionStatus,
    ) -> Self {
        Self {
            timezone: "America/New_York".to_string(),
            as_of_ny: None,
            regime_status,
            footprint_status,
            prior_status,
            reflection_status,
        }
    }

    pub fn with_as_of(mut self, timestamp: chrono::DateTime<chrono::Utc>) -> Self {
        let _ = timestamp.with_timezone(&New_York);
        self.as_of_ny = Some(format_ny(timestamp));
        self
    }

    pub fn to_compact_lines(&self) -> Vec<String> {
        vec![
            format!(
                "meta|tz={}|as_of={}",
                self.timezone,
                self.as_of_ny.as_deref().unwrap_or("n/a")
            ),
            format!(
                "regime|envelope={}|class={}",
                self.regime_status.envelope, self.regime_status.class
            ),
            format!(
                "footprint|bull={}|bear={}",
                compact_chain(&self.footprint_status.bull_chain),
                compact_chain(&self.footprint_status.bear_chain)
            ),
            format!(
                "prior|bull={:.3}|bear={:.3}|bull_h={}|bear_h={}",
                self.prior_status.bull_prior,
                self.prior_status.bear_prior,
                self.prior_status
                    .bull_hypothesis
                    .as_deref()
                    .unwrap_or("n/a"),
                self.prior_status
                    .bear_hypothesis
                    .as_deref()
                    .unwrap_or("n/a")
            ),
            format!(
                "reflection|outcome={}|verified={}|belief={}|tags={}",
                self.reflection_status.compact.outcome,
                self.reflection_status.compact.verified,
                self.reflection_status.belief_update.summary,
                compact_tags(&self.reflection_status.belief_update.structured_tags)
            ),
        ]
    }

    pub fn to_human_sections(&self) -> [String; 5] {
        [
            format!(
                "基本价格结构分析\nregime={} / {}",
                self.regime_status.envelope, self.regime_status.class
            ),
            format!(
                "技术面价格分析\nprior bull {:.3}, bear {:.3}",
                self.prior_status.bull_prior, self.prior_status.bear_prior
            ),
            format!(
                "SMT相关性分析\nbull chain: {}\nbear chain: {}",
                compact_chain_human(&self.footprint_status.bull_chain),
                compact_chain_human(&self.footprint_status.bear_chain)
            ),
            format!(
                "Regime分类结合贝叶斯分析并给推测概率\nbull {}, bear {}",
                self.prior_status
                    .bull_hypothesis
                    .as_deref()
                    .unwrap_or("n/a"),
                self.prior_status
                    .bear_hypothesis
                    .as_deref()
                    .unwrap_or("n/a")
            ),
            format!(
                "交易计划\nreflection={} [{}]",
                self.reflection_status.belief_update.summary,
                compact_tags(&self.reflection_status.belief_update.structured_tags)
            ),
        ]
    }
}

fn compact_chain(chain: &[String]) -> String {
    if chain.is_empty() {
        "n/a".to_string()
    } else {
        chain.join("->")
    }
}

fn compact_chain_human(chain: &[String]) -> String {
    if chain.is_empty() {
        "n/a".to_string()
    } else {
        chain.join(" -> ")
    }
}

fn compact_tags(tags: &[ReflectionMismatchTag]) -> String {
    if tags.is_empty() {
        "none".to_string()
    } else {
        tags.iter()
            .map(ReflectionMismatchTag::as_str)
            .collect::<Vec<_>>()
            .join(",")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{ReflectionBeliefUpdate, ReflectionCompactStatus};
    use chrono::Utc;

    #[test]
    fn compact_lines_stay_low_token_and_ny() {
        let snapshot = AgentMarketSnapshot::new(
            RegimeCompactStatus {
                envelope: "expansion".to_string(),
                class: "trend".to_string(),
            },
            FootprintCompactStatus {
                bull_chain: vec!["sweep".to_string(), "fvg".to_string()],
                bear_chain: vec![],
            },
            PriorCompactStatus {
                bull_prior: 0.62,
                bear_prior: 0.18,
                bull_hypothesis: Some("bull_cont".to_string()),
                bear_hypothesis: None,
            },
            ReflectionStatus {
                compact: ReflectionCompactStatus {
                    outcome: "win".to_string(),
                    verified: true,
                    ..ReflectionCompactStatus::default()
                },
                structured_mismatch_tags: vec![ReflectionMismatchTag::ExpectedChainMatched],
                belief_update: ReflectionBeliefUpdate {
                    structured_tags: vec![ReflectionMismatchTag::ExpectedChainMatched],
                    summary: "hold prior".to_string(),
                    ..ReflectionBeliefUpdate::default()
                },
                ..ReflectionStatus::default()
            },
        )
        .with_as_of(Utc::now());
        let lines = snapshot.to_compact_lines();
        assert!(lines[0].contains("America/New_York"));
        assert!(lines
            .iter()
            .any(|line| line.contains("expected_chain_matched")));
        assert_eq!(lines.len(), 5);
    }
}
