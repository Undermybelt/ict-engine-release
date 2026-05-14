use anyhow::{anyhow, bail, Context, Result};
use chrono::{DateTime, TimeDelta, Utc};
use reqwest::blocking::Client;
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use super::control_matrix_providers::{
    tradingview_mcp_config_from_env_or_local, TRADINGVIEW_MCP_ARGS_ENV, TRADINGVIEW_MCP_CMD_ENV,
};
use crate::types::Candle;

const MCP_TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_STDIO_CMD: &str = "uvx";
const DEFAULT_STDIO_ARGS: &[&str] = &["--from", "tradingview-mcp-server", "tradingview-mcp"];
const LOCAL_SOURCE_RELATIVE_PATH: &str = "tradingview-mcp/tradingview-mcp";

#[derive(Debug, Clone)]
pub(crate) struct TradingViewMcpClient {
    transport: TradingViewMcpTransport,
}

#[derive(Debug, Clone)]
enum TradingViewMcpTransport {
    Http { url: String, api_key: String },
    Stdio { command: String, args: Vec<String> },
}

impl TradingViewMcpClient {
    pub(crate) fn from_env_or_local() -> Self {
        let config = tradingview_mcp_config_from_env_or_local();
        if let Some(api_key) = config.api_key {
            return Self {
                transport: TradingViewMcpTransport::Http {
                    url: config.url,
                    api_key,
                },
            };
        }

        let (command, args) = stdio_command_from_env();
        Self {
            transport: TradingViewMcpTransport::Stdio { command, args },
        }
    }

    pub(crate) fn call_tool(&self, name: &str, arguments: Value) -> Result<Value> {
        ensure_allowed_tool(name)?;
        match &self.transport {
            TradingViewMcpTransport::Http { url, api_key } => {
                call_http_tool(url, api_key, name, arguments)
            }
            TradingViewMcpTransport::Stdio { command, args } => {
                call_stdio_tool(command, args, name, arguments)
            }
        }
    }
}

pub(crate) fn fetch_tradingview_ohlcv(
    symbol: &str,
    interval: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    count: usize,
) -> Result<Vec<Candle>> {
    let client = TradingViewMcpClient::from_env_or_local();
    fetch_tradingview_ohlcv_with_client(&client, symbol, interval, start, end, count)
}

pub(crate) fn fetch_tradingview_ohlcv_with_client(
    client: &TradingViewMcpClient,
    symbol: &str,
    interval: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    count: usize,
) -> Result<Vec<Candle>> {
    let normalized_interval = tradingview_interval(interval);
    let payload = client.call_tool(
        "get_ohlcv",
        serde_json::json!({
            "symbol": symbol,
            "interval": normalized_interval,
            "count": count,
            "summary": false
        }),
    )?;
    parse_tradingview_bars(&payload, normalized_interval, start, end)
        .with_context(|| format!("failed to parse tradingview_mcp OHLCV for '{}'", symbol))
}

pub(crate) fn parse_tradingview_bars(
    payload: &Value,
    interval: &str,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> Result<Vec<Candle>> {
    let bars = payload
        .get("bars")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("tradingview get_ohlcv returned no bars"))?;
    let end_tolerance = if interval.eq_ignore_ascii_case("1d") {
        TimeDelta::days(1)
    } else {
        TimeDelta::zero()
    };
    let mut candles = Vec::new();
    for (index, bar) in bars.iter().enumerate() {
        let ts = required_i64(bar, "t", index)?;
        let timestamp = DateTime::<Utc>::from_timestamp(ts, 0)
            .ok_or_else(|| anyhow!("invalid tradingview timestamp at bar {}", index))?;
        if timestamp < start || timestamp > end + end_tolerance {
            continue;
        }
        let open = required_f64(bar, "o", index)?;
        let high = required_f64(bar, "h", index)?;
        let low = required_f64(bar, "l", index)?;
        let close = required_f64(bar, "c", index)?;
        if open <= 0.0 || high <= 0.0 || low <= 0.0 || close <= 0.0 {
            bail!("non-positive OHLC value at tradingview bar {}", index);
        }
        if high < low {
            bail!("high below low at tradingview bar {}", index);
        }
        let volume = optional_f64(bar, "v")?.unwrap_or(0.0);
        if volume < 0.0 {
            bail!("negative volume at tradingview bar {}", index);
        }
        candles.push(Candle {
            timestamp,
            open,
            high,
            low,
            close,
            volume,
        });
    }
    candles.sort_by_key(|candle| candle.timestamp);
    candles.dedup_by(|left, right| left.timestamp == right.timestamp);
    if candles.is_empty() {
        bail!("tradingview returned no usable bars in requested range");
    }
    Ok(candles)
}

pub(crate) fn tradingview_interval(interval: &str) -> &str {
    match interval {
        "1m" => "1m",
        "2m" => "2m",
        "5m" => "5m",
        "15m" => "15m",
        "30m" => "30m",
        "60m" | "1h" => "1h",
        "90m" => "90m",
        "1d" | "1D" => "1d",
        _ => "1d",
    }
}

pub(crate) fn stdio_command_from_env() -> (String, Vec<String>) {
    if let Some(configured_command) = std::env::var(TRADINGVIEW_MCP_CMD_ENV)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    {
        let args = std::env::var(TRADINGVIEW_MCP_ARGS_ENV)
            .ok()
            .map(|value| split_args(&value))
            .filter(|items| !items.is_empty())
            .unwrap_or_else(default_stdio_args);
        return (configured_command, args);
    }

    if let Some(local_source) = local_source_checkout() {
        return (
            "uv".to_string(),
            vec![
                "--directory".to_string(),
                local_source,
                "run".to_string(),
                "tradingview-mcp".to_string(),
            ],
        );
    }

    (DEFAULT_STDIO_CMD.to_string(), default_stdio_args())
}

fn default_stdio_args() -> Vec<String> {
    DEFAULT_STDIO_ARGS
        .iter()
        .map(|item| item.to_string())
        .collect()
}

fn local_source_checkout() -> Option<String> {
    let home = std::env::var_os("HOME")?;
    let path = std::path::PathBuf::from(home).join(LOCAL_SOURCE_RELATIVE_PATH);
    path.join("pyproject.toml")
        .exists()
        .then(|| path.to_string_lossy().to_string())
}

fn split_args(raw: &str) -> Vec<String> {
    raw.split_whitespace()
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn call_http_tool(url: &str, api_key: &str, name: &str, arguments: Value) -> Result<Value> {
    let client = Client::builder()
        .timeout(MCP_TIMEOUT)
        .build()
        .context("failed to build tradingview MCP client")?;
    let response: Value = client
        .post(url)
        .header(
            reqwest::header::ACCEPT,
            "application/json, text/event-stream",
        )
        .bearer_auth(api_key)
        .json(&tool_call_request(1, name, arguments))
        .send()
        .with_context(|| format!("tradingview MCP call '{}' failed", name))?
        .error_for_status()
        .with_context(|| format!("tradingview MCP call '{}' returned error", name))?
        .json()
        .with_context(|| format!("failed to decode tradingview MCP response for '{}'", name))?;
    extract_tool_payload(name, response)
}

fn call_stdio_tool(command: &str, args: &[String], name: &str, arguments: Value) -> Result<Value> {
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| {
            format!(
                "failed to spawn tradingview MCP stdio command '{}'",
                command
            )
        })?;
    let stderr = capture_stderr(&mut child);
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("failed to capture tradingview MCP stdout"))?;
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines().map_while(|line| line.ok()) {
            let _ = tx.send(line);
        }
    });
    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow!("failed to capture tradingview MCP stdin"))?;

    let result = (|| -> Result<Value> {
        write_json_rpc(&mut stdin, &initialize_request(1))?;
        read_json_rpc_response(&rx, 1, &stderr)?;
        write_json_rpc(&mut stdin, &initialized_notification())?;
        write_json_rpc(&mut stdin, &tool_call_request(2, name, arguments))?;
        let response = read_json_rpc_response(&rx, 2, &stderr)?;
        extract_tool_payload(name, response)
    })();

    terminate_child(&mut child);
    result
}

fn capture_stderr(child: &mut Child) -> Arc<Mutex<String>> {
    let buffer = Arc::new(Mutex::new(String::new()));
    let Some(stderr) = child.stderr.take() else {
        return buffer;
    };
    let thread_buffer = Arc::clone(&buffer);
    std::thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines().map_while(|line| line.ok()) {
            let mut guard = thread_buffer.lock().expect("stderr buffer poisoned");
            if guard.len() < 4096 {
                guard.push_str(&line);
                guard.push('\n');
            }
        }
    });
    buffer
}

fn terminate_child(child: &mut Child) {
    if matches!(child.try_wait(), Ok(None)) {
        let _ = child.kill();
    }
    let _ = child.wait();
}

fn write_json_rpc(stdin: &mut std::process::ChildStdin, payload: &Value) -> Result<()> {
    serde_json::to_writer(&mut *stdin, payload).context("failed to encode MCP JSON-RPC request")?;
    stdin
        .write_all(b"\n")
        .context("failed to write MCP JSON-RPC newline")?;
    stdin
        .flush()
        .context("failed to flush MCP JSON-RPC request")
}

fn read_json_rpc_response(
    rx: &mpsc::Receiver<String>,
    id: i64,
    stderr: &Arc<Mutex<String>>,
) -> Result<Value> {
    let deadline = Instant::now() + MCP_TIMEOUT;
    loop {
        let now = Instant::now();
        if now >= deadline {
            bail!(
                "timed out waiting for tradingview MCP response id {}{}",
                id,
                stderr_suffix(stderr)
            );
        }
        let remaining = deadline.saturating_duration_since(now);
        let wait = remaining.min(Duration::from_millis(100));
        match rx.recv_timeout(wait) {
            Ok(line) => {
                let Ok(value) = serde_json::from_str::<Value>(&line) else {
                    continue;
                };
                if value.get("id").and_then(Value::as_i64) == Some(id) {
                    return Ok(value);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                bail!(
                    "tradingview MCP stdout closed before response id {}{}",
                    id,
                    stderr_suffix(stderr)
                );
            }
        }
    }
}

fn extract_tool_payload(name: &str, response: Value) -> Result<Value> {
    if let Some(error) = response.get("error") {
        bail!("tradingview MCP tool '{}' JSON-RPC error: {}", name, error);
    }
    if response
        .pointer("/result/isError")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        bail!(
            "tradingview MCP tool '{}' error: {}",
            name,
            response
                .pointer("/result/content/0/text")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
        );
    }
    let payload = if let Some(structured) = response.pointer("/result/structuredContent") {
        structured.clone()
    } else if let Some(text) = response
        .pointer("/result/content/0/text")
        .and_then(Value::as_str)
    {
        serde_json::from_str(text).unwrap_or_else(|_| serde_json::json!({ "text": text }))
    } else if let Some(result) = response.get("result") {
        result.clone()
    } else {
        response
    };
    if let Some(error_text) = payload.get("error").and_then(Value::as_str) {
        bail!("tradingview MCP tool '{}' error: {}", name, error_text);
    }
    if payload.get("success").and_then(Value::as_bool) == Some(false) {
        bail!(
            "tradingview MCP tool '{}' reported unsuccessful result",
            name
        );
    }
    Ok(payload)
}

fn initialize_request(id: i64) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "ict-engine",
                "version": env!("CARGO_PKG_VERSION")
            }
        }
    })
}

fn initialized_notification() -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "method": "notifications/initialized",
        "params": {}
    })
}

fn tool_call_request(id: i64, name: &str, arguments: Value) -> Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "tools/call",
        "params": {
            "name": name,
            "arguments": arguments,
        }
    })
}

fn ensure_allowed_tool(name: &str) -> Result<()> {
    match name {
        "get_ohlcv" | "yahoo_price" | "get_option_expirations" | "get_option_chain" => Ok(()),
        other => bail!(
            "tradingview MCP tool '{}' is not allowed by ict-engine",
            other
        ),
    }
}

fn required_i64(bar: &Value, key: &str, index: usize) -> Result<i64> {
    if let Some(value) = bar.get(key).and_then(Value::as_i64) {
        return Ok(value);
    }
    if let Some(value) = bar.get(key).and_then(Value::as_u64) {
        return i64::try_from(value)
            .with_context(|| format!("timestamp overflow at tradingview bar {}", index));
    }
    if let Some(value) = bar.get(key).and_then(Value::as_f64) {
        if value.is_finite() {
            return Ok(value as i64);
        }
    }
    bail!("missing or invalid '{}' at tradingview bar {}", key, index)
}

fn required_f64(bar: &Value, key: &str, index: usize) -> Result<f64> {
    let value = bar
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("missing '{}' at tradingview bar {}", key, index))?;
    if !value.is_finite() {
        bail!("non-finite '{}' at tradingview bar {}", key, index);
    }
    Ok(value)
}

fn optional_f64(bar: &Value, key: &str) -> Result<Option<f64>> {
    let Some(value) = bar.get(key) else {
        return Ok(None);
    };
    let Some(value) = value.as_f64() else {
        return Ok(None);
    };
    if !value.is_finite() {
        bail!("non-finite '{}' at tradingview bar", key);
    }
    Ok(Some(value))
}

fn stderr_suffix(stderr: &Arc<Mutex<String>>) -> String {
    let text = stderr
        .lock()
        .map(|guard| guard.trim().to_string())
        .unwrap_or_default();
    if text.is_empty() {
        String::new()
    } else {
        format!(": {}", text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn parses_sorts_and_deduplicates_bars() {
        let payload = serde_json::json!({
            "bars": [
                {"t": 1704153600, "o": 102.0, "h": 103.0, "l": 101.0, "c": 102.5, "v": 20.0},
                {"t": 1704067200, "o": 100.0, "h": 101.0, "l": 99.0, "c": 100.5, "v": 10.0},
                {"t": 1704067200, "o": 100.0, "h": 101.0, "l": 99.0, "c": 100.5, "v": 10.0}
            ]
        });
        let candles = parse_tradingview_bars(
            &payload,
            "1d",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
        )
        .unwrap();

        assert_eq!(candles.len(), 2);
        assert_eq!(candles[0].timestamp.timestamp(), 1704067200);
        assert_eq!(candles[1].timestamp.timestamp(), 1704153600);
    }

    #[test]
    fn rejects_missing_required_ohlc_values() {
        let payload = serde_json::json!({
            "bars": [
                {"t": 1704067200, "o": 100.0, "h": 101.0, "l": 99.0, "v": 10.0}
            ]
        });
        let err = parse_tradingview_bars(
            &payload,
            "1d",
            Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap(),
            Utc.with_ymd_and_hms(2024, 1, 2, 0, 0, 0).unwrap(),
        )
        .unwrap_err();

        assert!(err.to_string().contains("missing 'c'"));
    }

    #[test]
    fn parses_stdio_command_without_shell() {
        assert_eq!(
            split_args("--from tradingview-mcp-server tradingview-mcp"),
            vec!["--from", "tradingview-mcp-server", "tradingview-mcp"]
        );
        assert_eq!(
            default_stdio_args(),
            vec!["--from", "tradingview-mcp-server", "tradingview-mcp"]
        );
    }
}
