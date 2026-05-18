use anyhow::Result;
use csv::StringRecord;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutoQuantStrategyMaterialSummary {
    pub name: String,
    pub strategy_path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_csv_path: Option<String>,
    #[serde(default)]
    pub trade_rows: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_net_pnl: Option<f64>,
    #[serde(default)]
    pub tp_count: usize,
    #[serde(default)]
    pub sl_count: usize,
    #[serde(default)]
    pub be_count: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub average_score: Option<f64>,
}

#[derive(Debug, Clone, Default)]
struct StrategyCsvMetrics {
    trade_rows: usize,
    total_net_pnl: Option<f64>,
    tp_count: usize,
    sl_count: usize,
    be_count: usize,
    average_score: Option<f64>,
}

pub fn discover_strategy_materials(
    root: Option<&str>,
    limit: usize,
) -> Vec<AutoQuantStrategyMaterialSummary> {
    let Some(root) = root.map(str::trim).filter(|value| !value.is_empty()) else {
        return Vec::new();
    };
    let root = Path::new(root);
    if !root.is_dir() {
        return Vec::new();
    }
    discover_strategy_materials_from_root(root, limit).unwrap_or_default()
}

fn discover_strategy_materials_from_root(
    root: &Path,
    limit: usize,
) -> Result<Vec<AutoQuantStrategyMaterialSummary>> {
    let mut python_files = Vec::new();
    let mut csv_files = Vec::new();
    collect_material_files(root, &mut python_files, &mut csv_files)?;

    let mut csv_by_key: BTreeMap<String, Vec<PathBuf>> = BTreeMap::new();
    for csv_path in csv_files {
        let Some(stem) = csv_path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        for key in csv_lookup_keys(stem) {
            let entry = csv_by_key.entry(key).or_default();
            if !entry.iter().any(|existing| existing == &csv_path) {
                entry.push(csv_path.clone());
            }
        }
    }

    let mut materials = python_files
        .into_iter()
        .filter_map(|strategy_path| build_strategy_material(root, &strategy_path, &csv_by_key))
        .collect::<Vec<_>>();

    materials.sort_by(compare_materials);
    if limit > 0 {
        materials.truncate(limit);
    }
    Ok(materials)
}

fn collect_material_files(
    root: &Path,
    python_files: &mut Vec<PathBuf>,
    csv_files: &mut Vec<PathBuf>,
) -> Result<()> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            for entry in std::fs::read_dir(&path)? {
                stack.push(entry?.path());
            }
            continue;
        }
        let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if extension.eq_ignore_ascii_case("py") {
            python_files.push(path);
        } else if extension.eq_ignore_ascii_case("csv") && is_strategy_evidence_csv(&path) {
            csv_files.push(path);
        }
    }
    Ok(())
}

fn build_strategy_material(
    root: &Path,
    strategy_path: &Path,
    csv_by_key: &BTreeMap<String, Vec<PathBuf>>,
) -> Option<AutoQuantStrategyMaterialSummary> {
    let stem = strategy_path.file_stem()?.to_str()?;
    let strategy_relative_path = relative_path(root, strategy_path)?;
    let (evidence_csv_path, metrics) = strategy_lookup_keys(stem)
        .into_iter()
        .find_map(|key| {
            csv_by_key
                .get(&key)
                .and_then(|paths| choose_best_csv(root, paths))
        })
        .unwrap_or((None, StrategyCsvMetrics::default()));

    Some(AutoQuantStrategyMaterialSummary {
        name: stem.to_string(),
        strategy_path: strategy_relative_path,
        evidence_csv_path,
        trade_rows: metrics.trade_rows,
        total_net_pnl: metrics.total_net_pnl,
        tp_count: metrics.tp_count,
        sl_count: metrics.sl_count,
        be_count: metrics.be_count,
        average_score: metrics.average_score,
    })
}

fn is_strategy_evidence_csv(path: &Path) -> bool {
    let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
        return false;
    };
    let name = name.to_ascii_lowercase();
    !name.ends_with(".ohlcv-1m.csv") && name != "symbology.csv"
}

fn material_key(stem: &str) -> String {
    let mut key = stem.to_ascii_lowercase();
    loop {
        let mut changed = false;
        for suffix in ["_strategy", "_results", "_result", "_summary"] {
            if let Some(stripped) = key.strip_suffix(suffix) {
                key = stripped.to_string();
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    key
}

fn csv_lookup_keys(stem: &str) -> Vec<String> {
    let key = material_key(stem);
    let mut keys = vec![key.clone()];
    extend_family_bases(&mut keys, &key, &["_es", "_nq", "_ym", "_eur", "_xau"]);
    keys
}

fn strategy_lookup_keys(stem: &str) -> Vec<String> {
    let key = material_key(stem);
    let mut keys = vec![key.clone()];
    extend_family_bases(&mut keys, &key, &["_pro", "_final", "_v2", "_v3"]);
    keys
}

fn extend_family_bases(keys: &mut Vec<String>, initial: &str, suffixes: &[&str]) {
    let mut current = initial.to_string();
    while let Some(next) = suffixes
        .iter()
        .find_map(|suffix| current.strip_suffix(suffix).map(str::to_string))
    {
        if keys.iter().any(|existing| existing == &next) {
            break;
        }
        keys.push(next.clone());
        current = next;
    }
}

fn choose_best_csv(root: &Path, paths: &[PathBuf]) -> Option<(Option<String>, StrategyCsvMetrics)> {
    let mut best: Option<(PathBuf, StrategyCsvMetrics)> = None;
    for path in paths {
        let metrics = summarize_strategy_csv(path).unwrap_or_default();
        match &best {
            Some((_, current)) if compare_metrics(&metrics, current) == Ordering::Greater => {
                best = Some((path.clone(), metrics));
            }
            None => best = Some((path.clone(), metrics)),
            _ => {}
        }
    }
    best.map(|(path, metrics)| (relative_path(root, &path), metrics))
}

fn evidence_tier(trade_rows: usize) -> usize {
    match trade_rows {
        0 => 0,
        1..=99 => 1,
        _ => 2,
    }
}

fn seed_name_score(name: &str) -> i32 {
    let lower = name.to_ascii_lowercase();
    let alpha_chars = lower.chars().filter(|ch| ch.is_ascii_alphabetic()).count() as i32;
    let digit_chars = lower.chars().filter(|ch| ch.is_ascii_digit()).count() as i32;
    let separator_bonus = lower.chars().filter(|ch| matches!(ch, '_' | '-')).count() as i32;
    let experimental_penalty = if lower.starts_with("test") || lower.contains("backtest") {
        12
    } else {
        0
    };
    alpha_chars + separator_bonus - (digit_chars * 2) - experimental_penalty
}

fn compare_materials(
    left: &AutoQuantStrategyMaterialSummary,
    right: &AutoQuantStrategyMaterialSummary,
) -> Ordering {
    right
        .evidence_csv_path
        .is_some()
        .cmp(&left.evidence_csv_path.is_some())
        .then_with(|| evidence_tier(right.trade_rows).cmp(&evidence_tier(left.trade_rows)))
        .then_with(|| {
            right
                .average_score
                .is_some()
                .cmp(&left.average_score.is_some())
        })
        .then_with(|| compare_optional_f64_desc(right.average_score, left.average_score))
        .then_with(|| seed_name_score(&right.name).cmp(&seed_name_score(&left.name)))
        .then_with(|| compare_optional_f64_desc(right.total_net_pnl, left.total_net_pnl))
        .then_with(|| right.trade_rows.cmp(&left.trade_rows))
        .then_with(|| left.name.cmp(&right.name))
}

fn compare_metrics(left: &StrategyCsvMetrics, right: &StrategyCsvMetrics) -> Ordering {
    evidence_tier(left.trade_rows)
        .cmp(&evidence_tier(right.trade_rows))
        .then_with(|| {
            left.average_score
                .is_some()
                .cmp(&right.average_score.is_some())
        })
        .then_with(|| compare_optional_f64_desc(left.average_score, right.average_score))
        .then_with(|| left.trade_rows.cmp(&right.trade_rows))
        .then_with(|| compare_optional_f64_desc(left.total_net_pnl, right.total_net_pnl))
        .then_with(|| left.tp_count.cmp(&right.tp_count))
}

fn compare_optional_f64_desc(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.partial_cmp(&right).unwrap_or(Ordering::Equal),
        (Some(_), None) => Ordering::Greater,
        (None, Some(_)) => Ordering::Less,
        (None, None) => Ordering::Equal,
    }
}

fn summarize_strategy_csv(path: &Path) -> Result<StrategyCsvMetrics> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let pnl_index = header_index(&headers, &["netpnl"]);
    let result_index = header_index(&headers, &["result"]);
    let score_index = header_index(&headers, &["score"]);

    let mut metrics = StrategyCsvMetrics::default();
    let mut pnl_total = 0.0;
    let mut pnl_count = 0usize;
    let mut score_total = 0.0;
    let mut score_count = 0usize;

    for record in reader.records() {
        let record = record?;
        metrics.trade_rows += 1;
        if let Some(index) = pnl_index {
            if let Some(value) = record.get(index).and_then(parse_csv_f64) {
                pnl_total += value;
                pnl_count += 1;
            }
        }
        if let Some(index) = result_index {
            if let Some(value) = record.get(index) {
                match value.trim().to_ascii_uppercase().as_str() {
                    "TP" => metrics.tp_count += 1,
                    "SL" => metrics.sl_count += 1,
                    "BE" => metrics.be_count += 1,
                    _ => {}
                }
            }
        }
        if let Some(index) = score_index {
            if let Some(value) = record.get(index).and_then(parse_csv_f64) {
                score_total += value;
                score_count += 1;
            }
        }
    }

    if pnl_count > 0 {
        metrics.total_net_pnl = Some(pnl_total);
    }
    if score_count > 0 {
        metrics.average_score = Some(score_total / score_count as f64);
    }
    Ok(metrics)
}

fn header_index(headers: &StringRecord, accepted: &[&str]) -> Option<usize> {
    headers.iter().position(|value| {
        let normalized = normalize_header(value);
        accepted.iter().any(|item| normalized == *item)
    })
}

fn normalize_header(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase()
}

fn parse_csv_f64(value: &str) -> Option<f64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    trimmed.replace(',', "").parse::<f64>().ok()
}

fn relative_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok().unwrap_or(path);
    Some(relative.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_trade_csv(path: &Path, rows: usize, pnl: f64, result: &str, score: Option<f64>) {
        let mut content = match score {
            Some(_) => "Time,Net PnL,Result,Score\n".to_string(),
            None => "Time,Net PnL,Result\n".to_string(),
        };
        for index in 0..rows {
            let day = (index % 28) + 1;
            match score {
                Some(score) => {
                    content.push_str(&format!("2024-01-{day:02},{pnl},{result},{score}\n"))
                }
                None => content.push_str(&format!("2024-01-{day:02},{pnl},{result}\n")),
            }
        }
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn discovers_strategy_materials_from_root() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("alpha_strategy.py"), "print('alpha')").unwrap();
        std::fs::write(
            temp.path().join("alpha_results.csv"),
            "Time,Net PnL,Result,Score\n2024-01-01,10,TP,5\n2024-01-02,-4,SL,3\n",
        )
        .unwrap();
        std::fs::write(temp.path().join("beta.py"), "print('beta')").unwrap();
        std::fs::write(
            temp.path().join("beta.csv"),
            "Time,Net PnL,Result,Score\n2024-01-01,7,TP,4\n",
        )
        .unwrap();
        std::fs::write(temp.path().join("raw.ohlcv-1m.csv"), "ts,open\n1,2\n").unwrap();
        std::fs::write(temp.path().join("symbology.csv"), "root,symbol\n").unwrap();

        let materials = discover_strategy_materials(Some(temp.path().to_str().unwrap()), 10);
        assert_eq!(materials.len(), 2);
        let alpha = materials
            .iter()
            .find(|item| item.name == "alpha_strategy")
            .unwrap();
        assert_eq!(
            alpha.evidence_csv_path.as_deref(),
            Some("alpha_results.csv")
        );
        assert_eq!(alpha.trade_rows, 2);
        assert_eq!(alpha.tp_count, 1);
        assert_eq!(alpha.sl_count, 1);
        assert_eq!(alpha.be_count, 0);
        assert_eq!(alpha.total_net_pnl, Some(6.0));
        assert_eq!(alpha.average_score, Some(4.0));
    }

    #[test]
    fn ranking_prefers_richer_evidence_and_readable_names() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("ultimate_ict_strategy.py"),
            "print('ultimate')",
        )
        .unwrap();
        write_trade_csv(
            &temp.path().join("ultimate_ict_results.csv"),
            120,
            5.0,
            "TP",
            Some(5.0),
        );
        std::fs::write(temp.path().join("no_be_strategy.py"), "print('no_be')").unwrap();
        write_trade_csv(&temp.path().join("no_be_results.csv"), 110, 6.0, "TP", None);
        std::fs::write(temp.path().join("98wr0.8rrr41.07pf.py"), "print('numeric')").unwrap();
        write_trade_csv(
            &temp.path().join("98wr0.8rrr41.07pf.csv"),
            250,
            7.0,
            "TP",
            None,
        );

        let materials = discover_strategy_materials(Some(temp.path().to_str().unwrap()), 10);
        let names = materials
            .into_iter()
            .map(|item| item.name)
            .collect::<Vec<_>>();

        assert_eq!(
            names,
            vec![
                "ultimate_ict_strategy".to_string(),
                "no_be_strategy".to_string(),
                "98wr0.8rrr41.07pf".to_string(),
            ]
        );
    }

    #[test]
    fn family_matching_uses_market_and_edition_fallbacks() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("90wr1.5rrr_strategy.py"), "print('base')").unwrap();
        std::fs::write(temp.path().join("90wr1.5rrr_final.py"), "print('final')").unwrap();
        write_trade_csv(
            &temp.path().join("90wr1.5rrr_ES_results.csv"),
            120,
            9.0,
            "TP",
            None,
        );

        let materials = discover_strategy_materials(Some(temp.path().to_str().unwrap()), 10);
        let base = materials
            .iter()
            .find(|item| item.name == "90wr1.5rrr_strategy")
            .unwrap();
        let final_variant = materials
            .iter()
            .find(|item| item.name == "90wr1.5rrr_final")
            .unwrap();

        assert_eq!(
            base.evidence_csv_path.as_deref(),
            Some("90wr1.5rrr_ES_results.csv")
        );
        assert_eq!(
            final_variant.evidence_csv_path.as_deref(),
            Some("90wr1.5rrr_ES_results.csv")
        );
    }

    #[test]
    fn returns_empty_for_missing_root() {
        assert!(discover_strategy_materials(None, 3).is_empty());
        assert!(
            discover_strategy_materials(Some("/tmp/definitely-missing-material-root"), 3)
                .is_empty()
        );
    }
}
