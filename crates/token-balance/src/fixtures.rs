use crate::domain::{Clock, FrozenClock, LedgerKind, ProviderSnapshot, ProviderStatus};
use crate::fixture_board::{danger_specs, mixed_specs, multi_specs, unsigned_specs};
use crate::providers::{FetchFuture, Provider, RefreshPolicy};
use chrono::Utc;
use clap::ValueEnum;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum FixtureSet {
    Mixed,
    Unsigned,
    Danger,
    Error,
    Multi,
}

#[cfg(test)]
pub fn frozen_demo_clock() -> FrozenClock {
    use chrono::TimeZone;
    FrozenClock(Utc.with_ymd_and_hms(2026, 9, 15, 14, 32, 0).unwrap())
}

#[derive(Clone)]
pub(crate) enum Spec {
    Available {
        plan: Option<String>,
        windows: Vec<crate::domain::WindowSpec>,
        extra: Option<crate::domain::ExtraCredits>,
    },
    NotConfigured {
        hint: String,
    },
    Unsupported {
        reason: String,
    },
    Fail {
        message: String,
    },
}

#[derive(Clone)]
pub struct FixtureProvider {
    pub(crate) id: String,
    pub(crate) display_name: String,
    pub(crate) vendor: &'static str,
    pub(crate) docs: Option<&'static str>,
    pub(crate) clock: Arc<dyn Clock>,
    pub(crate) spec: Spec,
}

pub(crate) fn fp(
    id: impl Into<String>,
    display_name: impl Into<String>,
    vendor: &'static str,
    docs: Option<&'static str>,
    clock: &Arc<dyn Clock>,
    spec: Spec,
) -> FixtureProvider {
    FixtureProvider {
        id: id.into(),
        display_name: display_name.into(),
        vendor,
        docs,
        clock: Arc::clone(clock),
        spec,
    }
}

impl FixtureProvider {
    pub(crate) fn materialize(&self, now: chrono::DateTime<Utc>) -> ProviderStatus {
        match &self.spec {
            Spec::Available {
                plan,
                windows,
                extra,
            } => ProviderStatus::Available {
                plan: plan.clone(),
                windows: windows.iter().map(|w| w.materialize(now)).collect(),
                extra: extra.clone(),
            },
            Spec::NotConfigured { hint } => ProviderStatus::NotConfigured { hint: hint.clone() },
            Spec::Unsupported { reason } => ProviderStatus::Unsupported {
                reason: reason.clone(),
            },
            Spec::Fail { message } => ProviderStatus::Error {
                message: message.clone(),
                stale: None,
            },
        }
    }
}

impl Provider for FixtureProvider {
    fn id(&self) -> &str {
        &self.id
    }
    fn display_name(&self) -> &str {
        &self.display_name
    }
    fn vendor(&self) -> &str {
        self.vendor
    }
    fn docs_url(&self) -> Option<&'static str> {
        self.docs
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        let clock = Arc::clone(&self.clock);
        let this = self.clone();
        Box::pin(async move {
            if !cfg!(test) && !clock.is_frozen() {
                tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            }
            this.materialize(clock.now())
        })
    }
}

pub fn fixture_registry(set: FixtureSet, clock: Arc<dyn Clock>) -> Vec<Arc<dyn Provider>> {
    let list = match set {
        FixtureSet::Mixed => mixed_specs(&clock, false),
        FixtureSet::Unsigned => unsigned_specs(&clock),
        FixtureSet::Danger => danger_specs(&clock),
        FixtureSet::Error => mixed_specs(&clock, true),
        FixtureSet::Multi => multi_specs(&clock),
    };
    list.into_iter()
        .map(|p| Arc::new(p) as Arc<dyn Provider>)
        .collect()
}

pub fn mixed_available_snapshots(clock: &dyn Clock) -> Vec<ProviderSnapshot> {
    let now = clock.now();
    let arc: Arc<dyn Clock> = Arc::new(FrozenClock(now));
    mixed_specs(&arc, false)
        .into_iter()
        .map(|p| ProviderSnapshot {
            id: p.id().into(),
            display_name: p.display_name().into(),
            glyph: p.glyph_ascii().into(),
            ledger: LedgerKind::PlanRemaining,
            fetched_at: now,
            status: p.materialize(now),
            docs_url: p.docs_url().map(str::to_string),
        })
        .collect()
}
