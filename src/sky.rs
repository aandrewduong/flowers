//! The sky behind the garden: a wash of colour for the time of day, with a sun or moon
//! and, after dark, stars.

use std::f32::consts::TAU;
use std::ops::Range;

use crate::canvas::Canvas;
use crate::color::{Gradient, Rgb};
use crate::geometry::{Bounds, Point, smoothstep};
use crate::rng::Rng;

/// How far cloud cover pulls the sky toward grey, and how much it dims that grey.
const OVERCAST: f32 = 0.6;
const OVERCAST_DIM: f32 = 0.85;

/// The stars never move, so every garden shares one sky.
const STAR_SEED: u64 = 0x5747_4152;
const STARS_PER_PIXEL: f32 = 0.012;
/// Stars keep to this upper share of the sky; lower down the horizon's haze hides them.
const STAR_FIELD: f32 = 0.8;
const STAR_COLOR: Rgb = Rgb::new(1.0, 0.98, 0.9);
const STAR_BRIGHTNESS: Range<f32> = 0.3..1.0;
/// Seconds a star takes to dim and brighten again, and how much of its light it loses.
const TWINKLE_PERIOD: Range<f32> = 1.5..4.0;
const TWINKLE_DEPTH: f32 = 0.6;

/// How far a sun or moon's glow reaches past its disc, in disc radii.
const GLOW_REACH: f32 = 1.6;
/// Suns and moons never shrink below this many pixels across, or they'd break up.
const MIN_ORB_RADIUS: f32 = 1.5;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Sky {
    /// Peach and gold under a pale blue morning
    Dawn,
    /// Clear blue with the sun high
    Day,
    /// Indigo sinking into orange
    Dusk,
    /// Deep blue with stars and a crescent moon
    Night,
}

impl Sky {
    pub const ALL: [Self; 4] = [Self::Dawn, Self::Day, Self::Dusk, Self::Night];

    /// The name used on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Self::Dawn => "dawn",
            Self::Day => "day",
            Self::Dusk => "dusk",
            Self::Night => "night",
        }
    }

    /// The sky that follows this one as the day wears on, if any.
    pub fn later(self) -> Option<Self> {
        let position = Self::ALL.iter().position(|&sky| sky == self)?;
        Self::ALL.get(position + 1).copied()
    }

    pub fn is_dark(self) -> bool {
        self == Self::Night
    }

    /// Fills the whole canvas. `overcast` greys the sky and hides everything in it;
    /// `time` (seconds) makes the stars twinkle.
    pub fn paint(self, canvas: &mut Canvas, overcast: bool, time: f32) {
        let height = canvas.bounds().max.y;
        let colors = self.colors();
        canvas.paint(canvas.bounds(), |point| {
            let color = colors.at(point.y / height);
            Some(if overcast { clouded(color) } else { color })
        });
        if overcast {
            return;
        }
        if self.is_dark() {
            paint_stars(canvas, &colors, time);
        }
        self.orb().paint(canvas);
    }

    /// Colour from the top of the sky (0) down to the horizon (1).
    fn colors(self) -> Gradient {
        let stops = match self {
            Self::Dawn => vec![
                (0.0, Rgb::new(0.36, 0.47, 0.74)),
                (0.55, Rgb::new(0.86, 0.66, 0.72)),
                (1.0, Rgb::new(1.0, 0.83, 0.62)),
            ],
            Self::Day => vec![
                (0.0, Rgb::new(0.22, 0.5, 0.88)),
                (1.0, Rgb::new(0.66, 0.84, 0.97)),
            ],
            Self::Dusk => vec![
                (0.0, Rgb::new(0.13, 0.11, 0.33)),
                (0.5, Rgb::new(0.5, 0.22, 0.47)),
                (0.8, Rgb::new(0.9, 0.42, 0.33)),
                (1.0, Rgb::new(0.99, 0.66, 0.33)),
            ],
            Self::Night => vec![
                (0.0, Rgb::new(0.02, 0.03, 0.09)),
                (1.0, Rgb::new(0.08, 0.11, 0.26)),
            ],
        };
        Gradient::new(stops)
    }

    fn orb(self) -> Orb {
        match self {
            Self::Dawn => Orb {
                center: Point::new(0.2, 0.78),
                radius: 0.07,
                color: Rgb::new(1.0, 0.93, 0.72),
                glow: 0.5,
                shadow: None,
            },
            Self::Day => Orb {
                center: Point::new(0.82, 0.16),
                radius: 0.06,
                color: Rgb::new(1.0, 0.97, 0.8),
                glow: 0.45,
                shadow: None,
            },
            Self::Dusk => Orb {
                center: Point::new(0.78, 0.86),
                radius: 0.08,
                color: Rgb::new(1.0, 0.75, 0.42),
                glow: 0.5,
                shadow: None,
            },
            Self::Night => Orb {
                center: Point::new(0.82, 0.18),
                radius: 0.07,
                color: Rgb::new(0.95, 0.95, 0.86),
                glow: 0.25,
                shadow: Some(Point::new(0.5, -0.2)),
            },
        }
    }
}

fn clouded(color: Rgb) -> Rgb {
    let grey = (color.r + color.g + color.b) / 3.0 * OVERCAST_DIM;
    color.mix(Rgb::new(grey, grey, grey), OVERCAST)
}

/// A sun or moon.
struct Orb {
    /// Position as fractions of the sky's width and height.
    center: Point,
    /// Fraction of the sky's height.
    radius: f32,
    color: Rgb,
    /// Opacity of the glow where it meets the disc.
    glow: f32,
    /// Where the disc of shadow that carves a moon into a crescent sits, in radii from
    /// the centre; `None` for a full disc.
    shadow: Option<Point>,
}

impl Orb {
    fn paint(&self, canvas: &mut Canvas) {
        let sky = canvas.bounds().max;
        let center = Point::new(self.center.x * sky.x, self.center.y * sky.y);
        let radius = (self.radius * sky.y).max(MIN_ORB_RADIUS);
        let reach = radius * (1.0 + GLOW_REACH);
        canvas.blend(Bounds::around(center, reach, reach), |point| {
            let offset = (point - center) / radius;
            let distance = offset.length();
            if distance >= 1.0 {
                let fade = smoothstep(1.0 + GLOW_REACH, 1.0, distance);
                return Some((self.color, self.glow * fade * fade));
            }
            let lit = self
                .shadow
                .is_none_or(|shadow| (offset - shadow).length() >= 1.0);
            lit.then_some((self.color, 1.0))
        });
    }
}

fn paint_stars(canvas: &mut Canvas, sky_colors: &Gradient, time: f32) {
    let sky = canvas.bounds().max;
    // Reseeding every frame lays out the same stars each time.
    let mut rng = Rng::new(STAR_SEED);
    let count = (sky.x * sky.y * STARS_PER_PIXEL) as usize;
    for _ in 0..count {
        // Centred on a pixel, so each star lights exactly one.
        let center = Point::new(
            rng.range(0.0..sky.x).floor() + 0.5,
            rng.range(0.0..sky.y * STAR_FIELD).floor() + 0.5,
        );
        let brightness = rng.range(STAR_BRIGHTNESS);
        let period = rng.range(TWINKLE_PERIOD);
        let dimming = 0.5 + 0.5 * (TAU * (time / period + rng.unit())).sin();
        let shine = brightness * (1.0 - TWINKLE_DEPTH * dimming);
        let color = sky_colors.at(center.y / sky.y).mix(STAR_COLOR, shine);
        canvas.paint(Bounds::around(center, 0.5, 0.5), |_| Some(color));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::ValueEnum;

    fn painted(sky: Sky, overcast: bool, time: f32) -> Vec<Option<Rgb>> {
        let (width, height) = (40, 20);
        let mut canvas = Canvas::new(width, height, 2);
        sky.paint(&mut canvas, overcast, time);
        let image = canvas.resolve();
        (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .map(|(x, y)| image.pixel(x, y))
            .collect()
    }

    #[test]
    fn names_match_the_command_line() {
        assert_eq!(Sky::ALL.as_slice(), Sky::value_variants());
        for sky in Sky::ALL {
            let value = sky.to_possible_value().expect("no sky is hidden");
            assert_eq!(sky.name(), value.get_name());
        }
    }

    #[test]
    fn every_sky_covers_the_whole_canvas() {
        for sky in Sky::ALL {
            for overcast in [false, true] {
                assert!(
                    painted(sky, overcast, 0.0).iter().all(Option::is_some),
                    "{sky:?} left gaps"
                );
            }
        }
    }

    #[test]
    fn the_day_wears_on_from_dawn_to_night() {
        assert_eq!(Sky::Dawn.later(), Some(Sky::Day));
        assert_eq!(Sky::Dusk.later(), Some(Sky::Night));
        assert_eq!(Sky::Night.later(), None);
    }

    #[test]
    fn only_the_night_sky_changes_with_time() {
        assert_ne!(
            painted(Sky::Night, false, 0.0),
            painted(Sky::Night, false, 1.0)
        );
        assert_eq!(painted(Sky::Day, false, 0.0), painted(Sky::Day, false, 1.0));
    }

    #[test]
    fn clouds_hide_the_stars() {
        assert_eq!(
            painted(Sky::Night, true, 0.0),
            painted(Sky::Night, true, 1.0)
        );
    }
}
