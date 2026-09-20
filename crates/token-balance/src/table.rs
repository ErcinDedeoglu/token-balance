use crate::domain::{
    CreditUnit, ExtraCredits, LedgerKind, ProviderSnapshot, ProviderStatus, display_pct,
    constraint_window, effective_available, extra_line, format_countdown, monthly_window,
    session_window, weekly_window,
};
use crate::layout::{
    ScanView, clip, header_height, padded_inner, pager_prefix, table_name_width, table_row_chunks,
    visible_index_range,
};
use unicode_width::UnicodeWidthStr;
use crate::theme::Theme;
use chrono::{DateTime, Utc};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::Paragraph;

pub const TABLE_COL_HEADER: u16 = 1;

pub fn visible_table_rows(height: u16, width: u16) -> u16 {
    height
        .saturating_sub(header_height(width))
        .saturating_sub(1)
        .saturating_sub(TABLE_COL_HEADER)
}

pub fn footer_keys(sort: crate::domain::SortMode, view: ScanView) -> String {
    let sort = match sort {
        crate::domain::SortMode::Risk => "risk",
        crate::domain::SortMode::Name => "name",
    };
    format!(
        " r refresh   o sort:{sort}   t view:{}   ? help   q quit",
        view.label()
    )
}

pub fn paint_table(
    frame: &mut Frame<'_>,
    area: Rect,
    snapshots: &[ProviderSnapshot],
    selected_id: Option<&str>,
    now: DateTime<Utc>,
    theme: Theme,
    scroll_row: u16,
    width: u16,
    height: u16,
) {
    let inner = padded_inner(area);
    if inner.height == 0 || inner.width == 0 {
        return;
    }
    let vis = visible_table_rows(height, width);
    let (start, end) = visible_index_range(scroll_row, vis, 1, snapshots.len());
    let header = Rect {
        x: inner.x,
        y: inner.y,
        width: inner.width,
        height: 1.min(inner.height),
    };
    let longest = snapshots
        .iter()
        .map(|s| format!(" {} {}", s.glyph, s.display_name).width())
        .max()
        .unwrap_or(7);
    let name_w = table_name_width(longest, inner.width);
    paint_col_header(frame, header, theme, name_w);
    let mut y = inner.y.saturating_add(1);
    for i in start..end {
        if y >= inner.y.saturating_add(inner.height) {
            break;
        }
        let row = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        paint_row(
            frame,
            row,
            &snapshots[i],
            selected_id == Some(snapshots[i].id.as_str()),
            theme,
            now,
            name_w,
        );
        y = y.saturating_add(1);
    }
}

pub fn table_at(
    col: u16,
    row: u16,
    body: Rect,
    snapshots: &[ProviderSnapshot],
    scroll_row: u16,
    width: u16,
    height: u16,
) -> Option<String> {
    let inner = padded_inner(body);
    if row <= inner.y {
        return None;
    }
    let vis = visible_table_rows(height, width);
    let (start, end) = visible_index_range(scroll_row, vis, 1, snapshots.len());
    let idx = start + (row.saturating_sub(inner.y).saturating_sub(1) as usize);
    if idx >= end || col < inner.x || col >= inner.x.saturating_add(inner.width) {
        return None;
    }
    snapshots.get(idx).map(|s| s.id.clone())
}

fn paint_col_header(frame: &mut Frame<'_>, area: Rect, theme: Theme, name_w: u16) {
    let cells = table_row_chunks(area, name_w);
    let labels = ["account", "5h", "wk", "mo", "reset", "extra"];
    let style = Style::default().fg(theme.label);
    for (i, cell) in cells.into_iter().enumerate() {
        let text = labels.get(i).copied().unwrap_or("");
        frame.render_widget(
            Paragraph::new(clip(text, cell.width as usize)).style(style),
            cell,
        );
    }
}

fn paint_row(
    frame: &mut Frame<'_>,
    area: Rect,
    snap: &ProviderSnapshot,
    selected: bool,
    theme: Theme,
    now: DateTime<Utc>,
    name_w: u16,
) {
    let cells = table_row_chunks(area, name_w);
    if cells.len() < 6 {
        return;
    }
    let name_style = if selected {
        Style::default()
            .fg(theme.border_selected)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.text)
    };
    let name = format!(" {} {}", snap.glyph, snap.display_name);
    frame.render_widget(
        Paragraph::new(clip(&name, cells[0].width as usize)).style(name_style),
        cells[0],
    );
    if snap.ledger == LedgerKind::PrepaidWallet {
        paint_prepaid_cells(frame, &cells[1..], snap, theme);
        return;
    }
    match effective_available(&snap.status) {
        Some(av) if !av.windows.is_empty() => {
            paint_plan_cells(frame, &cells[1..], av.windows, av.extra.as_ref(), theme, now);
        }
        _ => paint_unsigned_cells(frame, &cells[1..], snap, theme),
    }
}

fn paint_plan_cells(
    frame: &mut Frame<'_>,
    cells: &[Rect],
    windows: &[crate::domain::QuotaWindow],
    extra: Option<&ExtraCredits>,
    theme: Theme,
    now: DateTime<Utc>,
) {
    pct_cell(frame, cells[0], session_window(windows), theme);
    pct_cell(frame, cells[1], weekly_window(windows), theme);
    pct_cell(frame, cells[2], monthly_window(windows), theme);
    let cd = format_countdown(
        now,
        constraint_window(windows).and_then(|w| w.resets_at),
    );
    let reset = if cd.is_empty() { "—".into() } else { cd };
    frame.render_widget(
        Paragraph::new(clip(&reset, cells[3].width as usize)).style(Style::default().fg(theme.label)),
        cells[3],
    );
    let ex = extra.map(compact_extra).unwrap_or_else(|| "—".into());
    frame.render_widget(
        Paragraph::new(clip(&ex, cells[4].width as usize)).style(Style::default().fg(theme.extra)),
        cells[4],
    );
}

fn paint_prepaid_cells(
    frame: &mut Frame<'_>,
    cells: &[Rect],
    snap: &ProviderSnapshot,
    theme: Theme,
) {
    let dash = Style::default().fg(theme.dim);
    frame.render_widget(Paragraph::new("—").style(dash), cells[0]);
    frame.render_widget(Paragraph::new("—").style(dash), cells[1]);
    frame.render_widget(Paragraph::new("—").style(dash), cells[2]);
    frame.render_widget(
        Paragraph::new("no reset").style(Style::default().fg(theme.label)),
        cells[3],
    );
    let amount = effective_available(&snap.status)
        .and_then(|a| a.extra.as_ref())
        .map(compact_extra)
        .unwrap_or_else(|| "—".into());
    frame.render_widget(
        Paragraph::new(clip(&amount, cells[4].width as usize)).style(
            Style::default()
                .fg(theme.extra)
                .add_modifier(Modifier::BOLD),
        ),
        cells[4],
    );
}

fn paint_unsigned_cells(
    frame: &mut Frame<'_>,
    cells: &[Rect],
    snap: &ProviderSnapshot,
    theme: Theme,
) {
    let dim = Style::default().fg(theme.dim);
    let word = match &snap.status {
        ProviderStatus::NotConfigured { .. } => "auth missing",
        ProviderStatus::Unsupported { .. } => "unsupported",
        ProviderStatus::Error { .. } => "error",
        ProviderStatus::Available { .. } => "unavailable",
    };
    frame.render_widget(Paragraph::new("—").style(dim), cells[0]);
    frame.render_widget(Paragraph::new("—").style(dim), cells[1]);
    frame.render_widget(Paragraph::new("—").style(dim), cells[2]);
    frame.render_widget(Paragraph::new("—").style(dim), cells[3]);
    frame.render_widget(
        Paragraph::new(clip(word, cells[4].width as usize)).style(dim),
        cells[4],
    );
}

fn pct_cell(
    frame: &mut Frame<'_>,
    area: Rect,
    window: Option<&crate::domain::QuotaWindow>,
    theme: Theme,
) {
    let Some(w) = window else {
        frame.render_widget(
            Paragraph::new("—").style(Style::default().fg(theme.dim)),
            area,
        );
        return;
    };
    let pct = display_pct(w.remaining_percent);
    let color = theme.remaining_color(pct);
    frame.render_widget(
        Paragraph::new(clip(&format!("{pct}%"), area.width as usize)).style(Style::default().fg(color)),
        area,
    );
}

fn compact_extra(ex: &ExtraCredits) -> String {
    match ex.unit {
        CreditUnit::Usd => format!("${:.2}", ex.remaining),
        CreditUnit::Credits => format!("{} cr", ex.remaining as i64),
        CreditUnit::Unknown => extra_line(ex),
    }
}

pub fn table_pager(scroll_row: u16, height: u16, width: u16, n: usize) -> Option<String> {
    let vis = visible_table_rows(height, width);
    let (start, end) = visible_index_range(scroll_row, vis, 1, n);
    pager_prefix(start, end, n)
}

#[cfg(test)]
#[path = "table_test.rs"]
mod tests;
