use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, Datelike, TimeZone, Timelike, Utc, Weekday};
use chrono_tz::{America::New_York, Asia::Tokyo, Europe::London};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use crate::types::Candle;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandleFileFormat {
    Json,
    Csv,
}

#[derive(Debug, Clone, Deserialize)]
struct CandleJson {
    timestamp: serde_json::Value,
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
    volume: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
struct CandleData {
    #[serde(rename = "symbol")]
    _symbol: Option<String>,
    candles: Vec<CandleJson>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
enum CandleJsonPayload {
    Wrapped(CandleData),
    Series(Vec<CandleJson>),
}

#[derive(Debug, Clone)]
struct RawCandleRow {
    timestamp: DateTime<Utc>,
    open: Option<f64>,
    high: Option<f64>,
    low: Option<f64>,
    close: Option<f64>,
    volume: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CleanedContinuousFuturesSummary {
    pub source_path: String,
    pub symbology_path: String,
    pub total_raw_rows: usize,
    pub matched_front_rows: usize,
    pub dropped_without_symbology: usize,
    pub dropped_non_front_contract: usize,
    pub continuous_candles: usize,
    pub aggregated_candles: usize,
    pub first_selected_timestamp: Option<DateTime<Utc>>,
    pub last_selected_timestamp: Option<DateTime<Utc>>,
    pub first_symbology_date: Option<String>,
    pub last_symbology_date: Option<String>,
    pub unique_selected_instruments: usize,
    #[serde(default)]
    pub dropped_session_gap_rows: usize,
    #[serde(default)]
    pub rebased_gap_events: usize,
    #[serde(default)]
    pub rebased_gap_rows: usize,
    #[serde(default)]
    pub asia_session_rows: usize,
    #[serde(default)]
    pub europe_session_rows: usize,
    #[serde(default)]
    pub us_session_rows: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IntradaySession {
    Asia,
    Europe,
    Us,
    Overnight,
    OffHours,
}

pub fn infer_candle_format<P: AsRef<Path>>(path: P) -> CandleFileFormat {
    match path
        .as_ref()
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("csv") => CandleFileFormat::Csv,
        _ => CandleFileFormat::Json,
    }
}

pub fn load_candles<P: AsRef<Path>>(path: P) -> Result<Vec<Candle>> {
    match infer_candle_format(&path) {
        CandleFileFormat::Json => load_candles_json(path),
        CandleFileFormat::Csv => load_candles_csv(path),
    }
}

pub fn load_candles_json<P: AsRef<Path>>(path: P) -> Result<Vec<Candle>> {
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read file: {:?}", path.as_ref()))?;

    let payload: CandleJsonPayload = serde_json::from_str(&content)
        .with_context(|| format!("Failed to parse JSON: {:?}", path.as_ref()))?;

    let rows = match payload {
        CandleJsonPayload::Wrapped(data) => data.candles,
        CandleJsonPayload::Series(series) => series,
    };

    normalize_candle_rows(
        rows.into_iter()
            .map(|row| {
                Ok(RawCandleRow {
                    timestamp: parse_timestamp_value(&row.timestamp)?,
                    open: row.open,
                    high: row.high,
                    low: row.low,
                    close: row.close,
                    volume: row.volume,
                })
            })
            .collect::<Result<Vec<_>>>()?,
    )
}

pub fn load_candles_csv<P: AsRef<Path>>(path: P) -> Result<Vec<Candle>> {
    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read file: {:?}", path.as_ref()))?;

    let mut reader = csv::Reader::from_reader(content.as_bytes());
    let headers = reader
        .headers()
        .context("Failed to read CSV header")?
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();

    let timestamp_idx =
        find_header_index(&headers, &["timestamp", "time", "datetime", "ts_event"])?;
    let open_idx = find_header_index(&headers, &["open", "o"])?;
    let high_idx = find_header_index(&headers, &["high", "h"])?;
    let low_idx = find_header_index(&headers, &["low", "l"])?;
    let close_idx = find_header_index(&headers, &["close", "c"])?;
    let volume_idx = headers
        .iter()
        .position(|header| header == "volume" || header == "v");

    let mut rows = Vec::new();
    for record in reader.records() {
        let record = record.context("Failed to read CSV record")?;
        rows.push(RawCandleRow {
            timestamp: parse_timestamp_str(
                record
                    .get(timestamp_idx)
                    .ok_or_else(|| anyhow!("missing CSV timestamp field"))?,
            )?,
            open: parse_optional_f64(record.get(open_idx)),
            high: parse_optional_f64(record.get(high_idx)),
            low: parse_optional_f64(record.get(low_idx)),
            close: parse_optional_f64(record.get(close_idx)),
            volume: volume_idx.and_then(|idx| parse_optional_f64(record.get(idx))),
        });
    }

    normalize_candle_rows(rows)
}

pub fn load_tomac_continuous_candles<P: AsRef<Path>, Q: AsRef<Path>>(
    ohlcv_path: P,
    symbology_path: Q,
) -> Result<(Vec<Candle>, CleanedContinuousFuturesSummary)> {
    let symbology_content = fs::read_to_string(&symbology_path).with_context(|| {
        format!(
            "Failed to read symbology file: {:?}",
            symbology_path.as_ref()
        )
    })?;
    let mut symbology_reader = csv::Reader::from_reader(symbology_content.as_bytes());
    let sym_headers = symbology_reader
        .headers()
        .context("Failed to read symbology CSV header")?
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();
    let sym_date_idx = find_header_index(&sym_headers, &["date"])?;
    let sym_instrument_idx = find_header_index(&sym_headers, &["instrument_id", "instrumentid"])?;

    let mut date_to_instrument = BTreeMap::<String, String>::new();
    let mut first_symbology_date = None::<String>;
    let mut last_symbology_date = None::<String>;
    for record in symbology_reader.records() {
        let record = record.context("Failed to read symbology CSV record")?;
        let date = record
            .get(sym_date_idx)
            .ok_or_else(|| anyhow!("missing symbology date field"))?
            .trim()
            .to_string();
        let instrument_id = record
            .get(sym_instrument_idx)
            .ok_or_else(|| anyhow!("missing symbology instrument_id field"))?
            .trim()
            .to_string();
        if date.is_empty() || instrument_id.is_empty() {
            continue;
        }
        first_symbology_date = Some(match &first_symbology_date {
            Some(current) if current <= &date => current.clone(),
            _ => date.clone(),
        });
        last_symbology_date = Some(match &last_symbology_date {
            Some(current) if current >= &date => current.clone(),
            _ => date.clone(),
        });
        date_to_instrument.insert(date, instrument_id);
    }
    if date_to_instrument.is_empty() {
        bail!("symbology file does not contain usable date/instrument_id rows");
    }

    let content = fs::read_to_string(&ohlcv_path)
        .with_context(|| format!("Failed to read file: {:?}", ohlcv_path.as_ref()))?;
    let mut reader = csv::Reader::from_reader(content.as_bytes());
    let headers = reader
        .headers()
        .context("Failed to read CSV header")?
        .iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .collect::<Vec<_>>();

    let timestamp_idx =
        find_header_index(&headers, &["timestamp", "time", "datetime", "ts_event"])?;
    let instrument_idx = find_header_index(&headers, &["instrument_id", "instrumentid"])?;
    let open_idx = find_header_index(&headers, &["open", "o"])?;
    let high_idx = find_header_index(&headers, &["high", "h"])?;
    let low_idx = find_header_index(&headers, &["low", "l"])?;
    let close_idx = find_header_index(&headers, &["close", "c"])?;
    let volume_idx = headers
        .iter()
        .position(|header| header == "volume" || header == "v");

    let mut selected_rows = Vec::new();
    let mut total_raw_rows = 0usize;
    let mut dropped_without_symbology = 0usize;
    let mut dropped_non_front_contract = 0usize;
    let mut first_selected_timestamp = None;
    let mut last_selected_timestamp = None;
    let mut selected_instruments = std::collections::BTreeSet::new();

    for record in reader.records() {
        let record = record.context("Failed to read CSV record")?;
        total_raw_rows += 1;
        let timestamp = parse_timestamp_str(
            record
                .get(timestamp_idx)
                .ok_or_else(|| anyhow!("missing CSV timestamp field"))?,
        )?;
        let date_key = timestamp.date_naive().to_string();
        let Some(expected_instrument_id) = date_to_instrument.get(&date_key) else {
            dropped_without_symbology += 1;
            continue;
        };
        let instrument_id = record
            .get(instrument_idx)
            .ok_or_else(|| anyhow!("missing CSV instrument_id field"))?
            .trim();
        if instrument_id != expected_instrument_id {
            dropped_non_front_contract += 1;
            continue;
        }
        selected_instruments.insert(instrument_id.to_string());
        if first_selected_timestamp.is_none() {
            first_selected_timestamp = Some(timestamp);
        }
        last_selected_timestamp = Some(timestamp);
        selected_rows.push(RawCandleRow {
            timestamp,
            open: parse_optional_f64(record.get(open_idx)),
            high: parse_optional_f64(record.get(high_idx)),
            low: parse_optional_f64(record.get(low_idx)),
            close: parse_optional_f64(record.get(close_idx)),
            volume: volume_idx.and_then(|idx| parse_optional_f64(record.get(idx))),
        });
    }

    let candles = normalize_candle_rows(selected_rows)?;
    let session_cleaning = clean_intraday_session_gaps(&candles)?;
    let summary = CleanedContinuousFuturesSummary {
        source_path: ohlcv_path.as_ref().to_string_lossy().to_string(),
        symbology_path: symbology_path.as_ref().to_string_lossy().to_string(),
        total_raw_rows,
        matched_front_rows: session_cleaning.cleaned.len(),
        dropped_without_symbology,
        dropped_non_front_contract,
        continuous_candles: session_cleaning.cleaned.len(),
        aggregated_candles: session_cleaning.cleaned.len(),
        first_selected_timestamp,
        last_selected_timestamp,
        first_symbology_date,
        last_symbology_date,
        unique_selected_instruments: selected_instruments.len(),
        dropped_session_gap_rows: session_cleaning.dropped_session_gap_rows,
        rebased_gap_events: session_cleaning.rebased_gap_events,
        rebased_gap_rows: session_cleaning.rebased_gap_rows,
        asia_session_rows: session_cleaning.asia_session_rows,
        europe_session_rows: session_cleaning.europe_session_rows,
        us_session_rows: session_cleaning.us_session_rows,
    };
    Ok((session_cleaning.cleaned, summary))
}

pub fn aggregate_candles_by_minutes(
    candles: &[Candle],
    interval_minutes: i64,
) -> Result<Vec<Candle>> {
    if interval_minutes <= 0 {
        bail!("interval_minutes must be positive");
    }
    if candles.is_empty() || interval_minutes == 1 {
        return Ok(candles.to_vec());
    }

    let bucket_seconds = interval_minutes * 60;
    let mut aggregated = Vec::new();
    let mut current_bucket = None::<i64>;
    let mut current = None::<Candle>;

    for candle in candles {
        let bucket = candle.timestamp.timestamp().div_euclid(bucket_seconds);
        if current_bucket != Some(bucket) {
            if let Some(finished) = current.take() {
                aggregated.push(finished);
            }
            current_bucket = Some(bucket);
            current = Some(Candle {
                timestamp: Utc
                    .timestamp_opt(bucket * bucket_seconds, 0)
                    .single()
                    .ok_or_else(|| anyhow!("invalid aggregated timestamp bucket"))?,
                open: candle.open,
                high: candle.high,
                low: candle.low,
                close: candle.close,
                volume: candle.volume,
            });
            continue;
        }

        if let Some(current) = &mut current {
            current.high = current.high.max(candle.high);
            current.low = current.low.min(candle.low);
            current.close = candle.close;
            current.volume += candle.volume;
        }
    }

    if let Some(finished) = current {
        aggregated.push(finished);
    }

    Ok(aggregated)
}

pub fn candles_to_prices(candles: &[Candle]) -> Vec<f64> {
    candles.iter().map(|c| c.close).collect()
}

pub fn candles_to_returns(candles: &[Candle]) -> Vec<f64> {
    candles
        .windows(2)
        .filter_map(|w| {
            if w[0].close.abs() <= f64::EPSILON {
                None
            } else {
                Some((w[1].close / w[0].close).ln())
            }
        })
        .collect()
}

fn find_header_index(headers: &[String], candidates: &[&str]) -> Result<usize> {
    headers
        .iter()
        .position(|header| candidates.iter().any(|candidate| header == candidate))
        .ok_or_else(|| anyhow!("CSV header missing one of {:?}", candidates))
}

fn parse_optional_f64(value: Option<&str>) -> Option<f64> {
    let value = value?.trim();
    if value.is_empty() {
        return None;
    }
    value
        .parse::<f64>()
        .ok()
        .filter(|parsed| parsed.is_finite())
}

fn parse_timestamp_value(value: &serde_json::Value) -> Result<DateTime<Utc>> {
    match value {
        serde_json::Value::String(text) => parse_timestamp_str(text),
        serde_json::Value::Number(number) => parse_timestamp_str(&number.to_string()),
        _ => bail!("unsupported timestamp JSON value: {value}"),
    }
}

fn parse_timestamp_str(value: &str) -> Result<DateTime<Utc>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        bail!("timestamp is empty");
    }

    if let Ok(parsed) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(parsed.with_timezone(&Utc));
    }

    if let Ok(parsed) = DateTime::parse_from_str(trimmed, "%Y-%m-%d %H:%M:%S%#z") {
        return Ok(parsed.with_timezone(&Utc));
    }

    if let Ok(integer) = trimmed.parse::<i64>() {
        let (seconds, nanos) = if trimmed.len() >= 13 {
            let seconds = integer.div_euclid(1_000);
            let millis = integer.rem_euclid(1_000) as u32;
            (seconds, millis * 1_000_000)
        } else {
            (integer, 0)
        };
        return Utc
            .timestamp_opt(seconds, nanos)
            .single()
            .ok_or_else(|| anyhow!("invalid unix timestamp: {trimmed}"));
    }

    bail!("unsupported timestamp format: {trimmed}")
}

fn normalize_candle_rows(rows: Vec<RawCandleRow>) -> Result<Vec<Candle>> {
    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let mut deduped = BTreeMap::new();
    for row in rows {
        deduped.insert(row.timestamp, row);
    }

    let mut candles = Vec::with_capacity(deduped.len());
    let mut previous_close = None;

    for row in deduped.into_values() {
        let close = row.close.or(previous_close).ok_or_else(|| {
            anyhow!(
                "missing close price at {} and no previous close to forward fill",
                row.timestamp
            )
        })?;
        let open = row.open.or(previous_close).unwrap_or(close);
        let high = row.high.unwrap_or(open.max(close));
        let low = row.low.unwrap_or(open.min(close));
        let volume = row.volume.unwrap_or(0.0).max(0.0);

        let normalized_high = high.max(open).max(close).max(low);
        let normalized_low = low.min(open).min(close).min(high);

        candles.push(Candle {
            timestamp: row.timestamp,
            open,
            high: normalized_high,
            low: normalized_low,
            close,
            volume,
        });
        previous_close = Some(close);
    }

    Ok(candles)
}

#[derive(Debug, Clone, Default)]
struct IntradaySessionCleaningResult {
    cleaned: Vec<Candle>,
    dropped_session_gap_rows: usize,
    rebased_gap_events: usize,
    rebased_gap_rows: usize,
    asia_session_rows: usize,
    europe_session_rows: usize,
    us_session_rows: usize,
}

fn clean_intraday_session_gaps(candles: &[Candle]) -> Result<IntradaySessionCleaningResult> {
    if candles.is_empty() {
        return Ok(IntradaySessionCleaningResult::default());
    }
    let mut cleaned = Vec::with_capacity(candles.len());
    let mut result = IntradaySessionCleaningResult::default();
    let mut active_gap_shift = 0.0f64;
    let mut previous: Option<&Candle> = None;
    for candle in candles {
        let session = classify_intraday_session(candle.timestamp);
        if session == IntradaySession::OffHours {
            result.dropped_session_gap_rows += 1;
            continue;
        }
        match session {
            IntradaySession::Asia => result.asia_session_rows += 1,
            IntradaySession::Europe => result.europe_session_rows += 1,
            IntradaySession::Us => result.us_session_rows += 1,
            IntradaySession::Overnight => {}
            IntradaySession::OffHours => {}
        }
        if let Some(previous) = previous {
            let minutes_gap = (candle.timestamp - previous.timestamp).num_minutes();
            let session_changed = classify_intraday_session(previous.timestamp) != session;
            let relative_gap = if previous.close.abs() > f64::EPSILON {
                ((candle.open - previous.close) / previous.close).abs()
            } else {
                0.0
            };
            if minutes_gap > 90 || (session_changed && relative_gap > 0.0035) {
                active_gap_shift += candle.open - previous.close;
                result.rebased_gap_events += 1;
            }
        }
        let rebased = Candle {
            timestamp: candle.timestamp,
            open: candle.open - active_gap_shift,
            high: candle.high - active_gap_shift,
            low: candle.low - active_gap_shift,
            close: candle.close - active_gap_shift,
            volume: candle.volume,
        };
        if active_gap_shift.abs() > f64::EPSILON {
            result.rebased_gap_rows += 1;
        }
        cleaned.push(rebased);
        previous = Some(candle);
    }
    result.cleaned = cleaned;
    Ok(result)
}

fn classify_intraday_session(timestamp: DateTime<Utc>) -> IntradaySession {
    let ny = timestamp.with_timezone(&New_York);
    if matches!(ny.weekday(), Weekday::Sat) {
        return IntradaySession::OffHours;
    }
    let ny_minutes = ny.hour() * 60 + ny.minute();
    if matches!(ny.weekday(), Weekday::Sun) {
        if ny_minutes < 18 * 60 {
            return IntradaySession::OffHours;
        }
    } else if (17 * 60..18 * 60).contains(&ny_minutes) {
        return IntradaySession::OffHours;
    }
    let tokyo = timestamp.with_timezone(&Tokyo);
    let london = timestamp.with_timezone(&London);
    let tokyo_minutes = tokyo.hour() * 60 + tokyo.minute();
    let london_minutes = london.hour() * 60 + london.minute();
    let us_minutes = ny.hour() * 60 + ny.minute();
    if (9 * 60..15 * 60).contains(&tokyo_minutes) {
        IntradaySession::Asia
    } else if (7 * 60..13 * 60).contains(&london_minutes) {
        IntradaySession::Europe
    } else if (8 * 60 + 30..16 * 60).contains(&us_minutes) {
        IntradaySession::Us
    } else {
        IntradaySession::Overnight
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_candles_csv_normalizes_sorts_and_deduplicates() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp.path(),
            "timestamp,open,high,low,close,volume\n\
             2024-01-01T00:01:00Z,101,102,100,101.5,\n\
             2024-01-01T00:00:00Z,100,101,99,100.5,1000\n\
             2024-01-01T00:01:00Z,102,103,101,102.5,2000\n",
        )
        .unwrap();

        let candles = load_candles_csv(temp.path()).unwrap();
        assert_eq!(candles.len(), 2);
        assert!(candles[0].timestamp < candles[1].timestamp);
        assert_eq!(candles[1].close, 102.5);
        assert_eq!(candles[1].volume, 2000.0);
    }

    #[test]
    fn test_load_candles_json_supports_array_and_missing_values() {
        let temp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            temp.path(),
            r#"
            [
              {"timestamp":"2024-01-01T00:00:00Z","open":100.0,"high":101.0,"low":99.0,"close":100.5,"volume":10.0},
              {"timestamp":"2024-01-01T00:01:00Z","open":null,"high":102.0,"low":100.0,"close":101.5}
            ]
            "#,
        )
        .unwrap();

        let candles = load_candles_json(temp.path()).unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[1].open, 100.5);
        assert_eq!(candles[1].volume, 0.0);
    }

    #[test]
    fn test_load_tomac_continuous_candles_filters_non_front_contracts() {
        let ohlcv = tempfile::NamedTempFile::new().unwrap();
        let symbology = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            ohlcv.path(),
            "ts_event,instrument_id,open,high,low,close,volume,symbol\n\
             2024-01-01T00:00:00Z,1,100,101,99,100.5,10,AAA\n\
             2024-01-01T00:00:00Z,2,200,201,199,200.5,20,BBB\n\
             2024-01-01T00:01:00Z,1,101,102,100,101.5,11,AAA\n\
             2024-01-01T00:01:00Z,2,201,202,200,201.5,21,BBB\n",
        )
        .unwrap();
        std::fs::write(
            symbology.path(),
            "raw_symbol,instrument_id,date\n\
             AAA,1,2024-01-01\n",
        )
        .unwrap();

        let (candles, summary) =
            load_tomac_continuous_candles(ohlcv.path(), symbology.path()).unwrap();
        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].close, 100.5);
        assert_eq!(summary.total_raw_rows, 4);
        assert_eq!(summary.dropped_non_front_contract, 2);
    }

    #[test]
    fn test_aggregate_candles_by_minutes_merges_bars() {
        let candles = vec![
            Candle {
                timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
                open: 100.0,
                high: 101.0,
                low: 99.0,
                close: 100.5,
                volume: 10.0,
            },
            Candle {
                timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 1, 0).unwrap(),
                open: 100.5,
                high: 102.0,
                low: 100.0,
                close: 101.5,
                volume: 15.0,
            },
            Candle {
                timestamp: Utc.with_ymd_and_hms(2024, 1, 1, 0, 5, 0).unwrap(),
                open: 101.5,
                high: 103.0,
                low: 101.0,
                close: 102.5,
                volume: 20.0,
            },
        ];

        let aggregated = aggregate_candles_by_minutes(&candles, 5).unwrap();
        assert_eq!(aggregated.len(), 2);
        assert_eq!(aggregated[0].open, 100.0);
        assert_eq!(aggregated[0].close, 101.5);
        assert_eq!(aggregated[0].high, 102.0);
        assert_eq!(aggregated[0].low, 99.0);
        assert_eq!(aggregated[0].volume, 25.0);
    }

    #[test]
    fn test_clean_intraday_session_gaps_drops_offhours_and_rebases_large_gap() {
        let candles = vec![
            Candle {
                timestamp: Utc.with_ymd_and_hms(2024, 1, 2, 15, 30, 0).unwrap(),
                open: 100.0,
                high: 101.0,
                low: 99.5,
                close: 100.5,
                volume: 10.0,
            },
            Candle {
                timestamp: Utc.with_ymd_and_hms(2024, 1, 2, 22, 30, 0).unwrap(),
                open: 104.0,
                high: 104.5,
                low: 103.5,
                close: 104.2,
                volume: 12.0,
            },
            Candle {
                timestamp: Utc.with_ymd_and_hms(2024, 1, 2, 23, 30, 0).unwrap(),
                open: 104.3,
                high: 104.6,
                low: 104.0,
                close: 104.1,
                volume: 9.0,
            },
        ];
        let cleaned = clean_intraday_session_gaps(&candles).unwrap();
        assert_eq!(cleaned.cleaned.len(), 2);
        assert!(cleaned.dropped_session_gap_rows >= 1);
        assert!(cleaned.rebased_gap_events >= 1);
        assert!((cleaned.cleaned[1].open - candles[0].close).abs() < 1e-6);
    }
}
