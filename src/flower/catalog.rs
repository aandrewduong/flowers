//! Species designs: the shapes, proportions and palettes that make each flower recognisable.
//!
//! Sizes are fractions of `scale`, the garden's height in pixels, so flowers grow with the canvas.

use std::f32::consts::{PI, TAU};
use std::ops::Range;

use super::lavender::{Floret, LavenderSpike};
use super::radial::{Center, CenterTexture, PetalLayer, RadialHead};
use super::tulip::TulipHead;
use super::{Head, Species};
use crate::color::{Gradient, Hsl};
use crate::geometry::{GOLDEN_ANGLE, Point};
use crate::rng::Rng;
use crate::shape::Taper;

/// Average gap between seeds in a flower's centre, in pixels.
const SEED_SPACING: f32 = 1.4;
const MIN_SEEDS: usize = 8;
/// How far one flower's colour may stray from its palette entry.
const HUE_JITTER: f32 = 6.0;
const SATURATION_JITTER: f32 = 0.05;
const LIGHTNESS_JITTER: f32 = 0.04;
/// Tulips lighter than this get a dark flame at the rim; darker ones get a light one.
const FLAME_LIGHTNESS_SPLIT: f32 = 0.58;

const DAISY_PETALS: [Hsl; 4] = [
    Hsl::new(45.0, 0.3, 0.96),   // white
    Hsl::new(335.0, 0.7, 0.86),  // blush
    Hsl::new(270.0, 0.55, 0.82), // lilac
    Hsl::new(52.0, 0.9, 0.8),    // butter
];
const SUNFLOWER_PETALS: [Hsl; 3] = [
    Hsl::new(46.0, 0.95, 0.55), // gold
    Hsl::new(36.0, 0.95, 0.52), // amber
    Hsl::new(12.0, 0.8, 0.45),  // bronze
];
const ROSE_PETALS: [Hsl; 7] = [
    Hsl::new(350.0, 0.8, 0.42),  // crimson
    Hsl::new(338.0, 0.7, 0.66),  // pink
    Hsl::new(8.0, 0.85, 0.62),   // coral
    Hsl::new(45.0, 0.9, 0.62),   // yellow
    Hsl::new(38.0, 0.6, 0.88),   // cream
    Hsl::new(22.0, 0.85, 0.74),  // peach
    Hsl::new(300.0, 0.35, 0.68), // lilac
];
const DAHLIA_PETALS: [Hsl; 6] = [
    Hsl::new(322.0, 0.7, 0.55),  // magenta
    Hsl::new(24.0, 0.92, 0.58),  // tangerine
    Hsl::new(278.0, 0.5, 0.55),  // violet
    Hsl::new(356.0, 0.75, 0.62), // coral
    Hsl::new(340.0, 0.65, 0.36), // wine
    Hsl::new(345.0, 0.65, 0.8),  // blush
];
const COSMOS_PETALS: [Hsl; 5] = [
    Hsl::new(330.0, 0.7, 0.72),  // pink
    Hsl::new(318.0, 0.72, 0.5),  // magenta
    Hsl::new(300.0, 0.2, 0.95),  // white
    Hsl::new(345.0, 0.65, 0.33), // burgundy
    Hsl::new(28.0, 0.95, 0.58),  // sulphur orange
];
const POPPY_PETALS: [Hsl; 5] = [
    Hsl::new(3.0, 0.9, 0.5),     // scarlet
    Hsl::new(20.0, 0.95, 0.55),  // orange
    Hsl::new(352.0, 0.75, 0.66), // coral
    Hsl::new(48.0, 0.9, 0.62),   // Iceland yellow
    Hsl::new(300.0, 0.45, 0.5),  // plum
];
const TULIP_PETALS: [Hsl; 6] = [
    Hsl::new(356.0, 0.85, 0.5), // red
    Hsl::new(336.0, 0.7, 0.7),  // pink
    Hsl::new(47.0, 0.95, 0.6),  // yellow
    Hsl::new(22.0, 0.92, 0.56), // orange
    Hsl::new(282.0, 0.5, 0.45), // purple
    Hsl::new(48.0, 0.35, 0.92), // white
];
const TULIP_FLAME_DARK: Hsl = Hsl::new(352.0, 0.8, 0.48);
const TULIP_FLAME_LIGHT: Hsl = Hsl::new(50.0, 0.9, 0.82);
const LAVENDER_HUES: Range<f32> = 255.0..285.0;

/// Everything the garden needs to grow one plant of a species.
pub struct Blueprint {
    pub head: Head,
    /// Height of the head's anchor above the ground, as a fraction of the garden height.
    pub stem_height: f32,
    /// In pixels.
    pub stem_thickness: f32,
    pub leaves: LeafPlan,
}

/// How a species carries its leaves. Lengths are in pixels.
pub struct LeafPlan {
    pub count: usize,
    pub length: f32,
    /// Half-width at the widest point.
    pub width: f32,
    /// Where leaves sprout along the stem (0 = ground, 1 = head).
    pub attach: Range<f32>,
    /// Angle from vertical, in radians.
    pub spread: Range<f32>,
    /// How far the tip sags, in pixels.
    pub droop: f32,
}

pub fn design(species: Species, scale: f32, rng: &mut Rng) -> Blueprint {
    match species {
        Species::Daisy => daisy(scale, rng),
        Species::Sunflower => sunflower(scale, rng),
        Species::Rose => rose(scale, rng),
        Species::Dahlia => dahlia(scale, rng),
        Species::Cosmos => cosmos(scale, rng),
        Species::Poppy => poppy(scale, rng),
        Species::Tulip => tulip(scale, rng),
        Species::Lavender => lavender(scale, rng),
    }
}

fn daisy(scale: f32, rng: &mut Rng) -> Blueprint {
    let petal = pick_color(&DAISY_PETALS, rng);
    let radius = scale * rng.range(0.13..0.17);
    let count = rng.integer(16..25);
    let width = rng.range(0.11..0.15);
    let rotation = rng.range(0.0..TAU);
    let half_step = PI / count as f32;
    let layers = vec![
        layer(
            count,
            1.0,
            width,
            Taper::STRAP,
            petal_gradient(petal.shifted(0.0, 0.0, -0.08), 0.25, 0.03),
            rotation + half_step,
            rng,
        ),
        layer(
            count,
            0.94,
            width,
            Taper::STRAP,
            petal_gradient(petal, 0.2, 0.04),
            rotation,
            rng,
        ),
    ];
    let center_radius = rng.range(0.24..0.3);
    let center = Center {
        radius: center_radius,
        colors: Gradient::between(
            Hsl::new(48.0, 0.95, 0.58).to_rgb(),
            Hsl::new(36.0, 0.95, 0.45).to_rgb(),
        ),
        texture: CenterTexture::Seeds {
            count: seed_count(center_radius * radius),
            color: Hsl::new(52.0, 1.0, 0.72).to_rgb(),
        },
    };
    Blueprint {
        head: Head::Radial(RadialHead {
            radius,
            tilt: rng.range(0.7..1.0),
            layers,
            center,
        }),
        stem_height: rng.range(0.35..0.6),
        stem_thickness: scale * 0.028,
        leaves: LeafPlan {
            count: rng.integer(1..3),
            length: scale * 0.13,
            width: scale * 0.03,
            attach: 0.15..0.5,
            spread: 0.8..1.2,
            droop: scale * 0.02,
        },
    }
}

fn sunflower(scale: f32, rng: &mut Rng) -> Blueprint {
    let petal = pick_color(&SUNFLOWER_PETALS, rng);
    let radius = scale * rng.range(0.19..0.24);
    let count = rng.integer(18..26);
    let width = rng.range(0.2..0.26);
    let rotation = rng.range(0.0..TAU);
    let half_step = PI / count as f32;
    let layers = vec![
        layer(
            count,
            1.0,
            width,
            Taper::POINTED,
            petal_gradient(petal.shifted(-4.0, 0.0, -0.1), 0.2, 0.06),
            rotation + half_step,
            rng,
        ),
        layer(
            count,
            0.92,
            width,
            Taper::POINTED,
            petal_gradient(petal, 0.18, 0.1),
            rotation,
            rng,
        ),
    ];
    let center_radius = rng.range(0.42..0.5);
    let center = Center {
        radius: center_radius,
        colors: Gradient::new(vec![
            (0.0, Hsl::new(25.0, 0.6, 0.1).to_rgb()),
            (0.6, Hsl::new(26.0, 0.6, 0.17).to_rgb()),
            (0.88, Hsl::new(34.0, 0.65, 0.27).to_rgb()),
            (1.0, Hsl::new(40.0, 0.7, 0.32).to_rgb()),
        ]),
        texture: CenterTexture::Seeds {
            count: seed_count(center_radius * radius),
            color: Hsl::new(36.0, 0.55, 0.34).to_rgb(),
        },
    };
    Blueprint {
        head: Head::Radial(RadialHead {
            radius,
            tilt: rng.range(0.75..1.0),
            layers,
            center,
        }),
        stem_height: rng.range(0.62..0.82),
        stem_thickness: scale * 0.045,
        leaves: LeafPlan {
            count: rng.integer(2..4),
            length: scale * 0.2,
            width: scale * 0.06,
            attach: 0.2..0.65,
            spread: 0.9..1.3,
            droop: scale * 0.05,
        },
    }
}

fn rose(scale: f32, rng: &mut Rng) -> Blueprint {
    let petal = pick_color(&ROSE_PETALS, rng);
    let radius = scale * rng.range(0.13..0.16);
    let rotation = rng.range(0.0..TAU);
    // Each ring sits deeper in the flower, so it is smaller, darker and richer.
    let layers = (0..4)
        .map(|ring| {
            let depth = ring as f32;
            let tone = petal.shifted(0.0, 0.04 * depth, -0.06 * depth);
            layer(
                5,
                1.0 - 0.18 * depth,
                0.62,
                Taper::ROUND,
                petal_gradient(tone, 0.22, 0.08),
                rotation + depth * GOLDEN_ANGLE,
                rng,
            )
        })
        .collect();
    let center = Center {
        radius: 0.3,
        colors: Gradient::between(
            petal.shifted(0.0, 0.1, -0.3).to_rgb(),
            petal.shifted(0.0, 0.06, -0.14).to_rgb(),
        ),
        texture: CenterTexture::Swirl {
            turns: rng.range(2.0..3.0),
        },
    };
    Blueprint {
        head: Head::Radial(RadialHead {
            radius,
            tilt: rng.range(0.75..1.0),
            layers,
            center,
        }),
        stem_height: rng.range(0.38..0.65),
        stem_thickness: scale * 0.03,
        leaves: LeafPlan {
            count: rng.integer(2..4),
            length: scale * 0.12,
            width: scale * 0.04,
            attach: 0.2..0.6,
            spread: 0.9..1.3,
            droop: scale * 0.02,
        },
    }
}

fn dahlia(scale: f32, rng: &mut Rng) -> Blueprint {
    let petal = pick_color(&DAHLIA_PETALS, rng);
    let radius = scale * rng.range(0.16..0.2);
    let count = rng.integer(12..16);
    let rotation = rng.range(0.0..TAU);
    let half_step = PI / count as f32;
    // Inner rings glow a little lighter and warmer, drawing the eye to the middle.
    let layers = (0..6)
        .map(|ring| {
            let depth = ring as f32;
            let tone = petal.shifted(4.0 * depth, -0.03 * depth, 0.035 * depth);
            layer(
                count,
                1.0 - 0.14 * depth,
                0.3,
                Taper::POINTED,
                petal_gradient(tone, 0.25, 0.1),
                rotation + depth * half_step,
                rng,
            )
        })
        .collect();
    let center_radius = 0.12;
    let center = Center {
        radius: center_radius,
        colors: Gradient::between(
            Hsl::new(48.0, 0.9, 0.55).to_rgb(),
            Hsl::new(40.0, 0.9, 0.45).to_rgb(),
        ),
        texture: CenterTexture::Seeds {
            count: seed_count(center_radius * radius),
            color: Hsl::new(55.0, 1.0, 0.7).to_rgb(),
        },
    };
    Blueprint {
        head: Head::Radial(RadialHead {
            radius,
            tilt: rng.range(0.75..1.0),
            layers,
            center,
        }),
        stem_height: rng.range(0.45..0.7),
        stem_thickness: scale * 0.032,
        leaves: LeafPlan {
            count: 2,
            length: scale * 0.14,
            width: scale * 0.045,
            attach: 0.2..0.55,
            spread: 0.9..1.3,
            droop: scale * 0.03,
        },
    }
}

fn cosmos(scale: f32, rng: &mut Rng) -> Blueprint {
    let petal = pick_color(&COSMOS_PETALS, rng);
    let radius = scale * rng.range(0.14..0.17);
    let colors = Gradient::new(vec![
        (0.0, petal.shifted(-10.0, 0.1, -0.28).to_rgb()),
        (0.3, petal.to_rgb()),
        (1.0, petal.shifted(0.0, 0.0, 0.06).to_rgb()),
    ]);
    let layers = vec![PetalLayer {
        notch: rng.range(0.12..0.16),
        ..layer(
            8,
            1.0,
            rng.range(0.38..0.44),
            Taper::ROUND,
            colors,
            rng.range(0.0..TAU),
            rng,
        )
    }];
    let center_radius = 0.2;
    let center = Center {
        radius: center_radius,
        colors: Gradient::between(
            Hsl::new(50.0, 0.95, 0.55).to_rgb(),
            Hsl::new(42.0, 0.95, 0.48).to_rgb(),
        ),
        texture: CenterTexture::Seeds {
            count: seed_count(center_radius * radius),
            color: Hsl::new(30.0, 0.95, 0.42).to_rgb(),
        },
    };
    Blueprint {
        head: Head::Radial(RadialHead {
            radius,
            tilt: rng.range(0.7..1.0),
            layers,
            center,
        }),
        stem_height: rng.range(0.45..0.75),
        stem_thickness: scale * 0.024,
        leaves: LeafPlan {
            count: rng.integer(1..3),
            length: scale * 0.12,
            width: scale * 0.015,
            attach: 0.2..0.6,
            spread: 0.8..1.2,
            droop: scale * 0.02,
        },
    }
}

fn poppy(scale: f32, rng: &mut Rng) -> Blueprint {
    let petal = pick_color(&POPPY_PETALS, rng);
    let radius = scale * rng.range(0.14..0.18);
    let rotation = rng.range(0.0..TAU);
    // Poppies wear a near-black blotch at the base of every petal.
    let blotch = Hsl::new(petal.hue, 0.5, 0.08).to_rgb();
    let blotched = |tone: Hsl| {
        Gradient::new(vec![
            (0.0, blotch),
            (0.2, blotch),
            (0.34, tone.shifted(0.0, 0.0, -0.08).to_rgb()),
            (1.0, tone.shifted(0.0, 0.0, 0.06).to_rgb()),
        ])
    };
    let layers = vec![
        layer(
            4,
            1.0,
            0.68,
            Taper::ROUND,
            blotched(petal.shifted(0.0, 0.0, -0.06)),
            rotation,
            rng,
        ),
        layer(
            4,
            0.86,
            0.62,
            Taper::ROUND,
            blotched(petal),
            rotation + PI / 4.0,
            rng,
        ),
    ];
    let center = Center {
        radius: 0.2,
        colors: Gradient::new(vec![
            (0.0, Hsl::new(80.0, 0.25, 0.52).to_rgb()),
            (0.5, Hsl::new(85.0, 0.25, 0.4).to_rgb()),
            (0.68, Hsl::new(280.0, 0.3, 0.12).to_rgb()),
            (1.0, Hsl::new(270.0, 0.2, 0.06).to_rgb()),
        ]),
        texture: CenterTexture::Rays {
            count: rng.integer(8..13),
            color: Hsl::new(70.0, 0.3, 0.7).to_rgb(),
        },
    };
    Blueprint {
        head: Head::Radial(RadialHead {
            radius,
            tilt: rng.range(0.7..0.95),
            layers,
            center,
        }),
        stem_height: rng.range(0.45..0.75),
        stem_thickness: scale * 0.024,
        leaves: LeafPlan {
            count: 1,
            length: scale * 0.12,
            width: scale * 0.03,
            attach: 0.1..0.4,
            spread: 0.8..1.2,
            droop: scale * 0.02,
        },
    }
}

fn tulip(scale: f32, rng: &mut Rng) -> Blueprint {
    let petal = pick_color(&TULIP_PETALS, rng);
    let height = scale * rng.range(0.17..0.22);
    let flame = if petal.lightness > FLAME_LIGHTNESS_SPLIT {
        TULIP_FLAME_DARK
    } else {
        TULIP_FLAME_LIGHT
    };
    let head = TulipHead {
        height,
        width: height * rng.range(0.72..0.85),
        colors: Gradient::new(vec![
            (0.0, petal.shifted(0.0, 0.0, -0.2).to_rgb()),
            (0.55, petal.to_rgb()),
            (1.0, petal.shifted(0.0, 0.0, 0.08).to_rgb()),
        ]),
        edge: rng.chance(0.3).then(|| flame.to_rgb()),
    };
    Blueprint {
        head: Head::Tulip(head),
        stem_height: rng.range(0.3..0.5),
        stem_thickness: scale * 0.032,
        leaves: LeafPlan {
            count: 2,
            length: scale * 0.3,
            width: scale * 0.05,
            attach: 0.05..0.25,
            spread: 0.25..0.55,
            droop: scale * 0.04,
        },
    }
}

fn lavender(scale: f32, rng: &mut Rng) -> Blueprint {
    let hue = rng.range(LAVENDER_HUES);
    let length = scale * rng.range(0.2..0.28);
    let floret_radius = scale * 0.034;
    let whorls = ((length / (floret_radius * 1.15)).round() as usize).max(4);
    let mut florets = Vec::with_capacity(whorls * 3);
    for whorl in 0..whorls {
        let t = whorl as f32 / (whorls - 1) as f32;
        // Whorls sit further apart near the bottom of the spike and crowd toward the tip.
        let height = length * t.powf(0.8);
        let size = floret_radius * (1.0 - 0.45 * t);
        // The middle floret comes last so it sits in front of its neighbours.
        for side in [-1.0, 1.0, 0.0] {
            florets.push(Floret {
                offset: Point::new(
                    side * size * 0.9 + rng.range(-0.3..0.3),
                    height + rng.range(-0.3..0.3),
                ),
                radius: size * rng.range(0.85..1.15),
                color: Hsl::new(
                    hue + rng.range(-8.0..8.0),
                    0.55,
                    0.34 + 0.3 * t + rng.range(-0.05..0.05),
                )
                .to_rgb(),
                opens_at: t,
            });
        }
    }
    Blueprint {
        head: Head::Lavender(LavenderSpike {
            length,
            stalk: Hsl::new(100.0, 0.25, 0.4).to_rgb(),
            florets,
        }),
        stem_height: rng.range(0.35..0.55),
        stem_thickness: scale * 0.02,
        leaves: LeafPlan {
            count: rng.integer(2..4),
            length: scale * 0.14,
            width: scale * 0.012,
            attach: 0.02..0.3,
            spread: 0.3..0.6,
            droop: scale * 0.01,
        },
    }
}

fn layer(
    count: usize,
    length: f32,
    width: f32,
    taper: Taper,
    colors: Gradient,
    rotation: f32,
    rng: &mut Rng,
) -> PetalLayer {
    PetalLayer {
        count,
        length,
        width,
        taper,
        notch: 0.0,
        colors,
        rotation,
        seed: rng.next_u64(),
    }
}

/// Petal colour that deepens toward the base and brightens toward the tip.
fn petal_gradient(color: Hsl, base_depth: f32, tip_glow: f32) -> Gradient {
    Gradient::new(vec![
        (0.0, color.shifted(0.0, 0.05, -base_depth).to_rgb()),
        (0.45, color.to_rgb()),
        (1.0, color.shifted(0.0, 0.0, tip_glow).to_rgb()),
    ])
}

/// A palette entry, nudged so no two flowers are quite the same colour.
fn pick_color(palette: &[Hsl], rng: &mut Rng) -> Hsl {
    rng.pick(palette).shifted(
        rng.range(-HUE_JITTER..HUE_JITTER),
        rng.range(-SATURATION_JITTER..SATURATION_JITTER),
        rng.range(-LIGHTNESS_JITTER..LIGHTNESS_JITTER),
    )
}

/// How many seeds fit a centre disc of `radius` pixels.
fn seed_count(radius: f32) -> usize {
    ((PI * radius * radius / (SEED_SPACING * SEED_SPACING)).round() as usize).max(MIN_SEEDS)
}
