use crate::data::realtime::market_support::AuxiliaryMarketEvidence;
use crate::state::LearningState;
use crate::types::Candle;

#[derive(Clone, Copy)]
pub struct AnalyzeBuildContext<'a> {
    pub symbol: &'a str,
    pub paired_candles: Option<&'a [Candle]>,
    pub auxiliary: Option<&'a AuxiliaryMarketEvidence>,
    pub learning_state: &'a LearningState,
    pub multi_timeframe_summary: &'a [String],
    pub native_frames: AnalyzeNativeFrames<'a>,
}

#[derive(Clone, Copy, Default)]
pub struct AnalyzeNativeFrames<'a> {
    pub d1: Option<&'a [Candle]>,
    pub h4: Option<&'a [Candle]>,
    pub h1: Option<&'a [Candle]>,
    pub m30: Option<&'a [Candle]>,
    pub m15: Option<&'a [Candle]>,
    pub m5: Option<&'a [Candle]>,
    pub m1: Option<&'a [Candle]>,
}
