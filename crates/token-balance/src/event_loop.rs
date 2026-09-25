use crate::domain::ProviderStatus;
use crate::tui::App;
use crossterm::event::{
    DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind, MouseEventKind,
};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use std::io::{self, stdout};
use std::time::Duration;
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

struct Restore;

impl Drop for Restore {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LoopCmd {
    Redraw,
    Respawn,
    Ignore,
}

fn classify(ev: Option<&Event>) -> LoopCmd {
    match ev {
        None => LoopCmd::Respawn,
        Some(Event::Key(key)) if key.kind != KeyEventKind::Release => LoopCmd::Redraw,
        Some(Event::Mouse(m)) if matches!(m.kind, MouseEventKind::Down(_)) => LoopCmd::Redraw,
        Some(Event::Resize(_, _)) => LoopCmd::Redraw,
        Some(_) => LoopCmd::Ignore,
    }
}

fn restore_tty() {
    let _ = enable_raw_mode();
    let _ = execute!(stdout(), EnableMouseCapture);
}

fn spawn_event_thread() -> UnboundedReceiver<Event> {
    let (tx, rx) = unbounded_channel();
    std::thread::spawn(move || loop {
        match crossterm::event::poll(Duration::from_millis(200)) {
            Ok(true) => match crossterm::event::read() {
                Ok(ev) => {
                    if tx.send(ev).is_err() {
                        break;
                    }
                }
                Err(_) => {
                    if tx.is_closed() {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            },
            Ok(false) => {}
            Err(_) => {
                if tx.is_closed() {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    });
    rx
}

pub async fn run_crossterm(
    mut app: App,
    mut fetch_rx: UnboundedReceiver<(String, ProviderStatus)>,
) -> io::Result<()> {
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let _restore = Restore;
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture);
        hook(info);
    }));
    let mut terminal = Terminal::new(CrosstermBackend::new(out))?;
    let mut rx = spawn_event_thread();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        if app.should_quit {
            break;
        }
        terminal.draw(|f| app.draw(f))?;
        loop {
            if app.should_quit {
                break;
            }
            tokio::select! {
                ev = rx.recv() => {
                    match classify(ev.as_ref()) {
                        LoopCmd::Redraw => {
                            match ev {
                                Some(Event::Key(key)) => app.handle_key(key.code),
                                Some(Event::Mouse(m)) => app.handle_mouse(m),
                                _ => {}
                            }
                            break;
                        }
                        LoopCmd::Respawn => {
                            restore_tty();
                            rx = spawn_event_thread();
                        }
                        LoopCmd::Ignore => {}
                    }
                }
                _ = tick.tick() => {
                    app.on_tick();
                    break;
                }
                msg = fetch_rx.recv(), if app.fetching => {
                    if let Some((id, status)) = msg {
                        app.on_fetch_result(id, status);
                    }
                    break;
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "event_loop_test.rs"]
mod tests;
