use ratatui::style::Color;

#[derive(Clone, Copy)]
pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub border: Color,
    pub border_selected: Color,
    pub label: Color,
    pub text: Color,
    pub dim: Color,
    pub hero: Color,
    pub warn: Color,
    pub danger: Color,
    pub extra: Color,
    pub error: Color,
    pub unknown: Color,
    pub plan_pill_bg: Color,
    pub plan_pill_fg: Color,
    pub warn_bold: bool,
}

impl Theme {
    pub fn ember_ledger() -> Self {
        Self {
            bg: Color::Rgb(0x0B, 0x0C, 0x0F),
            surface: Color::Rgb(0x14, 0x16, 0x1A),
            border: Color::Rgb(0x2A, 0x2E, 0x35),
            border_selected: Color::Rgb(0xE8, 0xA8, 0x38),
            label: Color::Rgb(0x9A, 0xA3, 0xAD),
            text: Color::Rgb(0xE6, 0xE8, 0xEB),
            dim: Color::Rgb(0x6B, 0x72, 0x80),
            hero: Color::Rgb(0xE8, 0xA8, 0x38),
            warn: Color::Rgb(0xF0, 0xC1, 0x4A),
            danger: Color::Rgb(0xE2, 0x4A, 0x3B),
            extra: Color::Rgb(0x3D, 0xCE, 0xC2),
            error: Color::Rgb(0xB4, 0x55, 0x4A),
            unknown: Color::Rgb(0x6B, 0x72, 0x80),
            plan_pill_bg: Color::Rgb(0x2A, 0x2E, 0x35),
            plan_pill_fg: Color::Rgb(0xE8, 0xA8, 0x38),
            warn_bold: false,
        }
    }

    pub fn select() -> Self {
        let truecolor = std::env::var("COLORTERM")
            .map(|v| v.eq_ignore_ascii_case("truecolor") || v.eq_ignore_ascii_case("24bit"))
            .unwrap_or(false);
        if truecolor {
            Self::ember_ledger()
        } else {
            Self::ember_ledger_ansi16()
        }
    }

    pub fn ember_ledger_ansi16() -> Self {
        Self {
            bg: Color::Black,
            surface: Color::Black,
            border: Color::DarkGray,
            border_selected: Color::Yellow,
            label: Color::Gray,
            text: Color::White,
            dim: Color::DarkGray,
            hero: Color::Yellow,
            warn: Color::Yellow,
            danger: Color::Red,
            extra: Color::Cyan,
            error: Color::LightRed,
            unknown: Color::DarkGray,
            plan_pill_bg: Color::DarkGray,
            plan_pill_fg: Color::Yellow,
            warn_bold: true,
        }
    }

    pub fn remaining_color(self, display_pct: u8) -> Color {
        if display_pct >= 50 {
            self.hero
        } else if display_pct >= 20 {
            self.warn
        } else {
            self.danger
        }
    }
}

const EIGHTH: [char; 8] = ['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

pub fn eighth_bar(frac: f32, width: u16) -> String {
    if width == 0 {
        return String::new();
    }
    let frac = frac.clamp(0.0, 1.0);
    let total = (frac * f32::from(width) * 8.0).round() as u32;
    let full = (total / 8) as u16;
    let rem = total % 8;
    let mut s = String::new();
    for i in 0..width {
        if i < full {
            s.push('█');
        } else if i == full && rem > 0 {
            s.push(EIGHTH[rem as usize - 1]);
        } else {
            s.push('░');
        }
    }
    s
}

pub fn unknown_bar(width: u16) -> String {
    "╌".repeat(width as usize)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::display_pct;

    #[test]
    fn bar_empty_and_full() {
        let empty = eighth_bar(0.0, 8);
        assert_eq!(empty.chars().count(), 8);
        assert!(empty.chars().all(|c| c == '░'));
        let full = eighth_bar(1.0, 8);
        assert!(full.chars().all(|c| c == '█'));
    }

    #[test]
    fn remaining_color_uses_display_integer() {
        let t = Theme::ember_ledger();
        assert_eq!(t.remaining_color(display_pct(19.4)), t.danger);
        assert_eq!(t.remaining_color(display_pct(19.5)), t.warn);
        assert_eq!(t.remaining_color(50), t.hero);
        let a = Theme::ember_ledger_ansi16();
        assert!(a.warn_bold);
        assert_eq!(a.remaining_color(20), a.warn);
    }
}
