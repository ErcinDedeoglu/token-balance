use crate::domain::{
    ProviderSnapshot, caption_duration, effective_available, extra_line, format_age,
    format_countdown,
};
use crate::layout::clip;
use crate::theme::Theme;
use chrono::{DateTime, Utc};
use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    None,
    Detail,
    Help,
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let width = width.min(area.width);
    let height = height.min(area.height);
    Rect {
        x: area.x + area.width.saturating_sub(width) / 2,
        y: area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    }
}

pub fn render_help(frame: &mut Frame<'_>, area: Rect, theme: Theme) {
    let box_area = centered(area, 60, 13);
    frame.render_widget(Clear, box_area);
    let text = [
        " h/l  arrows       move in row (clamp, no wrap)",
        " j/k  arrows       table: one row; cards: by column count",
        " enter/space       open detail",
        " enter/esc/q       close overlay",
        " r                 refresh all",
        " o                 sort risk | name",
        " t                 view table | cards",
        " ?                 this help",
        " q / esc           close overlay, else quit",
    ]
    .join("\n");
    let block = Block::default()
        .title(" keys ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_selected))
        .style(Style::default().bg(theme.surface).fg(theme.text));
    frame.render_widget(
        Paragraph::new(text)
            .block(block)
            .style(Style::default().fg(theme.label)),
        box_area,
    );
}

pub fn render_detail(
    frame: &mut Frame<'_>,
    area: Rect,
    snap: &ProviderSnapshot,
    theme: Theme,
    now: DateTime<Utc>,
) {
    let box_area = centered(area, 62, 18);
    frame.render_widget(Clear, box_area);
    let mut lines: Vec<Line> = Vec::new();
    let plan = effective_available(&snap.status)
        .and_then(|a| a.plan.clone())
        .unwrap_or_else(|| "—".into());
    lines.push(kv("plan", plan, theme));
    lines.push(kv("ledger", "plan remaining", theme));
    lines.push(Line::from(""));
    if let Some(av) = effective_available(&snap.status) {
        for w in av.windows {
            let cap = caption_duration(w);
            let body = format!(
                "used {:.1}%   remaining {:.1}%",
                w.used_percent, w.remaining_percent
            );
            lines.push(kv(cap, body, theme));
            if let Some(m) = w.duration_mins {
                lines.push(kv("", format!("duration {m}m"), theme));
            }
            if let Some(at) = w.resets_at {
                let cd = format_countdown(now, Some(at));
                lines.push(kv(
                    "",
                    format!("resets {} ({cd})", at.format("%Y-%m-%d %H:%M UTC")),
                    theme,
                ));
            }
        }
        match av.extra {
            Some(ex) => lines.push(kv("extra", extra_line(ex), theme)),
            None => lines.push(kv("extra", "n/a", theme)),
        }
    } else {
        match &snap.status {
            crate::domain::ProviderStatus::NotConfigured { hint } => {
                lines.push(kv("status", "auth missing", theme));
                lines.push(kv("hint", hint.clone(), theme));
            }
            crate::domain::ProviderStatus::Unsupported { reason } => {
                lines.push(kv("status", "unsupported", theme));
                lines.push(kv("reason", reason.clone(), theme));
            }
            crate::domain::ProviderStatus::Error { message, .. } => {
                lines.push(kv("error", message.clone(), theme));
            }
            _ => {}
        }
    }
    lines.push(kv("fetched", format_age(now, snap.fetched_at), theme));
    if let Some(url) = &snap.docs_url {
        lines.push(kv("docs", url.clone(), theme));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        " enter/esc/q close",
        Style::default().fg(theme.dim),
    )));
    let title = format!(" {}  detail ", snap.display_name);
    let block = Block::default()
        .title(clip(&title, 58))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.border_selected))
        .style(Style::default().bg(theme.surface).fg(theme.text));
    frame.render_widget(
        Paragraph::new(lines)
            .block(block)
            .wrap(Wrap { trim: true })
            .alignment(Alignment::Left),
        box_area,
    );
}

fn kv(k: impl Into<String>, v: impl Into<String>, theme: Theme) -> Line<'static> {
    let k = k.into();
    Line::from(vec![
        Span::styled(format!(" {k:<12} "), Style::default().fg(theme.label)),
        Span::styled(v.into(), Style::default().fg(theme.text)),
    ])
}
