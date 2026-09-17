use crate::domain::ProviderStatus;
use crate::tui::App;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind};
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
    let (tx, mut rx) = unbounded_channel();
    std::thread::spawn(move || {
        loop {
            match crossterm::event::poll(Duration::from_millis(200)) {
                Ok(true) => {
                    if let Ok(ev) = crossterm::event::read() {
                        if tx.send(ev).is_err() {
                            break;
                        }
                    }
                }
                Ok(false) => {}
                Err(_) => break,
            }
        }
    });
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    loop {
        if app.should_quit {
            break;
        }
        terminal.draw(|f| app.draw(f))?;
        tokio::select! {
            ev = rx.recv() => {
                match ev {
                    Some(Event::Key(key)) if key.kind != KeyEventKind::Release => {
                        app.handle_key(key.code);
                    }
                    Some(Event::Mouse(m)) => app.handle_mouse(m),
                    _ => {}
                }
            }
            _ = tick.tick() => {
                app.on_tick();
            }
            msg = fetch_rx.recv(), if app.fetching => {
                if let Some((id, status)) = msg {
                    app.on_fetch_result(id, status);
                }
            }
        }
    }
    Ok(())
}
