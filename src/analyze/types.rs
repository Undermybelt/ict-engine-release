use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct AnalyzeMultiTimeframeInterval {
    pub interval: String,
    pub bars: usize,
    pub source_detail: String,
}
