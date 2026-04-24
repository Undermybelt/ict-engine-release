use anyhow::Result;
use serde::Serialize;

use crate::application::multi_timeframe_inputs::resolve_tomac_root;

pub struct ExpansionSopCommandInput<'a> {
    pub root: Option<&'a str>,
    pub output_dir: &'a str,
    pub interval: &'a str,
    pub lookback: usize,
    pub atr_multiplier: f64,
    pub objective: &'a str,
    pub mutation_spec_path: Option<&'a str>,
    pub emit_mutation_evaluation: bool,
}

pub fn clean_futures_command<FMulti, FSingle, TMulti, TSingle>(
    root: Option<&str>,
    output_dir: &str,
    interval: &str,
    multi_timeframe: bool,
    run_multi_timeframe: FMulti,
    run_single: FSingle,
) -> Result<()>
where
    FMulti: Fn(&str, &str) -> Result<TMulti>,
    FSingle: Fn(&str, &str, &str) -> Result<TSingle>,
    TMulti: Serialize,
    TSingle: Serialize,
{
    let root = resolve_tomac_root(root)?;
    if multi_timeframe {
        let report = run_multi_timeframe(&root, output_dir)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        let report = run_single(&root, output_dir, interval)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
    }
    Ok(())
}

pub fn futures_sop_command<FRun, TReport>(
    root: Option<&str>,
    output_dir: &str,
    interval: &str,
    run_sop: FRun,
) -> Result<()>
where
    FRun: Fn(&str, &str, &str) -> Result<TReport>,
    TReport: Serialize,
{
    let root = resolve_tomac_root(root)?;
    let report = run_sop(&root, output_dir, interval)?;
    let report_path = std::path::Path::new(output_dir)
        .join(format!("futures_sop_report.{}.json", interval))
        .to_string_lossy()
        .to_string();
    std::fs::write(&report_path, serde_json::to_string_pretty(&report)?)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

pub fn expansion_sop_command<FLoad, FObjective, FRun, FPayload, TMutationSpec, TReport>(
    input: ExpansionSopCommandInput<'_>,
    parse_objective: FObjective,
    load_mutation_spec: FLoad,
    run_sop: FRun,
    build_payload: FPayload,
) -> Result<()>
where
    FLoad: Fn(&str) -> Result<TMutationSpec>,
    FObjective: Fn(&str) -> Result<crate::application::decision_utils::ResearchObjectiveMode>,
    FRun: Fn(
        &str,
        &str,
        &str,
        usize,
        f64,
        crate::application::decision_utils::ResearchObjectiveMode,
        Option<&TMutationSpec>,
    ) -> Result<TReport>,
    FPayload: Fn(&TReport, Option<&TMutationSpec>, bool) -> Result<serde_json::Value>,
    TMutationSpec: Serialize,
    TReport: Serialize,
{
    let ExpansionSopCommandInput {
        root,
        output_dir,
        interval,
        lookback,
        atr_multiplier,
        objective,
        mutation_spec_path,
        emit_mutation_evaluation,
    } = input;
    let objective_mode = parse_objective(objective)?;
    let root = resolve_tomac_root(root)?;
    let mutation_spec = mutation_spec_path.map(load_mutation_spec).transpose()?;
    let report = run_sop(
        &root,
        output_dir,
        interval,
        lookback,
        atr_multiplier,
        objective_mode,
        mutation_spec.as_ref(),
    )?;
    let report_path = std::path::Path::new(output_dir)
        .join(format!("expansion_sop_report.{}.json", interval))
        .to_string_lossy()
        .to_string();
    std::fs::write(&report_path, serde_json::to_string_pretty(&report)?)?;
    let payload = build_payload(&report, mutation_spec.as_ref(), emit_mutation_evaluation)?;
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    #[test]
    fn clean_futures_command_routes_to_single_interval_path() {
        let called_single = Arc::new(AtomicBool::new(false));
        let single_flag = Arc::clone(&called_single);

        clean_futures_command(
            Some("/tmp/root"),
            "/tmp/out",
            "15m",
            false,
            |_root, _out| Ok(json!({"mode": "multi"})),
            move |root, out, interval| {
                assert_eq!(root, "/tmp/root");
                assert_eq!(out, "/tmp/out");
                assert_eq!(interval, "15m");
                single_flag.store(true, Ordering::SeqCst);
                Ok(json!({"mode": "single"}))
            },
        )
        .unwrap();

        assert!(called_single.load(Ordering::SeqCst));
    }

    #[test]
    fn futures_sop_command_persists_report_file() {
        let temp = tempfile::tempdir().unwrap();
        futures_sop_command(
            Some("/tmp/root"),
            temp.path().to_str().unwrap(),
            "15m",
            |_root, _out, _interval| Ok(json!({"report": true})),
        )
        .unwrap();

        let report_path = temp.path().join("futures_sop_report.15m.json");
        assert!(report_path.exists());
    }

    #[test]
    fn expansion_sop_command_writes_payload_and_report() {
        let temp = tempfile::tempdir().unwrap();
        expansion_sop_command(
            ExpansionSopCommandInput {
                root: Some("/tmp/root"),
                output_dir: temp.path().to_str().unwrap(),
                interval: "15m",
                lookback: 20,
                atr_multiplier: 1.5,
                objective: "generic",
                mutation_spec_path: None,
                emit_mutation_evaluation: false,
            },
            |_objective| Ok(crate::application::decision_utils::ResearchObjectiveMode::Generic),
            |_path| Ok(json!({"mutation": true})),
            |_root, _out, _interval, _lookback, _atr_multiplier, _objective, _mutation_spec| {
                Ok(json!({"report": true}))
            },
            |report, mutation_spec, emit_mutation_evaluation| {
                Ok(json!({
                    "report": report,
                    "mutation_spec": mutation_spec,
                    "emit_mutation_evaluation": emit_mutation_evaluation
                }))
            },
        )
        .unwrap();

        let report_path = temp.path().join("expansion_sop_report.15m.json");
        assert!(report_path.exists());
    }
}
