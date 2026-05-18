use anyhow::Result;

use crate::analyze_builder_types::AnalyzeNativeFrames;
use crate::application::regime::native_frame_weight;
use crate::config::build_frame_features;
use crate::config::FrameFeatures;
use crate::hmm::{ForwardBackward, Viterbi};
use crate::types::{HMMParams, RegimeProbs};

#[derive(Clone)]
pub struct NativeFrameComputation {
    pub weight: f64,
    pub features: FrameFeatures,
    pub regime_probs: RegimeProbs,
    pub log_likelihood: f64,
    pub viterbi_log_likelihood: f64,
}

pub fn native_frame_computations(
    params: &HMMParams,
    native_frames: AnalyzeNativeFrames<'_>,
) -> Result<Vec<NativeFrameComputation>> {
    let mut signals = Vec::new();
    for (interval, candles) in [
        ("1d", native_frames.d1),
        ("4h", native_frames.h4),
        ("1h", native_frames.h1),
        ("15m", native_frames.m15),
        ("5m", native_frames.m5),
        ("1m", native_frames.m1),
    ] {
        let Some(candles) = candles else {
            continue;
        };
        let features = build_frame_features(candles)?;
        let (log_alpha, log_likelihood) = ForwardBackward::forward(&features.observations, params);
        let log_beta = ForwardBackward::backward(&features.observations, params);
        let gamma = ForwardBackward::compute_gamma(&log_alpha, &log_beta, log_likelihood);
        let (_, viterbi_log_likelihood) = Viterbi::decode(&features.observations, params);
        signals.push(NativeFrameComputation {
            weight: native_frame_weight(interval),
            regime_probs: regime_probs_from_log_gamma(gamma.last())?,
            log_likelihood,
            viterbi_log_likelihood,
            features,
        });
    }
    Ok(signals)
}

fn regime_probs_from_log_gamma(log_gamma: Option<&Vec<f64>>) -> Result<RegimeProbs> {
    let log_gamma =
        log_gamma.ok_or_else(|| anyhow::anyhow!("missing HMM posterior probabilities"))?;
    if log_gamma.len() < 3 {
        anyhow::bail!("expected 3 HMM states, got {}", log_gamma.len());
    }

    let accumulation = log_gamma[0].exp();
    let manipulation_expansion = log_gamma[1].exp();
    let distribution = log_gamma[2].exp();
    let sum = accumulation + manipulation_expansion + distribution;
    if sum <= f64::EPSILON {
        anyhow::bail!("invalid HMM posterior: probabilities sum to zero");
    }

    Ok(RegimeProbs {
        accumulation: accumulation / sum,
        manipulation_expansion: manipulation_expansion / sum,
        distribution: distribution / sum,
    })
}
