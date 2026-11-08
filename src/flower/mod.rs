//! Flower heads, and the species that choose their shapes and colours.

mod catalog;
mod lavender;
mod radial;
mod tulip;

pub use catalog::{Blueprint, LeafPlan, design};

use crate::canvas::Canvas;
use crate::geometry::Point;
use lavender::LavenderSpike;
use radial::RadialHead;
use tulip::TulipHead;

#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Species {
    /// White, blush or lilac rays around a golden eye
    Daisy,
    /// Big golden heads with spiralled seed centres
    Sunflower,
    /// Layered petals furled around a swirling heart
    Rose,
    /// Ring upon ring of pointed petals
    Dahlia,
    /// Eight notched petals on a slender stem
    Cosmos,
    /// Silky cups with a dark blotched eye
    Poppy,
    /// Side-on cups, sometimes flamed at the rim
    Tulip,
    /// Purple spikes of tiny florets
    Lavender,
}

impl Species {
    pub const ALL: [Self; 8] = [
        Self::Daisy,
        Self::Sunflower,
        Self::Rose,
        Self::Dahlia,
        Self::Cosmos,
        Self::Poppy,
        Self::Tulip,
        Self::Lavender,
    ];

    /// The name used on the command line.
    pub fn name(self) -> &'static str {
        match self {
            Self::Daisy => "daisy",
            Self::Sunflower => "sunflower",
            Self::Rose => "rose",
            Self::Dahlia => "dahlia",
            Self::Cosmos => "cosmos",
            Self::Poppy => "poppy",
            Self::Tulip => "tulip",
            Self::Lavender => "lavender",
        }
    }
}

/// How far a head reaches from where it meets its stem, in pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Extent {
    pub half_width: f32,
    pub above: f32,
}

pub enum Head {
    Radial(RadialHead),
    Tulip(TulipHead),
    Lavender(LavenderSpike),
}

impl Head {
    /// `anchor` is where the stem meets the head. `lean` (radians, counter-clockwise)
    /// tips side-on heads to follow their stem. `bloom` runs from 0 (bud) to 1 (open).
    pub fn paint(&self, canvas: &mut Canvas, anchor: Point, lean: f32, bloom: f32) {
        if bloom <= 0.0 {
            return;
        }
        match self {
            Self::Radial(head) => head.paint(canvas, anchor, bloom),
            Self::Tulip(head) => head.paint(canvas, anchor, lean, bloom),
            Self::Lavender(head) => head.paint(canvas, anchor, lean, bloom),
        }
    }

    pub fn extent(&self) -> Extent {
        match self {
            Self::Radial(head) => head.extent(),
            Self::Tulip(head) => head.extent(),
            Self::Lavender(head) => head.extent(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::ValueEnum;

    #[test]
    fn names_match_the_command_line() {
        for species in Species::ALL {
            let value = species.to_possible_value().expect("no species is hidden");
            assert_eq!(species.name(), value.get_name());
        }
    }

    #[test]
    fn all_lists_every_species() {
        assert_eq!(Species::ALL.as_slice(), Species::value_variants());
    }
}
