use crate::domain::{LedgerKind, ProviderStatus};
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

pub type FetchFuture = Pin<Box<dyn Future<Output = ProviderStatus> + Send + 'static>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshPolicy {
    Default,
    OnDemand,
    Interval(Duration),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RefreshTrigger {
    Timer,
    Manual,
}

pub fn allows_refresh(policy: RefreshPolicy, trigger: RefreshTrigger) -> bool {
    match (policy, trigger) {
        (RefreshPolicy::OnDemand, RefreshTrigger::Timer) => false,
        (RefreshPolicy::OnDemand, RefreshTrigger::Manual) => true,
        (RefreshPolicy::Default, _) => true,
        (RefreshPolicy::Interval(_), RefreshTrigger::Manual) => true,
        (RefreshPolicy::Interval(_), RefreshTrigger::Timer) => true,
    }
}

pub fn refresh_period(policy: RefreshPolicy) -> Option<Duration> {
    match policy {
        RefreshPolicy::Default => Some(Duration::from_secs(60)),
        RefreshPolicy::Interval(d) => Some(d),
        RefreshPolicy::OnDemand => None,
    }
}

pub fn error_backoff(message: &str) -> Duration {
    let m = message.to_ascii_lowercase();
    if m.contains("rate limit") {
        Duration::from_secs(180)
    } else if m.contains("timed out") {
        Duration::from_secs(15)
    } else {
        Duration::from_secs(30)
    }
}

#[derive(Clone, Debug)]
pub struct AccountIdentity {
    pub id: String,
    pub label: String,
    pub vendor: &'static str,
}

impl AccountIdentity {
    #[cfg(test)]
    pub fn vendor_default(vendor: &'static str, label: &'static str) -> Self {
        Self {
            id: vendor.to_string(),
            label: label.to_string(),
            vendor,
        }
    }
}

pub trait Provider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn vendor(&self) -> &str;
    fn glyph_ascii(&self) -> &'static str {
        glyph_ascii(self.vendor())
    }
    fn ledger(&self) -> LedgerKind {
        LedgerKind::PlanRemaining
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn docs_url(&self) -> Option<&'static str> {
        None
    }
    fn fetch_timeout(&self) -> Duration {
        Duration::from_secs(10)
    }
    fn fetch(&self) -> FetchFuture;
}

pub const GLYPHS: &[(&str, &str)] = &[
    ("claude", "L"),
    ("codex", "X"),
    ("kimi", "K"),
    ("grok", "G"),
    ("zai", "Z"),
    ("kiro", "R"),
    ("deepseek", "D"),
    ("muse", "M"),
    ("muse-web", "W"),
    ("fal", "F"),
    ("copilot", "C"),
    ("exa", "E"),
];

pub fn glyph_ascii(id: &str) -> &'static str {
    GLYPHS
        .iter()
        .find(|(i, _)| *i == id)
        .map(|(_, g)| *g)
        .unwrap_or("?")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn glyph_ascii_unique_for_v1_ids() {
        let mut seen = HashSet::new();
        for (id, g) in GLYPHS {
            assert_eq!(g.chars().count(), 1, "{id} glyph must be 1 column");
            assert!(seen.insert(*g), "duplicate glyph {g}");
        }
        assert_eq!(GLYPHS.len(), 12);
    }

    #[test]
    fn on_demand_skips_timer() {
        assert!(!allows_refresh(
            RefreshPolicy::OnDemand,
            RefreshTrigger::Timer
        ));
        assert!(allows_refresh(
            RefreshPolicy::OnDemand,
            RefreshTrigger::Manual
        ));
        assert!(allows_refresh(
            RefreshPolicy::Default,
            RefreshTrigger::Timer
        ));
        assert!(allows_refresh(
            RefreshPolicy::Interval(Duration::from_secs(30)),
            RefreshTrigger::Timer
        ));
    }

    #[test]
    fn error_backoff_rate_limit_is_longer_than_timeout() {
        assert_eq!(
            error_backoff("muse-web: Rate limit exceeded"),
            Duration::from_secs(180)
        );
        assert_eq!(error_backoff("timed out"), Duration::from_secs(15));
        assert_eq!(error_backoff("HTTP 502"), Duration::from_secs(30));
        assert_eq!(
            refresh_period(RefreshPolicy::Default),
            Some(Duration::from_secs(60))
        );
        assert_eq!(
            refresh_period(RefreshPolicy::Interval(Duration::from_secs(180))),
            Some(Duration::from_secs(180))
        );
        assert_eq!(refresh_period(RefreshPolicy::OnDemand), None);
    }
}
