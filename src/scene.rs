//! A garden in its surroundings: the sky behind it, the wind through it, and whatever is
//! flying or falling in front of it.

use crate::canvas::Canvas;
use crate::garden::{Garden, Planting};
use crate::geometry::smoothstep;
use crate::rng::Rng;
use crate::sky::Sky;
use crate::visitors::{Hours, Visitors};
use crate::weather::Weather;
use crate::wind::Wind;

/// Visitors start turning up once the garden is this far into bloom.
const VISITORS_ARRIVE: f32 = 0.85;

/// What surrounds the garden, independent of what's planted in it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Atmosphere {
    /// `None` leaves the terminal's own background showing.
    pub sky: Option<Sky>,
    pub weather: Weather,
    pub wind: bool,
    pub visitors: bool,
}

impl Atmosphere {
    /// Whether anything keeps moving once the garden has grown.
    pub fn is_lively(&self) -> bool {
        self.wind
            || self.visitors
            || self.weather.is_falling()
            || self.sky.is_some_and(Sky::is_dark)
    }

    fn hours(&self) -> Hours {
        if self.sky.is_some_and(Sky::is_dark) {
            Hours::AfterDark
        } else {
            Hours::Daylight
        }
    }
}

/// The instant a scene is painted at.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Moment {
    /// How far the garden has grown, from bare soil (0) to full bloom (1).
    pub progress: f32,
    /// Seconds on the clock that wind, weather and visitors move to.
    pub time: f32,
}

impl Moment {
    /// The garden in full bloom, with the clock at zero.
    pub const BLOOMED: Self = Self {
        progress: 1.0,
        time: 0.0,
    };
}

pub struct Scene {
    garden: Garden,
    visitors: Visitors,
}

impl Scene {
    /// Lays out `planting` and its visitors in `width` × `height` pixels.
    pub fn plan(planting: &Planting, width: usize, height: usize, rng: &mut Rng) -> Self {
        let garden = Garden::plan(planting, width, height, rng);
        // Planned after the garden so that the same seed grows the same flowers whether
        // or not anyone comes to visit them.
        let visitors = Visitors::plan(&garden.perches(), width as f32, height as f32, rng);
        Self { garden, visitors }
    }

    pub fn flower_count(&self) -> usize {
        self.garden.flower_count()
    }

    pub fn paint(&self, canvas: &mut Canvas, atmosphere: &Atmosphere, moment: Moment) {
        let Moment { progress, time } = moment;
        if let Some(sky) = atmosphere.sky {
            sky.paint(canvas, atmosphere.weather.is_overcast(), time);
        }
        let wind = if atmosphere.wind {
            Wind::Blowing { time }
        } else {
            Wind::Calm
        };
        self.garden.paint(canvas, progress, &wind);
        if atmosphere.visitors {
            let presence = smoothstep(VISITORS_ARRIVE, 1.0, progress);
            self.visitors
                .paint(canvas, atmosphere.hours(), time, presence);
        }
        atmosphere.weather.paint(canvas, time);
    }
}
