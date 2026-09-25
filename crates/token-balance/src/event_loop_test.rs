use super::*;
use crossterm::event::{
    Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
};

fn key(kind: KeyEventKind) -> Event {
    Event::Key(KeyEvent {
        code: KeyCode::Char('o'),
        modifiers: KeyModifiers::NONE,
        kind,
        state: crossterm::event::KeyEventState::NONE,
    })
}

fn mouse(kind: MouseEventKind) -> Event {
    Event::Mouse(MouseEvent {
        kind,
        column: 4,
        row: 3,
        modifiers: KeyModifiers::NONE,
    })
}

#[test]
fn press_and_click_redraw_release_and_move_do_not() {
    assert_eq!(classify(Some(&key(KeyEventKind::Press))), LoopCmd::Redraw);
    assert_eq!(classify(Some(&key(KeyEventKind::Release))), LoopCmd::Ignore);
    assert_eq!(
        classify(Some(&mouse(MouseEventKind::Down(MouseButton::Left)))),
        LoopCmd::Redraw
    );
    assert_eq!(
        classify(Some(&mouse(MouseEventKind::Moved))),
        LoopCmd::Ignore
    );
}

#[test]
fn closed_reader_respawns_focus_is_ignored() {
    assert_eq!(classify(None), LoopCmd::Respawn);
    assert_eq!(classify(Some(&Event::FocusGained)), LoopCmd::Ignore);
    assert_eq!(classify(Some(&Event::Resize(80, 24))), LoopCmd::Redraw);
}
