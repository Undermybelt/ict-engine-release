pub fn humanize_term(term: &str) -> String {
    match term {
        "pre_bayes_gate" => "先验证据门".to_string(),
        "pass_hard" => "高置信通过".to_string(),
        "pass_neutralized" => "通过但需确认".to_string(),
        "observe_only" => "仅观察".to_string(),
        "factor_alignment" => "因子方向一致性".to_string(),
        "factor_uncertainty" => "因子不确定性".to_string(),
        "multi_timeframe_resonance" => "多周期共振".to_string(),
        "expansion_manipulation" => "扩张/诱导识别目标".to_string(),
        other => other.to_string(),
    }
}

pub fn humanize_status(term: &str) -> String {
    humanize_term(term)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_term_is_humanized() {
        assert_eq!(humanize_term("pass_hard"), "高置信通过");
    }

    #[test]
    fn unknown_term_falls_back_to_original() {
        assert_eq!(humanize_term("unknown_token"), "unknown_token");
    }
}
