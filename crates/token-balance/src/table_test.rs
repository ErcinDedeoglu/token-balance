use crate::domain::{
    Clock, CreditUnit, ExtraCredits, LedgerKind, ProviderSnapshot, ProviderStatus, QuotaWindow,
    SortMode, WindowLabel, sort_snapshots,
};
use crate::fixtures::{FixtureSet, fixture_registry, frozen_demo_clock};
use crate::overlay::Overlay;
use crate::tui::App;
use chrono::Duration;
use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::sync::Arc;

fn render_string(app: &mut App, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| app.draw(f)).expect("draw");
    let buf = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..height {
        for x in 0..width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

async fn mixed_app() -> App {
    let clock: Arc<dyn Clock> = Arc::new(frozen_demo_clock());
    let providers = fixture_registry(FixtureSet::Mixed, Arc::clone(&clock));
    let (app, mut rx) = App::new(providers, clock, None);
    app.bootstrap(&mut rx).await
}

fn last_line(buf: &str) -> &str {
    buf.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
}

fn named_line<'a>(buf: &'a str, name: &str) -> &'a str {
    buf.lines()
        .find(|l| l.contains(name) && !l.contains('┌') && !l.contains("worst"))
        .unwrap_or_else(|| panic!("missing row {name} in\n{buf}"))
}

fn card_block(buf: &str, needle: &str) -> String {
    let lines: Vec<&str> = buf.lines().collect();
    let i = lines
        .iter()
        .position(|l| l.contains('┌') && l.contains(needle))
        .unwrap_or_else(|| panic!("missing card {needle} in\n{buf}"));
    let mut j = i;
    while j + 1 < lines.len() && !lines[j].contains('└') {
        j += 1;
    }
    lines[i..=j].join("\n")
}

#[tokio::test]
async fn table_default_mixed_80x24() {
    let mut app = mixed_app().await;
    let buf = render_string(&mut app, 80, 24);
    assert!(
        !buf.contains('┌'),
        "default must not be 7-row cards:\n{buf}"
    );
    assert!(
        buf.lines().any(|l| l.contains("5h") && l.contains("wk")),
        "column header 5h/wk missing:\n{buf}"
    );
    let foot = last_line(&buf);
    assert!(
        foot.contains("t view:table"),
        "footer t view:table missing: {foot:?}\n{buf}"
    );
    let codex = named_line(&buf, "Codex");
    assert!(codex.contains("18%"), "codex 5h:\n{codex}");
    assert!(codex.contains("63%"), "codex wk:\n{codex}");
    assert!(codex.contains("1h 12m"), "codex reset:\n{codex}");
    let grok = named_line(&buf, "Grok");
    assert!(grok.contains("27%"), "grok wk:\n{grok}");
    assert!(grok.contains("3d 4h"), "grok reset:\n{grok}");
    assert!(grok.contains("400 cr"), "grok extra:\n{grok}");
    let claude = named_line(&buf, "Claude");
    assert!(claude.contains("72%"), "claude 5h:\n{claude}");
    assert!(claude.contains("41%"), "claude wk:\n{claude}");
    assert!(claude.contains("$12.40"), "claude extra:\n{claude}");
    let kimi = named_line(&buf, "Kimi");
    assert!(kimi.contains("55%"), "kimi 5h:\n{kimi}");
    assert!(kimi.contains("88%"), "kimi wk:\n{kimi}");
}

#[tokio::test]
async fn table_toggle_restores_seven_row_cards() {
    let mut app = mixed_app().await;
    let table = render_string(&mut app, 80, 24);
    assert!(!table.contains('┌'), "pre-toggle table:\n{table}");
    app.handle_key(KeyCode::Char('t'));
    let cards = render_string(&mut app, 80, 24);
    for name in ["Codex", "Grok", "Claude", "Kimi", "z.ai", "Muse"] {
        let card = card_block(&cards, name);
        assert_eq!(card.lines().count(), 7, "{name} card:\n{card}");
    }
    let two_col = cards
        .lines()
        .any(|l| l.contains('┌') && l.contains("Codex") && l.contains("Grok"));
    assert!(two_col, "2-col Codex|Grok missing:\n{cards}");
    let foot = last_line(&cards);
    assert!(
        foot.contains("t view:cards"),
        "footer t view:cards missing: {foot:?}"
    );
    app.handle_key(KeyCode::Char('t'));
    let back = render_string(&mut app, 80, 24);
    assert!(!back.contains('┌'), "second t must return table:\n{back}");
    assert!(
        last_line(&back).contains("t view:table"),
        "table footer after second t:\n{back}"
    );
    let codex = named_line(&back, "Codex");
    assert!(codex.contains("18%") && codex.contains("63%"), "{codex}");
}

#[tokio::test]
async fn table_prepaid_section_below_plan() {
    let mut app = mixed_app().await;
    let now = app.snapshots[0].fetched_at;
    app.snapshots.push(ProviderSnapshot {
        id: "deepseek".into(),
        display_name: "deepseek".into(),
        glyph: "D".into(),
        ledger: LedgerKind::PrepaidWallet,
        fetched_at: now,
        status: ProviderStatus::Available {
            plan: None,
            windows: Vec::new(),
            extra: Some(ExtraCredits {
                label: "prepaid".into(),
                remaining: 3.38,
                unit: CreditUnit::Usd,
                limit: None,
            }),
        },
        docs_url: None,
    });
    sort_snapshots(&mut app.snapshots, app.sort);
    let buf = render_string(&mut app, 80, 24);
    let lines: Vec<&str> = buf.lines().collect();
    let plan_at = lines
        .iter()
        .position(|l| l.contains("Codex") && !l.contains('┌'))
        .expect("plan row");
    let prepaid_at = lines
        .iter()
        .position(|l| l.contains("deepseek"))
        .expect("prepaid row");
    assert!(
        prepaid_at > plan_at,
        "prepaid must sit below plan:\n{buf}"
    );
    let row = lines[prepaid_at];
    assert!(row.contains("$3.38"), "teal remaining:\n{row}");
    assert!(row.contains("no reset"), "{row}");
    assert!(!row.contains('%'), "no remaining-% gauge:\n{row}");
    assert!(
        !row.chars().any(|c| "▏▎▍▌▋▊▉█░".contains(c)),
        "no bar on prepaid:\n{row}"
    );
}

#[tokio::test]
async fn table_shows_session_and_weekly_percent() {
    let mut app = mixed_app().await;
    let now = app.snapshots[0].fetched_at;
    let kimi = app
        .snapshots
        .iter_mut()
        .find(|s| s.id == "kimi")
        .expect("kimi");
    kimi.status = ProviderStatus::Available {
        plan: Some("Moderato".into()),
        windows: vec![
            QuotaWindow::from_remaining_percent(
                WindowLabel::FiveHour,
                100.0,
                Some(now + Duration::hours(4)),
                Some(300),
            ),
            QuotaWindow::from_remaining_percent(
                WindowLabel::Weekly,
                80.0,
                Some(now + Duration::days(5)),
                Some(10080),
            ),
        ],
        extra: None,
    };
    sort_snapshots(&mut app.snapshots, SortMode::Risk);
    let buf = render_string(&mut app, 80, 24);
    let row = named_line(&buf, "Kimi");
    assert!(row.contains("100%"), "session remaining:\n{row}\n{buf}");
    assert!(row.contains("80%"), "weekly remaining:\n{row}\n{buf}");
}

#[tokio::test]
async fn table_empty_registry_hint() {
    let clock: Arc<dyn Clock> = Arc::new(frozen_demo_clock());
    let (mut app, _rx) = App::new(Vec::new(), clock, None);
    let buf = render_string(&mut app, 80, 24);
    assert!(
        buf.contains("accounts.toml"),
        "empty hint:\n{buf}"
    );
    let header = buf.lines().any(|l| l.contains("5h") && l.contains("wk"));
    assert!(!header, "no 5h/wk table header when empty:\n{buf}");
}

#[tokio::test]
async fn table_unsigned_never_zero_percent() {
    let clock: Arc<dyn Clock> = Arc::new(frozen_demo_clock());
    let providers = fixture_registry(FixtureSet::Unsigned, Arc::clone(&clock));
    let (app, mut rx) = App::new(providers, clock, None);
    let mut app = app.bootstrap(&mut rx).await;
    let buf = render_string(&mut app, 80, 24);
    let zai = named_line(&buf, "z.ai");
    let muse = named_line(&buf, "Muse");
    assert!(!zai.contains("0%"), "z.ai:\n{zai}\n{buf}");
    assert!(!muse.contains("0%"), "Muse:\n{muse}\n{buf}");
}

#[tokio::test]
async fn help_lists_table_toggle() {
    let mut app = mixed_app().await;
    app.handle_key(KeyCode::Char('?'));
    assert_eq!(app.overlay, Overlay::Help);
    let buf = render_string(&mut app, 80, 24);
    assert!(
        buf.contains('t') && buf.to_lowercase().contains("table"),
        "help t table|cards:\n{buf}"
    );
}

#[tokio::test]
async fn table_multi_pager_one_footer_row() {
    let clock: Arc<dyn Clock> = Arc::new(frozen_demo_clock());
    let providers = fixture_registry(FixtureSet::Multi, Arc::clone(&clock));
    let (app, mut rx) = App::new(providers, clock, None);
    let mut app = app.bootstrap(&mut rx).await;
    let buf = render_string(&mut app, 80, 8);
    let foot = last_line(&buf);
    assert!(
        foot.contains('/') && foot.contains("t view:table"),
        "pager + keys one row: {foot:?}\n{buf}"
    );
    let footer_lines = buf
        .lines()
        .filter(|l| l.contains("r refresh"))
        .count();
    assert_eq!(footer_lines, 1, "footer wrapped:\n{buf}");
}
