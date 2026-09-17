use super::*;
use crate::adapters::live_registry;
use crate::credentials::Credentials;
use crate::domain::{Clock, ProviderStatus};
use crate::fixtures::{FixtureSet, fixture_registry, frozen_demo_clock};
use crate::providers::RefreshTrigger;
use crate::overlay::Overlay;
use crossterm::event::{KeyCode, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::collections::BTreeMap;
use std::path::PathBuf;
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

fn assert_seven(card: &str, name: &str) {
    assert_eq!(card.lines().count(), 7, "{name} card:\n{card}");
}

fn last_line(buf: &str) -> &str {
    buf.lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or("")
}

#[tokio::test]
async fn mixed_snapshots_four_sizes() {
    let mut app = mixed_app().await;
    let sizes = [(80u16, 48u16), (80, 24), (120, 24), (140, 24)];
    for (w, h) in sizes {
        let buf = render_string(&mut app, w, h);
        assert!(
            buf.contains("72%"),
            "Claude hero 72% missing at ({w},{h}):\n{buf}"
        );
        let names = vec!["Codex", "Grok", "Claude", "Kimi", "z.ai", "Muse"];
        for name in names {
            assert_seven(&card_block(&buf, name), name);
        }
        let foot = last_line(&buf);
        assert!(
            !foot.contains("1–3 / 6"),
            "pager must be omitted at ({w},{h}): {foot:?}"
        );
        if h >= 48 {
            let foot_at = buf
                .lines()
                .position(|l| l.contains("r refresh"))
                .expect("footer");
            assert!(
                foot_at < 26,
                "keys sit under the grid, not the window bottom ({foot_at})"
            );
        }
        let lines: Vec<&str> = buf.lines().collect();
        if let Some(age_i) = lines[0].rfind("ago") {
            let worst_i = lines[0].find("worst").unwrap_or(0);
            assert!(
                age_i > worst_i + 16,
                "refresh age should sit on the right of the header:\n{}",
                lines[0]
            );
        }
        if w >= 80 && w < 140 {
            let two_col = buf
                .lines()
                .any(|l| l.contains('┌') && l.contains("Codex") && l.contains("Grok"));
            assert!(two_col, "2-col row Codex|Grok missing at ({w},{h}):\n{buf}");
        }
        let codex_at = buf
            .lines()
            .position(|l| l.contains('┌') && l.contains("Codex"))
            .expect("Codex card");
        assert!(
            codex_at <= 2,
            "grid stays under the header at ({w},{h}), got line {codex_at}"
        );
        for name in ["z.ai", "Muse"] {
            let card = card_block(&buf, name);
            assert!(!card.contains("0%"), "{name} must not show 0%:\n{card}");
            assert!(card.contains('╌'), "{name} dashed bar:\n{card}");
            assert!(
                !card.contains("https://"),
                "{name} card face must not show docs URL:\n{card}"
            );
            if name == "z.ai" {
                assert!(card.contains("auth missing"), "{card}");
            } else {
                assert!(card.contains("unsupported"), "{card}");
                assert!(
                    card.contains("requests"),
                    "Muse reason must wrap onto a second line:\n{card}"
                );
            }
        }
    }
}

#[tokio::test]
async fn dump_board_snapshots() {
    let mut app = mixed_app().await;
    if let Ok(dir) = std::env::var("TOKEN_BALANCE_DUMP") {
        let dir = std::path::PathBuf::from(dir);
        let _ = std::fs::create_dir_all(&dir);
        for (w, h) in [(80u16, 24u16), (90, 24), (80, 48), (120, 24), (140, 24)] {
            let buf = render_string(&mut app, w, h);
            std::fs::write(dir.join(format!("board-{w}x{h}.txt")), &buf).unwrap();
        }
        app.selected_id = Some("muse".into());
        app.handle_key(KeyCode::Enter);
        let overlay = render_string(&mut app, 90, 24);
        std::fs::write(dir.join("board-90x24-muse-detail.txt"), overlay).unwrap();
    }
    let mut app = mixed_app().await;
    let board = render_string(&mut app, 90, 24);
    assert!(!board.contains("https://"), "docs URL leaked onto the board:\n{board}");
    app.selected_id = Some("muse".into());
    app.handle_key(KeyCode::Enter);
    let overlay = render_string(&mut app, 90, 24);
    assert!(
        overlay.contains("meta.ai") || overlay.contains("docs"),
        "detail overlay should carry docs:\n{overlay}"
    );
}

#[tokio::test]
async fn two_col_j_from_codex_selects_claude() {
    let mut app = mixed_app().await;
    let _ = render_string(&mut app, 120, 24);
    app.selected_id = Some("codex".into());
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.selected_id.as_deref(), Some("claude"));
}

#[tokio::test]
async fn one_col_h_l_noop() {
    let mut app = mixed_app().await;
    let _ = render_string(&mut app, 70, 24);
    app.selected_id = Some("codex".into());
    app.handle_key(KeyCode::Char('h'));
    assert_eq!(app.selected_id.as_deref(), Some("codex"));
    app.handle_key(KeyCode::Char('l'));
    assert_eq!(app.selected_id.as_deref(), Some("codex"));
}

fn click(column: u16, row: u16) -> MouseEvent {
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column,
        row,
        modifiers: KeyModifiers::empty(),
    }
}

#[tokio::test]
async fn click_selects_card_like_arrows() {
    let mut app = mixed_app().await;
    let _ = render_string(&mut app, 80, 24);
    let start = app.selected_id.clone();
    app.handle_mouse(click(60, 3));
    let after = app.selected_id.clone();
    assert!(after.is_some());
    assert_ne!(after, start, "click on right column should change selection");
    app.handle_mouse(click(4, 3));
    assert_eq!(app.selected_id, start);
}

#[tokio::test]
async fn click_on_overlay_closes_it() {
    let mut app = mixed_app().await;
    let _ = render_string(&mut app, 80, 24);
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.overlay, Overlay::Detail);
    app.handle_mouse(click(10, 10));
    assert_eq!(app.overlay, Overlay::None);
}

#[tokio::test]
async fn overlay_swallows_hjkl_space_does_not_close() {
    let mut app = mixed_app().await;
    let _ = render_string(&mut app, 120, 24);
    app.selected_id = Some("codex".into());
    app.handle_key(KeyCode::Enter);
    assert_eq!(app.overlay, Overlay::Detail);
    app.handle_key(KeyCode::Char('j'));
    assert_eq!(app.selected_id.as_deref(), Some("codex"));
    assert_eq!(app.overlay, Overlay::Detail);
    app.handle_key(KeyCode::Char(' '));
    assert_eq!(app.overlay, Overlay::Detail);
    app.handle_key(KeyCode::Esc);
    assert_eq!(app.overlay, Overlay::None);
}

#[tokio::test]
async fn error_fixture_first_paint_stale_codex_bars() {
    let clock: Arc<dyn Clock> = Arc::new(frozen_demo_clock());
    let providers = fixture_registry(FixtureSet::Error, Arc::clone(&clock));
    let (app, mut rx) = App::new(providers, clock, Some(FixtureSet::Error));
    let mut app = app.bootstrap(&mut rx).await;
    let buf = render_string(&mut app, 80, 48);
    let card = card_block(&buf, "Codex");
    assert!(card.contains("18%"), "stale bars:\n{card}");
    assert!(card.contains("stale") || buf.contains("stale"), "{buf}");
}

#[tokio::test]
async fn fixture_providers_are_not_live() {
    let clock: Arc<dyn Clock> = Arc::new(frozen_demo_clock());
    let providers = fixture_registry(FixtureSet::Mixed, clock);
    let kimi = providers.iter().find(|p| p.id() == "kimi").unwrap();
    match kimi.fetch().await {
        ProviderStatus::Available { .. } => {}
        other => panic!("fixture kimi must be Available, got {other:?}"),
    }
}

fn live_home() -> PathBuf {
    let n = LIVE_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let p = std::env::temp_dir().join(format!("tb-tui-{}-{n}", std::process::id()));
    std::fs::create_dir_all(p.join(".config/token-balance")).unwrap();
    p
}

static LIVE_SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

fn write_accounts(home: &std::path::Path, body: &str) {
    std::fs::write(home.join(".config/token-balance/accounts.toml"), body).unwrap();
}

async fn live_app(home: PathBuf) -> App {
    live_app_rx(home).await.0
}

async fn live_app_rx(home: PathBuf) -> (App, tokio::sync::mpsc::UnboundedReceiver<(String, ProviderStatus)>) {
    let clock: Arc<dyn Clock> = Arc::new(frozen_demo_clock());
    let creds = Credentials::isolated(home, BTreeMap::new());
    let providers = live_registry(&creds, false).unwrap();
    let (app, mut rx) = App::new(providers, clock, None);
    let app = app.with_live(creds, false).bootstrap(&mut rx).await;
    (app, rx)
}

#[tokio::test]
async fn missing_accounts_file_empty_board_names_path() {
    let mut app = live_app(live_home()).await;
    assert!(app.snapshots.is_empty());
    let buf = render_string(&mut app, 80, 24);
    assert!(
        buf.contains(".config/token-balance/accounts.toml"),
        "empty hint:\n{buf}"
    );
    assert!(!buf.contains('┌'), "no provider cards:\n{buf}");
}

#[tokio::test]
async fn two_claude_cards_share_glyph_and_labels() {
    let home = live_home();
    write_accounts(
        &home,
        r#"
[[account]]
vendor = "claude"
id = "claude-work"
label = "work"
credentials = ".missing-a.json"
[[account]]
vendor = "claude"
id = "claude-home"
label = "home"
credentials = ".missing-b.json"
"#,
    );
    let mut app = live_app(home).await;
    let buf = render_string(&mut app, 80, 24);
    let work = card_block(&buf, "work");
    let home_card = card_block(&buf, "home");
    assert_seven(&work, "work");
    assert_seven(&home_card, "home");
    assert!(work.contains('L'), "{work}");
    assert!(home_card.contains('L'), "{home_card}");
    let ids: Vec<_> = app.snapshots.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids.len(), 2);
    assert!(ids.contains(&"claude-work"));
    assert!(ids.contains(&"claude-home"));
    assert!(!ids.iter().any(|id| *id == "muse"));
}

#[tokio::test]
async fn refresh_rereads_accounts_file() {
    let home = live_home();
    write_accounts(
        &home,
        r#"
[[account]]
vendor = "claude"
id = "claude-work"
label = "work"
credentials = ".a.json"
[[account]]
vendor = "kimi"
id = "kimi-team"
label = "team"
api_key_env = "KIMI_CODE_API_KEY"
"#,
    );
    let (mut app, mut rx) = live_app_rx(home.clone()).await;
    assert_eq!(app.snapshots.len(), 2);
    write_accounts(
        &home,
        r#"
[[account]]
vendor = "claude"
id = "claude-work"
label = "work"
credentials = ".a.json"
[[account]]
vendor = "kimi"
id = "kimi-team"
label = "team"
api_key_env = "KIMI_CODE_API_KEY"
[[account]]
vendor = "zai"
id = "zai-pack"
label = "z.ai"
api_key_env = "ZAI_API_KEY"
"#,
    );
    app.handle_key(KeyCode::Char('r'));
    app.drain_fetches(&mut rx).await;
    let ids: Vec<_> = app.snapshots.iter().map(|s| s.id.as_str()).collect();
    assert_eq!(ids.len(), 3, "{ids:?}");
    assert!(ids.contains(&"claude-work"));
    assert!(ids.contains(&"kimi-team"));
    assert!(ids.contains(&"zai-pack"));
    assert!(!ids.iter().any(|id| *id == "muse"));
}

#[tokio::test]
async fn fixture_multi_80x24_pager_and_seven_row_cards() {
    let clock: Arc<dyn Clock> = Arc::new(frozen_demo_clock());
    let providers = fixture_registry(FixtureSet::Multi, Arc::clone(&clock));
    assert!(providers.len() > 6);
    let (app, mut rx) = App::new(providers, clock, Some(FixtureSet::Multi));
    let mut app = app.bootstrap(&mut rx).await;
    let buf = render_string(&mut app, 80, 24);
    let two_col = buf
        .lines()
        .any(|l| l.contains('┌') && l.matches('┌').count() >= 2);
    assert!(two_col, "2-col missing:\n{buf}");
    for name in ["work", "home", "codex", "grok"] {
        if buf.contains(name) {
            let card = card_block(&buf, name);
            assert_seven(&card, name);
        }
    }
    let foot = last_line(&buf);
    let pager = regex_pager(foot);
    assert!(pager, "pager prefix missing: {foot:?}\n{buf}");
}

fn regex_pager(foot: &str) -> bool {
    let bytes = foot.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_digit() {
                j += 1;
            }
            let rest = &foot[j..];
            if let Some(r) = rest.strip_prefix('–') {
                let mut k = 0;
                let rb = r.as_bytes();
                while k < rb.len() && rb[k].is_ascii_digit() {
                    k += 1;
                }
                if k > 0 {
                    let after = &r[k..];
                    if after.trim_start().starts_with('/') {
                        return true;
                    }
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }
    false
}

#[tokio::test]
async fn timer_retries_error_only_after_backoff() {
    let clock: Arc<dyn Clock> = Arc::new(frozen_demo_clock());
    let providers = fixture_registry(FixtureSet::Mixed, Arc::clone(&clock));
    let (app, mut rx) = App::new(providers, clock, None);
    let mut app = app.bootstrap(&mut rx).await;
    app.on_fetch_result(
        "codex".into(),
        ProviderStatus::Error {
            message: "timed out".into(),
            stale: None,
        },
    );
    app.start_refresh(RefreshTrigger::Timer);
    assert_eq!(app.in_flight, 0, "frozen clock must not retry before backoff");
    app.retry_at
        .insert("codex".into(), app.clock.now());
    app.start_refresh(RefreshTrigger::Timer);
    assert!(app.in_flight > 0, "due retry must spawn");
    app.drain_fetches(&mut rx).await;
    assert_eq!(app.in_flight, 0);
}
