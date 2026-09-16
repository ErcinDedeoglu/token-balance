use crate::cards::render_card;
use crate::domain::{
    LedgerKind, ProviderSnapshot, SortMode, display_pct, effective_available, format_age,
    min_remaining,
};
use crate::layout::{
    CARD_ROWS, card_row_areas, clip, columns, padded_inner, pager_prefix, visible_card_rows,
    visible_index_range,
};
use crate::overlay::{Overlay, render_detail, render_help};
use crate::theme::Theme;
use chrono::{DateTime, Utc};
use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

pub fn paint(
    frame: &mut Frame<'_>,
    snapshots: &[ProviderSnapshot],
    selected_id: Option<&str>,
    sort: SortMode,
    fetching: bool,
    scroll_row: u16,
    last_refresh: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    theme: Theme,
    overlay: Overlay,
    selected: Option<&ProviderSnapshot>,
) {
    let area = frame.area();
    frame.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(theme.bg)),
        area,
    );
    let (header, body, footer) = pack_chrome(area, snapshots.len(), scroll_row);
    draw_header(frame, header, snapshots, last_refresh, now, fetching, theme);
    draw_grid(
        frame,
        body,
        snapshots,
        selected_id,
        scroll_row,
        now,
        theme,
        area.width,
        area.height,
    );
    draw_footer(
        frame,
        footer,
        snapshots,
        sort,
        scroll_row,
        area.width,
        area.height,
        theme,
    );
    match overlay {
        Overlay::None => {}
        Overlay::Help => render_help(frame, area, theme),
        Overlay::Detail => {
            if let Some(s) = selected {
                render_detail(frame, area, s, theme, now);
            }
        }
    }
}

fn pack_chrome(area: Rect, n: usize, scroll_row: u16) -> (Rect, Rect, Rect) {
    let hh = crate::layout::header_height(area.width);
    let cols = columns(area.width);
    let vis = visible_card_rows(area.height, area.width);
    let (start, end) = visible_index_range(scroll_row, vis, cols, n);
    let shown = end.saturating_sub(start);
    let rows = if cols == 0 {
        0
    } else {
        ((shown as u16) + cols - 1) / cols
    };
    let grid_h = rows.saturating_mul(CARD_ROWS);
    let avail = area.height.saturating_sub(hh).saturating_sub(1);
    let chunks = Layout::vertical([
        Constraint::Length(hh),
        Constraint::Length(grid_h.min(avail)),
        Constraint::Length(1),
        Constraint::Fill(1),
    ])
    .split(area);
    (chunks[0], chunks[1], chunks[2])
}

fn draw_header(
    frame: &mut Frame<'_>,
    area: Rect,
    snaps: &[ProviderSnapshot],
    last_refresh: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    fetching: bool,
    theme: Theme,
) {
    let clock = now.format("%H:%M").to_string();
    let worst = worst_label(snaps);
    let age = last_refresh
        .map(|t| format_age(now, t))
        .unwrap_or_else(|| "—".into());
    let spin = if fetching { " ⠋" } else { "" };
    let left = format!(" token-balance  {clock}  worst {worst}");
    let right = format!("{age}{spin} ");
    let right_w = (right.chars().count() as u16).min(area.width);
    let chunks = Layout::horizontal([Constraint::Fill(1), Constraint::Length(right_w)]).split(area);
    frame.render_widget(
        Paragraph::new(clip(&left, chunks[0].width as usize))
            .style(Style::default().fg(theme.text)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(right).style(Style::default().fg(theme.label)),
        chunks[1],
    );
}

fn draw_grid(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshots: &[ProviderSnapshot],
    selected_id: Option<&str>,
    scroll_row: u16,
    now: DateTime<Utc>,
    theme: Theme,
    width: u16,
    height: u16,
) {
    let cols = columns(width);
    let vis = visible_card_rows(height, width);
    let n = snapshots.len();
    let (start, end) = visible_index_range(scroll_row, vis, cols, n);
    let inner = padded_inner(area);
    let mut y = inner.y;
    let mut idx = start;
    while idx < end {
        let row_area = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: CARD_ROWS.min(inner.y + inner.height - y),
        };
        if row_area.height < CARD_ROWS {
            break;
        }
        let slots = card_row_areas(row_area, cols);
        for (c, slot) in slots.into_iter().enumerate() {
            let i = idx + c;
            if i >= end {
                break;
            }
            let snap = &snapshots[i];
            let selected = selected_id == Some(snap.id.as_str());
            render_card(frame, slot, snap, selected, theme, now);
        }
        idx += cols as usize;
        y = y.saturating_add(CARD_ROWS);
    }
}

fn draw_footer(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshots: &[ProviderSnapshot],
    sort: SortMode,
    scroll_row: u16,
    width: u16,
    height: u16,
    theme: Theme,
) {
    let cols = columns(width);
    let vis = visible_card_rows(height, width);
    let n = snapshots.len();
    let (start, end) = visible_index_range(scroll_row, vis, cols, n);
    let sort = match sort {
        SortMode::Risk => "risk",
        SortMode::Name => "name",
    };
    let keys = format!(" r refresh   o sort:{sort}   ? help   q quit");
    let text = match pager_prefix(start, end, n) {
        Some(p) => format!(" {p}  {keys}"),
        None => keys,
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(theme.label).add_modifier(Modifier::DIM),
        ))),
        area,
    );
}

fn worst_label(snaps: &[ProviderSnapshot]) -> String {
    let mut best: Option<(u8, &str)> = None;
    for s in snaps {
        if s.ledger != LedgerKind::PlanRemaining {
            continue;
        }
        let Some(av) = effective_available(&s.status) else {
            continue;
        };
        let Some(min) = min_remaining(av.windows) else {
            continue;
        };
        let pct = display_pct(min);
        match best {
            None => best = Some((pct, s.display_name.as_str())),
            Some((p, _)) if pct < p => best = Some((pct, s.display_name.as_str())),
            _ => {}
        }
    }
    match best {
        Some((pct, name)) => format!("{name} {pct}%"),
        None => "—".into(),
    }
}
