use crate::adapters::live_registry;
use crate::board::{Hit, hit_at, paint};
use crate::credentials::Credentials;
use crate::domain::{
    Clock, ProviderSnapshot, ProviderStatus, SortMode, apply_fetch, sort_snapshots,
};
use crate::fixtures::{FixtureSet, mixed_available_snapshots};
use crate::layout::{
    GridMove, ScanView, card_row, clamp_scroll, columns, move_index, visible_card_rows,
};
use crate::table::visible_table_rows;
use crate::overlay::Overlay;
use crate::providers::{
    Provider, RefreshTrigger, allows_refresh, error_backoff, refresh_period,
};
use crate::theme::Theme;
use std::collections::{HashMap, HashSet};
use chrono::{DateTime, Utc};
use crossterm::event::{KeyCode, MouseButton, MouseEvent, MouseEventKind};
use ratatui::Frame;
use ratatui::layout::Rect;
use std::sync::Arc;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub struct App {
    pub snapshots: Vec<ProviderSnapshot>,
    pub selected_id: Option<String>,
    pub overlay: Overlay,
    pub sort: SortMode,
    pub view: ScanView,
    pub fetching: bool,
    pub refresh_queued: bool,
    pub scroll_row: u16,
    pub should_quit: bool,
    pub last_area: Rect,
    clock: Arc<dyn Clock>,
    providers: Vec<Arc<dyn Provider>>,
    live: Option<(Credentials, bool)>,
    last_refresh: Option<DateTime<Utc>>,
    fetch_tx: UnboundedSender<(String, ProviderStatus)>,
    in_flight: u32,
    in_flight_ids: HashSet<String>,
    last_attempt: HashMap<String, DateTime<Utc>>,
    retry_at: HashMap<String, DateTime<Utc>>,
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
            view: ScanView::Table,
            fetching: false,
            refresh_queued: false,
            scroll_row: 0,
            should_quit: false,
            last_area: Rect::new(0, 0, 80, 24),
            clock,
            providers,
            live: None,
            last_refresh: None,
            fetch_tx,
            in_flight: 0,
            in_flight_ids: HashSet::new(),
            last_attempt: HashMap::new(),
            retry_at: HashMap::new(),
            theme: Theme::select(),
        };
        (app, fetch_rx)
    }

    pub fn with_live(mut self, creds: Credentials, muse_on_demand: bool) -> Self {
        self.live = Some((creds, muse_on_demand));
        self
    }

    fn reload_live(&mut self) {
        let Some((creds, muse)) = &self.live else {
            return;
        };
        let Ok(next) = live_registry(creds, *muse) else {
            return;
        };
        let ids: HashSet<String> = next.iter().map(|p| p.id().to_string()).collect();
        self.snapshots.retain(|s| ids.contains(&s.id));
        if self
            .selected_id
            .as_ref()
            .is_some_and(|id| !ids.contains(id))
        {
            self.selected_id = self.snapshots.first().map(|s| s.id.clone());
        }
        self.providers = next;
    }

    pub async fn bootstrap(mut self, rx: &mut UnboundedReceiver<(String, ProviderStatus)>) -> Self {
        self.start_refresh(RefreshTrigger::Manual);
        self.drain_fetches(rx).await;
        self
    }

    pub fn on_tick(&mut self) {
        self.start_refresh(RefreshTrigger::Timer);
    }

    pub fn start_refresh(&mut self, trigger: RefreshTrigger) {
        if trigger == RefreshTrigger::Manual && self.fetching {
            self.refresh_queued = true;
            return;
        }
        let ids = self.ids_to_fetch(trigger);
        self.spawn_ids(&ids);
    }

    fn ids_to_fetch(&self, trigger: RefreshTrigger) -> Vec<String> {
        let now = self.clock.now();
        self.providers
            .iter()
            .filter_map(|p| {
                let id = p.id();
                if self.in_flight_ids.contains(id) {
                    return None;
                }
                if !allows_refresh(p.refresh_policy(), trigger) {
                    return None;
                }
                if trigger == RefreshTrigger::Manual {
                    return Some(id.to_string());
                }
                if let Some(at) = self.retry_at.get(id) {
                    return (now >= *at).then(|| id.to_string());
                }
                let period = refresh_period(p.refresh_policy())?;
                match self.last_attempt.get(id) {
                    None => Some(id.to_string()),
                    Some(t) => {
                        let elapsed = now.signed_duration_since(*t).to_std().unwrap_or_default();
                        (elapsed >= period).then(|| id.to_string())
                    }
                }
            })
            .collect()
    }

    fn spawn_ids(&mut self, ids: &[String]) {
        if ids.is_empty() {
            return;
        }
        let now = self.clock.now();
        for id in ids {
            let Some(p) = self.providers.iter().find(|p| p.id() == *id) else {
                continue;
            };
            if !self.in_flight_ids.insert(id.clone()) {
                continue;
            }
            self.last_attempt.insert(id.clone(), now);
            let p = Arc::clone(p);
            let timeout = p.fetch_timeout();
            let tx = self.fetch_tx.clone();
            let spawn_id = id.clone();
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
                let _ = tx.send((spawn_id, status));
            });
            self.in_flight += 1;
        }
        self.fetching = self.in_flight > 0;
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
        self.in_flight_ids.remove(&id);
        match &status {
            ProviderStatus::Error { message, .. } => {
                let wait = error_backoff(message);
                let delta = chrono::Duration::from_std(wait)
                    .unwrap_or(chrono::Duration::seconds(30));
                self.retry_at
                    .insert(id.clone(), self.clock.now() + delta);
            }
            _ => {
                self.retry_at.remove(&id);
            }
        }
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
            KeyCode::Char('r') => {
                self.reload_live();
                self.start_refresh(RefreshTrigger::Manual);
            }
            KeyCode::Char('o') => {
                self.sort = match self.sort {
                    SortMode::Risk => SortMode::Name,
                    SortMode::Name => SortMode::Risk,
                };
                sort_snapshots(&mut self.snapshots, self.sort);
            }
            KeyCode::Char('t') => {
                self.view = self.view.toggle();
                self.sync_scroll();
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

    pub fn handle_mouse(&mut self, ev: MouseEvent) {
        if !matches!(ev.kind, MouseEventKind::Down(MouseButton::Left)) {
            return;
        }
        if self.overlay != Overlay::None {
            self.overlay = Overlay::None;
            return;
        }
        match hit_at(
            ev.column,
            ev.row,
            self.last_area,
            &self.snapshots,
            self.scroll_row,
            self.view,
            self.sort,
        ) {
            Some(Hit::Row(id)) => {
                if self.selected_id.as_deref() == Some(id.as_str()) {
                    self.overlay = Overlay::Detail;
                } else {
                    self.selected_id = Some(id);
                    self.sync_scroll();
                }
            }
            Some(Hit::Key(c)) => self.handle_key(KeyCode::Char(c)),
            None => {}
        }
    }

    fn move_sel(&mut self, mv: GridMove) {
        let n = self.snapshots.len();
        if n == 0 {
            return;
        }
        let cols = match self.view {
            ScanView::Table => 1,
            ScanView::Cards => columns(self.last_area.width) as usize,
        };
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
        let n = self.snapshots.len();
        let i = self
            .selected_id
            .as_ref()
            .and_then(|id| self.snapshots.iter().position(|s| &s.id == id))
            .unwrap_or(0);
        let (vis, total_rows, selected_row) = match self.view {
            ScanView::Table => {
                let vis = visible_table_rows(self.last_area.height, self.last_area.width);
                (vis, n as u16, i as u16)
            }
            ScanView::Cards => {
                let cols = columns(self.last_area.width);
                let vis = visible_card_rows(self.last_area.height, self.last_area.width);
                let total_rows = if cols == 0 {
                    0
                } else {
                    ((n as u16) + cols - 1) / cols
                };
                (vis, total_rows, card_row(i, cols as usize) as u16)
            }
        };
        self.scroll_row = clamp_scroll(self.scroll_row, selected_row, vis, total_rows);
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
            self.view,
        );
    }

    pub fn selected(&self) -> Option<&ProviderSnapshot> {
        let id = self.selected_id.as_ref()?;
        self.snapshots.iter().find(|s| &s.id == id)
    }
}

#[cfg(test)]
#[path = "tui_test.rs"]
mod tests;
