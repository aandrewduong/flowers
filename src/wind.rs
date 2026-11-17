//! A breeze that rolls across the garden in gusts.

use std::f32::consts::TAU;

/// The breeze leans everything this way even between gusts; positive is rightward.
const PREVAILING: f32 = 0.3;
/// Slow swells that travel across the garden, and the quicker flutter riding on them.
/// Together with `PREVAILING` the weights add up to 1, the hardest the wind ever pushes.
const SWELL: Ripple = Ripple {
    weight: 0.5,
    period: 3.6,
    wavelength: 70.0,
};
const FLUTTER: Ripple = Ripple {
    weight: 0.2,
    period: 1.3,
    wavelength: 26.0,
};

/// A wave of moving air: it repeats every `period` seconds at any one spot and every
/// `wavelength` pixels across the garden.
struct Ripple {
    weight: f32,
    period: f32,
    wavelength: f32,
}

impl Ripple {
    fn at(&self, x: f32, time: f32) -> f32 {
        self.weight * (TAU * (time / self.period - x / self.wavelength)).sin()
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Wind {
    Calm,
    /// `time` is in seconds; the gusts move as it advances.
    Blowing {
        time: f32,
    },
}

impl Wind {
    /// How hard the wind pushes sideways at horizontal position `x` (pixels), from -1
    /// (leftward) to 1 (rightward). Neighbouring plants feel nearly the same push, so
    /// gusts read as waves crossing the garden.
    pub fn push_at(&self, x: f32) -> f32 {
        match *self {
            Self::Calm => 0.0,
            Self::Blowing { time } => PREVAILING + SWELL.at(x, time) + FLUTTER.at(x, time),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calm_air_pushes_nothing() {
        assert_eq!(Wind::Calm.push_at(12.0), 0.0);
    }

    #[test]
    fn push_never_exceeds_full_strength() {
        for step in 0..2_000 {
            let wind = Wind::Blowing {
                time: step as f32 * 0.05,
            };
            let push = wind.push_at(step as f32 * 0.37);
            assert!((-1.0..=1.0).contains(&push), "{push} at step {step}");
        }
    }

    #[test]
    fn gusts_change_over_time_and_across_the_garden() {
        let now = Wind::Blowing { time: 1.0 };
        let later = Wind::Blowing { time: 2.0 };
        assert_ne!(now.push_at(10.0), later.push_at(10.0));
        assert_ne!(now.push_at(10.0), now.push_at(40.0));
    }
}
