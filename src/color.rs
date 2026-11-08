//! Colours and colour ramps. Channels are kept as floats in [0, 1] until output.

use std::ops::Add;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgb {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Rgb {
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0);
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0);

    pub const fn new(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b }
    }

    pub fn mix(self, other: Self, t: f32) -> Self {
        Self::new(
            self.r + (other.r - self.r) * t,
            self.g + (other.g - self.g) * t,
            self.b + (other.b - self.b) * t,
        )
    }

    pub fn lighten(self, amount: f32) -> Self {
        self.mix(Self::WHITE, amount)
    }

    pub fn darken(self, amount: f32) -> Self {
        self.mix(Self::BLACK, amount)
    }

    /// Multiplies every channel, e.g. to apply lighting or average a sum of samples.
    pub fn scale(self, factor: f32) -> Self {
        Self::new(self.r * factor, self.g * factor, self.b * factor)
    }

    pub fn to_bytes(self) -> [u8; 3] {
        [self.r, self.g, self.b].map(|channel| (channel.clamp(0.0, 1.0) * 255.0).round() as u8)
    }
}

impl Add for Rgb {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.r + other.r, self.g + other.g, self.b + other.b)
    }
}

/// Hue (degrees), saturation and lightness: the easiest space to design palettes in,
/// because "the same flower, a bit darker" is a single nudge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hsl {
    pub hue: f32,
    pub saturation: f32,
    pub lightness: f32,
}

impl Hsl {
    pub const fn new(hue: f32, saturation: f32, lightness: f32) -> Self {
        Self {
            hue,
            saturation,
            lightness,
        }
    }

    pub fn shifted(self, hue: f32, saturation: f32, lightness: f32) -> Self {
        Self::new(
            self.hue + hue,
            (self.saturation + saturation).clamp(0.0, 1.0),
            (self.lightness + lightness).clamp(0.0, 1.0),
        )
    }

    pub fn to_rgb(self) -> Rgb {
        let sector = self.hue.rem_euclid(360.0) / 60.0;
        let chroma = (1.0 - (2.0 * self.lightness - 1.0).abs()) * self.saturation;
        let secondary = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
        let (r, g, b) = match sector as u32 {
            0 => (chroma, secondary, 0.0),
            1 => (secondary, chroma, 0.0),
            2 => (0.0, chroma, secondary),
            3 => (0.0, secondary, chroma),
            4 => (secondary, 0.0, chroma),
            _ => (chroma, 0.0, secondary),
        };
        let lift = self.lightness - chroma / 2.0;
        Rgb::new(r + lift, g + lift, b + lift)
    }
}

/// Piecewise-linear colour ramp over [0, 1].
#[derive(Clone, Debug)]
pub struct Gradient {
    stops: Vec<(f32, Rgb)>,
}

impl Gradient {
    /// `stops` pair a position in [0, 1] with a colour, sorted by position.
    pub fn new(stops: Vec<(f32, Rgb)>) -> Self {
        assert!(!stops.is_empty(), "a gradient needs at least one colour");
        debug_assert!(
            stops.windows(2).all(|pair| pair[0].0 <= pair[1].0),
            "gradient stops must be sorted by position"
        );
        Self { stops }
    }

    pub fn between(start: Rgb, end: Rgb) -> Self {
        Self::new(vec![(0.0, start), (1.0, end)])
    }

    pub fn at(&self, t: f32) -> Rgb {
        let (first_position, first_color) = self.stops[0];
        if t <= first_position {
            return first_color;
        }
        for pair in self.stops.windows(2) {
            let ((start, from), (end, to)) = (pair[0], pair[1]);
            if t <= end {
                let span = end - start;
                let local = if span > 0.0 { (t - start) / span } else { 1.0 };
                return from.mix(to, local);
            }
        }
        self.stops[self.stops.len() - 1].1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: Rgb, expected: Rgb) {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        assert!(
            close(actual.r, expected.r)
                && close(actual.g, expected.g)
                && close(actual.b, expected.b),
            "{actual:?} != {expected:?}"
        );
    }

    #[test]
    fn hsl_primaries() {
        assert_close(Hsl::new(0.0, 1.0, 0.5).to_rgb(), Rgb::new(1.0, 0.0, 0.0));
        assert_close(Hsl::new(120.0, 1.0, 0.5).to_rgb(), Rgb::new(0.0, 1.0, 0.0));
        assert_close(Hsl::new(240.0, 1.0, 0.5).to_rgb(), Rgb::new(0.0, 0.0, 1.0));
        assert_close(Hsl::new(-120.0, 1.0, 0.5).to_rgb(), Rgb::new(0.0, 0.0, 1.0));
    }

    #[test]
    fn hsl_extremes_of_lightness() {
        assert_close(Hsl::new(200.0, 0.8, 0.0).to_rgb(), Rgb::BLACK);
        assert_close(Hsl::new(200.0, 0.8, 1.0).to_rgb(), Rgb::WHITE);
    }

    #[test]
    fn gradient_interpolates_between_stops() {
        let gradient = Gradient::new(vec![
            (0.0, Rgb::BLACK),
            (0.5, Rgb::new(1.0, 0.0, 0.0)),
            (1.0, Rgb::WHITE),
        ]);
        assert_close(gradient.at(-1.0), Rgb::BLACK);
        assert_close(gradient.at(0.25), Rgb::new(0.5, 0.0, 0.0));
        assert_close(gradient.at(0.75), Rgb::new(1.0, 0.5, 0.5));
        assert_close(gradient.at(2.0), Rgb::WHITE);
    }

    #[test]
    fn bytes_round_and_clamp() {
        assert_eq!(Rgb::new(1.5, 0.5, -0.2).to_bytes(), [255, 128, 0]);
    }
}
