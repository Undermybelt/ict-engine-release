use anyhow::{anyhow, bail, Context, Result};
use chrono::Utc;
use reqwest::blocking::Client;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use super::openalice::{OptionsChainSummary, Quote};

const OPENCLI_DAEMON_URL: &str = "http://127.0.0.1:19825";
static COMMAND_COUNTER: AtomicU64 = AtomicU64::new(1);

pub fn yahoo_finance_quote(symbol: &str) -> Result<Quote> {
    if let Ok(quote) = yahoo_finance_quote_via_bb_browser(symbol) {
        return Ok(quote);
    }
    let client = daemon_client()?;
    let tab_id = navigate(
        &client,
        &format!("https://finance.yahoo.com/quote/{}/", symbol),
    )?;
    let value = exec(
        &client,
        tab_id,
        &format!(
            r#"(async () => {{
                const sym = {symbol:?}.toUpperCase().trim();
                const chartUrl = 'https://query1.finance.yahoo.com/v8/finance/chart/' + encodeURIComponent(sym) + '?interval=1d&range=1d';
                const resp = await fetch(chartUrl, {{ credentials: 'include' }});
                if (!resp.ok) throw new Error('HTTP ' + resp.status);
                const d = await resp.json();
                const chart = d?.chart?.result?.[0];
                if (!chart) throw new Error('Missing chart result');
                const meta = chart.meta || {{}};
                const prevClose = meta.previousClose || meta.chartPreviousClose || null;
                const price = meta.regularMarketPrice ?? null;
                return {{
                    symbol: meta.symbol || sym,
                    name: meta.shortName || meta.longName || sym,
                    price,
                    bid: meta.bid || null,
                    ask: meta.ask || null,
                    prevClose,
                    timestamp: meta.regularMarketTime || null
                }};
            }})()"#
        ),
    )?;

    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("unexpected quote payload from opencli daemon"))?;
    let last = object
        .get("price")
        .and_then(Value::as_f64)
        .ok_or_else(|| anyhow!("quote payload missing price"))?;
    let bid = object.get("bid").and_then(Value::as_f64).unwrap_or(last);
    let ask = object.get("ask").and_then(Value::as_f64).unwrap_or(last);

    Ok(Quote {
        symbol: object
            .get("symbol")
            .and_then(Value::as_str)
            .unwrap_or(symbol)
            .to_string(),
        bid,
        ask,
        last,
        timestamp: Utc::now(),
    })
}

fn yahoo_finance_quote_via_bb_browser(symbol: &str) -> Result<Quote> {
    let bb_browser =
        find_executable("bb-browser").ok_or_else(|| anyhow!("bb-browser executable not found"))?;

    let output = Command::new(bb_browser)
        .args(["site", "yahoo-finance/quote", symbol, "--json"])
        .output()
        .with_context(|| format!("failed to execute bb-browser quote for '{}'", symbol))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "bb-browser quote failed for '{}': {} {}",
            symbol,
            stdout.trim(),
            stderr.trim()
        );
    }

    let stdout =
        String::from_utf8(output.stdout).context("bb-browser output is not valid UTF-8")?;
    let value: Value = serde_json::from_str(&stdout).context("failed to parse bb-browser json")?;
    extract_bb_browser_quote(symbol, &value)
}

fn extract_bb_browser_quote(symbol: &str, value: &Value) -> Result<Quote> {
    if let Some(success) = value.get("success").and_then(Value::as_bool) {
        if !success {
            bail!(
                "bb-browser returned error for '{}': {}",
                symbol,
                value
                    .get("error")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown error")
            );
        }
        if let Some(data) = value.get("data") {
            return extract_bb_browser_quote(symbol, data);
        }
    }

    if let Some(array) = value.as_array() {
        if let Some(first) = array.first() {
            return extract_bb_browser_quote(symbol, first);
        }
    }

    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("unsupported bb-browser quote payload"))?;

    let last = find_number(object, &["price", "last", "last_price"])
        .ok_or_else(|| anyhow!("bb-browser quote missing price"))?;
    Ok(Quote {
        symbol: object
            .get("symbol")
            .and_then(Value::as_str)
            .unwrap_or(symbol)
            .to_string(),
        bid: find_number(object, &["bid", "bid_price"]).unwrap_or(last),
        ask: find_number(object, &["ask", "ask_price"]).unwrap_or(last),
        last,
        timestamp: Utc::now(),
    })
}

pub fn yahoo_finance_options_summary(symbol: &str) -> Result<OptionsChainSummary> {
    let client = daemon_client()?;
    let tab_id = navigate(
        &client,
        &format!(
            "https://finance.yahoo.com/quote/{}/options?p={}",
            symbol, symbol
        ),
    )?;
    let value = exec(
        &client,
        tab_id,
        &format!(
            r#"(async () => {{
                const sym = {symbol:?}.toUpperCase().trim();
                async function fetchChain(dateEpoch) {{
                    const url = new URL('https://query2.finance.yahoo.com/v7/finance/options/' + encodeURIComponent(sym));
                    if (dateEpoch) url.searchParams.set('date', String(dateEpoch));
                    const resp = await fetch(url.toString(), {{ credentials: 'include' }});
                    if (!resp.ok) throw new Error('HTTP ' + resp.status);
                    return await resp.json();
                }}

                const first = await fetchChain(null);
                const result = first?.optionChain?.result?.[0];
                if (!result) throw new Error('Missing options chain result');
                const expirationDates = result.expirationDates || [];
                const contracts = [];
                const pushContracts = (series, type, dte) => {{
                    for (const item of series || []) {{
                        contracts.push({{
                            option_type: type,
                            strike: item.strike ?? null,
                            open_interest: item.openInterest ?? null,
                            volume: item.volume ?? null,
                            implied_volatility: item.impliedVolatility ?? null,
                            dte
                        }});
                    }}
                }};
                const firstDte = expirationDates.length
                    ? Math.max(0, Math.ceil((expirationDates[0] * 1000 - Date.now()) / 86400000))
                    : null;
                pushContracts(result.options?.[0]?.calls, 'call', firstDte);
                pushContracts(result.options?.[0]?.puts, 'put', firstDte);

                for (const epoch of expirationDates.slice(1, 8)) {{
                    try {{
                        const chain = await fetchChain(epoch);
                        const item = chain?.optionChain?.result?.[0]?.options?.[0];
                        const dte = Math.max(0, Math.ceil((epoch * 1000 - Date.now()) / 86400000));
                        pushContracts(item?.calls, 'call', dte);
                        pushContracts(item?.puts, 'put', dte);
                    }} catch (_) {{}}
                }}

                if (!contracts.length) throw new Error('No contracts');
                const underlyingPrice = result.quote?.regularMarketPrice ?? null;
                const callOi = contracts.filter(x => x.option_type === 'call').reduce((s, x) => s + (x.open_interest || 0), 0);
                const putOi = contracts.filter(x => x.option_type === 'put').reduce((s, x) => s + (x.open_interest || 0), 0);
                const callVol = contracts.filter(x => x.option_type === 'call').reduce((s, x) => s + (x.volume || 0), 0);
                const putVol = contracts.filter(x => x.option_type === 'put').reduce((s, x) => s + (x.volume || 0), 0);
                const ivs = contracts.filter(x => underlyingPrice && x.dte != null && x.dte <= 45 && x.strike != null && Math.abs(x.strike - underlyingPrice) / Math.max(underlyingPrice, 1e-9) <= 0.1 && x.implied_volatility != null).map(x => x.implied_volatility);
                const nearestDte = contracts.map(x => x.dte).filter(x => x != null).sort((a, b) => a - b)[0] ?? null;
                const avgIv = ivs.length ? ivs.reduce((s, x) => s + x, 0) / ivs.length : null;

                return {{
                    symbol: sym,
                    underlying_price: underlyingPrice,
                    call_open_interest: callOi,
                    put_open_interest: putOi,
                    put_call_oi_ratio: callOi > 0 ? putOi / callOi : null,
                    call_volume: callVol,
                    put_volume: putVol,
                    put_call_volume_ratio: callVol > 0 ? putVol / callVol : null,
                    near_atm_implied_volatility: avgIv,
                    nearest_expiration_dte: nearestDte
                }};
            }})()"#
        ),
    )?;

    serde_json::from_value(value).context("failed to decode browser options summary")
}

fn daemon_client() -> Result<Client> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("failed to build daemon http client")?;
    let status: Value = client
        .get(format!("{OPENCLI_DAEMON_URL}/status"))
        .header("X-OpenCLI", "1")
        .send()
        .context("failed to query opencli daemon status")?
        .error_for_status()
        .context("opencli daemon status returned error")?
        .json()
        .context("failed to parse opencli daemon status")?;
    if !status
        .get("extensionConnected")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        bail!("opencli daemon is reachable but browser extension is not connected");
    }
    Ok(client)
}

fn navigate(client: &Client, url: &str) -> Result<u64> {
    let response = send_command(
        client,
        json!({
            "action": "navigate",
            "url": url,
            "workspace": "ict-engine"
        }),
    )?;
    response
        .get("tabId")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("opencli navigate response missing tabId"))
}

fn exec(client: &Client, tab_id: u64, code: &str) -> Result<Value> {
    send_command(
        client,
        json!({
            "action": "exec",
            "code": code,
            "workspace": "ict-engine",
            "tabId": tab_id
        }),
    )
}

fn send_command(client: &Client, body: Value) -> Result<Value> {
    let id = format!(
        "ict_engine_{}",
        COMMAND_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let mut object = body
        .as_object()
        .cloned()
        .ok_or_else(|| anyhow!("command body must be object"))?;
    object.insert("id".to_string(), Value::String(id));

    let response: Value = client
        .post(format!("{OPENCLI_DAEMON_URL}/command"))
        .header("X-OpenCLI", "1")
        .json(&object)
        .send()
        .context("failed to send opencli daemon command")?
        .error_for_status()
        .context("opencli daemon returned HTTP error")?
        .json()
        .context("failed to parse opencli daemon command response")?;

    if !response.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        bail!(
            "opencli daemon command failed: {}",
            response
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("unknown error")
        );
    }

    response
        .get("data")
        .cloned()
        .ok_or_else(|| anyhow!("opencli daemon response missing data"))
}

fn find_executable(name: &str) -> Option<PathBuf> {
    let path_lookup = std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|dir| dir.join(name))
            .find(|candidate| candidate.exists())
    });
    if path_lookup.is_some() {
        return path_lookup;
    }

    ["/opt/homebrew/bin", "/usr/local/bin"]
        .into_iter()
        .map(|dir| PathBuf::from(dir).join(name))
        .find(|candidate| candidate.exists())
}

fn find_number(map: &serde_json::Map<String, Value>, keys: &[&str]) -> Option<f64> {
    for key in keys {
        let value = map.get(*key)?;
        if let Some(number) = value.as_f64() {
            return Some(number);
        }
        if let Some(text) = value.as_str() {
            if let Ok(number) = text.replace(',', "").parse::<f64>() {
                return Some(number);
            }
        }
    }
    None
}
