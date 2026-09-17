use super::*;
use chrono::TimeZone;

fn frozen() -> FrozenClock {
    FrozenClock(Utc.with_ymd_and_hms(2026, 9, 15, 14, 32, 0).unwrap())
}

fn window(label: WindowLabel, remaining: f32, mins: Option<u32>) -> QuotaWindow {
    QuotaWindow::from_remaining_percent(label, remaining, None, mins)
}

fn available(id: &str, name: &str, windows: Vec<QuotaWindow>) -> ProviderSnapshot {
    let now = frozen().now();
    ProviderSnapshot {
        id: id.into(),
        display_name: name.into(),
        glyph: id.chars().next().unwrap_or('?').to_string(),
        ledger: LedgerKind::PlanRemaining,
        fetched_at: now,
        status: ProviderStatus::Available {
            plan: None,
            windows,
            extra: None,
        },
        docs_url: None,
    }
}

#[test]
fn used_25_yields_remaining_75() {
    let w = QuotaWindow::from_used_percent(WindowLabel::FiveHour, 25.0, None, Some(300));
    assert_eq!(w.used_percent, 25.0);
    assert_eq!(w.remaining_percent, 75.0);
}

#[test]
fn used_zero_and_hundred_and_clamp() {
    let z = QuotaWindow::from_used_percent(WindowLabel::Weekly, 0.0, None, None);
    assert_eq!(z.remaining_percent, 100.0);
    let f = QuotaWindow::from_used_percent(WindowLabel::Weekly, 100.0, None, None);
    assert_eq!(f.remaining_percent, 0.0);
    let c = QuotaWindow::from_used_percent(WindowLabel::Weekly, 140.0, None, None);
    assert_eq!(c.used_percent, 100.0);
    assert_eq!(c.remaining_percent, 0.0);
}

#[test]
fn from_remaining_percent_40() {
    let w = QuotaWindow::from_remaining_percent(WindowLabel::Weekly, 40.0, None, None);
    assert_eq!(w.used_percent, 60.0);
    assert_eq!(w.remaining_percent, 40.0);
}

#[test]
fn apply_fetch_error_keeps_last_available_windows() {
    let prev = available(
        "codex",
        "Codex",
        vec![window(WindowLabel::FiveHour, 18.0, Some(300))],
    );
    let status = apply_fetch(
        Some(&prev),
        ProviderStatus::Error {
            message: "401".into(),
            stale: None,
        },
    );
    let av = effective_available(&status).expect("stale available");
    assert_eq!(av.windows[0].remaining_percent, 18.0);
    match status {
        ProviderStatus::Error { stale: Some(s), .. } => {
            assert!(matches!(s.status, ProviderStatus::Available { .. }));
        }
        other => panic!("expected Error with stale, got {other:?}"),
    }
}

#[test]
fn apply_fetch_second_error_does_not_nest() {
    let prev = available(
        "codex",
        "Codex",
        vec![window(WindowLabel::FiveHour, 18.0, Some(300))],
    );
    let first = apply_fetch(
        Some(&prev),
        ProviderStatus::Error {
            message: "401".into(),
            stale: None,
        },
    );
    let mid = ProviderSnapshot {
        status: first,
        ..prev.clone()
    };
    let second = apply_fetch(
        Some(&mid),
        ProviderStatus::Error {
            message: "timeout".into(),
            stale: None,
        },
    );
    match second {
        ProviderStatus::Error { stale: Some(s), .. } => {
            assert!(matches!(s.status, ProviderStatus::Available { .. }));
            assert!(!matches!(s.status, ProviderStatus::Error { .. }));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn not_configured_clears_bars() {
    let prev = available(
        "kimi",
        "Kimi",
        vec![window(WindowLabel::FiveHour, 55.0, Some(300))],
    );
    let status = apply_fetch(
        Some(&prev),
        ProviderStatus::NotConfigured {
            hint: "export KIMI_API_KEY".into(),
        },
    );
    assert!(effective_available(&status).is_none());
}

#[test]
fn risk_sort_codex_18_above_kimi_55() {
    let codex = available(
        "codex",
        "Codex",
        vec![window(WindowLabel::FiveHour, 18.0, Some(300))],
    );
    let kimi = available(
        "kimi",
        "Kimi",
        vec![window(WindowLabel::FiveHour, 55.0, Some(300))],
    );
    let mut snaps = vec![kimi, codex];
    sort_snapshots(&mut snaps, SortMode::Risk);
    assert_eq!(snaps[0].id, "codex");
    assert_eq!(snaps[1].id, "kimi");
}

#[test]
fn stale_danger_sorts_with_available() {
    let mut kimi = available(
        "kimi",
        "Kimi",
        vec![window(WindowLabel::FiveHour, 55.0, Some(300))],
    );
    let codex_av = available(
        "codex",
        "Codex",
        vec![window(WindowLabel::FiveHour, 18.0, Some(300))],
    );
    let stale = apply_fetch(
        Some(&codex_av),
        ProviderStatus::Error {
            message: "401".into(),
            stale: None,
        },
    );
    kimi.id = "kimi".into();
    let mut codex = codex_av;
    codex.status = stale;
    let mut snaps = vec![kimi, codex];
    sort_snapshots(&mut snaps, SortMode::Risk);
    assert_eq!(snaps[0].id, "codex");
}

#[test]
fn hero_15m_secondary_weekly_skips_60m() {
    let w15 = window(WindowLabel::Other("15m".into()), 80.0, Some(15));
    let w60 = window(WindowLabel::Other("60m".into()), 50.0, Some(60));
    let wk = window(WindowLabel::Weekly, 40.0, Some(10080));
    let windows = vec![w15, w60, wk];
    let hero = hero_window(&windows).unwrap();
    let sec = secondary_window(&windows).unwrap();
    assert_eq!(hero.duration_mins, Some(15));
    assert!(matches!(sec.label, WindowLabel::Weekly));
}

#[test]
fn frozen_clock_offsets_materialize_from_now() {
    let clock = frozen();
    let spec = WindowSpec {
        label: WindowLabel::FiveHour,
        used_percent: 82.0,
        resets_in: Duration::minutes(72),
        duration_mins: Some(300),
    };
    let w = spec.materialize(clock.now());
    assert_eq!(w.resets_at.unwrap(), clock.now() + Duration::minutes(72));
    assert_eq!(w.remaining_percent, 18.0);
}

#[test]
fn display_pct_rounds_for_color() {
    assert_eq!(display_pct(19.4), 19);
    assert_eq!(display_pct(19.5), 20);
}

#[test]
fn extra_line_shows_remaining_and_used_when_limit_present() {
    let extra = ExtraCredits {
        label: "extra usage".into(),
        remaining: 536.34,
        unit: CreditUnit::Usd,
        limit: Some(1500.0),
    };
    assert_eq!(extra_line(&extra), "$536.34 left  $963.66/$1500.00 used");
}
