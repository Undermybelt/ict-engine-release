use chrono::{DateTime, TimeZone, Utc};
use ict_engine::factor_lab::factor_definition::FactorDefinition;
use ict_engine::factor_lab::{BacktestConfig, FactorBacktestEngine, FactorContext, FactorEngine};
use ict_engine::factors::FactorRegistry;
use ict_engine::types::{Candle, Regime};
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
struct PocResult {
    symbol: String,
    total_return: f64,
    sharpe: f64,
    win_rate: f64,
    trade_count: usize,
    expansion_trades: usize,
    expansion_win_rate: f64,
    reversal_trades: usize,
    reversal_win_rate: f64,
    regime_distribution: HashMap<String, usize>,
}

fn load_candles(path: &str, after: DateTime<Utc>) -> Vec<Candle> {
    let content = std::fs::read_to_string(path).expect("read file");
    let wrapper: serde_json::Value = serde_json::from_str(&content).expect("parse json");
    let arr = wrapper["candles"].as_array().expect("candles array");
    arr.iter()
        .filter_map(|v| {
            let ts_str = v["timestamp"].as_str()?;
            let ts = DateTime::parse_from_rfc3339(ts_str)
                .ok()?
                .with_timezone(&Utc);
            if ts < after {
                return None;
            }
            Some(Candle {
                timestamp: ts,
                open: v["open"].as_f64()?,
                high: v["high"].as_f64()?,
                low: v["low"].as_f64()?,
                close: v["close"].as_f64()?,
                volume: v["volume"].as_f64()?,
            })
        })
        .collect()
}

fn run_symbol(symbol: &str, path: &str, after: DateTime<Utc>) -> PocResult {
    let candles = load_candles(path, after);
    assert!(
        candles.len() >= 500,
        "{}: insufficient candles ({})",
        symbol,
        candles.len()
    );

    let mut registry = FactorRegistry::default();
    registry.set_enabled("volatility_mean_reversion", false);
    registry.set_enabled("structure_ict", false);
    registry.set_enabled("cross_market_smt", false);
    registry.set_enabled("options_hedging", false);
    registry.register(FactorDefinition::trend_momentum());

    let engine = FactorEngine::new(registry);
    let backtest = FactorBacktestEngine::new(engine);
    let ctx = FactorContext::default();

    run_config(symbol, &backtest, &candles, &ctx)
}

fn run_config(
    symbol: &str,
    backtest: &FactorBacktestEngine,
    candles: &[Candle],
    ctx: &FactorContext,
) -> PocResult {
    let config = BacktestConfig {
        train_bars: 120,
        test_bars: 60,
        step_bars: 30,
        ..BacktestConfig::default()
    };

    let result = backtest
        .run(candles, ctx, None, &config)
        .expect("backtest run");
    let fr = &result.factor_results[0];
    let metrics = &fr.metrics;
    let trades = &fr.trades;

    let mut regime_dist: HashMap<String, usize> = HashMap::new();
    let mut expansion_wins = 0usize;
    let mut expansion_total = 0usize;
    let mut reversal_wins = 0usize;
    let mut reversal_total = 0usize;

    for t in trades.iter() {
        let key = match t.regime_at_entry {
            Regime::ManipulationExpansion => "expansion",
            Regime::Accumulation => "accumulation",
            Regime::Distribution => "distribution",
        };
        *regime_dist.entry(key.to_string()).or_insert(0) += 1;

        let win = t.pnl > 0.0;
        match t.regime_at_entry {
            Regime::ManipulationExpansion => {
                expansion_total += 1;
                if win {
                    expansion_wins += 1;
                }
            }
            _ => {
                reversal_total += 1;
                if win {
                    reversal_wins += 1;
                }
            }
        }
    }

    PocResult {
        symbol: symbol.to_string(),
        total_return: metrics.total_return,
        sharpe: metrics.sharpe,
        win_rate: metrics.win_rate,
        trade_count: metrics.trade_count,
        expansion_trades: expansion_total,
        expansion_win_rate: if expansion_total > 0 {
            expansion_wins as f64 / expansion_total as f64
        } else {
            0.0
        },
        reversal_trades: reversal_total,
        reversal_win_rate: if reversal_total > 0 {
            reversal_wins as f64 / reversal_total as f64
        } else {
            0.0
        },
        regime_distribution: regime_dist,
    }
}

#[test]
fn eml_poc_nq_and_es() {
    let Ok(root) = std::env::var("ICT_ENGINE_EML_POC_ROOT") else {
        eprintln!("skipping eml_poc_nq_and_es: ICT_ENGINE_EML_POC_ROOT is not set");
        return;
    };

    let nq_after = TimeZone::with_ymd_and_hms(&Utc, 2023, 1, 1, 0, 0, 0).unwrap();
    let nq_path = Path::new(&root).join("nq").join("nq.continuous-15m.json");
    let nq = run_symbol("NQ", &nq_path.to_string_lossy(), nq_after);

    let es_after = TimeZone::with_ymd_and_hms(&Utc, 2024, 1, 1, 0, 0, 0).unwrap();
    let es_path = Path::new(&root).join("es").join("es.continuous-15m.json");
    let es = run_symbol("ES", &es_path.to_string_lossy(), es_after);

    println!("\n========== EML PoC Baseline Snapshot ==========");
    for r in [&nq, &es] {
        println!(
            "{} | return={:>7.4} | sharpe={:>6.3} | win_rate={:>5.3} | trades={:>3} | expansion_win={:>5.3} ({:>2}) | reversal_win={:>5.3} ({:>2}) | regimes={:?}",
            r.symbol,
            r.total_return,
            r.sharpe,
            r.win_rate,
            r.trade_count,
            r.expansion_win_rate,
            r.expansion_trades,
            r.reversal_win_rate,
            r.reversal_trades,
            r.regime_distribution,
        );
    }

    for r in [&nq, &es] {
        assert!(
            r.total_return.is_finite(),
            "{} total_return NaN/Inf",
            r.symbol
        );
        assert!(r.sharpe.is_finite(), "{} sharpe NaN/Inf", r.symbol);
    }
}
