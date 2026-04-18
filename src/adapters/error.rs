use crate::adapters::contract::{ExternalDiagnostics, ExternalErrorCategory, ExternalToolError};

pub fn retryable_for_category(category: &ExternalErrorCategory) -> bool {
    matches!(category, ExternalErrorCategory::Network | ExternalErrorCategory::RateLimit)
}

pub fn classify_error(category: &str, message: impl Into<String>) -> ExternalToolError {
    let normalized = match category {
        "api" => ExternalErrorCategory::Api,
        "auth" => ExternalErrorCategory::Auth,
        "network" => ExternalErrorCategory::Network,
        "rate_limit" => ExternalErrorCategory::RateLimit,
        "validation" => ExternalErrorCategory::Validation,
        "config" => ExternalErrorCategory::Config,
        "io" => ExternalErrorCategory::Io,
        "parse" => ExternalErrorCategory::Parse,
        _ => ExternalErrorCategory::Unknown,
    };
    ExternalToolError {
        retryable: retryable_for_category(&normalized),
        category: normalized,
        message: message.into(),
        diagnostics: ExternalDiagnostics::default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn network_and_rate_limit_are_retryable() {
        assert!(retryable_for_category(&ExternalErrorCategory::Network));
        assert!(retryable_for_category(&ExternalErrorCategory::RateLimit));
        assert!(!retryable_for_category(&ExternalErrorCategory::Validation));
    }

    #[test]
    fn classify_error_maps_known_categories() {
        let err = classify_error("rate_limit", "too many requests");
        assert!(err.retryable);
        assert!(matches!(err.category, ExternalErrorCategory::RateLimit));
    }
}
