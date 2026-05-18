use ict_engine::application::regime::consumer_bundle_adapter::{
    BundleStatus, ExecutionTreeHint, RegimeBbnEvidenceApplicationStatus, RegimeBbnEvidenceStrength,
    RegimeConsumerBundleAdapter,
};
use ict_engine::bbn::adapters::belief_evidence_packet_from_pre_bayes_filter;
use ict_engine::state::PreBayesEvidenceFilter;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;

#[test]
fn disabled_adapter_is_noop_default() {
    let adapter = RegimeConsumerBundleAdapter::load_optional(None, false).unwrap();

    assert_eq!(adapter.status, BundleStatus::Disabled);
    assert!(!adapter.is_loaded());
    assert_eq!(
        adapter.execution_tree_hint(),
        ExecutionTreeHint::UnknownAbstain
    );
    assert!(adapter.bbn_evidence_hint().is_none());
}

#[test]
fn valid_bundle_loads_known_fields() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "single_label_99",
                "trade_usable": true,
                "final_label": "primary::TrendExpansion",
                "label_set": ["primary::TrendExpansion"],
                "abstain_reasons": []
            },
            "consumer_hints": {
                "execution_tree_hint": "accept_regime",
                "bbn_evidence_hint": {"regime_trade_usable": true},
                "path_ranker_context": {"regime_label": "primary::TrendExpansion"},
                "user_vrp_nq_context": {"qqq_hv_level": 0.22}
            }
        })
        .to_string(),
    )
    .unwrap();

    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();

    assert_eq!(adapter.status, BundleStatus::Loaded);
    assert!(adapter.is_loaded());
    assert_eq!(
        adapter.execution_tree_hint(),
        ExecutionTreeHint::AcceptRegime
    );
    assert_eq!(
        adapter.latest_decision.as_ref().unwrap().decision_state,
        "single_label_99"
    );
    assert!(adapter.latest_decision.as_ref().unwrap().trade_usable);
    assert!(adapter.bbn_evidence_hint().is_some());
}

#[test]
fn path_ranker_context_single_branch_path_emits_assignment_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    let branch_path = "Crisis -> CrisisReliefCarry -> StopManagedPanicRecovery -> SourceRootStopCarryLongHorizonV1:crisis_carry_h8_sl048_tp12";
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "single_label_95",
                "trade_usable": true,
                "final_label": "primary::ExtremeStress",
                "label_set": ["primary::ExtremeStress"],
                "abstain_reasons": []
            },
            "consumer_hints": {
                "execution_tree_hint": "accept_regime",
                "path_ranker_context": {
                    "regime_profit_branch_path": branch_path,
                    "stable_profit_score": 85.7407
                }
            }
        })
        .to_string(),
    )
    .unwrap();

    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let entries = adapter.path_ranker_assignment_entries();
    let branch_paths_json = entries
        .iter()
        .find(|(key, _)| key == "regime_bundle_branch_paths_json")
        .map(|(_, value)| value)
        .expect("branch path assignment");
    let branch_paths: Vec<String> = serde_json::from_str(branch_paths_json).unwrap();

    assert_eq!(branch_paths, vec![branch_path.to_string()]);
    assert_eq!(
        entries
            .iter()
            .find(|(key, _)| key == "regime_bundle_stable_profit_score")
            .map(|(_, value)| value.as_str()),
        Some("0.857407")
    );
}

#[test]
fn single_branch_path_survives_pre_bayes_into_bbn_assignments() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    let branch_path = "Crisis -> CrisisReliefCarry -> StopManagedPanicRecovery -> SourceRootStopCarryLongHorizonV1:crisis_carry_h8_sl048_tp12";
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "single_label_95",
                "trade_usable": true,
                "final_label": "primary::ExtremeStress",
                "label_set": ["primary::ExtremeStress", branch_path],
                "abstain_reasons": []
            },
            "consumer_hints": {
                "execution_tree_hint": "accept_regime",
                "bbn_evidence_hint": {
                    "regime_decision_state": "single_label_95",
                    "regime_trade_usable": true,
                    "regime_label": "primary::ExtremeStress",
                    "regime_label_set": ["primary::ExtremeStress", branch_path],
                    "regime_transition_hazard": 0.0,
                    "regime_decision_reasons": ["branch_rc_spa_passed", "root=Crisis"]
                },
                "path_ranker_context": {
                    "regime_profit_branch_path": branch_path,
                    "stable_profit_score": 85.7407
                }
            }
        })
        .to_string(),
    )
    .unwrap();

    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let mut filter = PreBayesEvidenceFilter::default();
    for (key, value) in adapter.path_ranker_assignment_entries() {
        filter.evidence_assignments.insert(key, value);
    }
    adapter.append_read_only_bbn_filter_diagnostics(&mut filter);

    assert_eq!(
        filter.evidence_assignments["regime_profit_branch_path"],
        branch_path
    );
    assert_eq!(filter.evidence_assignments["parent_regime_root"], "Crisis");
    assert_eq!(filter.evidence_assignments["main_regime"], "Crisis");
    assert_eq!(
        filter.evidence_assignments["sub_regime"],
        "CrisisReliefCarry"
    );
    assert_eq!(
        filter.evidence_assignments["sub_sub_regime_or_profit_factor"],
        "StopManagedPanicRecovery"
    );
    assert_eq!(
        filter.evidence_assignments["profit_factor"],
        "SourceRootStopCarryLongHorizonV1:crisis_carry_h8_sl048_tp12"
    );

    let packet =
        belief_evidence_packet_from_pre_bayes_filter("NQ", Some("NQ"), &filter, None, None, None);

    assert_eq!(
        packet.evidence_assignments["regime_profit_branch_path"],
        branch_path
    );
    assert_eq!(packet.evidence_assignments["parent_regime_root"], "Crisis");
    assert_eq!(
        packet.evidence_assignments["profit_factor"],
        "SourceRootStopCarryLongHorizonV1:crisis_carry_h8_sl048_tp12"
    );
}

#[test]
fn missing_bundle_non_strict_is_neutral_noop() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.json");

    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();

    assert_eq!(adapter.status, BundleStatus::Missing);
    assert!(!adapter.is_loaded());
    assert_eq!(
        adapter.execution_tree_hint(),
        ExecutionTreeHint::UnknownAbstain
    );
    assert!(adapter.error.as_ref().unwrap().contains("missing"));
}

#[test]
fn missing_bundle_strict_errors_before_state_mutation() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.json");

    let err = RegimeConsumerBundleAdapter::load_optional(Some(&path), true).unwrap_err();

    assert!(err.to_string().contains("missing"));
}

#[test]
fn invalid_schema_non_strict_is_neutral_noop() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.json");
    fs::write(&path, json!({"schema_version": "wrong/v1"}).to_string()).unwrap();

    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();

    assert_eq!(adapter.status, BundleStatus::Invalid);
    assert!(!adapter.is_loaded());
    assert_eq!(
        adapter.execution_tree_hint(),
        ExecutionTreeHint::UnknownAbstain
    );
}

#[test]
fn invalid_schema_strict_errors() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("invalid.json");
    fs::write(&path, json!({"schema_version": "wrong/v1"}).to_string()).unwrap();

    let err = RegimeConsumerBundleAdapter::load_optional(Some(&path), true).unwrap_err();

    assert!(err.to_string().contains("schema"));
}

#[test]
fn loaded_adapter_emits_compact_read_only_trace_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "single_label_95",
                "trade_usable": true,
                "final_label": "primary::TrendExpansion",
                "label_set": ["primary::TrendExpansion"],
                "abstain_reasons": []
            },
            "consumer_hints": {"execution_tree_hint": "accept_regime"}
        })
        .to_string(),
    )
    .unwrap();

    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let trace = adapter.trace_entries(Some(&path));

    assert!(trace.contains(&"regime_bundle_status=loaded".to_string()));
    assert!(trace
        .iter()
        .any(|line| line.starts_with("regime_bundle_path=")));
    assert!(trace.contains(&"regime_decision_state=single_label_95".to_string()));
    assert!(trace.contains(&"regime_trade_usable=true".to_string()));
    assert!(trace.contains(&"regime_execution_tree_hint=accept_regime".to_string()));
}

#[test]
fn missing_adapter_emits_neutral_trace_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.json");

    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let trace = adapter.trace_entries(Some(&path));

    assert!(trace.contains(&"regime_bundle_status=missing".to_string()));
    assert!(trace
        .iter()
        .any(|line| line.starts_with("regime_bundle_error=")));
    assert!(trace.contains(&"regime_execution_tree_hint=unknown_abstain".to_string()));
}

#[test]
fn single_label_99_maps_to_strong_read_only_bbn_soft_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "single_label_99",
                "trade_usable": true,
                "final_label": "primary::TrendExpansion",
                "label_set": ["primary::TrendExpansion"],
                "abstain_reasons": []
            },
            "consumer_hints": {
                "execution_tree_hint": "accept_regime",
                "bbn_evidence_hint": {
                    "regime_decision_state": "single_label_99",
                    "regime_trade_usable": true,
                    "regime_label": "primary::TrendExpansion",
                    "regime_transition_hazard": 0.0,
                    "regime_decision_reasons": []
                }
            }
        })
        .to_string(),
    )
    .unwrap();

    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let evidence = adapter.to_read_only_bbn_soft_evidence();

    assert_eq!(evidence.strength, RegimeBbnEvidenceStrength::Strong);
    assert_eq!(evidence.label.as_deref(), Some("primary::TrendExpansion"));
    assert_eq!(evidence.decision_state, "single_label_99");
    assert_eq!(evidence.trade_usable, Some(true));
    assert!(evidence.weight > 0.85);
    assert!(evidence.reasons.is_empty());
}

#[test]
fn single_label_95_maps_to_moderate_read_only_bbn_soft_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "single_label_95",
                "trade_usable": true,
                "final_label": "primary::TrendExpansion",
                "label_set": ["primary::TrendExpansion"],
                "abstain_reasons": []
            },
            "consumer_hints": {
                "execution_tree_hint": "accept_regime",
                "bbn_evidence_hint": {
                    "regime_decision_state": "single_label_95",
                    "regime_trade_usable": true,
                    "regime_label": "primary::TrendExpansion",
                    "regime_transition_hazard": 0.15,
                    "regime_decision_reasons": []
                }
            }
        })
        .to_string(),
    )
    .unwrap();

    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let evidence = adapter.to_read_only_bbn_soft_evidence();

    assert_eq!(evidence.strength, RegimeBbnEvidenceStrength::Moderate);
    assert_eq!(evidence.label.as_deref(), Some("primary::TrendExpansion"));
    assert_eq!(evidence.weight, 0.65);
}

#[test]
fn loaded_adapter_emits_compact_bbn_soft_evidence_trace_entries() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "single_label_99",
                "trade_usable": true,
                "final_label": "primary::TrendExpansion",
                "label_set": ["primary::TrendExpansion"],
                "abstain_reasons": []
            },
            "consumer_hints": {
                "execution_tree_hint": "accept_regime",
                "bbn_evidence_hint": {
                    "regime_decision_state": "single_label_99",
                    "regime_trade_usable": true,
                    "regime_label": "primary::TrendExpansion",
                    "regime_transition_hazard": 0.03,
                    "regime_decision_reasons": []
                }
            }
        })
        .to_string(),
    )
    .unwrap();

    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let trace = adapter.bbn_soft_evidence_trace_entries();

    assert!(trace.contains(&"regime_bbn_soft_evidence_strength=strong".to_string()));
    assert!(trace.contains(&"regime_bbn_soft_evidence_weight=0.900".to_string()));
    assert!(trace.contains(&"regime_bbn_decision_state=single_label_99".to_string()));
    assert!(trace.contains(&"regime_bbn_label=primary::TrendExpansion".to_string()));
    assert!(trace.contains(&"regime_bbn_transition_hazard=0.030".to_string()));
}

#[test]
fn strong_bundle_applies_to_pre_bayes_soft_market_regime_when_opted_in() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "single_label_99",
                "trade_usable": true,
                "final_label": "primary::TrendExpansion",
                "label_set": ["primary::TrendExpansion"],
                "abstain_reasons": []
            },
            "consumer_hints": {
                "execution_tree_hint": "accept_regime",
                "bbn_evidence_hint": {
                    "regime_decision_state": "single_label_99",
                    "regime_trade_usable": true,
                    "regime_label": "primary::TrendExpansion",
                    "regime_transition_hazard": 0.03,
                    "regime_decision_reasons": []
                }
            }
        })
        .to_string(),
    )
    .unwrap();
    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let mut filter = PreBayesEvidenceFilter {
        uses_soft_evidence: false,
        filtered_market_regime_label: "range".to_string(),
        soft_market_regime_distribution: BTreeMap::from([
            ("bull".to_string(), 0.2),
            ("bear".to_string(), 0.2),
            ("range".to_string(), 0.6),
        ]),
        ..PreBayesEvidenceFilter::default()
    };

    let status = adapter.apply_bbn_soft_evidence_to_pre_bayes_filter(&mut filter, true);

    assert_eq!(status, RegimeBbnEvidenceApplicationStatus::Applied);
    assert!(filter.uses_soft_evidence);
    assert_eq!(filter.filtered_market_regime_label, "bull");
    assert_eq!(filter.soft_market_regime_distribution["bull"], 0.9);
    assert!(filter.rationale.contains(
        &"regime_bundle_bbn_evidence_applied=strength:strong label:primary::TrendExpansion"
            .to_string()
    ));
}

#[test]
fn bear_relief_bundle_applies_to_bear_pre_bayes_soft_evidence() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "single_label_95",
                "trade_usable": true,
                "final_label": "primary::BearReliefCarry",
                "label_set": [
                    "primary::BearReliefCarry",
                    "Bear -> BearReliefCarry -> StopManagedRecoveryCarry -> SourceRootStopCarryLongHorizonV1:bear_carry_h20_sl048_tp12"
                ],
                "abstain_reasons": []
            },
            "consumer_hints": {
                "execution_tree_hint": "accept_regime",
                "bbn_evidence_hint": {
                    "regime_decision_state": "single_label_95",
                    "regime_trade_usable": true,
                    "regime_label": "primary::BearReliefCarry",
                    "regime_label_set": [
                        "primary::BearReliefCarry",
                        "Bear -> BearReliefCarry -> StopManagedRecoveryCarry -> SourceRootStopCarryLongHorizonV1:bear_carry_h20_sl048_tp12"
                    ],
                    "regime_transition_hazard": 0.0,
                    "regime_decision_reasons": ["branch_rc_spa_passed", "root=Bear"]
                }
            }
        })
        .to_string(),
    )
    .unwrap();
    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let mut filter = PreBayesEvidenceFilter {
        uses_soft_evidence: false,
        filtered_market_regime_label: "range".to_string(),
        soft_market_regime_distribution: BTreeMap::from([
            ("bull".to_string(), 0.2),
            ("bear".to_string(), 0.2),
            ("range".to_string(), 0.6),
        ]),
        ..PreBayesEvidenceFilter::default()
    };

    let status = adapter.apply_bbn_soft_evidence_to_pre_bayes_filter(&mut filter, true);

    assert_eq!(status, RegimeBbnEvidenceApplicationStatus::Applied);
    assert!(filter.uses_soft_evidence);
    assert_eq!(filter.filtered_market_regime_label, "bear");
    assert_eq!(filter.soft_market_regime_distribution["bear"], 0.65);
    assert!(filter.rationale.contains(
        &"regime_bundle_bbn_evidence_applied=strength:moderate label:primary::BearReliefCarry"
            .to_string()
    ));
}

#[test]
fn absent_opt_in_or_neutral_bundle_skips_pre_bayes_mutation() {
    let adapter = RegimeConsumerBundleAdapter::disabled();
    let mut filter = PreBayesEvidenceFilter {
        uses_soft_evidence: false,
        filtered_market_regime_label: "range".to_string(),
        soft_market_regime_distribution: BTreeMap::from([
            ("bull".to_string(), 0.2),
            ("bear".to_string(), 0.2),
            ("range".to_string(), 0.6),
        ]),
        ..PreBayesEvidenceFilter::default()
    };

    let disabled_status = adapter.apply_bbn_soft_evidence_to_pre_bayes_filter(&mut filter, true);
    assert_eq!(disabled_status, RegimeBbnEvidenceApplicationStatus::Skipped);
    assert_eq!(filter.filtered_market_regime_label, "range");
    assert_eq!(filter.soft_market_regime_distribution["range"], 0.6);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "unknown_abstain",
                "trade_usable": false,
                "final_label": "",
                "label_set": [],
                "abstain_reasons": ["confidence_gate_failed"]
            },
            "consumer_hints": {
                "execution_tree_hint": "unknown_abstain",
                "bbn_evidence_hint": {
                    "regime_decision_state": "unknown_abstain",
                    "regime_trade_usable": false,
                    "regime_label": "primary::RangeConsolidation",
                    "regime_transition_hazard": 0.8,
                    "regime_decision_reasons": ["confidence_gate_failed"]
                }
            }
        })
        .to_string(),
    )
    .unwrap();
    let neutral = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let no_opt_status = neutral.apply_bbn_soft_evidence_to_pre_bayes_filter(&mut filter, false);
    assert_eq!(no_opt_status, RegimeBbnEvidenceApplicationStatus::Skipped);
    let neutral_status = neutral.apply_bbn_soft_evidence_to_pre_bayes_filter(&mut filter, true);
    assert_eq!(neutral_status, RegimeBbnEvidenceApplicationStatus::Skipped);
    assert_eq!(filter.filtered_market_regime_label, "range");
}

#[test]
fn accepted_legacy_bundle_applies_as_moderate_range_when_opted_in() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("regime_consumer_bundle.json");
    fs::write(
        &path,
        json!({
            "schema_version": "regime-consumer-bundle/v1",
            "latest_decision": {
                "decision_state": "accepted",
                "trade_usable": true,
                "final_label": "RangeConsolidation/WideRange",
                "label_set": ["RangeConsolidation", "WideRange"],
                "abstain_reasons": []
            },
            "consumer_hints": {
                "execution_tree_hint": "accept_regime",
                "bbn_evidence_hint": {
                    "market_regime": "range",
                    "liquidity_context": "favorable"
                }
            }
        })
        .to_string(),
    )
    .unwrap();
    let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
    let mut filter = PreBayesEvidenceFilter {
        filtered_market_regime_label: "bull".to_string(),
        soft_market_regime_distribution: BTreeMap::from([
            ("bull".to_string(), 0.6),
            ("bear".to_string(), 0.2),
            ("range".to_string(), 0.2),
        ]),
        ..PreBayesEvidenceFilter::default()
    };

    let evidence = adapter.to_read_only_bbn_soft_evidence();
    let status = adapter.apply_bbn_soft_evidence_to_pre_bayes_filter(&mut filter, true);

    assert_eq!(evidence.strength, RegimeBbnEvidenceStrength::Moderate);
    assert_eq!(status, RegimeBbnEvidenceApplicationStatus::Applied);
    assert_eq!(filter.filtered_market_regime_label, "range");
    assert_eq!(filter.soft_market_regime_distribution["range"], 0.65);
}

#[test]
fn abstain_or_missing_bundle_maps_to_neutral_bbn_soft_evidence() {
    let missing = RegimeConsumerBundleAdapter::load_optional(None, false).unwrap();
    let missing_evidence = missing.to_read_only_bbn_soft_evidence();
    assert_eq!(
        missing_evidence.strength,
        RegimeBbnEvidenceStrength::Neutral
    );
    assert_eq!(missing_evidence.weight, 0.0);

    let dir = tempfile::tempdir().unwrap();
    let invalid_path = dir.path().join("invalid.json");
    fs::write(
        &invalid_path,
        json!({"schema_version": "wrong/v1"}).to_string(),
    )
    .unwrap();
    let invalid = RegimeConsumerBundleAdapter::load_optional(Some(&invalid_path), false).unwrap();
    let invalid_evidence = invalid.to_read_only_bbn_soft_evidence();
    assert_eq!(
        invalid_evidence.strength,
        RegimeBbnEvidenceStrength::Neutral
    );
    assert_eq!(invalid_evidence.weight, 0.0);

    for decision_state in ["label_set", "transitional", "unknown_abstain"] {
        let path = dir.path().join(format!("{decision_state}.json"));
        fs::write(
            &path,
            json!({
                "schema_version": "regime-consumer-bundle/v1",
                "latest_decision": {
                    "decision_state": decision_state,
                    "trade_usable": false,
                    "final_label": "",
                    "label_set": ["primary::TrendExpansion", "primary::RangeConsolidation"],
                    "abstain_reasons": ["transition_guardrail"]
                },
                "consumer_hints": {
                    "execution_tree_hint": if decision_state == "unknown_abstain" { "unknown_abstain" } else { "transition_guardrail" },
                    "bbn_evidence_hint": {
                        "regime_decision_state": decision_state,
                        "regime_trade_usable": false,
                        "regime_label_set": ["primary::TrendExpansion", "primary::RangeConsolidation"],
                        "regime_transition_hazard": 0.72,
                        "regime_decision_reasons": ["transition_guardrail"]
                    }
                }
            })
            .to_string(),
        )
        .unwrap();

        let adapter = RegimeConsumerBundleAdapter::load_optional(Some(&path), false).unwrap();
        let evidence = adapter.to_read_only_bbn_soft_evidence();

        assert_eq!(evidence.strength, RegimeBbnEvidenceStrength::Neutral);
        assert_eq!(evidence.weight, 0.0);
        assert_eq!(evidence.transition_hazard, Some(0.72));
        assert!(evidence
            .reasons
            .contains(&"transition_guardrail".to_string()));
    }
}
