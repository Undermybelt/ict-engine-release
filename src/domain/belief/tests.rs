use super::types::{ConformalUncertaintyPacket, MarketPolicyPacket, MicrostructureContextPacket};

#[cfg(test)]
mod packet_tests {
    use super::*;

    #[test]
    fn conformal_uncertainty_packet_round_trip() {
        let packet = ConformalUncertaintyPacket {
            method: "temporal_conformal".into(),
            target: "next_return".into(),
            nominal_coverage: 0.9,
            empirical_coverage: Some(0.88),
            interval_width: Some(1.2),
            nonconformity_score: Some(0.33),
            abstain_threshold: Some(0.4),
            abstain: false,
            notes: vec!["phase1".into()],
        };

        let json = serde_json::to_string(&packet).expect("serialize conformal packet");
        let parsed: ConformalUncertaintyPacket =
            serde_json::from_str(&json).expect("deserialize conformal packet");
        assert_eq!(parsed.method, packet.method);
        assert_eq!(parsed.target, packet.target);
        assert_eq!(parsed.nominal_coverage, packet.nominal_coverage);
        assert_eq!(parsed.abstain, packet.abstain);
    }

    #[test]
    fn microstructure_context_packet_defaults_evidence_false() {
        let packet = MicrostructureContextPacket::default();
        assert!(!packet.usable_as_evidence);
    }

    #[test]
    fn market_policy_packet_round_trip() {
        let mut packet = MarketPolicyPacket {
            market_family: Some("index_futures".into()),
            market_behavior_profile: Some("mean_revert".into()),
            policy_mode: "phase1".into(),
            evidence_reliability: std::collections::BTreeMap::new(),
            abstention_bias: Some(0.2),
            notes: vec!["packet-only".into()],
        };
        packet.evidence_reliability.insert("bbn".into(), 0.7);

        let json = serde_json::to_string(&packet).expect("serialize market policy packet");
        let parsed: MarketPolicyPacket =
            serde_json::from_str(&json).expect("deserialize market policy packet");
        assert_eq!(parsed.policy_mode, "phase1");
        assert_eq!(parsed.evidence_reliability.get("bbn").copied(), Some(0.7));
    }
}
