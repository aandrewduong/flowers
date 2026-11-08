//! Stems, leaves and grass.

use std::ops::Range;

use crate::canvas::Canvas;
use crate::color::{Gradient, Hsl};
use crate::geometry::{Bounds, Point, smoothstep};
use crate::rng::Rng;
use crate::shape::Taper;

/// Polyline resolution used to approximate a stem's curve.
const STEM_SEGMENTS: usize = 24;
/// Stems are this much thinner at the head than at the ground.
const STEM_TAPER: f32 = 0.3;
/// Anything much under a pixel wide drops below the canvas's coverage threshold and
/// breaks up, so stems never taper thinner than this.
const MIN_STEM_HALF_WIDTH: f32 = 0.6;
/// The underside of a leaf, folded away from the light, is a little darker.
const LEAF_FOLD_SHADE: f32 = 0.15;
const LEAF_EDGE_SHADE: f32 = 0.2;
/// A pale midrib runs down the middle of every leaf.
const LEAF_MIDRIB_WIDTH: f32 = 0.18;
const LEAF_MIDRIB_GLOW: f32 = 0.12;
/// Grass blades are darker at their edges than along their spine.
const GRASS_EDGE_SHADE: f32 = 0.2;

const FOLIAGE_HUES: Range<f32> = 95.0..128.0;
const FOLIAGE_SATURATION: Range<f32> = 0.4..0.58;
const FOLIAGE_LIGHTNESS: Range<f32> = 0.27..0.35;

/// A fresh green for one plant's stem and leaves.
pub fn foliage_green(rng: &mut Rng) -> Hsl {
    Hsl::new(
        rng.range(FOLIAGE_HUES),
        rng.range(FOLIAGE_SATURATION),
        rng.range(FOLIAGE_LIGHTNESS),
    )
}

/// A gently curved stem: a quadratic Bézier from the ground up to the head.
pub struct Stem {
    pub base: Point,
    pub control: Point,
    pub tip: Point,
    /// Pixels across at the base.
    pub thickness: f32,
    /// Colour across the stem, from its lit left side (0) to its shaded right side (1).
    pub colors: Gradient,
}

impl Stem {
    pub fn point_at(&self, t: f32) -> Point {
        let inverse = 1.0 - t;
        self.base * (inverse * inverse) + self.control * (2.0 * inverse * t) + self.tip * (t * t)
    }

    /// Radians from vertical at `t`, counter-clockwise, as side-on heads expect.
    pub fn lean_at(&self, t: f32) -> f32 {
        let direction =
            (self.control - self.base) * (2.0 * (1.0 - t)) + (self.tip - self.control) * (2.0 * t);
        (-direction.x).atan2(-direction.y)
    }

    /// Paints the stem from the ground up to `growth` (0–1) of its length.
    pub fn paint(&self, canvas: &mut Canvas, growth: f32) {
        if growth <= 0.0 {
            return;
        }
        let points: Vec<Point> = (0..=STEM_SEGMENTS)
            .map(|i| self.point_at(growth * i as f32 / STEM_SEGMENTS as f32))
            .collect();
        canvas.paint(Bounds::enclosing(&points).expand(self.thickness), |point| {
            let (distance, along) = nearest_on_polyline(&points, point);
            let half_width = (0.5 * self.thickness * (1.0 - STEM_TAPER * along * growth))
                .max(MIN_STEM_HALF_WIDTH);
            (distance.abs() < half_width).then(|| self.colors.at(0.5 + 0.5 * distance / half_width))
        });
    }
}

/// Signed distance from `point` to the polyline (positive on its right-hand side, seen
/// walking from first point to last on screen) and how far along it, 0–1, the nearest point lies.
fn nearest_on_polyline(points: &[Point], point: Point) -> (f32, f32) {
    let segments = (points.len() - 1) as f32;
    let mut nearest = (f32::INFINITY, 0.0, 0.0);
    for (index, pair) in points.windows(2).enumerate() {
        let (start, end) = (pair[0], pair[1]);
        let edge = end - start;
        let edge_length_squared = edge.dot(edge);
        let t = if edge_length_squared > 0.0 {
            ((point - start).dot(edge) / edge_length_squared).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let offset = point - (start + edge * t);
        let distance = offset.length();
        if distance < nearest.0 {
            let signed = distance.copysign(edge.cross(offset));
            nearest = (distance, signed, (index as f32 + t) / segments);
        }
    }
    (nearest.1, nearest.2)
}

pub struct Leaf {
    pub anchor: Point,
    /// Direction from anchor to tip, in screen radians (y points down).
    pub angle: f32,
    pub length: f32,
    /// Half-width at the widest point.
    pub width: f32,
    /// Sideways sag of the tip, in pixels; its sign picks which side it sags toward.
    pub droop: f32,
    /// Colour from base (0) to tip (1).
    pub colors: Gradient,
}

impl Leaf {
    /// Paints the leaf unfurled to `growth` (0–1) of its full size.
    pub fn paint(&self, canvas: &mut Canvas, growth: f32) {
        let length = self.length * growth;
        if length <= 0.0 {
            return;
        }
        let width = self.width * growth;
        let droop = self.droop * growth;
        let tip = self.anchor + Point::from_angle(self.angle) * length;
        let bounds = Bounds::enclosing(&[self.anchor, tip]).expand(width + droop.abs());
        canvas.paint(bounds, |point| {
            let local = (point - self.anchor).rotate(-self.angle);
            let along = local.x / length;
            let half_width = width * Taper::LANCE.width_at(along);
            if half_width <= 0.0 {
                return None;
            }
            let across = (local.y - droop * along * along) / half_width;
            if across.abs() >= 1.0 {
                return None;
            }
            let underside = across * self.droop.signum() > 0.0;
            let shade =
                if underside { LEAF_FOLD_SHADE } else { 0.0 } + LEAF_EDGE_SHADE * across.powi(4);
            let midrib = LEAF_MIDRIB_GLOW * smoothstep(LEAF_MIDRIB_WIDTH, 0.0, across.abs());
            Some(self.colors.at(along).darken(shade).lighten(midrib))
        });
    }
}

pub struct GrassBlade {
    pub base: Point,
    pub height: f32,
    /// Horizontal offset of the tip from the base, in pixels.
    pub sway: f32,
    /// Half-width at the base.
    pub width: f32,
    /// Colour from base (0) to tip (1).
    pub colors: Gradient,
}

impl GrassBlade {
    pub fn paint(&self, canvas: &mut Canvas, growth: f32) {
        let height = self.height * growth;
        if height <= 0.0 {
            return;
        }
        let sway = self.sway * growth;
        let tip = self.base + Point::new(sway, -height);
        canvas.paint(
            Bounds::enclosing(&[self.base, tip]).expand(self.width),
            |point| {
                let along = (self.base.y - point.y) / height;
                if !(0.0..1.0).contains(&along) {
                    return None;
                }
                let spine = self.base.x + sway * along * along;
                let half_width = self.width * (1.0 - along);
                let across = (point.x - spine) / half_width;
                (across.abs() < 1.0).then(|| {
                    self.colors
                        .at(along)
                        .darken(GRASS_EDGE_SHADE * across.abs())
                })
            },
        );
    }
}
