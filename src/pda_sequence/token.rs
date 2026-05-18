use serde::{Deserialize, Serialize};

/// Discrete PDA (Premium/Discount Array) "word" types used as the vocabulary
/// for `PdaSequence` clustering.
///
/// Each variant represents an observable ICT structural event. The set is
/// deliberately small and stable: adding a new kind is a breaking change to
/// the clustering fixture because it shifts the DTW cost surface. New kinds
/// should only be introduced when they are provably distinct from the
/// existing ones at detection time, not derived post-hoc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PdaTokenKind {
    FairValueGap,
    OrderBlock,
    LiquiditySweep,
    StructureBreak,
    RejectionBlock,
    PropulsionBlock,
    Cisd,
}

impl PdaTokenKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::FairValueGap => "fair_value_gap",
            Self::OrderBlock => "order_block",
            Self::LiquiditySweep => "liquidity_sweep",
            Self::StructureBreak => "structure_break",
            Self::RejectionBlock => "rejection_block",
            Self::PropulsionBlock => "propulsion_block",
            Self::Cisd => "cisd",
        }
    }
}

/// One PDA event in a trading-session sequence. The numeric fields stay in
/// `[0.0, 1.0]` (or clamp to it) so the DTW cost function can treat them as
/// comparable across symbols / timeframes without a normalization phase.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PdaToken {
    pub kind: PdaTokenKind,
    /// Bar index in the underlying candle series (for backtracking, not used by DTW).
    pub bar_index: usize,
    /// Fraction of this token's span that overlaps with an active prior token.
    pub overlap: f64,
    /// Liquidity was swept within or immediately after this token.
    pub liquidity_swept: bool,
    /// Signed volume imbalance (-1.0 strong bear delta .. +1.0 strong bull delta).
    pub volume_imbalance_ratio: f64,
}

impl PdaToken {
    pub fn new(kind: PdaTokenKind, bar_index: usize) -> Self {
        Self {
            kind,
            bar_index,
            overlap: 0.0,
            liquidity_swept: false,
            volume_imbalance_ratio: 0.0,
        }
    }

    pub fn with_overlap(mut self, overlap: f64) -> Self {
        self.overlap = overlap.clamp(0.0, 1.0);
        self
    }

    pub fn with_liquidity_swept(mut self, swept: bool) -> Self {
        self.liquidity_swept = swept;
        self
    }

    pub fn with_volume_imbalance(mut self, imbalance: f64) -> Self {
        self.volume_imbalance_ratio = imbalance.clamp(-1.0, 1.0);
        self
    }
}

/// Cost of aligning two PDA tokens. Used by DTW. Kind mismatch is the
/// dominant term because "FVG vs LiquiditySweep" is structurally very
/// different regardless of metadata overlap.
pub fn pda_token_cost(a: &PdaToken, b: &PdaToken) -> f64 {
    let kind_cost = if a.kind == b.kind { 0.0 } else { 1.0 };
    let overlap_cost = (a.overlap - b.overlap).abs();
    let swept_cost = if a.liquidity_swept == b.liquidity_swept {
        0.0
    } else {
        0.5
    };
    let volume_cost = (a.volume_imbalance_ratio - b.volume_imbalance_ratio).abs() / 2.0;
    // Weights sum to 1.0 at the extremes (kind=1 dominates single-metadata deltas).
    0.60 * kind_cost + 0.15 * overlap_cost + 0.15 * swept_cost + 0.10 * volume_cost
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_tokens_have_zero_cost() {
        let token = PdaToken::new(PdaTokenKind::FairValueGap, 5)
            .with_overlap(0.4)
            .with_liquidity_swept(true)
            .with_volume_imbalance(0.3);
        assert!((pda_token_cost(&token, &token)).abs() < f64::EPSILON);
    }

    #[test]
    fn different_kinds_dominate_cost() {
        let a = PdaToken::new(PdaTokenKind::FairValueGap, 1);
        let b = PdaToken::new(PdaTokenKind::LiquiditySweep, 1);
        let cost = pda_token_cost(&a, &b);
        assert!(
            cost >= 0.6,
            "kind mismatch should contribute at least 0.6, got {cost}"
        );
    }

    #[test]
    fn metadata_deltas_scale_within_expected_bounds() {
        let a = PdaToken::new(PdaTokenKind::OrderBlock, 1)
            .with_overlap(0.0)
            .with_liquidity_swept(false)
            .with_volume_imbalance(-1.0);
        let b = PdaToken::new(PdaTokenKind::OrderBlock, 1)
            .with_overlap(1.0)
            .with_liquidity_swept(true)
            .with_volume_imbalance(1.0);
        let cost = pda_token_cost(&a, &b);
        // kind matches → kind_cost=0; max metadata delta = 0.15 + 0.15 + 0.10 = 0.40
        assert!(
            cost <= 0.40 + 1e-9,
            "metadata-only cost must stay ≤ 0.40, got {cost}"
        );
    }

    #[test]
    fn overlap_and_imbalance_are_clamped() {
        let token = PdaToken::new(PdaTokenKind::Cisd, 0)
            .with_overlap(5.0)
            .with_volume_imbalance(-12.0);
        assert!((token.overlap - 1.0).abs() < f64::EPSILON);
        assert!((token.volume_imbalance_ratio + 1.0).abs() < f64::EPSILON);
    }
}
