use crate::domain::{
    LedgerKind, ProviderSnapshot, ProviderStatus, caption_duration, display_pct,
    effective_available, extra_line, format_age, format_countdown,
};
use crate::layout::{clip, wrap_words};
use crate::theme::{Theme, eighth_bar, unknown_bar};
use chrono::{DateTime, Utc};
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

pub fn render_card(
    frame: &mut Frame<'_>,
    area: Rect,
    snap: &ProviderSnapshot,
    selected: bool,
    theme: Theme,
    now: DateTime<Utc>,
) {
    let border = if selected {
        theme.border_selected
    } else {
        theme.border
    };
    let plan = effective_available(&snap.status).and_then(|a| a.plan.clone());
    let mut title = vec![Span::styled(
        format!(" {} {} ", snap.glyph, snap.display_name),
        Style::default().fg(theme.text),
    )];
    if let Some(p) = plan {
        title.push(Span::styled(
            format!("[{p}] "),
            Style::default()
                .fg(theme.plan_pill_fg)
                .bg(theme.plan_pill_bg),
        ));
    }
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(border))
        .style(Style::default().bg(theme.surface).fg(theme.text))
        .title(Line::from(title));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height < 5 || inner.width == 0 {
        return;
    }
    let n = inner.height as usize;
    let rows = Layout::vertical(vec![Constraint::Length(1); n]).split(inner);
    match effective_available(&snap.status) {
        Some(av) if snap.ledger == LedgerKind::PlanRemaining && !av.windows.is_empty() => {
            paint_available(
                frame,
                &rows,
                snap,
                av.windows,
                av.extra.as_ref(),
                theme,
                now,
            );
        }
        Some(av) if snap.ledger == LedgerKind::PrepaidWallet => {
            paint_prepaid(frame, &rows, snap, av.extra.as_ref(), theme, now);
        }
        _ => paint_unsigned(frame, &rows, snap, theme),
    }
}

fn paint_available(
    frame: &mut Frame<'_>,
    rows: &[Rect],
    snap: &ProviderSnapshot,
    windows: &[crate::domain::QuotaWindow],
    extra: Option<&crate::domain::ExtraCredits>,
    theme: Theme,
    now: DateTime<Utc>,
) {
    let hero = crate::domain::hero_window(windows);
    if let Some(h) = hero {
        let pct = display_pct(h.remaining_percent);
        let color = theme.remaining_color(pct);
        let mut style = Style::default().fg(color);
        if theme.warn_bold && (20..50).contains(&pct) {
            style = style.add_modifier(Modifier::BOLD);
        }
        let label = format!(" {pct}%  ");
        let bar_w = rows[0].width.saturating_sub(label.len() as u16);
        let bar = eighth_bar(h.remaining_percent / 100.0, bar_w);
        let line = Line::from(vec![Span::styled(label, style), Span::styled(bar, style)]);
        frame.render_widget(Paragraph::new(line), rows[0]);
        let cd = format_countdown(now, h.resets_at);
        let cap = if cd.is_empty() {
            format!("       {}", caption_duration(h))
        } else {
            format!("       {}   resets in {cd}", caption_duration(h))
        };
        frame.render_widget(
            Paragraph::new(clip(&cap, rows[1].width as usize))
                .style(Style::default().fg(theme.label)),
            rows[1],
        );
    }
    if let Some(sec) = crate::domain::secondary_window(windows) {
        let pct = display_pct(sec.remaining_percent);
        let color = theme.remaining_color(pct);
        let cd = format_countdown(now, sec.resets_at);
        let prefix = format!(" {}   ", caption_duration(sec));
        let suffix = format!("  {pct}%   {cd}");
        let bar_w = rows[2]
            .width
            .saturating_sub(prefix.len() as u16 + suffix.len() as u16);
        let bar = eighth_bar(sec.remaining_percent / 100.0, bar_w);
        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(prefix, Style::default().fg(theme.label)),
                Span::styled(bar, Style::default().fg(color)),
                Span::styled(suffix, Style::default().fg(theme.label)),
            ])),
            rows[2],
        );
    }
    if let Some(ex) = extra {
        if rows.len() > 4 {
            frame.render_widget(
                Paragraph::new(clip(
                    &format!(" {}", extra_line(ex)),
                    rows[3].width as usize,
                ))
                .style(Style::default().fg(theme.extra)),
                rows[3],
            );
        }
    }
    let foot = match &snap.status {
        ProviderStatus::Error { .. } => " stale".into(),
        _ => format!(" {}", format_age(now, snap.fetched_at)),
    };
    let foot_style = if matches!(snap.status, ProviderStatus::Error { .. }) {
        Style::default().fg(theme.error)
    } else {
        Style::default().fg(theme.dim)
    };
    let last = rows.len().saturating_sub(1);
    frame.render_widget(Paragraph::new(foot).style(foot_style), rows[last]);
}

fn paint_prepaid(
    frame: &mut Frame<'_>,
    rows: &[Rect],
    snap: &ProviderSnapshot,
    extra: Option<&crate::domain::ExtraCredits>,
    theme: Theme,
    now: DateTime<Utc>,
) {
    let last = rows.len().saturating_sub(1);
    if let Some(ex) = extra {
        let hero = match ex.unit {
            crate::domain::CreditUnit::Usd => format!("${:.2}", ex.remaining),
            crate::domain::CreditUnit::Credits => format!("{} cr", ex.remaining as i64),
            crate::domain::CreditUnit::Unknown => extra_line(ex),
        };
        let body = last.saturating_sub(1);
        let mid = body / 2;
        frame.render_widget(
            Paragraph::new(clip(&hero, rows[mid].width as usize))
                .alignment(Alignment::Center)
                .style(
                    Style::default()
                        .fg(theme.extra)
                        .add_modifier(Modifier::BOLD),
                ),
            rows[mid],
        );
        let cap = mid + 1;
        if cap < last {
            frame.render_widget(
                Paragraph::new("prepaid   no reset")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(theme.label)),
                rows[cap],
            );
        }
    }
    frame.render_widget(
        Paragraph::new(format!(" {}", format_age(now, snap.fetched_at)))
            .style(Style::default().fg(theme.dim)),
        rows[last],
    );
}

fn paint_unsigned(frame: &mut Frame<'_>, rows: &[Rect], snap: &ProviderSnapshot, theme: Theme) {
    let dim = Style::default().fg(theme.dim);
    let (word, hint) = match &snap.status {
        ProviderStatus::NotConfigured { hint } => ("auth missing", hint.as_str()),
        ProviderStatus::Unsupported { reason } => ("unsupported", reason.as_str()),
        ProviderStatus::Error { message, .. } => ("error", message.as_str()),
        ProviderStatus::Available { .. } => ("unavailable", ""),
    };
    frame.render_widget(Paragraph::new(format!(" {word}")).style(dim), rows[0]);
    let last = rows.len().saturating_sub(1);
    let hint_w = rows.get(1).map(|r| r.width.saturating_sub(1) as usize).unwrap_or(0);
    let wrapped = wrap_words(hint, hint_w);
    let hint_lines = wrapped.len().min(last.saturating_sub(1));
    for (i, line) in wrapped.iter().take(hint_lines).enumerate() {
        frame.render_widget(Paragraph::new(format!(" {line}")).style(dim), rows[1 + i]);
    }
    let bar_i = (1 + hint_lines).min(last);
    let bar = unknown_bar(rows[bar_i].width.saturating_sub(2));
    frame.render_widget(
        Paragraph::new(format!(" {bar}")).style(Style::default().fg(theme.unknown)),
        rows[bar_i],
    );
}
