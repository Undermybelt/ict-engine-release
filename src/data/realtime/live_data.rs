use anyhow::{bail, Result};
use chrono::{DateTime, Utc};

use crate::types::Candle;

use super::{
    nofx::NofxProvider,
    openalice::{
        AuxiliaryMarketEvidence, OpenAliceProvider, OptionsChainSummary, SpotInstrumentKind,
    },
    openbb::OpenBBProvider,
    provider::RealtimeDataProvider,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LiveDataBackend {
    OpenAlice,
    OpenBB,
    Nofx,
}

impl LiveDataBackend {
    pub fn parse(input: &str) -> Result<Self> {
        match input.trim().to_ascii_lowercase().as_str() {
            "openalice" => Ok(Self::OpenAlice),
            "openbb" => Ok(Self::OpenBB),
            "nofx" => Ok(Self::Nofx),
            "yfinance" => Ok(Self::OpenBB),
            other => bail!("unsupported live data backend '{}'", other),
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAlice => "openalice",
            Self::OpenBB => "openbb",
            Self::Nofx => "nofx",
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

impl IntegratedLiveDataSource for OpenAliceProvider {
    fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        OpenAliceProvider::fetch_futures_candles(self, symbol, interval, start, end)
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
        OpenAliceProvider::fetch_spot_candles(self, kind, symbol, interval, start, end)
    }

    fn fetch_spot_last_price(&self, _kind: SpotInstrumentKind, symbol: &str) -> Result<f64> {
        Ok(futures::executor::block_on(self.get_quote(symbol))?.last)
    }

    fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary> {
        OpenAliceProvider::fetch_options_chain_summary(self, symbol)
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
        OpenAliceProvider::build_auxiliary_evidence(
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
        OpenAliceProvider::apply_auxiliary_evidence_to_outcome(
            self,
            base_distribution,
            directional_bias,
            uncertainty_penalty,
        )
    }
}

impl IntegratedLiveDataSource for OpenBBProvider {
    fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        OpenBBProvider::fetch_futures_candles(self, symbol, interval, start, end)
    }

    fn fetch_futures_last_price(&self, symbol: &str) -> Result<f64> {
        Ok(OpenBBProvider::fetch_futures_quote(self, symbol)?.last)
    }

    fn fetch_spot_candles(
        &self,
        kind: SpotInstrumentKind,
        symbol: &str,
        interval: Option<&str>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        OpenBBProvider::fetch_spot_candles(self, kind, symbol, interval, start, end)
    }

    fn fetch_spot_last_price(&self, kind: SpotInstrumentKind, symbol: &str) -> Result<f64> {
        Ok(OpenBBProvider::fetch_spot_quote(self, kind, symbol)?.last)
    }

    fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary> {
        OpenBBProvider::fetch_options_chain_summary(self, symbol)
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
        OpenBBProvider::build_auxiliary_evidence(
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
        OpenBBProvider::apply_auxiliary_evidence_to_outcome(
            self,
            base_distribution,
            directional_bias,
            uncertainty_penalty,
        )
    }
}

impl IntegratedLiveDataSource for NofxProvider {
    fn fetch_futures_candles(
        &self,
        symbol: &str,
        interval: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Result<Vec<Candle>> {
        NofxProvider::fetch_futures_candles(self, symbol, interval, start, end)
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
        NofxProvider::fetch_spot_candles(self, kind, symbol, interval, start, end)
    }

    fn fetch_spot_last_price(&self, _kind: SpotInstrumentKind, _symbol: &str) -> Result<f64> {
        bail!("nofx backend does not provide spot quote data")
    }

    fn fetch_options_chain_summary(&self, symbol: &str) -> Result<OptionsChainSummary> {
        NofxProvider::fetch_options_chain_summary(self, symbol)
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
        NofxProvider::build_auxiliary_evidence(
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
        NofxProvider::apply_auxiliary_evidence_to_outcome(
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
        LiveDataBackend::OpenAlice => Box::new(OpenAliceProvider::new(base_url, None)),
        LiveDataBackend::OpenBB => Box::new(OpenBBProvider::new(base_url)),
        LiveDataBackend::Nofx => Box::new(NofxProvider::new(base_url)),
    }
}
