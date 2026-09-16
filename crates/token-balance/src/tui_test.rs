use super::*;
use crate::domain::{Clock, ProviderStatus};
use crate::fixtures::{FixtureSet, fixture_registry, frozen_demo_clock};
use crate::overlay::Overlay;
use crossterm::event::KeyCode;
use std::sync::Arc;

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
