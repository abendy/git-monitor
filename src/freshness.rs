//! Data freshness: when data was last read, why, and whether the latest try failed.
//!
//! Git data uses this today. Provider data can reuse the same states later, so
//! nothing here knows where the data comes from.

use std::time::{Duration, Instant};

/// What triggered a refresh
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshReason {
    /// First load when the app starts
    Startup,
    /// The file watcher saw a change
    FileChange,
    /// A command finished and asked for a refresh
    Command,
    /// The user pressed refresh
    Manual,
    /// The view needed different data (page, mode, branch)
    View,
}

impl RefreshReason {
    /// Short label for display
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Startup => "start",
            Self::FileChange => "files",
            Self::Command => "command",
            Self::Manual => "manual",
            Self::View => "view",
        }
    }
}

/// A refresh that failed
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshFailure {
    /// When it failed
    pub at: Instant,
    /// What triggered it
    pub reason: RefreshReason,
    /// The error, in full
    pub error: String,
}

/// Where a data source stands right now
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FreshnessState<'a> {
    /// Nothing loaded or tried yet
    Unknown,
    /// The latest try succeeded
    Current {
        /// Time since the data was read
        age: Duration,
        /// What triggered the read
        reason: RefreshReason,
    },
    /// The latest try failed; older data, if any, is still shown
    Stale {
        /// Time since the shown data was read, if any was
        age: Option<Duration>,
        /// The failure
        failure: &'a RefreshFailure,
    },
}

/// Freshness of one data source
#[derive(Debug, Clone, Default)]
pub struct Freshness {
    /// Last successful read and its trigger
    loaded: Option<(Instant, RefreshReason)>,
    /// Latest failure, cleared by the next success
    failure: Option<RefreshFailure>,
}

impl Freshness {
    /// Record a successful read
    pub fn record_success(&mut self, reason: RefreshReason, now: Instant) {
        self.loaded = Some((now, reason));
        self.failure = None;
    }

    /// Record a failed read; the previous data stays in place
    pub fn record_failure(
        &mut self,
        reason: RefreshReason,
        now: Instant,
        error: impl Into<String>,
    ) {
        self.failure = Some(RefreshFailure {
            at: now,
            reason,
            error: error.into(),
        });
    }

    /// The latest failure, kept until a read succeeds
    #[must_use]
    pub const fn failure(&self) -> Option<&RefreshFailure> {
        self.failure.as_ref()
    }

    /// State as of `now`
    #[must_use]
    pub fn state(&self, now: Instant) -> FreshnessState<'_> {
        let age = self
            .loaded
            .map(|(at, _)| now.saturating_duration_since(at));
        match (&self.failure, self.loaded) {
            (Some(failure), _) => FreshnessState::Stale { age, failure },
            (None, Some((at, reason))) => FreshnessState::Current {
                age: now.saturating_duration_since(at),
                reason,
            },
            (None, None) => FreshnessState::Unknown,
        }
    }
}

/// Compact age: `now` under 5 seconds, then `12s`, `3m`, `2h`, `4d`
#[must_use]
pub fn format_age(age: Duration) -> String {
    let secs = age.as_secs();
    match secs {
        0..5 => "now".to_string(),
        5..60 => format!("{secs}s"),
        60..3600 => format!("{}m", secs / 60),
        3600..86_400 => format!("{}h", secs / 3600),
        _ => format!("{}d", secs / 86_400),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nothing_loaded_is_unknown() {
        assert_eq!(
            Freshness::default().state(Instant::now()),
            FreshnessState::Unknown
        );
    }

    #[test]
    fn success_is_current_with_its_age_and_reason() {
        let start = Instant::now();
        let mut freshness = Freshness::default();
        freshness.record_success(RefreshReason::FileChange, start);

        assert_eq!(
            freshness.state(start + Duration::from_secs(12)),
            FreshnessState::Current {
                age: Duration::from_secs(12),
                reason: RefreshReason::FileChange,
            }
        );
    }

    #[test]
    fn failure_is_stale_and_keeps_the_old_age() {
        let start = Instant::now();
        let mut freshness = Freshness::default();
        freshness.record_success(RefreshReason::Startup, start);
        freshness.record_failure(
            RefreshReason::FileChange,
            start + Duration::from_mins(1),
            "git error: bad index",
        );

        let FreshnessState::Stale { age, failure } =
            freshness.state(start + Duration::from_mins(3))
        else {
            panic!("expected stale");
        };
        assert_eq!(age, Some(Duration::from_mins(3)));
        assert_eq!(failure.error, "git error: bad index");
        assert_eq!(
            failure.reason,
            RefreshReason::FileChange
        );
    }

    #[test]
    fn success_clears_the_failure() {
        let start = Instant::now();
        let mut freshness = Freshness::default();
        freshness.record_failure(RefreshReason::Manual, start, "boom");
        freshness.record_success(RefreshReason::Manual, start);

        assert!(freshness.failure().is_none());
        assert!(matches!(
            freshness.state(start),
            FreshnessState::Current { .. }
        ));
    }

    #[test]
    fn failure_before_any_data_is_stale_without_an_age() {
        let start = Instant::now();
        let mut freshness = Freshness::default();
        freshness.record_failure(RefreshReason::Startup, start, "boom");

        assert!(matches!(
            freshness.state(start),
            FreshnessState::Stale { age: None, .. }
        ));
    }

    #[test]
    fn ages_read_compactly() {
        assert_eq!(
            format_age(Duration::from_secs(2)),
            "now"
        );
        assert_eq!(
            format_age(Duration::from_secs(12)),
            "12s"
        );
        assert_eq!(
            format_age(Duration::from_secs(185)),
            "3m"
        );
        assert_eq!(
            format_age(Duration::from_secs(7_300)),
            "2h"
        );
        assert_eq!(
            format_age(Duration::from_hours(96)),
            "4d"
        );
    }
}
