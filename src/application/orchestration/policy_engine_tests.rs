use super::*;

#[test]
fn catboost_policy_engine_loads_sample_artifact() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/application/orchestration/catboost_policy.sample.json");
    let engine = CatBoostCompatiblePolicyEngine::load_from_file(&path).unwrap();
    assert_eq!(engine.artifact_version(), "catboost-policy-v1-sample");
    assert_eq!(engine.model_artifact.model_family, "catboost");
    assert_eq!(
        engine.model_artifact.feature_schema_version,
        "policy_features_v2_execution_setup"
    );
}

#[test]
fn catboost_policy_engine_infer_uses_loaded_artifact_version() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/application/orchestration/catboost_policy.sample.json");
    let engine = CatBoostCompatiblePolicyEngine::load_from_file(&path).unwrap();
    let decision = engine.infer(&PolicyFeatureVector {
        factor_alignment: "mixed".to_string(),
        factor_uncertainty: "low".to_string(),
        gating_status: "trend".to_string(),
        selected_entry_quality: "medium".to_string(),
        recommended_command: "update".to_string(),
        evidence_quality_score: 0.82,
        selected_direction: "Bull".to_string(),
        risk_reward: 2.4,
        kelly_fraction: 0.12,
        setup_family: "order_block".to_string(),
        entry_style: "market_confirmation".to_string(),
        risk_template: "structure_external".to_string(),
        setup_quality: "high".to_string(),
        signal_bar_pattern: "displacement".to_string(),
        session_model: "silver_bullet".to_string(),
        higher_tf_bias_match: true,
        discount_premium_correct: true,
        liquidity_swept: true,
        signal_bar_present: true,
        pda_signal_overlap: true,
        timed_pda_active_nearby: true,
        timed_pda_inversed_nearby: false,
        timed_pda_stale_nearby: false,
        pda_distance_bps: 12.0,
        pda_width_bps: 18.0,
        overlap_ratio: 0.75,
        displacement_strength: 0.82,
        sweep_depth_bps: 24.0,
        entry_price_offset_bps: 4.0,
        sl_distance_bps: 14.0,
        tp_rr_ratio: 2.4,
    });
    assert_eq!(decision.policy_version, "catboost-policy-v1-sample");
    assert_eq!(decision.qualification, "qualified");
    assert_eq!(decision.action, "Bull");
    assert_eq!(decision.leaf_id, "qualified-bull");
    assert!(decision.recommended_command.contains("ict-engine update"));
}

#[test]
fn catboost_policy_engine_falls_back_when_feature_conditions_do_not_match() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("src/application/orchestration/catboost_policy.sample.json");
    let engine = CatBoostCompatiblePolicyEngine::load_from_file(&path).unwrap();
    let decision = engine.infer(&PolicyFeatureVector {
        factor_alignment: "bearish".to_string(),
        factor_uncertainty: "low".to_string(),
        gating_status: "transition".to_string(),
        selected_entry_quality: "low".to_string(),
        recommended_command: "update".to_string(),
        evidence_quality_score: 0.82,
        selected_direction: "Bull".to_string(),
        risk_reward: 2.4,
        kelly_fraction: 0.12,
        setup_family: "none".to_string(),
        entry_style: "observe".to_string(),
        risk_template: "observe_only".to_string(),
        setup_quality: "low".to_string(),
        signal_bar_pattern: "none".to_string(),
        session_model: "standard".to_string(),
        higher_tf_bias_match: false,
        discount_premium_correct: false,
        liquidity_swept: false,
        signal_bar_present: false,
        pda_signal_overlap: false,
        timed_pda_active_nearby: false,
        timed_pda_inversed_nearby: false,
        timed_pda_stale_nearby: true,
        pda_distance_bps: 0.0,
        pda_width_bps: 0.0,
        overlap_ratio: 0.0,
        displacement_strength: 0.0,
        sweep_depth_bps: 0.0,
        entry_price_offset_bps: 0.0,
        sl_distance_bps: 0.0,
        tp_rr_ratio: 2.4,
    });
    assert_ne!(decision.leaf_id, "qualified-bull");
}

#[test]
fn catboost_policy_engine_load_default_or_placeholder_prefers_file() {
    let engine = CatBoostCompatiblePolicyEngine::load_default_or_placeholder();
    assert_eq!(engine.artifact_version(), "catboost-policy-v1-sample");
}
