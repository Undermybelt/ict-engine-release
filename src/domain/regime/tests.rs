use super::types::{RegimeSegmentationPacket, RegimeValidationPacket, StructuralBreakPacket};

#[cfg(test)]
mod packet_tests {
    use super::*;

    #[test]
    fn regime_segmentation_packet_round_trip() {
        let mut packet = RegimeSegmentationPacket {
            method: "jump_model".into(),
            segmentation_version: "v1".into(),
            active_regime_cluster: Some("trend_high_vol".into()),
            transition_hazard: Some(0.23),
            duration_elapsed_bars: None,
            duration_model: None,
            duration_remaining_expected_bars: None,
            regime_membership: std::collections::BTreeMap::new(),
            feature_attribution: std::collections::BTreeMap::new(),
            evidence: vec!["phase1".into()],
            wasserstein_label: None,
            wasserstein_distance: None,
            governor_confidence: None,
            governor_entropy: None,
            governor_min_hold_active: None,
            timeframe_alignment: None,
            timeframe_alignment_score: None,
        };
        packet
            .regime_membership
            .insert("trend_high_vol".into(), 0.81);
        packet.feature_attribution.insert("volatility".into(), 0.44);

        let json = serde_json::to_string(&packet).expect("serialize segmentation packet");
        let parsed: RegimeSegmentationPacket =
            serde_json::from_str(&json).expect("deserialize segmentation packet");
        assert_eq!(parsed.method, packet.method);
        assert_eq!(parsed.segmentation_version, packet.segmentation_version);
        assert_eq!(
            parsed.regime_membership.get("trend_high_vol").copied(),
            Some(0.81)
        );
    }

    #[test]
    fn regime_segmentation_packet_round_trip_with_hybrid_fields() {
        let mut packet = RegimeSegmentationPacket {
            method: "hybrid_regime_first_pass_v1".into(),
            segmentation_version: "v2".into(),
            active_regime_cluster: Some("range_calm".into()),
            transition_hazard: Some(0.18),
            duration_elapsed_bars: Some(3),
            duration_model: Some("negative_binomial".into()),
            duration_remaining_expected_bars: Some(4.5),
            regime_membership: std::collections::BTreeMap::new(),
            feature_attribution: std::collections::BTreeMap::new(),
            evidence: vec!["governor_commit=true".into()],
            wasserstein_label: Some("range_calm".into()),
            wasserstein_distance: Some(0.12),
            governor_confidence: Some(0.74),
            governor_entropy: Some(0.81),
            governor_min_hold_active: Some(false),
            timeframe_alignment: Some(true),
            timeframe_alignment_score: Some(1.0),
        };
        packet.regime_membership.insert("range_calm".into(), 0.74);
        packet
            .regime_membership
            .insert("trend_impulse".into(), 0.26);

        let json = serde_json::to_string(&packet).expect("serialize segmentation packet");
        let parsed: RegimeSegmentationPacket =
            serde_json::from_str(&json).expect("deserialize segmentation packet");
        assert_eq!(parsed.wasserstein_label.as_deref(), Some("range_calm"));
        assert_eq!(parsed.timeframe_alignment, Some(true));
        assert_eq!(parsed.governor_confidence, Some(0.74));
        assert_eq!(parsed.duration_elapsed_bars, Some(3));
    }

    #[test]
    fn structural_break_packet_round_trip() {
        let packet = StructuralBreakPacket {
            method: "cusum".into(),
            break_family: "nonparametric".into(),
            detected: true,
            break_score: Some(0.73),
            break_index: Some(112),
            lookback_window: Some(240),
            affected_features: vec!["range".into(), "volatility".into()],
            rationale: vec!["phase1".into()],
        };

        let json = serde_json::to_string(&packet).expect("serialize break packet");
        let parsed: StructuralBreakPacket =
            serde_json::from_str(&json).expect("deserialize break packet");
        assert!(parsed.detected);
        assert_eq!(parsed.break_family, "nonparametric");
        assert_eq!(parsed.break_index, Some(112));
    }

    #[test]
    fn regime_validation_packet_round_trip() {
        let packet = RegimeValidationPacket {
            validation_scope: "backtest_split".into(),
            segmentation_consistency: Some(0.66),
            hindsight_risk_flags: vec!["label_leakage_risk".into()],
            abstain_recommended: true,
            notes: vec!["validator".into()],
        };

        let json = serde_json::to_string(&packet).expect("serialize validation packet");
        let parsed: RegimeValidationPacket =
            serde_json::from_str(&json).expect("deserialize validation packet");
        assert_eq!(parsed.validation_scope, "backtest_split");
        assert!(parsed.abstain_recommended);
    }
}
