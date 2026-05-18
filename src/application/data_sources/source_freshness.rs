use chrono::{DateTime, Duration, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct DataFreshness {
    pub status: String,
    pub age_seconds: i64,
}

pub fn classify_freshness(fetched_at: DateTime<Utc>, now: DateTime<Utc>) -> DataFreshness {
    let age = now.signed_duration_since(fetched_at);
    let status = if age <= Duration::minutes(5) {
        "fresh"
    } else if age <= Duration::minutes(30) {
        "aging"
    } else {
        "stale"
    };
    DataFreshness {
        status: status.to_string(),
        age_seconds: age.num_seconds(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_marks_stale_when_old() {
        let now = Utc::now();
        let stale = classify_freshness(now - Duration::minutes(31), now);
        assert_eq!(stale.status, "stale");
    }
}
