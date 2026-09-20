use crate::domain::{
    Clock, CreditUnit, ExtraCredits, LedgerKind, ProviderSnapshot, ProviderStatus, QuotaWindow,
    SortMode, WindowLabel, sort_snapshots,
};
use crate::fixtures::{FixtureSet, fixture_registry, frozen_demo_clock};
use crate::overlay::Overlay;
use crate::theme::Theme;
use crate::tui::App;
use chrono::Duration;
use crossterm::event::KeyCode;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;
use ratatui::style::Color;
use std::sync::Arc;
use unicode_width::UnicodeWidthChar;

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

fn buffer_line(buf: &Buffer, y: u16) -> String {
    (0..buf.area.width)
        .map(|x| buf[(x, y)].symbol().to_string())
        .collect()
}

fn named_row_y(buf: &Buffer, name: &str) -> u16 {
    for y in 0..buf.area.height {
        let line = buffer_line(buf, y);
        if line.contains(name) && !line.contains('┌') && !line.contains("worst") {
            return y;
        }
    }
    panic!("missing row {name}");
}

fn highlight_span(buf: &Buffer, y: u16, bg: Color) -> u16 {
    let xs: Vec<u16> = (0..buf.area.width)
        .filter(|&x| buf[(x, y)].bg == bg)
        .collect();
    match (xs.first(), xs.last()) {
        (Some(&a), Some(&b)) => b - a + 1,
        _ => 0,
    }
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

fn header_line(buf: &str) -> &str {
    buf.lines()
        .find(|l| l.contains("5h") && l.contains("wk") && l.contains("mo"))
        .unwrap_or_else(|| panic!("missing packed header in\n{buf}"))
}

fn col_start(header: &str, label: &str) -> usize {
    match label {
        "mo" => {
            let wk = header.find("wk").expect("wk before mo");
            wk + 2 + header[wk + 2..].find("mo").expect("mo column after wk")
        }
        _ => header
            .find(label)
            .unwrap_or_else(|| panic!("{label} in {header}")),
    }
}

fn byte_at_col(s: &str, col: usize) -> usize {
    let mut c = 0usize;
    for (i, ch) in s.char_indices() {
        if c >= col {
            return i;
        }
        c += UnicodeWidthChar::width(ch).unwrap_or(0);
    }
    s.len()
}

fn cell<'a>(header: &str, row: &'a str, label: &str) -> &'a str {
    let start_col = col_start(header, label);
    let w = match label {
        "5h" | "wk" | "mo" => 5,
        "reset" => 8,
        "extra" => 12,
        _ => 8,
    };
    let a = byte_at_col(row, start_col);
    let b = byte_at_col(row, start_col + w);
    row.get(a..b).unwrap_or("").trim()
}

fn other_plan(
    id: &str,
    name: &str,
    remaining: f32,
    extra: Option<ExtraCredits>,
    now: chrono::DateTime<chrono::Utc>,
) -> ProviderSnapshot {
    ProviderSnapshot {
        id: id.into(),
        display_name: name.into(),
        glyph: "L".into(),
        ledger: LedgerKind::PlanRemaining,
        fetched_at: now,
        status: ProviderStatus::Available {
            plan: Some("Max".into()),
            windows: vec![QuotaWindow::from_remaining_percent(
                WindowLabel::Other("mo".into()),
                remaining,
                None,
                None,
            )],
            extra,
        },
        docs_url: None,
    }
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
    assert!(
        codex.contains("4d 2h"),
        "codex weekly reset not 5h:\n{codex}"
    );
    assert!(
        !codex.contains("1h 12m"),
        "must not show 5h clock:\n{codex}"
    );
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
    assert!(kimi.contains("6d 1h"), "kimi weekly reset:\n{kimi}");
    assert!(!kimi.contains("3h 40m"), "must not show 5h clock:\n{kimi}");
    let at = |n: &str| {
        buf.lines()
            .position(|l| l.contains(n) && !l.contains('┌') && !l.contains("worst"))
            .unwrap_or_else(|| panic!("missing {n}\n{buf}"))
    };
    assert!(at("Grok") < at("Codex"), "sooner weekly first:\n{buf}");
    assert!(at("Codex") < at("Claude"), "reset order:\n{buf}");
    assert!(at("Claude") < at("Kimi"), "later weekly last:\n{buf}");
}

#[tokio::test]
async fn table_selected_row_highlights_full_width() {
    let mut app = mixed_app().await;
    app.selected_id = Some("grok".into());
    let backend = TestBackend::new(120, 24);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal.draw(|f| app.draw(f)).expect("draw");
    let buf = terminal.backend().buffer();
    let theme = Theme::select();
    let grok_y = named_row_y(buf, "Grok");
    let kimi_y = named_row_y(buf, "Kimi");
    let grok = buffer_line(buf, grok_y);
    let name_x = grok.find("Grok").expect("Grok") as u16;
    let extra_x = grok.find("400").expect("extra") as u16;
    assert_eq!(buf[(name_x, grok_y)].bg, theme.border, "name bg:\n{grok}");
    assert_eq!(buf[(extra_x, grok_y)].bg, theme.border, "extra bg:\n{grok}");
    let span = highlight_span(buf, grok_y, theme.border);
    assert!(span > 20, "highlight span {span}:\n{grok}");
    assert_eq!(
        highlight_span(buf, kimi_y, theme.border),
        0,
        "unselected Kimi:\n{}",
        buffer_line(buf, kimi_y)
    );
    assert!(!grok.contains('│'), "no side bars:\n{grok}");
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
        .any(|l| l.contains('┌') && l.contains("Grok") && l.contains("Codex"));
    assert!(two_col, "2-col Grok|Codex missing:\n{cards}");
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
    assert!(prepaid_at > plan_at, "prepaid must sit below plan:\n{buf}");
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
                40.0,
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
    assert!(row.contains("40%"), "session remaining:\n{row}\n{buf}");
    assert!(row.contains("80%"), "weekly remaining:\n{row}\n{buf}");
    let header = header_line(&buf);
    assert_eq!(cell(header, row, "mo"), "—", "mo dash:\n{row}\n{header}");
    let reset = cell(header, row, "reset");
    assert!(
        reset.contains("5d"),
        "reset is weekly even when 5h remaining is lower:\n{reset:?}\n{row}"
    );
    assert!(
        !reset.contains("4h"),
        "must not show 5h clock:\n{reset:?}\n{row}"
    );
}

#[tokio::test]
async fn table_pack_columns_120() {
    let mut app = mixed_app().await;
    let buf = render_string(&mut app, 120, 24);
    let header = header_line(&buf);
    assert!(
        header.trim_end().ends_with("extra"),
        "leftover after extra:\n{header:?}"
    );
    let row = named_line(&buf, "Codex");
    let name_end = row.find("Codex").expect("Codex") + 5;
    let five = row.find("18%").expect("18%");
    assert!(
        five <= name_end + 4,
        "5h must sit next to the name cell (gap {}):\n{row}",
        five - name_end
    );
    assert!(row.contains("63%"), "codex wk:\n{row}");
    assert!(row.contains("4d 2h"), "codex weekly reset:\n{row}");
    assert!(!row.contains("1h 12m"), "must not show 5h clock:\n{row}");
    assert_eq!(cell(header, row, "5h"), "18%");
    assert_eq!(cell(header, row, "wk"), "63%");
}

#[tokio::test]
async fn table_pack_columns_80() {
    let mut app = mixed_app().await;
    let buf = render_string(&mut app, 80, 24);
    let header = header_line(&buf);
    for label in ["5h", "wk", "mo", "reset", "extra"] {
        assert!(header.contains(label), "missing {label}:\n{header}");
    }
    assert!(
        header.trim_end().ends_with("extra"),
        "leftover after extra:\n{header:?}"
    );
    let row = named_line(&buf, "Codex");
    let name_end = row.find("Codex").expect("Codex") + 5;
    let five = row.find("18%").expect("18%");
    assert!(
        five <= name_end + 4,
        "80-col 5h packed (gap {}):\n{row}",
        five - name_end
    );
}

#[tokio::test]
async fn table_monthly_percent_and_extra() {
    let mut app = mixed_app().await;
    let now = app.snapshots[0].fetched_at;
    app.snapshots.push(other_plan(
        "claude-mo",
        "claude mo",
        10.0,
        Some(ExtraCredits {
            label: "extra usage".into(),
            remaining: 203.79,
            unit: CreditUnit::Usd,
            limit: None,
        }),
        now,
    ));
    sort_snapshots(&mut app.snapshots, SortMode::Risk);
    let buf = render_string(&mut app, 120, 24);
    let header = header_line(&buf);
    let row = named_line(&buf, "claude mo");
    assert_eq!(
        cell(header, row, "5h"),
        "—",
        "5h must not hold monthly:\n{row}\n{header}"
    );
    assert_eq!(cell(header, row, "wk"), "—", "{row}");
    assert_eq!(
        cell(header, row, "mo"),
        "10%",
        "monthly remaining:\n{row}\n{header}"
    );
    assert!(row.contains("$203.79"), "extra:\n{row}");
}

#[tokio::test]
async fn table_monthly_only_percent() {
    let mut app = mixed_app().await;
    let now = app.snapshots[0].fetched_at;
    app.snapshots
        .push(other_plan("kiro-mo", "kiro mo", 83.0, None, now));
    sort_snapshots(&mut app.snapshots, SortMode::Risk);
    let buf = render_string(&mut app, 120, 24);
    let header = header_line(&buf);
    let row = named_line(&buf, "kiro mo");
    assert_eq!(cell(header, row, "5h"), "—", "not in 5h:\n{row}\n{header}");
    assert_eq!(cell(header, row, "wk"), "—", "{row}");
    assert_eq!(
        cell(header, row, "mo"),
        "83%",
        "monthly only:\n{row}\n{header}"
    );
}

#[tokio::test]
async fn table_empty_registry_hint() {
    let clock: Arc<dyn Clock> = Arc::new(frozen_demo_clock());
    let (mut app, _rx) = App::new(Vec::new(), clock, None);
    let buf = render_string(&mut app, 80, 24);
    assert!(buf.contains("accounts.toml"), "empty hint:\n{buf}");
    let header = buf
        .lines()
        .any(|l| l.contains("5h") && l.contains("wk") && l.contains("mo"));
    assert!(!header, "no 5h/wk/mo table header when empty:\n{buf}");
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
    let footer_lines = buf.lines().filter(|l| l.contains("r refresh")).count();
    assert_eq!(footer_lines, 1, "footer wrapped:\n{buf}");
}
