//! Butterflies and bees that hop from bloom to bloom by day, and fireflies after dark.

use std::f32::consts::{PI, TAU};
use std::ops::Range;

use crate::canvas::Canvas;
use crate::color::{Gradient, Hsl, Rgb};
use crate::geometry::{Bounds, Point, smoothstep};
use crate::rng::Rng;

/// A garden draws one butterfly or bee for every few flowers, up to a limit.
const FLOWERS_PER_FLIER: usize = 3;
const MAX_FLIERS: usize = 8;
const BEE_SHARE: f32 = 0.4;
/// Blooms on the round a flier keeps making.
const ROUTE_STOPS: usize = 4;
/// Share of each leg of the round spent settled on a bloom rather than in the air.
const REST_SHARE: f32 = 0.35;
/// How high a flier arcs between blooms: a fraction of the gap, plus a few body sizes so
/// that even a hop back onto the same bloom lifts off.
const HOP_ARC: f32 = 0.25;
const HOP_LIFT: f32 = 1.5;

/// A butterfly's wing length as a fraction of the garden height, within limits in pixels.
const BUTTERFLY_SIZE: f32 = 0.07;
const BUTTERFLY_SIZE_LIMITS: Range<f32> = 2.5..6.0;
/// Half a bee's body length next to a butterfly's wing.
const BEE_SCALE: f32 = 0.6;

const BUTTERFLY_HABITS: Habits = Habits {
    leg_seconds: 4.0..6.5,
    bob_rate: 2.2,
    bob_reach: 0.3,
};
const BEE_HABITS: Habits = Habits {
    leg_seconds: 2.2..3.4,
    bob_rate: 9.0,
    bob_reach: 0.25,
};

const BUTTERFLY_BODY: Rgb = Rgb::new(0.16, 0.1, 0.08);
const BODY_HALF_WIDTH: f32 = 0.12;
const BODY_HALF_LENGTH: f32 = 0.55;
/// Wingbeats per second, and how narrow the wings look when folded mid-beat.
const WINGBEAT_RATE: f32 = 2.6;
const FOLDED_WINGS: f32 = 0.3;
/// One side's wings, in wing lengths from the body with y pointing down. The other side
/// mirrors them.
const WINGS: [Lobe; 2] = [
    Lobe {
        center: Point::new(0.55, -0.22),
        radii: Point::new(0.5, 0.42),
    },
    Lobe {
        center: Point::new(0.42, 0.3),
        radii: Point::new(0.36, 0.3),
    },
];
const WING_COLORS: [Hsl; 5] = [
    Hsl::new(28.0, 0.95, 0.55),
    Hsl::new(210.0, 0.85, 0.6),
    Hsl::new(52.0, 0.95, 0.62),
    Hsl::new(60.0, 0.3, 0.92),
    Hsl::new(325.0, 0.8, 0.68),
];
/// How much darker a wing's rim is than the rest, and where the darkening starts.
const WING_RIM_SHADE: f32 = 0.55;
const WING_RIM_START: f32 = 0.7;

const BEE_YELLOW: Rgb = Rgb::new(0.98, 0.78, 0.12);
const BEE_DARK: Rgb = Rgb::new(0.14, 0.1, 0.06);
const BEE_WING_COLOR: Rgb = Rgb::new(0.88, 0.93, 1.0);
const BEE_BANDS: f32 = 3.0;
/// A bee's body is this much shorter top to bottom than nose to tail.
const BEE_BODY_ASPECT: f32 = 0.65;
/// The blur of wings above a bee's back, in body lengths.
const BEE_WINGS: Lobe = Lobe {
    center: Point::new(-0.1, -0.8),
    radii: Point::new(0.6, 0.45),
};
const BEE_WING_FLICKER_RATE: f32 = 11.0;
const BEE_WING_FLICKER: f32 = 0.35;

/// Fireflies per pixel of garden, within limits.
const FIREFLIES_PER_PIXEL: f32 = 0.0025;
const FIREFLY_LIMITS: Range<usize> = 3..40;
/// Fireflies keep to the lower part of the garden, among the flowers.
const FIREFLY_CEILING: f32 = 0.3;
/// How far a firefly wanders from home, as a fraction of the garden height.
const FIREFLY_ROAM: Range<f32> = 0.03..0.1;
/// Seconds to drift out and back, and to fade in and out.
const FIREFLY_DRIFT_PERIOD: Range<f32> = 5.0..11.0;
const FIREFLY_BLINK_PERIOD: Range<f32> = 2.5..5.0;
/// Up and down takes a different time than side to side, so paths don't retrace.
const FIREFLY_VERTICAL_PACE: f32 = 1.7;
const FIREFLY_COLOR: Rgb = Rgb::new(0.86, 1.0, 0.42);
/// Radius of a firefly's bright core as a fraction of the garden height, and its minimum
/// in pixels.
const FIREFLY_CORE: f32 = 0.018;
const MIN_FIREFLY_CORE: f32 = 0.7;
/// Radius of the glow around the core, in core radii, and its opacity next to the core.
const FIREFLY_HALO: f32 = 3.5;
const FIREFLY_HALO_OPACITY: f32 = 0.45;

/// Which visitors are out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hours {
    Daylight,
    AfterDark,
}

pub struct Visitors {
    fliers: Vec<Flier>,
    fireflies: Vec<Firefly>,
}

impl Visitors {
    /// `perches` are where on each flower a visitor can settle, in a garden of
    /// `width` × `height` pixels.
    pub fn plan(perches: &[Point], width: f32, height: f32, rng: &mut Rng) -> Self {
        let fliers = if perches.is_empty() {
            Vec::new()
        } else {
            let count = (perches.len() / FLOWERS_PER_FLIER).clamp(1, MAX_FLIERS);
            (0..count)
                .map(|_| Flier::plan(perches, height, rng))
                .collect()
        };
        let fireflies = ((width * height * FIREFLIES_PER_PIXEL) as usize)
            .clamp(FIREFLY_LIMITS.start, FIREFLY_LIMITS.end);
        Self {
            fliers,
            fireflies: (0..fireflies)
                .map(|_| Firefly::plan(width, height, rng))
                .collect(),
        }
    }

    /// Paints whoever is out during `hours` where they've got to at `time` (seconds).
    /// `presence` (0–1) scales them, so they can arrive gradually.
    pub fn paint(&self, canvas: &mut Canvas, hours: Hours, time: f32, presence: f32) {
        if presence <= 0.0 {
            return;
        }
        match hours {
            Hours::Daylight => {
                for flier in &self.fliers {
                    flier.paint(canvas, time, presence);
                }
            }
            Hours::AfterDark => {
                for firefly in &self.fireflies {
                    firefly.paint(canvas, time, presence);
                }
            }
        }
    }
}

/// How one kind of flier moves.
struct Habits {
    /// Seconds from one bloom to the next, including the rest there.
    leg_seconds: Range<f32>,
    /// How fast it bobs about its path, in radians per second, and how far in body sizes.
    bob_rate: f32,
    bob_reach: f32,
}

enum Look {
    /// Wing colour from the body (0) out to the rim (1).
    Butterfly {
        wings: Gradient,
    },
    Bee,
}

impl Look {
    fn habits(&self) -> &'static Habits {
        match self {
            Self::Butterfly { .. } => &BUTTERFLY_HABITS,
            Self::Bee => &BEE_HABITS,
        }
    }
}

/// A butterfly or bee making its round of the blooms.
struct Flier {
    look: Look,
    /// Blooms visited in turn, forever.
    route: Vec<Point>,
    leg_seconds: f32,
    /// How many legs into its round the flier starts, so they don't all move as one.
    head_start: f32,
    /// Wing length of a butterfly or half the body length of a bee, in pixels.
    size: f32,
    /// Sets this flier's bobbing and wingbeats apart from the others'.
    phase: f32,
}

impl Flier {
    fn plan(perches: &[Point], garden_height: f32, rng: &mut Rng) -> Self {
        let butterfly_size = (garden_height * BUTTERFLY_SIZE)
            .clamp(BUTTERFLY_SIZE_LIMITS.start, BUTTERFLY_SIZE_LIMITS.end);
        let (look, size) = if rng.chance(BEE_SHARE) {
            (Look::Bee, butterfly_size * BEE_SCALE)
        } else {
            let wings = butterfly_wings(*rng.pick(&WING_COLORS));
            (Look::Butterfly { wings }, butterfly_size)
        };
        Self {
            route: (0..ROUTE_STOPS).map(|_| *rng.pick(perches)).collect(),
            leg_seconds: rng.range(look.habits().leg_seconds.clone()),
            head_start: rng.range(0.0..ROUTE_STOPS as f32),
            size,
            phase: rng.range(0.0..TAU),
            look,
        }
    }

    fn position(&self, time: f32) -> Point {
        let journey = time / self.leg_seconds + self.head_start;
        let leg = journey as usize;
        let from = self.route[leg % self.route.len()];
        let to = self.route[(leg + 1) % self.route.len()];
        // Easing in and out of each leg is what makes the flier linger on its blooms.
        let travelled = smoothstep(0.5 * REST_SHARE, 1.0 - 0.5 * REST_SHARE, journey.fract());
        let arc = HOP_ARC * (to - from).length() + HOP_LIFT * self.size;
        let habits = self.look.habits();
        let bob = Point::new(
            (habits.bob_rate * time + self.phase).sin(),
            (habits.bob_rate * 1.5 * time + self.phase).cos(),
        ) * (habits.bob_reach * self.size);
        from.lerp(to, travelled) - Point::new(0.0, arc * (PI * travelled).sin()) + bob
    }

    fn paint(&self, canvas: &mut Canvas, time: f32, presence: f32) {
        let center = self.position(time);
        let size = self.size * presence;
        match &self.look {
            Look::Butterfly { wings } => {
                let beat = (WINGBEAT_RATE * TAU * time + self.phase).sin().abs();
                let spread = FOLDED_WINGS + (1.0 - FOLDED_WINGS) * beat;
                paint_butterfly(canvas, center, size, wings, spread);
            }
            Look::Bee => {
                let flicker = (BEE_WING_FLICKER_RATE * TAU * time + self.phase)
                    .sin()
                    .abs();
                paint_bee(canvas, center, size, 1.0 - BEE_WING_FLICKER * flicker);
            }
        }
    }
}

fn butterfly_wings(color: Hsl) -> Gradient {
    Gradient::new(vec![
        (0.0, color.shifted(8.0, 0.0, 0.12).to_rgb()),
        (WING_RIM_START, color.to_rgb()),
        (1.0, color.to_rgb().darken(WING_RIM_SHADE)),
    ])
}

/// An ellipse that makes up part of a visitor's outline.
struct Lobe {
    center: Point,
    radii: Point,
}

impl Lobe {
    /// 0 at the lobe's centre rising to 1 at its outline, or `None` outside it.
    fn depth(&self, point: Point) -> Option<f32> {
        let offset = point - self.center;
        let scaled = Point::new(offset.x / self.radii.x, offset.y / self.radii.y);
        let depth = scaled.dot(scaled);
        (depth < 1.0).then_some(depth)
    }
}

/// `size` is the wing length in pixels; `spread` (0–1) is how far open the wings are.
fn paint_butterfly(canvas: &mut Canvas, center: Point, size: f32, wings: &Gradient, spread: f32) {
    let reach = WINGS
        .iter()
        .map(|wing| wing.center + wing.radii)
        .fold(Point::new(0.0, BODY_HALF_LENGTH), |reach, corner| {
            Point::new(reach.x.max(corner.x), reach.y.max(corner.y))
        });
    let bounds = Bounds::around(center, reach.x * size, reach.y * size);
    canvas.paint(bounds, |point| {
        let local = (point - center) / size;
        let side = local.x.abs();
        if side < BODY_HALF_WIDTH && local.y.abs() < BODY_HALF_LENGTH {
            return Some(BUTTERFLY_BODY);
        }
        // Wings fold toward the body, so undo the fold to find where on the open wing
        // this point lies.
        let open = Point::new(side / spread, local.y);
        WINGS
            .iter()
            .find_map(|wing| wing.depth(open))
            .map(|depth| wings.at(depth))
    });
}

/// `size` is half the body length in pixels; `wing_size` (0–1) shrinks the wings as they beat.
fn paint_bee(canvas: &mut Canvas, center: Point, size: f32, wing_size: f32) {
    let body = Lobe {
        center: Point::new(0.0, 0.0),
        radii: Point::new(1.0, BEE_BODY_ASPECT),
    };
    let wings = Lobe {
        center: BEE_WINGS.center,
        radii: BEE_WINGS.radii * wing_size,
    };
    let top = BEE_WINGS.center.y.abs() + BEE_WINGS.radii.y;
    canvas.paint(Bounds::around(center, size, top * size), |point| {
        let local = (point - center) / size;
        if body.depth(local).is_some() {
            let band = ((local.x + 1.0) * 0.5 * BEE_BANDS) as usize;
            return Some(if band.is_multiple_of(2) {
                BEE_YELLOW
            } else {
                BEE_DARK
            });
        }
        wings.depth(local).map(|_| BEE_WING_COLOR)
    });
}

/// A point of light that drifts around its home and fades in and out.
struct Firefly {
    home: Point,
    /// How far it drifts each way, in pixels.
    roam: Point,
    drift_period: f32,
    blink_period: f32,
    phase: f32,
    /// Radius of its bright core, in pixels.
    core: f32,
}

impl Firefly {
    fn plan(width: f32, height: f32, rng: &mut Rng) -> Self {
        Self {
            home: Point::new(
                rng.range(0.0..width),
                rng.range(height * FIREFLY_CEILING..height),
            ),
            roam: Point::new(rng.range(FIREFLY_ROAM), rng.range(FIREFLY_ROAM)) * height,
            drift_period: rng.range(FIREFLY_DRIFT_PERIOD),
            blink_period: rng.range(FIREFLY_BLINK_PERIOD),
            phase: rng.range(0.0..TAU),
            core: (height * FIREFLY_CORE).max(MIN_FIREFLY_CORE),
        }
    }

    fn paint(&self, canvas: &mut Canvas, time: f32, presence: f32) {
        let pulse = (TAU * time / self.blink_period + self.phase).sin();
        // Dark for a good part of each cycle, like the real thing.
        let glow = smoothstep(0.0, 0.7, pulse);
        if glow <= 0.0 {
            return;
        }
        let drift = TAU * time / self.drift_period + self.phase;
        let center = self.home
            + Point::new(
                self.roam.x * drift.sin(),
                self.roam.y * (FIREFLY_VERTICAL_PACE * drift).sin(),
            );
        let core = self.core * presence;
        let halo = core * FIREFLY_HALO;
        canvas.blend(Bounds::around(center, halo, halo), |point| {
            let distance = (point - center).length();
            if distance < core {
                return Some((FIREFLY_COLOR, glow));
            }
            let fade = smoothstep(halo, core, distance);
            (fade > 0.0).then_some((FIREFLY_COLOR, glow * FIREFLY_HALO_OPACITY * fade * fade))
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: usize = 60;
    const HEIGHT: usize = 40;

    fn visitors(perches: &[Point]) -> Visitors {
        Visitors::plan(perches, WIDTH as f32, HEIGHT as f32, &mut Rng::new(5))
    }

    fn painted(visitors: &Visitors, hours: Hours, time: f32, presence: f32) -> Vec<Option<Rgb>> {
        let mut canvas = Canvas::new(WIDTH, HEIGHT, 2);
        visitors.paint(&mut canvas, hours, time, presence);
        let image = canvas.resolve();
        (0..HEIGHT)
            .flat_map(|y| (0..WIDTH).map(move |x| (x, y)))
            .map(|(x, y)| image.pixel(x, y))
            .collect()
    }

    fn blooms() -> Vec<Point> {
        vec![
            Point::new(10.0, 14.0),
            Point::new(30.0, 20.0),
            Point::new(50.0, 12.0),
        ]
    }

    #[test]
    fn fliers_rest_on_blooms_between_hops() {
        let perches = blooms();
        let visitors = visitors(&perches);
        for flier in &visitors.fliers {
            let reach = flier.look.habits().bob_reach * flier.size * 1.5;
            for leg in 0..2 * ROUTE_STOPS {
                // A whole number of legs into the round, the flier is mid-rest.
                let time =
                    (leg as f32 + flier.head_start.ceil() - flier.head_start) * flier.leg_seconds;
                let position = flier.position(time);
                assert!(
                    perches
                        .iter()
                        .any(|&perch| (position - perch).length() <= reach),
                    "{position:?} is not on a bloom"
                );
            }
        }
    }

    #[test]
    fn a_lone_flower_still_gets_a_visitor() {
        let visitors = visitors(&[Point::new(30.0, 20.0)]);
        assert_eq!(visitors.fliers.len(), 1);
        let pixels = painted(&visitors, Hours::Daylight, 0.0, 1.0);
        assert!(pixels.iter().any(Option::is_some));
    }

    #[test]
    fn fliers_move_as_time_passes() {
        let visitors = visitors(&blooms());
        assert_ne!(
            painted(&visitors, Hours::Daylight, 0.0, 1.0),
            painted(&visitors, Hours::Daylight, 1.0, 1.0)
        );
    }

    #[test]
    fn nobody_shows_before_they_arrive() {
        let visitors = visitors(&blooms());
        for hours in [Hours::Daylight, Hours::AfterDark] {
            assert!(
                painted(&visitors, hours, 0.0, 0.0)
                    .iter()
                    .all(Option::is_none)
            );
        }
    }

    #[test]
    fn fireflies_light_up_after_dark() {
        let visitors = visitors(&blooms());
        let lit = |time| {
            painted(&visitors, Hours::AfterDark, time, 1.0)
                .iter()
                .flatten()
                .count()
        };
        assert!((0..20).any(|step| lit(step as f32 * 0.5) > 0));
    }
}
