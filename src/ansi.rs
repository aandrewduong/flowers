//! Turns an image into coloured half-block text a terminal can show.

use crate::canvas::Image;
use crate::color::Rgb;

/// Colour support of the terminal being drawn to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum ColorDepth {
    /// 24-bit colour, supported by most modern terminals
    Truecolor,
    /// The xterm 256-colour palette, for terminals without truecolor
    Ansi256,
}

const UPPER_HALF_BLOCK: char = '▀';
const LOWER_HALF_BLOCK: char = '▄';
const BLANK: char = ' ';

const RESET: &str = "\x1b[0m";
const DEFAULT_FOREGROUND: &str = "\x1b[39m";
const DEFAULT_BACKGROUND: &str = "\x1b[49m";
const FOREGROUND_SGR: u8 = 38;
const BACKGROUND_SGR: u8 = 48;

/// Channel values of the 6×6×6 colour cube at xterm indices 16–231.
const CUBE_LEVELS: [u8; 6] = [0, 95, 135, 175, 215, 255];
const CUBE_START: u8 = 16;
/// The grey ramp at xterm indices 232–255 runs from 8 to 238 in steps of 10.
const GRAY_START: u8 = 232;
const GRAY_FIRST_LEVEL: i32 = 8;
const GRAY_STEP: i32 = 10;
const GRAY_STEPS: i32 = 24;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TermColor {
    Rgb([u8; 3]),
    Indexed(u8),
}

impl TermColor {
    fn new(color: Rgb, depth: ColorDepth) -> Self {
        let bytes = color.to_bytes();
        match depth {
            ColorDepth::Truecolor => Self::Rgb(bytes),
            ColorDepth::Ansi256 => Self::Indexed(nearest_xterm_index(bytes)),
        }
    }

    fn escape(self, layer: u8) -> String {
        match self {
            Self::Rgb([r, g, b]) => format!("\x1b[{layer};2;{r};{g};{b}m"),
            Self::Indexed(index) => format!("\x1b[{layer};5;{index}m"),
        }
    }
}

/// Escape sequence that sets the text colour, for text drawn alongside the art.
pub fn foreground(color: Rgb, depth: ColorDepth) -> String {
    TermColor::new(color, depth).escape(FOREGROUND_SGR)
}

/// Encodes `image` as one string per terminal row.
///
/// Each cell stacks two pixels using half-block glyphs: the upper pixel is the glyph's
/// foreground and the lower one its background. Terminal cells are about twice as tall as
/// they are wide, so this makes pixels roughly square.
pub fn encode(image: &Image, depth: ColorDepth) -> Vec<String> {
    (0..image.height().div_ceil(2))
        .map(|row| encode_row(image, row * 2, depth))
        .collect()
}

fn encode_row(image: &Image, top: usize, depth: ColorDepth) -> String {
    let mut line = String::new();
    let mut pen = Pen::default();
    for x in 0..image.width() {
        let upper = image.pixel(x, top).map(|c| TermColor::new(c, depth));
        let lower = (top + 1 < image.height())
            .then(|| image.pixel(x, top + 1))
            .flatten()
            .map(|c| TermColor::new(c, depth));
        let (glyph, foreground, background) = match (upper, lower) {
            (Some(upper), lower) => (UPPER_HALF_BLOCK, Some(upper), lower),
            (None, Some(lower)) => (LOWER_HALF_BLOCK, Some(lower), None),
            // A blank cell shows only its background, so leave the foreground alone.
            (None, None) => (BLANK, pen.foreground, None),
        };
        pen.switch_to(&mut line, foreground, background);
        line.push(glyph);
    }
    line.push_str(RESET);
    line
}

/// Tracks the colours already in effect so each cell only emits the escapes it changes.
#[derive(Default)]
struct Pen {
    foreground: Option<TermColor>,
    background: Option<TermColor>,
}

impl Pen {
    fn switch_to(
        &mut self,
        line: &mut String,
        foreground: Option<TermColor>,
        background: Option<TermColor>,
    ) {
        if foreground != self.foreground {
            match foreground {
                Some(color) => line.push_str(&color.escape(FOREGROUND_SGR)),
                None => line.push_str(DEFAULT_FOREGROUND),
            }
            self.foreground = foreground;
        }
        if background != self.background {
            match background {
                Some(color) => line.push_str(&color.escape(BACKGROUND_SGR)),
                None => line.push_str(DEFAULT_BACKGROUND),
            }
            self.background = background;
        }
    }
}

fn nearest_xterm_index(rgb: [u8; 3]) -> u8 {
    let cube_steps = rgb.map(nearest_cube_step);
    let cube_color = cube_steps.map(|step| CUBE_LEVELS[step]);
    let cube_index = CUBE_START + (36 * cube_steps[0] + 6 * cube_steps[1] + cube_steps[2]) as u8;

    let average = rgb.iter().map(|&c| i32::from(c)).sum::<i32>() / 3;
    let gray_step =
        ((average - GRAY_FIRST_LEVEL + GRAY_STEP / 2) / GRAY_STEP).clamp(0, GRAY_STEPS - 1);
    let gray_level = (GRAY_FIRST_LEVEL + gray_step * GRAY_STEP) as u8;
    let gray_index = GRAY_START + gray_step as u8;

    if distance_squared(rgb, [gray_level; 3]) < distance_squared(rgb, cube_color) {
        gray_index
    } else {
        cube_index
    }
}

fn nearest_cube_step(channel: u8) -> usize {
    (0..CUBE_LEVELS.len())
        .min_by_key(|&step| CUBE_LEVELS[step].abs_diff(channel))
        .unwrap_or(0)
}

fn distance_squared(a: [u8; 3], b: [u8; 3]) -> u32 {
    a.iter()
        .zip(b)
        .map(|(&x, y)| u32::from(x.abs_diff(y)).pow(2))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canvas::Canvas;
    use crate::geometry::{Bounds, Point};

    #[test]
    fn xterm_palette_matches_known_colors() {
        assert_eq!(nearest_xterm_index([255, 0, 0]), 196);
        assert_eq!(nearest_xterm_index([255, 255, 255]), 231);
        assert_eq!(nearest_xterm_index([0, 0, 0]), 16);
        assert_eq!(nearest_xterm_index([128, 128, 128]), 244);
    }

    #[test]
    fn stacks_two_pixels_per_cell() {
        // Column 0: both pixels painted. Column 1: only the top. Column 2: only the bottom.
        let mut canvas = Canvas::new(4, 2, 1);
        let red = Rgb::new(1.0, 0.0, 0.0);
        canvas.paint(Bounds::around(Point::new(1.0, 0.5), 1.0, 0.5), |_| {
            Some(red)
        });
        canvas.paint(Bounds::around(Point::new(1.0, 1.5), 1.0, 0.5), |p| {
            (p.x < 1.0).then_some(red)
        });
        canvas.paint(Bounds::around(Point::new(2.5, 1.5), 0.5, 0.5), |_| {
            Some(red)
        });

        let lines = encode(&canvas.resolve(), ColorDepth::Truecolor);
        assert_eq!(
            lines,
            vec![format!(
                "\x1b[38;2;255;0;0m\x1b[48;2;255;0;0m▀\x1b[49m▀▄ {RESET}"
            )]
        );
    }
}
