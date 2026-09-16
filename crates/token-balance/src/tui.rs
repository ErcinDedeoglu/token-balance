use crate::board::paint;
use crate::domain::{
    Clock, ProviderSnapshot, ProviderStatus, SortMode, apply_fetch, sort_snapshots,
};
use crate::fixtures::{FixtureSet, mixed_available_snapshots};
use crate::layout::{GridMove, card_row, clamp_scroll, columns, move_index, visible_card_rows};
use crate::overlay::Overlay;
use crate::providers::{Provider, RefreshTrigger, allows_refresh};
use crate::theme::Theme;
use chrono::{DateTime, Utc};
use crossterm::event::KeyCode;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub struct App {
    pub snapshots: Vec<ProviderSnapshot>,
    pub selected_id: Option<String>,
    pub overlay: Overlay,
    pub sort: SortMode,
    pub fetching: bool,
    pub refresh_queued: bool,
    pub scroll_row: u16,
    pub should_quit: bool,
    pub last_area: Rect,
    clock: Arc<dyn Clock>,
    providers: Vec<Arc<dyn Provider>>,
    last_refresh: Option<DateTime<Utc>>,
    fetch_tx: UnboundedSender<(String, ProviderStatus)>,
    in_flight: u32,
    theme: Theme,
}

impl App {
    pub fn new(
        providers: Vec<Arc<dyn Provider>>,
        clock: Arc<dyn Clock>,
        seed: Option<FixtureSet>,
    ) -> (Self, UnboundedReceiver<(String, ProviderStatus)>) {
        let snapshots = if seed == Some(FixtureSet::Error) {
            mixed_available_snapshots(&*clock)
        } else {
            Vec::new()
        };
        let selected_id = snapshots.first().map(|s| s.id.clone());
        let (fetch_tx, fetch_rx) = unbounded_channel();
        let app = Self {
            snapshots,
            selected_id,
            overlay: Overlay::None,
            sort: SortMode::Risk,
            fetching: false,
            refresh_queued: false,
            scroll_row: 0,
            should_quit: false,
            last_area: Rect::new(0, 0, 80, 24),
            clock,
            providers,
            last_refresh: None,
            fetch_tx,
            in_flight: 0,
            theme: Theme::select(),
        };
        (app, fetch_rx)
    }

    pub async fn bootstrap(mut self, rx: &mut UnboundedReceiver<(String, ProviderStatus)>) -> Self {
        self.start_refresh(RefreshTrigger::Manual);
        self.drain_fetches(rx).await;
        self
    }

    pub fn start_refresh(&mut self, trigger: RefreshTrigger) {
        if self.fetching {
            if trigger == RefreshTrigger::Manual {
                self.refresh_queued = true;
            }
            return;
        }
        let now = self.clock.now();
        let mut spawned = 0u32;
        for p in &self.providers {
            if !allows_refresh(p.refresh_policy(), trigger) {
                continue;
            }
            let p = Arc::clone(p);
            let id = p.id().to_string();
            let timeout = p.fetch_timeout();
            let tx = self.fetch_tx.clone();
            tokio::spawn(async move {
                let work = async move {
                    match tokio::time::timeout(timeout, p.fetch()).await {
                        Ok(s) => s,
                        Err(_) => ProviderStatus::Error {
                            message: "timed out".into(),
                            stale: None,
                        },
                    }
                };
                let status = match tokio::task::spawn(work).await {
                    Ok(s) => s,
                    Err(e) if e.is_panic() => ProviderStatus::Error {
                        message: "adapter panicked".into(),
                        stale: None,
                    },
                    Err(_) => ProviderStatus::Error {
                        message: "adapter cancelled".into(),
                        stale: None,
                    },
                };
                let _ = tx.send((id, status));
            });
            spawned += 1;
        }
        if spawned == 0 {
            self.fetching = false;
            return;
        }
        self.in_flight = spawned;
        self.fetching = true;
        self.last_refresh = Some(now);
    }

    pub async fn drain_fetches(&mut self, rx: &mut UnboundedReceiver<(String, ProviderStatus)>) {
        while self.in_flight > 0 {
            match rx.recv().await {
                Some((id, status)) => self.on_fetch_result(id, status),
                None => break,
            }
        }
    }

    pub fn on_fetch_result(&mut self, id: String, status: ProviderStatus) {
        self.apply_one(&id, status);
        self.in_flight = self.in_flight.saturating_sub(1);
        if self.in_flight == 0 {
            self.fetching = false;
            if self.refresh_queued {
                self.refresh_queued = false;
                self.start_refresh(RefreshTrigger::Manual);
            }
        }
    }

    pub fn apply_one(&mut self, id: &str, status: ProviderStatus) {
        let now = self.clock.now();
        let prev = self.snapshots.iter().find(|s| s.id == id).cloned();
        let merged = apply_fetch(prev.as_ref(), status);
        if let Some(existing) = self.snapshots.iter_mut().find(|s| s.id == id) {
            existing.status = merged;
            existing.fetched_at = now;
        } else if let Some(p) = self.providers.iter().find(|p| p.id() == id) {
            self.snapshots.push(ProviderSnapshot {
                id: p.id().into(),
                display_name: p.display_name().into(),
                glyph: p.glyph_ascii().into(),
                ledger: p.ledger(),
                fetched_at: now,
                status: merged,
                docs_url: p.docs_url().map(str::to_string),
            });
        }
        sort_snapshots(&mut self.snapshots, self.sort);
        if self.selected_id.is_none() {
            self.selected_id = self.snapshots.first().map(|s| s.id.clone());
        }
    }

    pub fn handle_key(&mut self, code: KeyCode) {
        if self.overlay != Overlay::None {
            match code {
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Enter => self.overlay = Overlay::None,
                KeyCode::Char(' ') => {}
                _ => {}
            }
            return;
        }
        match code {
            KeyCode::Char('q') | KeyCode::Esc => self.should_quit = true,
            KeyCode::Char('r') => self.start_refresh(RefreshTrigger::Manual),
            KeyCode::Char('o') => {
                self.sort = match self.sort {
                    SortMode::Risk => SortMode::Name,
                    SortMode::Name => SortMode::Risk,
                };
                sort_snapshots(&mut self.snapshots, self.sort);
            }
            KeyCode::Char('?') => self.overlay = Overlay::Help,
            KeyCode::Enter | KeyCode::Char(' ') => self.overlay = Overlay::Detail,
            KeyCode::Char('h') | KeyCode::Left => self.move_sel(GridMove::Left),
            KeyCode::Char('l') | KeyCode::Right => self.move_sel(GridMove::Right),
            KeyCode::Char('j') | KeyCode::Down => self.move_sel(GridMove::Down),
            KeyCode::Char('k') | KeyCode::Up => self.move_sel(GridMove::Up),
            _ => {}
        }
    }

    fn move_sel(&mut self, mv: GridMove) {
        let n = self.snapshots.len();
        if n == 0 {
            return;
        }
        let cols = columns(self.last_area.width) as usize;
        let i = self
            .selected_id
            .as_ref()
            .and_then(|id| self.snapshots.iter().position(|s| &s.id == id))
            .unwrap_or(0);
        let j = move_index(i, n, cols, mv);
        self.selected_id = Some(self.snapshots[j].id.clone());
        self.sync_scroll();
    }

    fn sync_scroll(&mut self) {
        let cols = columns(self.last_area.width);
        let vis = visible_card_rows(self.last_area.height, self.last_area.width);
        let n = self.snapshots.len();
        let i = self
            .selected_id
            .as_ref()
            .and_then(|id| self.snapshots.iter().position(|s| &s.id == id))
            .unwrap_or(0);
        let total_rows = if cols == 0 {
            0
        } else {
            ((n as u16) + cols - 1) / cols
        };
        self.scroll_row = clamp_scroll(
            self.scroll_row,
            card_row(i, cols as usize) as u16,
            vis,
            total_rows,
        );
    }

    pub fn draw(&mut self, frame: &mut Frame<'_>) {
        self.last_area = frame.area();
        self.sync_scroll();
        let selected = self.selected().cloned();
        paint(
            frame,
            &self.snapshots,
            self.selected_id.as_deref(),
            self.sort,
            self.fetching,
            self.scroll_row,
            self.last_refresh,
            self.clock.now(),
            self.theme,
            self.overlay,
            selected.as_ref(),
        );
    }

    pub fn selected(&self) -> Option<&ProviderSnapshot> {
        let id = self.selected_id.as_ref()?;
        self.snapshots.iter().find(|s| &s.id == id)
    }
}

#[cfg(test)]
pub fn render_string(app: &mut App, width: u16, height: u16) -> String {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
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

#[cfg(test)]
#[path = "tui_test.rs"]
mod tests;
