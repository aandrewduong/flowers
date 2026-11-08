//! Flowers seen face-on: rings of petals around a centre disc.

use std::f32::consts::{PI, TAU};

use super::Extent;
use crate::canvas::Canvas;
use crate::color::{Gradient, Rgb};
use crate::geometry::{Bounds, GOLDEN_ANGLE, Point, smoothstep};
use crate::rng::hash_unit;
use crate::shape::Taper;

/// How far individual petals stray from their layer's length and tone.
const PETAL_LENGTH_JITTER: f32 = 0.08;
const PETAL_TONE_JITTER: f32 = 0.06;
/// Darkening toward a petal's outline, so overlapping petals read as separate.
const PETAL_RIM_SHADE: f32 = 0.35;
/// Shadow where a petal tucks under the layers above it, and how far up it reaches.
const PETAL_BASE_SHADE: f32 = 0.35;
const PETAL_BASE_SHADOW_REACH: f32 = 0.35;
/// Faint lengthwise streaks across each petal.
const PETAL_VEINS: f32 = 3.0;
const PETAL_VEIN_SHADE: f32 = 0.08;
/// Shadow cast by each layer onto the one beneath, and how far past its outline it falls
/// (as a fraction of the distance from the centre).
const LAYER_SHADOW: f32 = 0.3;
const LAYER_SHADOW_REACH: f32 = 0.1;
/// Brightening toward the upper left, where the light comes from.
const HEAD_LIGHTING: f32 = 0.1;

/// Seed size relative to the gap between seeds.
const SEED_SIZE: f32 = 0.75;
/// Sharpness and reach of the ridges on a poppy's seed pod.
const RAY_SHARPNESS: i32 = 8;
const RAY_REACH: f32 = 0.85;
/// Darkness and width of the creases between a rosebud's furled petals.
const SWIRL_CREASE_SHADE: f32 = 0.45;
const SWIRL_CREASE_WIDTH: f32 = 0.2;

pub struct RadialHead {
    /// Pixels from the centre to the petal tips.
    pub radius: f32,
    /// Vertical squash: 1 faces the viewer, smaller values tip the face upward.
    pub tilt: f32,
    /// Outermost layer first; each later layer sits on top of the ones before.
    pub layers: Vec<PetalLayer>,
    pub center: Center,
}

impl RadialHead {
    pub fn paint(&self, canvas: &mut Canvas, center: Point, bloom: f32) {
        let radius = self.radius * bloom;
        let reach = radius * (1.0 + PETAL_LENGTH_JITTER);
        canvas.paint(Bounds::around(center, reach, reach * self.tilt), |point| {
            let offset = point - center;
            self.shade(Point::new(offset.x, offset.y / self.tilt) / radius)
        });
    }

    pub fn extent(&self) -> Extent {
        let reach = self.radius * (1.0 + PETAL_LENGTH_JITTER);
        Extent {
            half_width: reach,
            above: reach * self.tilt,
        }
    }

    /// `local` is in head radii from the centre, with the tilt already undone.
    fn shade(&self, local: Point) -> Option<Rgb> {
        let distance = local.length();
        let angle = local.y.atan2(local.x);
        let color = self
            .center
            .shade(local, distance, angle)
            .or_else(|| self.shade_petals(distance, angle))?;
        Some(color.scale(1.0 - HEAD_LIGHTING * (local.x + local.y)))
    }

    fn shade_petals(&self, distance: f32, angle: f32) -> Option<Rgb> {
        let (index, petal) = self
            .layers
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, layer)| {
                layer
                    .locate(distance, angle)
                    .map(|petal| (index, layer.shade(petal)))
            })?;
        // Whatever sits on top casts a soft shadow just beyond its outline.
        let nudged_inward = distance * (1.0 - LAYER_SHADOW_REACH);
        let shadowed = nudged_inward < self.center.radius
            || self.layers[index + 1..]
                .iter()
                .any(|upper| upper.locate(nudged_inward, angle).is_some());
        Some(if shadowed {
            petal.darken(LAYER_SHADOW)
        } else {
            petal
        })
    }
}

pub struct PetalLayer {
    pub count: usize,
    /// Tip distance from the centre, as a fraction of the head radius.
    pub length: f32,
    /// Widest half-width, as a fraction of the petal's length.
    pub width: f32,
    pub taper: Taper,
    /// Depth of the serrations cut into each tip; 0 for a smooth tip.
    pub notch: f32,
    /// Colour from petal base (0) to tip (1).
    pub colors: Gradient,
    pub rotation: f32,
    /// Varies the petals within the layer.
    pub seed: u64,
}

/// Where a point falls on the petal that covers it.
#[derive(Clone, Copy)]
struct PetalSpot {
    /// 0 at the petal's base, 1 at its tip.
    along: f32,
    /// -1 to 1 from one edge of the petal to the other.
    side: f32,
    /// This petal's individual variation, in [-1, 1].
    jitter: f32,
}

impl PetalLayer {
    /// Finds the petal covering a point at `distance` head radii and `angle`, if any.
    fn locate(&self, distance: f32, angle: f32) -> Option<PetalSpot> {
        // Work in the frame of the nearest petal: `along` its axis and `across` it.
        let spacing = TAU / self.count as f32;
        let turns = ((angle - self.rotation) / spacing).round();
        let offset = angle - self.rotation - turns * spacing;
        let petal = (turns as i64).rem_euclid(self.count as i64) as u64;
        let jitter = hash_unit(self.seed.wrapping_add(petal)) * 2.0 - 1.0;

        let length = self.length * (1.0 + PETAL_LENGTH_JITTER * jitter);
        let along = distance * offset.cos() / length;
        let across = distance * offset.sin() / length;
        let notched_along = along / (1.0 - self.notch * tip_serration(across / self.width));
        let half_width = self.width * self.taper.width_at(notched_along);
        if half_width <= 0.0 || across.abs() >= half_width {
            return None;
        }
        Some(PetalSpot {
            along,
            side: across / half_width,
            jitter,
        })
    }

    fn shade(&self, spot: PetalSpot) -> Rgb {
        let veins = 0.5 - 0.5 * (spot.side * PETAL_VEINS * PI).cos();
        let shade = PETAL_RIM_SHADE * spot.side.abs().powi(4)
            + PETAL_BASE_SHADE * smoothstep(PETAL_BASE_SHADOW_REACH, 0.0, spot.along)
            + PETAL_VEIN_SHADE * veins;
        self.colors
            .at(spot.along)
            .scale(1.0 + PETAL_TONE_JITTER * spot.jitter)
            .darken(shade)
    }
}

/// Three-pointed scallop across a petal tip: 0 at the points, 1 in the notches.
fn tip_serration(across: f32) -> f32 {
    (across * 1.5 * PI).sin().abs()
}

pub struct Center {
    /// Fraction of the head radius.
    pub radius: f32,
    /// Colour from the middle (0) out to the rim (1).
    pub colors: Gradient,
    pub texture: CenterTexture,
}

pub enum CenterTexture {
    /// Florets packed in the spiral a sunflower uses.
    Seeds { count: usize, color: Rgb },
    /// Ridges radiating from the middle, like a poppy's seed pod.
    Rays { count: usize, color: Rgb },
    /// Petals furled into a tight spiral, like the heart of a rose.
    Swirl { turns: f32 },
}

impl Center {
    fn shade(&self, local: Point, distance: f32, angle: f32) -> Option<Rgb> {
        if distance >= self.radius {
            return None;
        }
        let t = distance / self.radius;
        let base = self.colors.at(t);
        let color = match self.texture {
            CenterTexture::Seeds { count, color } => {
                base.mix(color, seed_highlight(local / self.radius, count))
            }
            CenterTexture::Rays { count, color } => {
                let ridge = (angle * count as f32 / 2.0).cos().abs().powi(RAY_SHARPNESS);
                base.mix(color, ridge * smoothstep(RAY_REACH, 0.0, t))
            }
            CenterTexture::Swirl { turns } => {
                let phase = (angle / TAU + t * turns).rem_euclid(1.0);
                let crease = smoothstep(SWIRL_CREASE_WIDTH, 0.0, phase.min(1.0 - phase));
                base.darken(SWIRL_CREASE_SHADE * crease)
            }
        };
        Some(color)
    }
}

/// How strongly `point` (in the unit disc) sits on one of `count` seeds laid out in
/// Vogel's sunflower spiral: 1 on a seed's centre, fading to 0 at its edge.
fn seed_highlight(point: Point, count: usize) -> f32 {
    let seed_radius = SEED_SIZE / (count as f32).sqrt();
    let nearest = (0..count)
        .map(|i| {
            let seed_distance = ((i as f32 + 0.5) / count as f32).sqrt();
            let seed = Point::from_angle(i as f32 * GOLDEN_ANGLE) * seed_distance;
            (seed - point).length()
        })
        .fold(f32::INFINITY, f32::min);
    smoothstep(seed_radius, 0.0, nearest)
}
