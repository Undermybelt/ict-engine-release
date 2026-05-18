use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use crate::types::Candle;

use super::{
    crypto_public_runtime::CryptoPublicRuntimeProvider,
    external_http_runtime::ExternalHttpRuntimeProvider,
    market_support::{AuxiliaryMarketEvidence, OptionsChainSummary, SpotInstrumentKind},
    provider::RealtimeDataProvider,
    tradingview_mcp_runtime::TradingViewMcpRuntimeProvider,
    yfinance_runtime::YahooFinanceProvider,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveDataBackend {
    ExternalHttp,
    Yfinance,
    CryptoPublic,
    TradingViewMcp,
}

impl LiveDataBackend {
    pub fn parse(input: &str) -> Result<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "external_http" | "external_http_runtime" => Ok(Self::ExternalHttp),
            "yfinance" => Ok(Self::Yfinance),
            "crypto_public" | "crypto_public_runtime" => Ok(Self::CryptoPublic),
            "tradingview" | "tradingview_mcp" | "tv_mcp" => Ok(Self::TradingViewMcp),
            other => bail!("unsupported live data backend '{}'", other),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::ExternalHttp => "external_http_runtime",
            Self::Yfinance => "yfinance",
            Self::CryptoPublic => "crypto_public_runtime",
            Self::TradingViewMcp => "tradingview_mcp",
        }
    }
}

pub trait IntegratedLiveDataSource: Send + Sync {
    fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>>;

    fn fetch_futures_last_price(&self, symbol: &str) -> Result<f64>;

    fn fetch_spot_candles(
        &self,
        kind: SpotInstrumentKind,
        symbol: &str,
        interval: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>>;

    fn fetch_spot_last_price(&self, kind: SpotInstrumentKind, symbol: &str) -> Result<f64>;

    fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary>;

    fn fetch_options_volatility_proxy_summary(
        &self,
        proxy_symbol: &str,
        underlying_symbol: &str,
    ) -> Result<OptionsChainSummary>;

    fn build_auxiliary_evidence(
        &self,
        spot_kind: SpotInstrumentKind,
        spot_symbol: &str,
        options_symbol: &str,
        futures_candles: &[Candle],
        spot_candles: &[Candle],
        options_summary: &OptionsChainSummary,
    ) -> AuxiliaryMarketEvidence;

    fn apply_auxiliary_evidence_to_outcome(
        &self,
        base_distribution: &[f64],
        directional_bias: f64,
        uncertainty_penalty: f64,
    ) -> Vec<f64>;
}

impl IntegratedLiveDataSource for ExternalHttpRuntimeProvider {
    fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        ExternalHttpRuntimeProvider::fetch_futures_candles(self, symbol, interval, start, end)
    }

    fn fetch_futures_last_price(&self, symbol: &str) -> Result<f64> {
        Ok(futures::executor::block_on(self.get_quote(symbol))?.last)
    }

    fn fetch_spot_candles(
        &self,
        kind: SpotInstrumentKind,
        symbol: &str,
        interval: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        ExternalHttpRuntimeProvider::fetch_spot_candles(self, kind, symbol, interval, start, end)
    }

    fn fetch_spot_last_price(&self, _kind: SpotInstrumentKind, symbol: &str) -> Result<f64> {
        Ok(futures::executor::block_on(self.get_quote(symbol))?.last)
    }

    fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary> {
        ExternalHttpRuntimeProvider::fetch_options_chain_summary(self, symbol)
    }

    fn fetch_options_volatility_proxy_summary(
        &self,
        _proxy_symbol: &str,
        _underlying_symbol: &str,
    ) -> Result<OptionsChainSummary> {
        bail!("external_http runtime does not support options volatility proxy fallback")
    }

    fn build_auxiliary_evidence(
        &self,
        spot_kind: SpotInstrumentKind,
        spot_symbol: &str,
        options_symbol: &str,
        futures_candles: &[Candle],
        spot_candles: &[Candle],
        options_summary: &OptionsChainSummary,
    ) -> AuxiliaryMarketEvidence {
        ExternalHttpRuntimeProvider::build_auxiliary_evidence(
            self,
            spot_kind,
            spot_symbol,
            options_symbol,
            futures_candles,
            spot_candles,
            options_summary,
        )
    }

    fn apply_auxiliary_evidence_to_outcome(
        &self,
        base_distribution: &[f64],
        directional_bias: f64,
        uncertainty_penalty: f64,
    ) -> Vec<f64> {
        ExternalHttpRuntimeProvider::apply_auxiliary_evidence_to_outcome(
            self,
            base_distribution,
            directional_bias,
            uncertainty_penalty,
        )
    }
}

impl IntegratedLiveDataSource for YahooFinanceProvider {
    fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        YahooFinanceProvider::fetch_futures_candles(self, symbol, interval, start, end)
    }

    fn fetch_futures_last_price(&self, symbol: &str) -> Result<f64> {
        Ok(YahooFinanceProvider::fetch_futures_quote(self, symbol)?.last)
    }

    fn fetch_spot_candles(
        &self,
        kind: SpotInstrumentKind,
        symbol: &str,
        interval: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        YahooFinanceProvider::fetch_spot_candles(self, kind, symbol, interval, start, end)
    }

    fn fetch_spot_last_price(&self, kind: SpotInstrumentKind, symbol: &str) -> Result<f64> {
        Ok(YahooFinanceProvider::fetch_spot_quote(self, kind, symbol)?.last)
    }

    fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary> {
        YahooFinanceProvider::fetch_options_chain_summary(self, symbol)
    }

    fn fetch_options_volatility_proxy_summary(
        &self,
        proxy_symbol: &str,
        underlying_symbol: &str,
    ) -> Result<OptionsChainSummary> {
        YahooFinanceProvider::fetch_options_volatility_proxy_summary(
            self,
            proxy_symbol,
            underlying_symbol,
        )
    }

    fn build_auxiliary_evidence(
        &self,
        spot_kind: SpotInstrumentKind,
        spot_symbol: &str,
        options_symbol: &str,
        futures_candles: &[Candle],
        spot_candles: &[Candle],
        options_summary: &OptionsChainSummary,
    ) -> AuxiliaryMarketEvidence {
        YahooFinanceProvider::build_auxiliary_evidence(
            self,
            spot_kind,
            spot_symbol,
            options_symbol,
            futures_candles,
            spot_candles,
            options_summary,
        )
    }

    fn apply_auxiliary_evidence_to_outcome(
        &self,
        base_distribution: &[f64],
        directional_bias: f64,
        uncertainty_penalty: f64,
    ) -> Vec<f64> {
        YahooFinanceProvider::apply_auxiliary_evidence_to_outcome(
            self,
            base_distribution,
            directional_bias,
            uncertainty_penalty,
        )
    }
}

impl IntegratedLiveDataSource for TradingViewMcpRuntimeProvider {
    fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        TradingViewMcpRuntimeProvider::fetch_futures_candles(self, symbol, interval, start, end)
    }

    fn fetch_futures_last_price(&self, symbol: &str) -> Result<f64> {
        Ok(TradingViewMcpRuntimeProvider::fetch_futures_quote(self, symbol)?.last)
    }

    fn fetch_spot_candles(
        &self,
        kind: SpotInstrumentKind,
        symbol: &str,
        interval: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        TradingViewMcpRuntimeProvider::fetch_spot_candles(self, kind, symbol, interval, start, end)
    }

    fn fetch_spot_last_price(&self, kind: SpotInstrumentKind, symbol: &str) -> Result<f64> {
        Ok(TradingViewMcpRuntimeProvider::fetch_spot_quote(self, kind, symbol)?.last)
    }

    fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary> {
        TradingViewMcpRuntimeProvider::fetch_options_chain_summary(self, symbol)
    }

    fn fetch_options_volatility_proxy_summary(
        &self,
        proxy_symbol: &str,
        underlying_symbol: &str,
    ) -> Result<OptionsChainSummary> {
        TradingViewMcpRuntimeProvider::fetch_options_volatility_proxy_summary(
            self,
            proxy_symbol,
            underlying_symbol,
        )
    }

    fn build_auxiliary_evidence(
        &self,
        spot_kind: SpotInstrumentKind,
        spot_symbol: &str,
        options_symbol: &str,
        futures_candles: &[Candle],
        spot_candles: &[Candle],
        options_summary: &OptionsChainSummary,
    ) -> AuxiliaryMarketEvidence {
        TradingViewMcpRuntimeProvider::build_auxiliary_evidence(
            self,
            spot_kind,
            spot_symbol,
            options_symbol,
            futures_candles,
            spot_candles,
            options_summary,
        )
    }

    fn apply_auxiliary_evidence_to_outcome(
        &self,
        base_distribution: &[f64],
        directional_bias: f64,
        uncertainty_penalty: f64,
    ) -> Vec<f64> {
        TradingViewMcpRuntimeProvider::apply_auxiliary_evidence_to_outcome(
            self,
            base_distribution,
            directional_bias,
            uncertainty_penalty,
        )
    }
}

impl IntegratedLiveDataSource for CryptoPublicRuntimeProvider {
    fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        CryptoPublicRuntimeProvider::fetch_futures_candles(self, symbol, interval, start, end)
    }

    fn fetch_futures_last_price(&self, symbol: &str) -> Result<f64> {
        Ok(futures::executor::block_on(self.get_quote(symbol))?.last)
    }

    fn fetch_spot_candles(
        &self,
        kind: SpotInstrumentKind,
        symbol: &str,
        interval: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        CryptoPublicRuntimeProvider::fetch_spot_candles(self, kind, symbol, interval, start, end)
    }

    fn fetch_spot_last_price(&self, _kind: SpotInstrumentKind, _symbol: &str) -> Result<f64> {
        bail!("crypto_public runtime does not provide spot quote data")
    }

    fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary> {
        CryptoPublicRuntimeProvider::fetch_options_chain_summary(self, symbol)
    }

    fn fetch_options_volatility_proxy_summary(
        &self,
        _proxy_symbol: &str,
        _underlying_symbol: &str,
    ) -> Result<OptionsChainSummary> {
        bail!("crypto_public runtime does not support options volatility proxy fallback")
    }

    fn build_auxiliary_evidence(
        &self,
        spot_kind: SpotInstrumentKind,
        spot_symbol: &str,
        options_symbol: &str,
        futures_candles: &[Candle],
        spot_candles: &[Candle],
        options_summary: &OptionsChainSummary,
    ) -> AuxiliaryMarketEvidence {
        CryptoPublicRuntimeProvider::build_auxiliary_evidence(
            self,
            spot_kind,
            spot_symbol,
            options_symbol,
            futures_candles,
            spot_candles,
            options_summary,
        )
    }

    fn apply_auxiliary_evidence_to_outcome(
        &self,
        base_distribution: &[f64],
        directional_bias: f64,
        uncertainty_penalty: f64,
    ) -> Vec<f64> {
        CryptoPublicRuntimeProvider::apply_auxiliary_evidence_to_outcome(
            self,
            base_distribution,
            directional_bias,
            uncertainty_penalty,
        )
    }
}

pub fn build_live_data_source(
    backend: LiveDataBackend,
    base_url: &str,
) -> Box<dyn IntegratedLiveDataSource> {
    match backend {
        LiveDataBackend::ExternalHttp => Box::new(ExternalHttpRuntimeProvider::new(base_url, None)),
        LiveDataBackend::Yfinance => Box::new(YahooFinanceProvider::new(base_url)),
        LiveDataBackend::CryptoPublic => Box::new(CryptoPublicRuntimeProvider::new(base_url)),
        LiveDataBackend::TradingViewMcp => Box::new(TradingViewMcpRuntimeProvider::new(base_url)),
    }
}
