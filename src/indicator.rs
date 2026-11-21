// SPDX-License-Identifier: GPL-3.0-only

//! The "now playing" equalizer bars shown on the panel button.

use cosmic::{
    iced::{Alignment, Border, Color, Length},
    widget::{container, row, space},
    Element,
};

/// One 8-frame wave; each bar reads it at its own phase offset.
const WAVE: [u8; 8] = [3, 6, 9, 10, 7, 4, 2, 5];
const PHASE: [u32; 4] = [0, 3, 6, 1];

/// Heights in px (2..=10) of the four bars for an animation frame; repeats every 8 frames.
pub fn bar_heights(frame: u32) -> [u8; 4] {
    PHASE.map(|p| WAVE[((frame + p) % 8) as usize])
}

/// Four accent-coloured bars, bottom-aligned in a 10px box.
pub fn view<'a, M: 'a>(frame: u32) -> Element<'a, M> {
    let bars = bar_heights(frame).map(|h| {
        container(space::horizontal().width(Length::Fixed(3.0)).height(Length::Fixed(h as f32)))
            .class(cosmic::theme::Container::custom(|theme| container::Style {
                background: Some(Color::from(theme.cosmic().accent_color()).into()),
                border: Border { radius: 1.0.into(), ..Default::default() },
                ..Default::default()
            }))
            .into()
    });
    container(row(bars).spacing(2).align_y(Alignment::End))
        .height(Length::Fixed(10.0))
        .align_y(Alignment::End)
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_in_range() {
        for f in 0..32 {
            assert!(bar_heights(f).iter().all(|h| (2..=10).contains(h)));
        }
    }

    #[test]
    fn cycles_every_8() {
        assert_eq!(bar_heights(3), bar_heights(11));
    }

    #[test]
    fn bars_not_all_equal() {
        assert!((0..8).any(|f| {
            let h = bar_heights(f);
            h.iter().any(|x| *x != h[0])
        }));
    }
}
