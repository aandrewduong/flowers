//! Weather falling over the garden.

use std::ops::Range;

use crate::canvas::Canvas;
use crate::color::Rgb;
use crate::geometry::{Bounds, Point};
use crate::rng::Rng;

/// Every shower shares one pattern of drops; only where they've fallen to changes.
const RAIN_SEED: u64 = 0x5241_494E;
const DROPS_PER_PIXEL: f32 = 0.012;
/// Pixels a drop drifts sideways for each pixel it falls.
const SLANT: f32 = 0.25;
/// Garden heights fallen per second.
const FALL_SPEED: Range<f32> = 1.3..2.0;
/// Length of a drop's streak, as a fraction of the garden height.
const STREAK_LENGTH: Range<f32> = 0.06..0.12;
const STREAK_HALF_WIDTH: f32 = 0.55;
const RAIN_COLOR: Rgb = Rgb::new(0.74, 0.82, 0.93);
/// A streak is faint where it trails off and strongest at the falling end.
const STREAK_OPACITY: Range<f32> = 0.3..0.75;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Weather {
    /// Nothing falling
    #[default]
    Clear,
    /// Slanting rain under a grey sky
    Rain,
}

impl Weather {
    /// Whether clouds cover the sky.
    pub fn is_overcast(self) -> bool {
        self == Self::Rain
    }

    /// Whether the weather moves, and so needs redrawing as time passes.
    pub fn is_falling(self) -> bool {
        self == Self::Rain
    }

    /// Paints whatever is falling at `time` (seconds) over the whole canvas.
    pub fn paint(self, canvas: &mut Canvas, time: f32) {
        match self {
            Self::Clear => {}
            Self::Rain => paint_rain(canvas, time),
        }
    }
}

fn paint_rain(canvas: &mut Canvas, time: f32) {
    let Point {
        x: width,
        y: height,
    } = canvas.bounds().max;
    // Reseeding every frame deals out the same drops each time.
    let mut rng = Rng::new(RAIN_SEED);
    let count = (width * height * DROPS_PER_PIXEL) as usize;
    for _ in 0..count {
        let length = height * rng.range(STREAK_LENGTH);
        let speed = height * rng.range(FALL_SPEED);
        // A drop starts just above the garden and wraps around once it has fallen clear.
        let travel = height + length;
        let top = (rng.range(0.0..travel) + speed * time) % travel - length;
        // Drops enter far enough to the left that their slant still reaches that edge.
        let entry = rng.range(-SLANT * height..width);
        paint_streak(canvas, Point::new(entry + SLANT * top, top), length);
    }
}

/// `tail` is the upper end of the streak, which runs `length` pixels down from it.
fn paint_streak(canvas: &mut Canvas, tail: Point, length: f32) {
    let bounds = Bounds {
        min: Point::new(tail.x - STREAK_HALF_WIDTH, tail.y),
        max: Point::new(tail.x + SLANT * length + STREAK_HALF_WIDTH, tail.y + length),
    };
    canvas.blend(bounds, |point| {
        let fallen = point.y - tail.y;
        let spine = tail.x + SLANT * fallen;
        let along = fallen / length;
        let opacity = STREAK_OPACITY.start + (STREAK_OPACITY.end - STREAK_OPACITY.start) * along;
        ((point.x - spine).abs() < STREAK_HALF_WIDTH).then_some((RAIN_COLOR, opacity))
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn painted(weather: Weather, time: f32) -> Vec<Option<Rgb>> {
        let (width, height) = (40, 30);
        let mut canvas = Canvas::new(width, height, 2);
        weather.paint(&mut canvas, time);
        let image = canvas.resolve();
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| image.pixel(x, y))
            .collect()
    }

    #[test]
    fn clear_weather_paints_nothing() {
        assert!(painted(Weather::Clear, 0.0).iter().all(Option::is_none));
    }

    #[test]
    fn rain_falls_as_time_passes() {
        let start = painted(Weather::Rain, 0.0);
        assert!(start.iter().any(Option::is_some));
        assert_ne!(start, painted(Weather::Rain, 0.2));
    }
}
