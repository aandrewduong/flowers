//! The two lines under the interactive garden: what's growing, and what the keys do.

use super::controls::KEY_HINTS;
use super::session::Session;
use crate::ansi::{self, ColorDepth};
use crate::color::Hsl;
use crate::flower::Species;
use crate::garden::Planting;

pub const STATUS_ROWS: usize = 2;

const RESET: &str = "\x1b[0m";
const DIM: &str = "\x1b[2m";
const BOLD: &str = "\x1b[1m";

pub fn status_lines(
    session: &Session,
    flowers: usize,
    columns: usize,
    depth: ColorDepth,
) -> [String; STATUS_ROWS] {
    // Keep clear of the last column, so the cursor never wraps out of the bottom-right cell.
    let width = columns.saturating_sub(1);
    [
        fit(&species_spans(session.planting()), width, depth),
        fit(&info_spans(session.seed(), flowers), width, depth),
    ]
}

enum Tone {
    Dim,
    Bold,
    Swatch(Hsl),
}

impl Tone {
    fn escape(&self, depth: ColorDepth) -> String {
        match self {
            Self::Dim => DIM.to_owned(),
            Self::Bold => BOLD.to_owned(),
            Self::Swatch(color) => format!("{BOLD}{}", ansi::foreground(color.to_rgb(), depth)),
        }
    }
}

struct Span {
    text: String,
    tone: Tone,
}

impl Span {
    fn new(text: impl Into<String>, tone: Tone) -> Self {
        Self {
            text: text.into(),
            tone,
        }
    }
}

/// Each species by number, in its own colour if it's growing and dimmed if not.
fn species_spans(planting: &Planting) -> Vec<Span> {
    (1..)
        .zip(Species::ALL)
        .flat_map(|(number, species)| {
            let tone = if planting.species.contains(&species) {
                Tone::Swatch(swatch(species))
            } else {
                Tone::Dim
            };
            [
                Span::new(format!(" {number} "), Tone::Dim),
                Span::new(species.name(), tone),
            ]
        })
        .collect()
}

fn info_spans(seed: u64, flowers: usize) -> Vec<Span> {
    let noun = if flowers == 1 { "flower" } else { "flowers" };
    let mut spans = vec![
        Span::new(format!(" seed {seed}"), Tone::Bold),
        Span::new(format!(" · {flowers} {noun} "), Tone::Dim),
    ];
    for (key, action) in KEY_HINTS {
        spans.push(Span::new(format!("  {key}"), Tone::Bold));
        spans.push(Span::new(format!(" {action}"), Tone::Dim));
    }
    spans
}

/// Renders spans left to right, cutting off whatever doesn't fit in `width` columns.
/// Every character used in the status bar is one column wide.
fn fit(spans: &[Span], width: usize, depth: ColorDepth) -> String {
    let mut line = String::new();
    let mut remaining = width;
    for span in spans {
        if remaining == 0 {
            break;
        }
        let text: String = span.text.chars().take(remaining).collect();
        remaining -= text.chars().count();
        line.push_str(&span.tone.escape(depth));
        line.push_str(&text);
        line.push_str(RESET);
    }
    line
}

/// The colour a growing species' name is shown in. Mid-tones, so names stay legible on
/// light and dark terminals alike.
fn swatch(species: Species) -> Hsl {
    match species {
        Species::Daisy => Hsl::new(48.0, 0.9, 0.6),
        Species::Sunflower => Hsl::new(40.0, 0.95, 0.5),
        Species::Rose => Hsl::new(350.0, 0.8, 0.55),
        Species::Dahlia => Hsl::new(322.0, 0.7, 0.58),
        Species::Cosmos => Hsl::new(330.0, 0.75, 0.65),
        Species::Poppy => Hsl::new(8.0, 0.9, 0.55),
        Species::Tulip => Hsl::new(22.0, 0.92, 0.55),
        Species::Lavender => Hsl::new(270.0, 0.6, 0.65),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The text a terminal would show, without colour escapes.
    fn visible(line: &str) -> String {
        let mut text = String::new();
        let mut in_escape = false;
        for character in line.chars() {
            match (in_escape, character) {
                (false, '\x1b') => in_escape = true,
                (true, 'm') => in_escape = false,
                (false, _) => text.push(character),
                (true, _) => {}
            }
        }
        text
    }

    #[test]
    fn fit_cuts_off_at_the_width() {
        let spans = [Span::new("abc", Tone::Bold), Span::new("defg", Tone::Dim)];
        assert_eq!(visible(&fit(&spans, 5, ColorDepth::Truecolor)), "abcde");
        assert_eq!(visible(&fit(&spans, 50, ColorDepth::Truecolor)), "abcdefg");
        assert_eq!(fit(&spans, 0, ColorDepth::Truecolor), "");
    }

    #[test]
    fn species_row_dims_species_that_are_not_growing() {
        let planting = Planting {
            count: None,
            species: vec![Species::Rose],
            grass: true,
        };
        let spans = species_spans(&planting);
        let tone_of = |name: &str| {
            &spans
                .iter()
                .find(|span| span.text == name)
                .expect("listed")
                .tone
        };
        assert!(matches!(tone_of("rose"), Tone::Swatch(_)));
        assert!(matches!(tone_of("daisy"), Tone::Dim));
        assert!(
            visible(&fit(&spans, 200, ColorDepth::Truecolor))
                .starts_with(" 1 daisy 2 sunflower 3 rose")
        );
    }

    #[test]
    fn flower_count_reads_naturally() {
        let line = |flowers| visible(&fit(&info_spans(42, flowers), 200, ColorDepth::Truecolor));
        assert!(line(1).starts_with(" seed 42 · 1 flower "));
        assert!(line(7).starts_with(" seed 42 · 7 flowers "));
    }
}
