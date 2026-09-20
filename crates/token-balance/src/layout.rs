use ratatui::layout::{Constraint, Layout, Rect};
use unicode_width::UnicodeWidthStr;

pub const CARD_ROWS: u16 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridMove {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanView {
    Table,
    Cards,
}

impl ScanView {
    pub fn toggle(self) -> Self {
        match self {
            Self::Table => Self::Cards,
            Self::Cards => Self::Table,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Table => "table",
            Self::Cards => "cards",
        }
    }
}

pub fn columns(width: u16) -> u16 {
    if width >= 140 {
        3
    } else if width >= 80 {
        2
    } else {
        1
    }
}

pub fn header_height(width: u16) -> u16 {
    if width >= 80 { 1 } else { 2 }
}

pub fn visible_card_rows(height: u16, width: u16) -> u16 {
    let inner = height
        .saturating_sub(header_height(width))
        .saturating_sub(1);
    inner / CARD_ROWS
}

pub fn card_row(index: usize, cols: usize) -> usize {
    if cols == 0 { 0 } else { index / cols }
}

pub fn move_index(i: usize, n: usize, cols: usize, mv: GridMove) -> usize {
    if n == 0 {
        return 0;
    }
    let cols = cols.max(1);
    let row = i / cols;
    match mv {
        GridMove::Left => {
            let start = row * cols;
            i.saturating_sub(1).max(start)
        }
        GridMove::Right => {
            let end = ((row + 1) * cols - 1).min(n - 1);
            (i + 1).min(end)
        }
        GridMove::Down => {
            let j = i + cols;
            if j < n { j } else { i }
        }
        GridMove::Up => {
            if i >= cols {
                i - cols
            } else {
                i
            }
        }
    }
}

pub fn clamp_scroll(scroll_row: u16, selected_row: u16, visible_rows: u16, total_rows: u16) -> u16 {
    if visible_rows == 0 {
        return 0;
    }
    let mut scroll = scroll_row.min(total_rows.saturating_sub(visible_rows));
    if selected_row < scroll {
        scroll = selected_row;
    } else if selected_row >= scroll + visible_rows {
        scroll = selected_row.saturating_sub(visible_rows.saturating_sub(1));
    }
    scroll
}

pub fn visible_index_range(
    scroll_row: u16,
    visible_rows: u16,
    cols: u16,
    n: usize,
) -> (usize, usize) {
    let start = (scroll_row as usize) * (cols as usize);
    let count = (visible_rows as usize).saturating_mul(cols as usize);
    let end = (start + count).min(n);
    (start.min(n), end)
}

pub fn pager_prefix(start: usize, end: usize, n: usize) -> Option<String> {
    if end < n && n > 0 && end > start {
        Some(format!("{}–{} / {n}", start + 1, end))
    } else {
        None
    }
}

pub fn clip(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.width() <= max {
        return s.to_string();
    }
    let mut out = String::new();
    for c in s.chars() {
        let next = format!("{out}{c}");
        if next.width() > max {
            break;
        }
        out = next;
    }
    out
}

pub fn wrap_words(s: &str, max: usize) -> Vec<String> {
    if max == 0 {
        return Vec::new();
    }
    let mut lines = Vec::new();
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let trial = if cur.is_empty() {
            word.to_string()
        } else {
            format!("{cur} {word}")
        };
        if trial.width() <= max {
            cur = trial;
            continue;
        }
        if !cur.is_empty() {
            lines.push(cur);
        }
        cur = clip(word, max);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

pub fn padded_inner(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(1),
        y: area.y,
        width: area.width.saturating_sub(2),
        height: area.height,
    }
}

pub fn card_row_areas(inner: Rect, cols: u16) -> Vec<Rect> {
    if cols == 0 || inner.width == 0 {
        return Vec::new();
    }
    let mut constraints = Vec::new();
    for i in 0..cols {
        if i > 0 {
            constraints.push(Constraint::Length(1));
        }
        constraints.push(Constraint::Fill(1));
    }
    Layout::horizontal(constraints)
        .split(inner)
        .iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0)
        .map(|(_, r)| *r)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    #[test]
    fn breakpoints() {
        assert_eq!(columns(79), 1);
        assert_eq!(columns(80), 2);
        assert_eq!(columns(139), 2);
        assert_eq!(columns(140), 3);
    }

    #[test]
    fn two_col_j_from_codex_is_claude_index() {
        // mixed order: 0 codex, 1 grok, 2 claude
        assert_eq!(move_index(0, 6, 2, GridMove::Down), 2);
        assert_eq!(move_index(0, 6, 2, GridMove::Right), 1);
        assert_eq!(move_index(0, 6, 1, GridMove::Left), 0);
        assert_eq!(move_index(0, 6, 1, GridMove::Right), 0);
    }

    #[test]
    fn wrap_words_fits_narrow_col() {
        let lines = wrap_words("no remaining API; polling burns Everyday requests", 28);
        assert!(lines.len() >= 2);
        assert!(lines.iter().any(|l| l.contains("Everyday")));
        assert!(lines.iter().all(|l| l.width() <= 28));
    }

    #[test]
    fn pager_only_when_clipped() {
        assert_eq!(pager_prefix(0, 3, 6).as_deref(), Some("1–3 / 6"));
        assert_eq!(pager_prefix(0, 6, 6), None);
        assert_eq!(pager_prefix(0, 6, 6), None);
    }
}
