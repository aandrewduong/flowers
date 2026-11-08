//! Tulips, seen side-on: a cup of three overlapping petals.

use super::Extent;
use crate::canvas::Canvas;
use crate::color::{Gradient, Rgb};
use crate::geometry::{Bounds, Point, smoothstep};
use crate::shape::Taper;

/// Darkening toward each petal's sides, so the cup reads as rounded.
const CUP_CURVE_SHADE: f32 = 0.4;
/// The two rear petals sit in the front petal's shadow.
const BACK_PETAL_SHADE: f32 = 0.22;
/// A soft vertical highlight on the side facing the light.
const HIGHLIGHT_STRENGTH: f32 = 0.18;
const HIGHLIGHT_POSITION: f32 = -0.35;
const HIGHLIGHT_SPREAD: f32 = 0.25;
/// Where a flamed variety's rim colour starts taking over, measured toward the outline.
const FLAME_START: f32 = 0.7;
/// The side petals flare outward, so the head is a little wider than its cup.
const HEAD_REACH: f32 = 0.65;

/// Where one of the three visible petals sits, relative to the whole cup.
struct PetalPlacement {
    /// Sideways shift of the petal's base, as a fraction of the cup width.
    offset: f32,
    /// Radians the petal leans outward.
    lean: f32,
    /// Fraction of the cup's width and height the petal spans.
    width: f32,
    height: f32,
    shade: f32,
}

/// Front petal first: it hides the two behind it.
const PETALS: [PetalPlacement; 3] = [
    PetalPlacement {
        offset: 0.0,
        lean: 0.0,
        width: 0.78,
        height: 0.94,
        shade: 0.0,
    },
    PetalPlacement {
        offset: -0.2,
        lean: 0.17,
        width: 0.62,
        height: 1.0,
        shade: BACK_PETAL_SHADE,
    },
    PetalPlacement {
        offset: 0.2,
        lean: -0.17,
        width: 0.62,
        height: 1.0,
        shade: BACK_PETAL_SHADE,
    },
];

pub struct TulipHead {
    /// Pixels from the base of the cup to the petal tips.
    pub height: f32,
    /// Pixels across the widest part of the cup.
    pub width: f32,
    /// Colour from the base of each petal (0) to its tip (1).
    pub colors: Gradient,
    /// Contrasting rim colour of "flamed" varieties.
    pub edge: Option<Rgb>,
}

impl TulipHead {
    /// `base` is where the stem meets the cup; `lean` (radians, counter-clockwise)
    /// tips the head to follow the stem.
    pub fn paint(&self, canvas: &mut Canvas, base: Point, lean: f32, bloom: f32) {
        let height = self.height * bloom;
        let width = self.width * bloom;
        let reach = height.max(width);
        canvas.paint(Bounds::around(base, reach, reach), |point| {
            let offset = point - base;
            let local = Point::new(offset.x, -offset.y).rotate(-lean);
            PETALS
                .iter()
                .find_map(|petal| self.shade_petal(petal, local, width, height))
        });
    }

    pub fn extent(&self) -> Extent {
        Extent {
            half_width: self.width * HEAD_REACH,
            above: self.height,
        }
    }

    /// `local` has its origin at the cup's base and y pointing up.
    fn shade_petal(
        &self,
        petal: &PetalPlacement,
        local: Point,
        width: f32,
        height: f32,
    ) -> Option<Rgb> {
        let petal_local = (local - Point::new(petal.offset * width, 0.0)).rotate(-petal.lean);
        let along = petal_local.y / (height * petal.height);
        let half_width = 0.5 * width * petal.width * Taper::CUP.width_at(along);
        if half_width <= 0.0 || petal_local.x.abs() >= half_width {
            return None;
        }

        let across = petal_local.x / half_width;
        let mut color = self.colors.at(along);
        if let Some(edge) = self.edge {
            let toward_rim = across.abs().max(along.powi(3));
            color = color.mix(edge, smoothstep(FLAME_START, 1.0, toward_rim));
        }
        let highlight =
            HIGHLIGHT_STRENGTH * (-(across - HIGHLIGHT_POSITION).powi(2) / HIGHLIGHT_SPREAD).exp();
        Some(
            color
                .darken(CUP_CURVE_SHADE * across * across + petal.shade)
                .lighten(highlight),
        )
    }
}
