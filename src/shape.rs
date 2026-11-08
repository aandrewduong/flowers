//! Outline profiles shared by petals, leaves and tulip cups.

/// How wide a petal or leaf is along its length.
///
/// The half-width at `u` (0 = base, 1 = tip) is `u^rise * (1 - u)^fall`, normalised so the
/// widest point is 1. Exponents below 1 round an end off; near or above 1 they draw it to a point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Taper {
    pub rise: f32,
    pub fall: f32,
}

impl Taper {
    /// Long and even with rounded ends, like a daisy ray.
    pub const STRAP: Self = Self::new(0.3, 0.3);
    /// Widest past the middle with a broad, round tip, like a rose or poppy petal.
    pub const ROUND: Self = Self::new(0.9, 0.45);
    /// Lens-shaped with a sharp tip, like sunflower and dahlia petals.
    pub const POINTED: Self = Self::new(0.7, 0.9);
    /// Bulging low and drawn up to a point, like a tulip petal.
    pub const CUP: Self = Self::new(0.4, 0.6);
    /// Broad near the base, narrowing to a long point, like a leaf.
    pub const LANCE: Self = Self::new(0.55, 0.9);

    pub const fn new(rise: f32, fall: f32) -> Self {
        Self { rise, fall }
    }

    pub fn width_at(self, u: f32) -> f32 {
        if !(0.0..=1.0).contains(&u) {
            return 0.0;
        }
        let peak = self.rise / (self.rise + self.fall);
        let widest = peak.powf(self.rise) * (1.0 - peak).powf(self.fall);
        u.powf(self.rise) * (1.0 - u).powf(self.fall) / widest
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPES: [Taper; 5] = [
        Taper::STRAP,
        Taper::ROUND,
        Taper::POINTED,
        Taper::CUP,
        Taper::LANCE,
    ];

    #[test]
    fn widest_point_is_one() {
        for taper in SHAPES {
            let peak = taper.rise / (taper.rise + taper.fall);
            assert!((taper.width_at(peak) - 1.0).abs() < 1e-5, "{taper:?}");
        }
    }

    #[test]
    fn closes_at_both_ends_and_outside() {
        for taper in SHAPES {
            assert_eq!(taper.width_at(0.0), 0.0);
            assert_eq!(taper.width_at(1.0), 0.0);
            assert_eq!(taper.width_at(-0.1), 0.0);
            assert_eq!(taper.width_at(1.1), 0.0);
        }
    }
}
