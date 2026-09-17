use crate::domain::{
    CreditUnit, ExtraCredits, ProviderSnapshot, ProviderStatus, QuotaWindow, SortMode, WindowLabel,
    effective_available,
};
use chrono::{DateTime, Utc};

fn session_duration(w: &QuotaWindow) -> u32 {
    w.duration_mins.unwrap_or(300)
}

pub fn hero_window(windows: &[QuotaWindow]) -> Option<&QuotaWindow> {
    let mut sessions: Vec<&QuotaWindow> = windows.iter().filter(|w| w.is_session()).collect();
    if !sessions.is_empty() {
        sessions.sort_by_key(|w| session_duration(w));
        return Some(sessions[0]);
    }
    windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Weekly))
        .or_else(|| {
            windows
                .iter()
                .find(|w| matches!(w.label, WindowLabel::Other(_)))
        })
        .or_else(|| windows.first())
}

pub fn secondary_window(windows: &[QuotaWindow]) -> Option<&QuotaWindow> {
    let hero = hero_window(windows)?;
    if let Some(wk) = windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Weekly))
    {
        if !std::ptr::eq(wk, hero) {
            return Some(wk);
        }
    }
    let mut sessions: Vec<&QuotaWindow> = windows
        .iter()
        .filter(|w| w.is_session() && !std::ptr::eq(*w, hero))
        .collect();
    sessions.sort_by_key(|w| session_duration(w));
    sessions.first().copied()
}

pub fn min_remaining(windows: &[QuotaWindow]) -> Option<f32> {
    windows.iter().map(|w| w.remaining_percent).reduce(f32::min)
}

fn sort_group(status: &ProviderStatus) -> u8 {
    if effective_available(status).is_some() {
        0
    } else if matches!(status, ProviderStatus::Error { .. }) {
        1
    } else if matches!(status, ProviderStatus::NotConfigured { .. }) {
        2
    } else {
        3
    }
}

fn soonest_reset(windows: &[QuotaWindow]) -> Option<DateTime<Utc>> {
    windows.iter().filter_map(|w| w.resets_at).min()
}

pub fn sort_snapshots(snaps: &mut [ProviderSnapshot], mode: SortMode) {
    snaps.sort_by(|a, b| {
        let ga = sort_group(&a.status);
        let gb = sort_group(&b.status);
        if mode == SortMode::Name {
            let sink_a = ga >= 2;
            let sink_b = gb >= 2;
            return sink_a
                .cmp(&sink_b)
                .then_with(|| ga.cmp(&gb).then(a.display_name.cmp(&b.display_name)));
        }
        ga.cmp(&gb).then_with(|| {
            if ga != 0 {
                return a.id.cmp(&b.id);
            }
            let wa = effective_available(&a.status)
                .map(|r| r.windows)
                .unwrap_or(&[]);
            let wb = effective_available(&b.status)
                .map(|r| r.windows)
                .unwrap_or(&[]);
            let ra = min_remaining(wa).unwrap_or(100.0);
            let rb = min_remaining(wb).unwrap_or(100.0);
            ra.partial_cmp(&rb)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| match (soonest_reset(wa), soonest_reset(wb)) {
                    (Some(x), Some(y)) => x.cmp(&y),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                })
                .then_with(|| a.id.cmp(&b.id))
        })
    });
}

pub fn display_pct(remaining_percent: f32) -> u8 {
    remaining_percent.round().clamp(0.0, 100.0) as u8
}

pub fn caption_duration(w: &QuotaWindow) -> String {
    if matches!(w.label, WindowLabel::Weekly) {
        return "wk".into();
    }
    match w.duration_mins {
        Some(15) => "15m".into(),
        Some(60) => "1h".into(),
        Some(300) => "5h".into(),
        Some(m) if m % 60 == 0 => format!("{}h", m / 60),
        Some(m) => format!("{m}m"),
        None if matches!(w.label, WindowLabel::FiveHour) => "5h".into(),
        None => match &w.label {
            WindowLabel::Other(s) => s.clone(),
            _ => "5h".into(),
        },
    }
}

pub fn format_countdown(now: DateTime<Utc>, resets_at: Option<DateTime<Utc>>) -> String {
    let Some(at) = resets_at else {
        return String::new();
    };
    let delta = at.signed_duration_since(now);
    if delta.num_seconds() < 0 {
        return "reset due".into();
    }
    let days = delta.num_days();
    let hours = delta.num_hours() % 24;
    let mins = delta.num_minutes() % 60;
    if days > 0 {
        format!("{days}d {hours}h")
    } else if hours > 0 {
        format!("{hours}h {mins:02}m")
    } else {
        format!("{mins}m")
    }
}

pub fn format_age(now: DateTime<Utc>, fetched_at: DateTime<Utc>) -> String {
    let secs = now.signed_duration_since(fetched_at).num_seconds().max(0);
    if secs < 1 {
        "just now".into()
    } else if secs < 60 {
        format!("{secs}s ago")
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}

pub fn extra_line(extra: &ExtraCredits) -> String {
    let used = extra.limit.map(|l| (l - extra.remaining).max(0.0));
    match (extra.unit, extra.limit, used) {
        (CreditUnit::Credits, Some(limit), Some(used)) => {
            format!(
                "{} left  {}/{} used",
                extra.remaining as i64, used as i64, limit as i64
            )
        }
        (CreditUnit::Usd, Some(limit), Some(used)) => {
            format!(
                "${:.2} left  ${:.2}/${:.2} used",
                extra.remaining, used, limit
            )
        }
        (CreditUnit::Unknown, Some(limit), Some(used)) => {
            format!("{} left  {}/{}", extra.remaining, used, limit)
        }
        (CreditUnit::Usd, _, _) => format!("extra  ${:.2}", extra.remaining),
        (CreditUnit::Credits, _, _) => format!("extra  {} cr", extra.remaining as i64),
        (CreditUnit::Unknown, _, _) => format!("extra  {}", extra.remaining),
    }
}
