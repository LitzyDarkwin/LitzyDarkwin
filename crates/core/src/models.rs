//! Canonical domain types shared by all services.

use serde::{Deserialize, Serialize};

/// Money is integer copper everywhere in the domain. Presentation formats gold.
pub type Copper = i64;
/// Unix timestamp in seconds (UTC). Addon uses `GetServerTime()`.
pub type UnixSecs = i64;

/// Data freshness state. Stale data may still be usable with reduced confidence;
/// stale and missing are NOT the same thing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Freshness {
    Fresh,
    Aging,
    Stale,
    Unavailable,
    Unknown,
}

/// Thresholds (seconds) describing when a source transitions fresh→aging→stale.
#[derive(Debug, Clone, Copy)]
pub struct FreshnessPolicy {
    pub fresh_secs: i64,
    pub stale_secs: i64,
}

impl FreshnessPolicy {
    pub const fn new(fresh_secs: i64, stale_secs: i64) -> Self {
        Self { fresh_secs, stale_secs }
    }

    /// Classify `updated_at` (None = never recorded) observed at `now`.
    pub fn classify(&self, updated_at: Option<UnixSecs>, now: UnixSecs) -> Freshness {
        match updated_at {
            None => Freshness::Unknown,
            Some(ts) if ts <= 0 => Freshness::Unknown,
            Some(ts) => {
                let age = now.saturating_sub(ts);
                if age <= self.fresh_secs {
                    Freshness::Fresh
                } else if age <= self.stale_secs {
                    Freshness::Aging
                } else {
                    Freshness::Stale
                }
            }
        }
    }
}

/// Source-specific freshness policies (defaults; overridable via settings).
pub mod policies {
    use super::FreshnessPolicy;
    /// TSM public data refreshes roughly daily.
    pub const MARKET: FreshnessPolicy = FreshnessPolicy::new(12 * 3600, 72 * 3600);
    /// Addon SV sync requires user action + WoW flush.
    pub const ADDON_SYNC: FreshnessPolicy = FreshnessPolicy::new(10 * 60, 24 * 3600);
    /// Crowd aggregates are bucketed daily.
    pub const CROWD: FreshnessPolicy = FreshnessPolicy::new(24 * 3600, 14 * 24 * 3600);
}

/// Confidence in 0.0–1.0 derived from measured factors (never a bare guess).
/// Use `Confidence::label()` for UI buckets.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Confidence(pub f64);

impl Confidence {
    pub fn new(v: f64) -> Self {
        Self(v.clamp(0.0, 1.0))
    }
    pub fn label(&self) -> &'static str {
        if self.0 >= 0.7 {
            "High"
        } else if self.0 >= 0.4 {
            "Medium"
        } else {
            "Low"
        }
    }
}

/// Probability band for estimates that carry uncertainty. No false precision.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Band {
    pub p10: f64,
    pub median: f64,
    pub p90: f64,
}

/// Canonical farm-category keys. Extensible without schema changes.
pub mod category {
    pub const MINING: &str = "mining";
    pub const HERBALISM: &str = "herbalism";
    pub const FISHING: &str = "fishing";
    pub const SKINNING: &str = "skinning";
    pub const TREASURE: &str = "treasure";
    pub const MOB: &str = "mob";
    pub const OTHER: &str = "other";
}

/// Observation provenance. Stored lowercase in `resource_observations.source`.
pub mod source {
    pub const SEED: &str = "seed";
    pub const ADDON: &str = "addon";
    pub const IMPORT: &str = "import";
    pub const CROWD: &str = "crowd";
}

/// Session lifecycle states (stored lowercase).
pub mod session_status {
    pub const IDLE: &str = "idle";
    pub const PREPARED: &str = "prepared";
    pub const ACTIVE: &str = "active";
    pub const PAUSED: &str = "paused";
    pub const ENDED: &str = "ended";
    pub const IMPORTED: &str = "imported";
    pub const ANALYZED: &str = "analyzed";
}

/// Format copper as a gold string, e.g. 12_3456 → "1.23g". Presentation helper
/// shared so Rust-produced strings and TS formatting agree.
pub fn format_gold(copper: Copper) -> String {
    let gold = copper as f64 / 10_000.0;
    if gold >= 1000.0 {
        format!("{:.0}g", gold)
    } else if gold >= 100.0 {
        format!("{gold:.1}g")
    } else {
        format!("{gold:.2}g")
    }
}

/// Current Unix time (UTC). Single choke point so tests can avoid wall-clock
/// dependence by passing explicit timestamps instead.
pub fn now_unix() -> UnixSecs {
    chrono::Utc::now().timestamp()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn freshness_boundaries() {
        let p = FreshnessPolicy::new(60, 120);
        assert_eq!(p.classify(None, 1000), Freshness::Unknown);
        assert_eq!(p.classify(Some(0), 1000), Freshness::Unknown);
        assert_eq!(p.classify(Some(950), 1000), Freshness::Fresh);
        assert_eq!(p.classify(Some(900), 1000), Freshness::Aging);
        assert_eq!(p.classify(Some(800), 1000), Freshness::Stale);
    }

    #[test]
    fn confidence_clamps_and_labels() {
        assert_eq!(Confidence::new(9.0).0, 1.0);
        assert_eq!(Confidence::new(-1.0).0, 0.0);
        assert_eq!(Confidence::new(0.9).label(), "High");
        assert_eq!(Confidence::new(0.5).label(), "Medium");
        assert_eq!(Confidence::new(0.1).label(), "Low");
    }

    #[test]
    fn gold_formatting() {
        assert_eq!(format_gold(0), "0.00g");
        assert_eq!(format_gold(12_345), "1.23g");
        assert_eq!(format_gold(1_234_567), "123.5g");
        assert_eq!(format_gold(12_000_000), "1200g");
    }
}
