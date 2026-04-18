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
            regime_membership: std::collections::BTreeMap::new(),
            feature_attribution: std::collections::BTreeMap::new(),
            evidence: vec!["phase1".into()],
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
