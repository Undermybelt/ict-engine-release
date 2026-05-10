//! Trading-session classifier focused on ICT kill zones.
//!
//! `data/loader.rs` already classifies candles into Asia/Europe/Us
//! buckets for data-cleaning purposes (gap rebasing). That classifier
//! is private and tuned for cleaning, not for canonical-setup
//! matching: it merges silver-bullet and judas windows into the
//! generic NY session and uses non-strict "Off-Hours / Overnight"
//! buckets that don't map cleanly to ICT theory.
//!
//! This module exposes a stable, public surface dedicated to the
//! kill-zone windows that canonical-setup matchers need:
//!
//! - `AsiaSession`     09:00-15:00 Tokyo (canonical Asia kill zone)
//! - `LondonSession`   07:00-13:00 London (canonical London KZ)
//! - `NySession`       08:30-16:00 NY (canonical NY KZ — the broad
//!   window; the silver bullet / judas zones below are subsets of
//!   this one)
//! - `SilverBulletAm`  03:00-04:00 NY (London open kill zone)
//! - `SilverBulletPm`  10:00-11:00 NY (NY am session kill zone)
//! - `JudasWindow`     08:30-09:30 NY (first hour of NY equity open)
//!
//! `classify_session_zones` returns **all** matching zones for a
//! timestamp (silver bullet windows are subsets of NySession), so
//! matchers can pick the most specific one without re-running the
//! classifier.

use chrono::{DateTime, Datelike, Timelike, Utc, Weekday};
use chrono_tz::America::New_York;
use chrono_tz::Asia::Tokyo;
use chrono_tz::Europe::London;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SessionKillZone {
    AsiaSession,
    LondonSession,
    NySession,
    SilverBulletAm,
    SilverBulletPm,
    JudasWindow,
}

impl SessionKillZone {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AsiaSession => "asia_session",
            Self::LondonSession => "london_session",
            Self::NySession => "ny_session",
            Self::SilverBulletAm => "silver_bullet_am",
            Self::SilverBulletPm => "silver_bullet_pm",
            Self::JudasWindow => "judas_window",
        }
    }
}

/// Returns every kill zone the timestamp falls inside. Silver-bullet
/// and judas windows are subsets of `NySession`, so a typical NY-am
/// timestamp may produce up to two entries.
///
/// Saturdays and Sundays before the futures-open window (18:00 NY)
/// produce an empty `Vec` because no kill zone is in effect.
pub fn classify_session_zones(timestamp: DateTime<Utc>) -> Vec<SessionKillZone> {
    let mut out = Vec::new();
    let ny = timestamp.with_timezone(&New_York);
    let weekday = ny.weekday();
    if matches!(weekday, Weekday::Sat) {
        return out;
    }
    let ny_minutes = ny.hour() * 60 + ny.minute();
    if matches!(weekday, Weekday::Sun) && ny_minutes < 18 * 60 {
        return out;
    }

    let tokyo = timestamp.with_timezone(&Tokyo);
    let london = timestamp.with_timezone(&London);
    let tokyo_minutes = tokyo.hour() * 60 + tokyo.minute();
    let london_minutes = london.hour() * 60 + london.minute();

    if (9 * 60..15 * 60).contains(&tokyo_minutes) {
        out.push(SessionKillZone::AsiaSession);
    }
    if (7 * 60..13 * 60).contains(&london_minutes) {
        out.push(SessionKillZone::LondonSession);
    }
    // NY session is 08:30-16:00 NY local.
    let ny_open = 8 * 60 + 30;
    let ny_close = 16 * 60;
    if (ny_open..ny_close).contains(&ny_minutes) {
        out.push(SessionKillZone::NySession);
        // Judas: first hour of NY equity open.
        if (ny_open..(ny_open + 60)).contains(&ny_minutes) {
            out.push(SessionKillZone::JudasWindow);
        }
        // SilverBulletPm: 10:00-11:00 NY.
        if (10 * 60..11 * 60).contains(&ny_minutes) {
            out.push(SessionKillZone::SilverBulletPm);
        }
    }
    // SilverBulletAm: 03:00-04:00 NY (precedes the NY equity open
    // and the London am — overlap with LondonSession is expected).
    if (3 * 60..4 * 60).contains(&ny_minutes) {
        out.push(SessionKillZone::SilverBulletAm);
    }
    out
}

pub fn is_in_zone(timestamp: DateTime<Utc>, zone: SessionKillZone) -> bool {
    classify_session_zones(timestamp).contains(&zone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn ny_time(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
        New_York
            .with_ymd_and_hms(year, month, day, hour, minute, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn empty_for_saturday() {
        // Saturday should never produce any zone.
        let ts = ny_time(2026, 1, 3, 12, 0); // 2026-01-03 was a Saturday
        assert!(classify_session_zones(ts).is_empty());
    }

    #[test]
    fn judas_and_ny_session_overlap_for_first_hour() {
        // 2026-01-05 is a Monday. 08:45 NY -> NySession + JudasWindow.
        let ts = ny_time(2026, 1, 5, 8, 45);
        let zones = classify_session_zones(ts);
        assert!(zones.contains(&SessionKillZone::NySession));
        assert!(zones.contains(&SessionKillZone::JudasWindow));
        assert!(!zones.contains(&SessionKillZone::SilverBulletPm));
    }

    #[test]
    fn silver_bullet_pm_inside_ny_session() {
        // 10:30 NY -> NySession + SilverBulletPm.
        let ts = ny_time(2026, 1, 5, 10, 30);
        let zones = classify_session_zones(ts);
        assert!(zones.contains(&SessionKillZone::NySession));
        assert!(zones.contains(&SessionKillZone::SilverBulletPm));
        assert!(!zones.contains(&SessionKillZone::JudasWindow));
    }

    #[test]
    fn silver_bullet_am_independent_of_ny_session() {
        // 03:30 NY -> SilverBulletAm only (NY equity hasn't opened).
        let ts = ny_time(2026, 1, 5, 3, 30);
        let zones = classify_session_zones(ts);
        assert!(zones.contains(&SessionKillZone::SilverBulletAm));
        assert!(!zones.contains(&SessionKillZone::NySession));
    }

    #[test]
    fn asia_session_overnight_in_ny() {
        // 21:00 NY Sunday corresponds to ~10:00 Tokyo Monday → Asia.
        let ts = ny_time(2026, 1, 4, 21, 0); // Sunday 21:00 NY
        let zones = classify_session_zones(ts);
        assert!(zones.contains(&SessionKillZone::AsiaSession));
    }

    #[test]
    fn london_morning_window() {
        // 04:30 NY ≈ 09:30 London → LondonSession.
        let ts = ny_time(2026, 1, 5, 4, 30);
        let zones = classify_session_zones(ts);
        assert!(zones.contains(&SessionKillZone::LondonSession));
    }

    #[test]
    fn is_in_zone_dispatches_through_classifier() {
        let ts = ny_time(2026, 1, 5, 10, 30);
        assert!(is_in_zone(ts, SessionKillZone::SilverBulletPm));
        assert!(is_in_zone(ts, SessionKillZone::NySession));
        assert!(!is_in_zone(ts, SessionKillZone::SilverBulletAm));
    }

    #[test]
    fn off_hours_window_returns_empty() {
        // 17:30 NY weekday — between cash close and futures re-open
        // -> nothing.
        let ts = ny_time(2026, 1, 5, 17, 30);
        let zones = classify_session_zones(ts);
        assert!(
            zones.is_empty(),
            "expected empty zone list at off-hours, got {:?}",
            zones
        );
    }
}
