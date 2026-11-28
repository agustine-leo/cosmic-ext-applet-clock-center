// SPDX-License-Identifier: GPL-3.0-only

//! The "now playing" strip on the panel button: the track title scrolls
//! dimmed underneath while a wave of accent bars rolls across on top.

use cosmic::{
    iced::{Alignment, Border, Color, Length},
    widget::{container, row, space},
    Element,
};

/// Animation frame interval while playing (20 fps: smooth scroll, light on the panel).
pub const FRAME: std::time::Duration = std::time::Duration::from_millis(50);

/// Width of the now-playing strip on the panel.
pub const STRIP_WIDTH: f32 = 140.0;
const STRIP_HEIGHT: f32 = 18.0;
const TITLE_SIZE: f32 = 12.0;
const BAR_WIDTH: f32 = 3.0;
const BAR_GAP: f32 = 2.0;
/// Pixels the title moves per animation frame.
pub const SCROLL_STEP: f32 = 1.0;
/// Space between the end of the title and its next pass.
const LOOP_GAP: &str = "      ·      ";

/// Bar heights (px) across the strip for a frame: two travelling sine waves,
/// so the wave rolls left to right instead of pulsing in place.
pub fn wave_heights(frame: u32, bars: usize) -> Vec<f32> {
    let t = frame as f32 * 0.22;
    (0..bars)
        .map(|i| {
            let x = i as f32;
            let v = 0.55 * ((x * 0.55) - t).sin() + 0.45 * ((x * 0.23) + t * 0.6).sin();
            2.0 + (v * 0.5 + 0.5) * (STRIP_HEIGHT - 4.0)
        })
        .collect()
}

/// Rendered width of the title at the strip's size, so the marquee loops
/// without a jump.
pub fn text_width(s: &str) -> f32 {
    use cosmic::iced::advanced::text::{self, Paragraph as _};
    let p = <cosmic::Renderer as text::Renderer>::Paragraph::with_text(text::Text {
        content: s,
        bounds: cosmic::iced::Size::new(f32::INFINITY, STRIP_HEIGHT * 2.0),
        size: TITLE_SIZE.into(),
        line_height: text::LineHeight::default(),
        font: cosmic::font::default(),
        align_x: text::Alignment::Left,
        align_y: cosmic::iced::alignment::Vertical::Top,
        shaping: text::Shaping::Advanced,
        wrapping: text::Wrapping::None,
        ellipsize: text::Ellipsize::None,
    });
    p.min_width()
}

/// Marquee offset (px) for a frame, looping every `period` px.
pub fn scroll_offset(frame: u32, period: f32) -> f32 {
    if period <= 0.0 {
        return 0.0;
    }
    (frame as f32 * SCROLL_STEP) % period
}

/// Now-playing strip: the title scrolls dimmed underneath while a wave of
/// accent bars rolls across on top. `period` is the title's pass width
/// (`text_width` of `title + LOOP_GAP`), cached by the caller.
pub fn strip<'a, M: 'a>(frame: u32, title: &str, period: f32) -> Element<'a, M> {
    use cosmic::iced::widget::{pin, stack};
    let pass = format!("{title}{LOOP_GAP}");
    // Two passes back to back; shifting by one period lands on the same picture.
    let marquee = cosmic::widget::text(format!("{pass}{pass}"))
        .size(TITLE_SIZE)
        .wrapping(cosmic::iced::widget::text::Wrapping::None)
        .class(cosmic::theme::Text::Custom(|t| {
            let mut c: Color = t.cosmic().on_bg_color().into();
            c.a *= 0.85;
            cosmic::iced::widget::text::Style { color: Some(c), ..Default::default() }
        }));
    let title_layer = container(pin(marquee).x(-scroll_offset(frame, period)).y((STRIP_HEIGHT - TITLE_SIZE * 1.3) / 2.0))
        .width(Length::Fixed(STRIP_WIDTH))
        .height(Length::Fixed(STRIP_HEIGHT))
        .clip(true);

    let bars = (STRIP_WIDTH / (BAR_WIDTH + BAR_GAP)) as usize;
    let wave = wave_heights(frame, bars).into_iter().map(|h| {
        container(space::horizontal().width(Length::Fixed(BAR_WIDTH)).height(Length::Fixed(h)))
            .class(cosmic::theme::Container::custom(|theme| {
                let mut c: Color = theme.cosmic().accent_color().into();
                c.a = 0.45;
                container::Style {
                    background: Some(c.into()),
                    border: Border { radius: 1.0.into(), ..Default::default() },
                    ..Default::default()
                }
            }))
            .into()
    });
    let wave_layer = container(row(wave).spacing(BAR_GAP).align_y(Alignment::End))
        .width(Length::Fixed(STRIP_WIDTH))
        .height(Length::Fixed(STRIP_HEIGHT))
        .align_y(Alignment::End);

    container(stack![title_layer, wave_layer])
        .width(Length::Fixed(STRIP_WIDTH))
        .height(Length::Fixed(STRIP_HEIGHT))
        .padding([0, 0, 0, 0])
        .into()
}

/// Strip title: "Title · Artist", or whichever is known.
pub fn strip_title(title: &str, artist: &str) -> String {
    match (title.trim(), artist.trim()) {
        ("", a) => a.to_string(),
        (t, "") => t.to_string(),
        (t, a) => format!("{t} · {a}"),
    }
}

/// The text one marquee pass covers (title plus the gap before it repeats).
pub fn pass_text(title: &str) -> String {
    format!("{title}{LOOP_GAP}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wave_stays_inside_strip_and_moves() {
        let a = wave_heights(0, 28);
        assert!(a.iter().all(|h| (2.0..=STRIP_HEIGHT - 2.0 + 0.01).contains(h)));
        assert_ne!(a, wave_heights(1, 28));
    }

    #[test]
    fn marquee_loops_within_one_period() {
        assert_eq!(scroll_offset(0, 100.0), 0.0);
        assert!(scroll_offset(250, 100.0) < 100.0);
        assert_eq!(scroll_offset(10, 0.0), 0.0);
    }

    #[test]
    fn strip_title_joins_known_parts() {
        assert_eq!(strip_title("Who Else", "Bethel Music"), "Who Else · Bethel Music");
        assert_eq!(strip_title("Who Else", " "), "Who Else");
        assert_eq!(strip_title("", "Bethel Music"), "Bethel Music");
    }



}
