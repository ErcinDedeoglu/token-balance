use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

pub const SESSION_MAX_MINS: u32 = 360;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LedgerKind {
    PlanRemaining,
    PrepaidWallet,
    LocalConsumed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum WindowLabel {
    FiveHour,
    Weekly,
    Other(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaWindow {
    pub label: WindowLabel,
    pub used_percent: f32,
    pub remaining_percent: f32,
    pub resets_at: Option<DateTime<Utc>>,
    pub duration_mins: Option<u32>,
    pub limit: Option<f64>,
    pub remaining: Option<f64>,
}

impl QuotaWindow {
    pub fn from_used_percent(
        label: WindowLabel,
        used_percent: f32,
        resets_at: Option<DateTime<Utc>>,
        duration_mins: Option<u32>,
    ) -> Self {
        let used = used_percent.clamp(0.0, 100.0);
        Self {
            label,
            used_percent: used,
            remaining_percent: (100.0 - used).clamp(0.0, 100.0),
            resets_at,
            duration_mins,
            limit: None,
            remaining: None,
        }
    }

    pub fn from_remaining_percent(
        label: WindowLabel,
        remaining_percent: f32,
        resets_at: Option<DateTime<Utc>>,
        duration_mins: Option<u32>,
    ) -> Self {
        let remaining = remaining_percent.clamp(0.0, 100.0);
        Self::from_used_percent(label, 100.0 - remaining, resets_at, duration_mins)
    }

    pub fn is_session(&self) -> bool {
        match self.duration_mins {
            Some(m) => m <= SESSION_MAX_MINS,
            None => matches!(self.label, WindowLabel::FiveHour),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortMode {
    Risk,
    Name,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CreditUnit {
    Usd,
    Credits,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtraCredits {
    pub label: String,
    pub remaining: f64,
    pub unit: CreditUnit,
    pub limit: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProviderStatus {
    Available {
        plan: Option<String>,
        windows: Vec<QuotaWindow>,
        extra: Option<ExtraCredits>,
    },
    NotConfigured {
        hint: String,
    },
    Unsupported {
        reason: String,
    },
    Error {
        message: String,
        stale: Option<Box<ProviderSnapshot>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderSnapshot {
    pub id: String,
    pub display_name: String,
    pub glyph: String,
    pub ledger: LedgerKind,
    pub fetched_at: DateTime<Utc>,
    pub status: ProviderStatus,
    pub docs_url: Option<String>,
}

pub trait Clock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
    fn is_frozen(&self) -> bool {
        false
    }
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

pub struct FrozenClock(pub DateTime<Utc>);

impl Clock for FrozenClock {
    fn now(&self) -> DateTime<Utc> {
        self.0
    }
    fn is_frozen(&self) -> bool {
        true
    }
}

#[derive(Clone)]
pub struct WindowSpec {
    pub label: WindowLabel,
    pub used_percent: f32,
    pub resets_in: Duration,
    pub duration_mins: Option<u32>,
}

impl WindowSpec {
    pub fn materialize(&self, now: DateTime<Utc>) -> QuotaWindow {
        QuotaWindow::from_used_percent(
            self.label.clone(),
            self.used_percent,
            Some(now + self.resets_in),
            self.duration_mins,
        )
    }
}

pub fn last_available(snap: &ProviderSnapshot) -> Option<ProviderSnapshot> {
    match &snap.status {
        ProviderStatus::Available { .. } => Some(snap.clone()),
        ProviderStatus::Error {
            stale: Some(inner), ..
        } => last_available(inner),
        _ => None,
    }
}

pub fn apply_fetch(prev: Option<&ProviderSnapshot>, new_status: ProviderStatus) -> ProviderStatus {
    match new_status {
        ProviderStatus::Error { message, stale } => {
            let available = stale
                .as_deref()
                .and_then(last_available)
                .or_else(|| prev.and_then(last_available));
            ProviderStatus::Error {
                message,
                stale: available.map(Box::new),
            }
        }
        other => other,
    }
}

#[derive(Clone, Copy)]
pub struct AvailableRef<'a> {
    pub plan: &'a Option<String>,
    pub windows: &'a [QuotaWindow],
    pub extra: &'a Option<ExtraCredits>,
}

pub fn effective_available(status: &ProviderStatus) -> Option<AvailableRef<'_>> {
    match status {
        ProviderStatus::Available {
            plan,
            windows,
            extra,
        } => Some(AvailableRef {
            plan,
            windows,
            extra,
        }),
        ProviderStatus::Error {
            stale: Some(inner), ..
        } => match &inner.status {
            ProviderStatus::Available {
                plan,
                windows,
                extra,
            } => Some(AvailableRef {
                plan,
                windows,
                extra,
            }),
            _ => None,
        },
        _ => None,
    }
}

pub use crate::windows::{
    caption_duration, display_pct, extra_line, format_age, format_countdown, hero_window,
    min_remaining, secondary_window, session_window, soonest_reset, sort_snapshots, weekly_window,
};

#[cfg(test)]
#[path = "domain_test.rs"]
mod tests;
