//! Lavender, seen side-on: a spike of tiny florets that opens from the bottom up.

use super::Extent;
use crate::canvas::Canvas;
use crate::color::Rgb;
use crate::geometry::{Bounds, Point, smoothstep};

/// Florets are a little narrower than they are tall.
const FLORET_ASPECT: f32 = 0.8;
const FLORET_EDGE_SHADE: f32 = 0.4;
const FLORET_HIGHLIGHT: f32 = 0.2;
/// Where on a floret the light catches, in floret radii from its centre (y up).
const FLORET_HIGHLIGHT_SPOT: Point = Point::new(-0.35, 0.35);
const FLORET_HIGHLIGHT_SPREAD: f32 = 0.9;
/// Half-width in pixels of the stalk the florets cling to.
const STALK_HALF_WIDTH: f32 = 0.6;

pub struct Floret {
    /// Pixels from the spike's base, with y pointing up.
    pub offset: Point,
    pub radius: f32,
    pub color: Rgb,
    /// Bloom progress at which this floret appears.
    pub opens_at: f32,
}

impl Floret {
    fn shade(&self, local: Point, bloom: f32) -> Option<Rgb> {
        if self.opens_at > bloom {
            return None;
        }
        let offset = local - self.offset;
        let normalized = Point::new(
            offset.x / (self.radius * FLORET_ASPECT),
            offset.y / self.radius,
        );
        let distance_squared = normalized.dot(normalized);
        if distance_squared >= 1.0 {
            return None;
        }
        let highlight = FLORET_HIGHLIGHT
            * smoothstep(
                FLORET_HIGHLIGHT_SPREAD,
                0.0,
                (normalized - FLORET_HIGHLIGHT_SPOT).length(),
            );
        Some(
            self.color
                .darken(FLORET_EDGE_SHADE * distance_squared)
                .lighten(highlight),
        )
    }
}

pub struct LavenderSpike {
    /// Pixels from the base of the spike to its tip.
    pub length: f32,
    pub stalk: Rgb,
    /// Painted in order, so later florets sit in front.
    pub florets: Vec<Floret>,
}

impl LavenderSpike {
    /// `base` is where the stem meets the spike; `lean` (radians, counter-clockwise)
    /// tips the spike to follow the stem.
    pub fn paint(&self, canvas: &mut Canvas, base: Point, lean: f32, bloom: f32) {
        let reach = self.extent().above;
        canvas.paint(Bounds::around(base, reach, reach), |point| {
            let offset = point - base;
            let local = Point::new(offset.x, -offset.y).rotate(-lean);
            self.florets
                .iter()
                .rev()
                .find_map(|floret| floret.shade(local, bloom))
                .or_else(|| self.shade_stalk(local, bloom))
        });
    }

    pub fn extent(&self) -> Extent {
        let floret_reach = |f: &Floret| f.offset.x.abs() + f.radius;
        let tallest = |f: &Floret| f.offset.y + f.radius;
        Extent {
            half_width: self.florets.iter().map(floret_reach).fold(0.0, f32::max),
            above: self.florets.iter().map(tallest).fold(self.length, f32::max),
        }
    }

    fn shade_stalk(&self, local: Point, bloom: f32) -> Option<Rgb> {
        let on_stalk =
            local.x.abs() < STALK_HALF_WIDTH && (0.0..self.length * bloom).contains(&local.y);
        on_stalk.then_some(self.stalk)
    }
}
