//! Arranges plants across the canvas and grows them over time.

use std::f32::consts::FRAC_PI_2;
use std::ops::Range;

use crate::canvas::Canvas;
use crate::color::{Gradient, Hsl};
use crate::flower::{self, Blueprint, Head, LeafPlan, Species};
use crate::foliage::{GrassBlade, Leaf, Stem, foliage_green};
use crate::geometry::{Point, smoothstep};
use crate::rng::Rng;

/// Gap between flowers, as a fraction of the garden height, when no count is given.
const NATURAL_SPACING: f32 = 0.3;
/// How far a flower may wander from the middle of its slot, as a fraction of a slot.
const SLOT_JITTER: f32 = 0.35;
/// Flowers with room to spare grow up to this much larger than in a crowded bed.
const MAX_ROOMINESS: f32 = 1.6;
/// Sideways drift of a stem's base and bow of its middle, as fractions of its length.
const STEM_LEAN: f32 = 0.12;
const STEM_BEND: f32 = 0.1;
/// Pixels kept clear between a flower head and the top edge.
const TOP_MARGIN: f32 = 1.0;
/// How much each leaf may differ from its species' typical length.
const LEAF_SIZE_JITTER: Range<f32> = 0.85..1.15;

const GRASS_BLADES_PER_PIXEL: f32 = 1.3;
const GRASS_HEIGHT: Range<f32> = 0.04..0.13;
const GRASS_SWAY: Range<f32> = -0.5..0.5;
const GRASS_WIDTH: Range<f32> = 0.5..0.9;

/// Plants start sprouting at staggered moments within this opening share of the animation.
const SPROUT_STAGGER: f32 = 0.3;
/// Phases of each plant's own growth, as fractions of its time line.
const STEM_GROWTH_END: f32 = 0.55;
const BLOOM_PHASE: Range<f32> = 0.45..1.0;
/// Share of the stem's growth over which a leaf unfurls once the stem reaches it.
const LEAF_UNFURL: f32 = 0.35;
const GRASS_GROWTH_END: f32 = 0.35;

/// Most flowers a single garden will hold.
pub const MAX_FLOWERS: u16 = 500;

/// What to grow, independent of the space it's grown in.
#[derive(Clone, Debug, PartialEq)]
pub struct Planting {
    /// `None` grows as many as comfortably fit.
    pub count: Option<usize>,
    /// Species to choose from; must not be empty.
    pub species: Vec<Species>,
    pub grass: bool,
}

impl Planting {
    /// How many flowers this planting puts in a garden of `width` × `height` pixels.
    pub fn count_for(&self, width: usize, height: usize) -> usize {
        self.count.unwrap_or_else(|| {
            ((width as f32 / (height as f32 * NATURAL_SPACING)).round() as usize).max(1)
        })
    }
}

/// The ground a garden is planted in, in pixels.
#[derive(Clone, Copy)]
struct Bed {
    width: f32,
    height: f32,
}

pub struct Garden {
    /// Back to front.
    plants: Vec<Plant>,
    grass: Vec<GrassBlade>,
}

impl Garden {
    /// Lays out `planting` in a garden of `width` × `height` pixels.
    pub fn plan(planting: &Planting, width: usize, height: usize, rng: &mut Rng) -> Self {
        let count = planting.count_for(width, height);
        let bed = Bed {
            width: width as f32,
            height: height as f32,
        };
        let slot = bed.width / count as f32;
        let natural_slot = bed.height * NATURAL_SPACING;
        let roominess = (slot / natural_slot).clamp(1.0, MAX_ROOMINESS);
        // Wide slots don't mean more wandering, or a lone flower could end up at the edge.
        let wander = SLOT_JITTER * slot.min(natural_slot);
        let mut plants: Vec<Plant> = (0..count)
            .map(|index| {
                let species = *rng.pick(&planting.species);
                let x = (index as f32 + 0.5) * slot + rng.range(-wander..wander);
                Plant::grow(species, x, bed.height * roominess, bed, rng)
            })
            .collect();
        // Taller plants stand further back, so they're painted first.
        plants.sort_by(|a, b| a.stem.tip.y.total_cmp(&b.stem.tip.y));
        let grass = if planting.grass {
            plan_grass(bed, rng)
        } else {
            Vec::new()
        };
        Self { plants, grass }
    }

    pub fn flower_count(&self) -> usize {
        self.plants.len()
    }

    /// Paints the garden `progress` (0–1) of the way from bare soil to full bloom.
    pub fn paint(&self, canvas: &mut Canvas, progress: f32) {
        for plant in &self.plants {
            plant.paint(canvas, progress);
        }
        let grass_growth = smoothstep(0.0, GRASS_GROWTH_END, progress);
        for blade in &self.grass {
            blade.paint(canvas, grass_growth);
        }
    }
}

struct Plant {
    stem: Stem,
    leaves: Vec<StemLeaf>,
    head: Head,
    sprouts_at: f32,
}

/// A leaf, and where along its stem (0 = ground, 1 = head) it sprouts.
struct StemLeaf {
    leaf: Leaf,
    along: f32,
}

impl Plant {
    /// `scale` sizes the flower itself; it's at least the garden height.
    fn grow(species: Species, x: f32, scale: f32, bed: Bed, rng: &mut Rng) -> Self {
        let Bed { width, height } = bed;
        let Blueprint {
            head,
            stem_height,
            stem_thickness,
            leaves,
        } = flower::design(species, scale, rng);

        // Keep the head on the canvas, even if that means a shorter stem.
        let extent = head.extent();
        let tip = Point::new(
            x.clamp(
                extent.half_width,
                (width - extent.half_width).max(extent.half_width),
            ),
            (height * (1.0 - stem_height)).max(extent.above + TOP_MARGIN),
        );
        let stem_length = height - tip.y;
        let base = Point::new(
            tip.x + rng.range(-STEM_LEAN..STEM_LEAN) * stem_length,
            height,
        );
        let bow = Point::new(rng.range(-STEM_BEND..STEM_BEND) * stem_length, 0.0);
        let green = foliage_green(rng);
        let stem = Stem {
            base,
            control: base.lerp(tip, 0.5) + bow,
            tip,
            thickness: stem_thickness,
            colors: Gradient::new(vec![
                (0.0, green.shifted(0.0, 0.0, 0.08).to_rgb()),
                (0.5, green.to_rgb()),
                (1.0, green.shifted(0.0, 0.0, -0.1).to_rgb()),
            ]),
        };
        let leaves = sprout_leaves(&stem, &leaves, green, rng);
        Self {
            stem,
            leaves,
            head,
            sprouts_at: rng.range(0.0..SPROUT_STAGGER),
        }
    }

    fn paint(&self, canvas: &mut Canvas, progress: f32) {
        let age = ((progress - self.sprouts_at) / (1.0 - SPROUT_STAGGER)).clamp(0.0, 1.0);
        let stem_growth = smoothstep(0.0, STEM_GROWTH_END, age);
        self.stem.paint(canvas, stem_growth);
        for StemLeaf { leaf, along } in &self.leaves {
            let unfurled = (along + LEAF_UNFURL).min(1.0);
            leaf.paint(canvas, smoothstep(*along, unfurled, stem_growth));
        }
        self.head.paint(
            canvas,
            self.stem.point_at(stem_growth),
            self.stem.lean_at(stem_growth),
            smoothstep(BLOOM_PHASE.start, BLOOM_PHASE.end, age),
        );
    }
}

fn sprout_leaves(stem: &Stem, plan: &LeafPlan, green: Hsl, rng: &mut Rng) -> Vec<StemLeaf> {
    let first_side = rng.sign();
    let colors = Gradient::between(
        green.shifted(0.0, 0.0, -0.08).to_rgb(),
        green.shifted(-6.0, 0.0, 0.1).to_rgb(),
    );
    (0..plan.count)
        .map(|index| {
            let side = if index % 2 == 0 {
                first_side
            } else {
                -first_side
            };
            let along = rng.range(plan.attach.clone());
            let size = rng.range(LEAF_SIZE_JITTER);
            let leaf = Leaf {
                anchor: stem.point_at(along),
                // Straight up, then tipped toward this leaf's side.
                angle: -FRAC_PI_2 + side * rng.range(plan.spread.clone()),
                length: plan.length * size,
                width: plan.width * size,
                droop: plan.droop * side,
                colors: colors.clone(),
            };
            StemLeaf { leaf, along }
        })
        .collect()
}

fn plan_grass(bed: Bed, rng: &mut Rng) -> Vec<GrassBlade> {
    let Bed { width, height } = bed;
    let count = (width * GRASS_BLADES_PER_PIXEL).round() as usize;
    (0..count)
        .map(|_| {
            let green = foliage_green(rng);
            let blade_height = height * rng.range(GRASS_HEIGHT);
            GrassBlade {
                base: Point::new(rng.range(0.0..width), height),
                height: blade_height,
                sway: blade_height * rng.range(GRASS_SWAY),
                width: rng.range(GRASS_WIDTH),
                colors: Gradient::between(
                    green.shifted(0.0, 0.0, -0.1).to_rgb(),
                    green.shifted(-5.0, 0.0, 0.12).to_rgb(),
                ),
            }
        })
        .collect()
}
