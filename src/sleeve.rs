// SPDX-License-Identifier: GPL-3.0-only
//! "LP Sleeve" visual system for the popup, in the active COSMIC theme's
//! colours: the popup background is the sleeve, foreground text the print,
//! dimmed foreground the secondary line, the theme divider the hairlines,
//! the theme accent the attention colour (unread, playing, today), and
//! the theme destructive colour for critical urgency only. Album art and sender pictures are the only images;
//! there are no cards and no shadows. See DESIGN.md.

use cosmic::iced::{Background, Color, Length};
use cosmic::widget::{button, container, text};
use cosmic::{Element, Theme};

/// A colour role, resolved against the active theme at draw time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Print: titles, figures, day numbers (theme foreground).
    Fg,
    /// Secondary line: bodies, bylines, captions (dimmed foreground).
    Dim,
    /// Attention: unread, critical, playing (theme accent text colour).
    Accent,
    /// Text on an accent fill (today's cell).
    OnAccent,
    /// Critical urgency (theme destructive text colour), kept apart from
    /// accent so a critical row never reads as merely unread.
    Critical,
}

pub const CREAM: Tone = Tone::Fg;
pub const GREY: Tone = Tone::Dim;
pub const ORANGE: Tone = Tone::Accent;

/// Opacity of the dimmed foreground; keeps ≥4.5:1 on COSMIC's dark and light backgrounds.
const DIM_ALPHA: f32 = 0.68;

pub fn rgb(theme: &Theme, tone: Tone) -> Color {
    let c = theme.cosmic();
    match tone {
        Tone::Fg => c.on_bg_color().into(),
        Tone::Dim => {
            let mut x: Color = c.on_bg_color().into();
            x.a *= DIM_ALPHA;
            x
        }
        Tone::Accent => c.accent_text_color().into(),
        Tone::OnAccent => c.on_accent_color().into(),
        Tone::Critical => c.destructive_text_color().into(),
    }
}

pub fn rule_color(theme: &Theme) -> Color {
    theme.cosmic().bg_divider().into()
}

/// Text class for a tone (`Text::Custom` needs a plain fn per tone).
pub fn color(tone: Tone) -> cosmic::theme::Text {
    fn style(c: Color) -> cosmic::iced::widget::text::Style {
        cosmic::iced::widget::text::Style { color: Some(c), ..Default::default() }
    }
    cosmic::theme::Text::Custom(match tone {
        Tone::Fg => |t| style(rgb(t, Tone::Fg)),
        Tone::Dim => |t| style(rgb(t, Tone::Dim)),
        Tone::Accent => |t| style(rgb(t, Tone::Accent)),
        Tone::OnAccent => |t| style(rgb(t, Tone::OnAccent)),
        Tone::Critical => |t| style(rgb(t, Tone::Critical)),
    })
}

/// The tracklist's one text size.
pub const ROW: u16 = 14;
/// Small caps labels (side names, credits headings, weekday initials).
pub const CAPTION: u16 = 11;

/// Uppercase caption in the dimmed tone: a side or credits heading.
pub fn caption<'a, M: 'a>(label: impl Into<String>) -> cosmic::widget::Text<'a, cosmic::Theme> {
    text(label.into().to_uppercase()).size(CAPTION).font(cosmic::font::bold()).class(color(Tone::Dim))
}

/// Figures (`HH:MM`, `31°`) in fixed cells, so columns of times line up
/// (iced cannot request the font's tabular figures).
pub fn figures<M: 'static>(label: &str, size: u16, bold: bool, tone: Tone) -> Element<'static, M> {
    let digit = f32::from(size) * 0.6;
    let mut r = cosmic::iced::widget::Row::new();
    for ch in label.chars() {
        let w = if ch.is_ascii_digit() { digit } else if ch == ':' { digit * 0.5 } else { 0.0 };
        let mut t = text(ch.to_string()).size(size).class(color(tone));
        if bold {
            t = t.font(cosmic::font::bold());
        }
        r = r.push(if w > 0.0 { Element::from(container(t).center_x(Length::Fixed(w))) } else { t.into() });
    }
    r.into()
}

/// Full-width 1px hairline in the theme's divider colour.
pub fn hairline<M: 'static>() -> Element<'static, M> {
    container(cosmic::widget::Space::new().height(Length::Fixed(1.0)).width(Length::Fill))
        .height(Length::Fixed(1.0))
        .width(Length::Fill)
        .class(cosmic::theme::Container::custom(|t| cosmic::iced::widget::container::Style {
            background: Some(Background::Color(rule_color(t))),
            ..Default::default()
        }))
        .into()
}

/// 1px vertical rule in the theme's divider colour.
pub fn vrule<M: 'static>() -> Element<'static, M> {
    container(cosmic::widget::Space::new().width(Length::Fixed(1.0)).height(Length::Fill))
        .width(Length::Fixed(1.0))
        .height(Length::Fill)
        .class(cosmic::theme::Container::custom(|t| cosmic::iced::widget::container::Style {
            background: Some(Background::Color(rule_color(t))),
            ..Default::default()
        }))
        .into()
}

/// Flat text/icon button: no fill at rest, the theme's hover/pressed
/// component fills under the pointer.
pub fn flat(tone: Tone) -> cosmic::theme::Button {
    fn style(theme: &Theme, tone: Tone, fill: Option<Color>) -> button::Style {
        let fg = rgb(theme, if fill.is_some() && tone == Tone::Dim { Tone::Fg } else { tone });
        let mut s = button::Style::new();
        s.background = fill.map(Background::Color);
        s.border_radius = 4.0.into();
        s.text_color = Some(fg);
        s.icon_color = Some(fg);
        s
    }
    cosmic::theme::Button::Custom {
        active: Box::new(move |_, t| style(t, tone, None)),
        disabled: Box::new(move |t| {
            let mut s = style(t, Tone::Dim, None);
            s.text_color = Some(rule_color(t));
            s.icon_color = Some(rule_color(t));
            s
        }),
        hovered: Box::new(move |_, t| style(t, tone, Some(t.cosmic().background(t.transparent).component.hover.into()))),
        pressed: Box::new(move |_, t| style(t, tone, Some(t.cosmic().background(t.transparent).component.pressed.into()))),
    }
}

/// Accent-filled square (today's cell).
pub fn accent_cell() -> cosmic::theme::Container<'static> {
    cosmic::theme::Container::custom(|t| cosmic::iced::widget::container::Style {
        background: Some(Background::Color(t.cosmic().accent_color().into())),
        border: cosmic::iced::Border { radius: 2.0.into(), ..Default::default() },
        ..Default::default()
    })
}

/// Placeholder tile behind a missing album cover.
pub fn blank_cover() -> cosmic::theme::Container<'static> {
    cosmic::theme::Container::custom(|t| cosmic::iced::widget::container::Style {
        background: Some(Background::Color(rule_color(t))),
        icon_color: Some(rgb(t, Tone::Dim)),
        ..Default::default()
    })
}

/// Accent unread mark (a vinyl centre-label dot).
pub fn dot<M: 'static>() -> Element<'static, M> {
    container(cosmic::widget::Space::new().width(Length::Fixed(7.0)).height(Length::Fixed(7.0)))
        .class(cosmic::theme::Container::custom(|t| cosmic::iced::widget::container::Style {
            background: Some(Background::Color(t.cosmic().accent_color().into())),
            border: cosmic::iced::Border { radius: 3.5.into(), ..Default::default() },
            ..Default::default()
        }))
        .into()
}

/// Dismiss glyph, embedded so icon themes that blank `window-close-symbolic`
/// (to hide titlebar close buttons) do not hide it.
pub const CLOSE_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 16 16"><path fill="#232323" d="M4.3 4.3a1 1 0 0 1 1.4 0L8 6.6l2.3-2.3a1 1 0 1 1 1.4 1.4L9.4 8l2.3 2.3a1 1 0 0 1-1.4 1.4L8 9.4l-2.3 2.3a1 1 0 0 1-1.4-1.4L6.6 8 4.3 5.7a1 1 0 0 1 0-1.4z"/></svg>"##;

pub fn close_glyph(size: u16) -> cosmic::widget::Icon {
    cosmic::widget::icon::icon(cosmic::widget::icon::from_svg_bytes(CLOSE_SVG).symbolic(true)).size(size)
}
