//! Command-line interface.

use clap::{Parser, value_parser};

use crate::ansi::ColorDepth;
use crate::flower::Species;
use crate::garden::MAX_FLOWERS;

/// Smallest garden that still fits a flower.
pub const MIN_COLUMNS: u16 = 16;
pub const MIN_ROWS: u16 = 8;

/// Grow a garden of colorful flowers in your terminal.
#[derive(Debug, Parser)]
#[command(version, about)]
pub struct Args {
    /// How many flowers to grow [default: as many as fit the width]
    #[arg(short = 'n', long, value_parser = value_parser!(u16).range(1..=i64::from(MAX_FLOWERS)))]
    pub count: Option<u16>,

    /// Only grow these species (comma-separated or repeated) [default: all]
    #[arg(short, long = "kind", value_enum, value_delimiter = ',')]
    pub kinds: Vec<Species>,

    /// Seed for the garden; reuse one to regrow a garden you liked
    #[arg(short, long)]
    pub seed: Option<u64>,

    /// Width in terminal columns [default: terminal width]
    #[arg(short = 'W', long, value_parser = value_parser!(u16).range(i64::from(MIN_COLUMNS)..))]
    pub width: Option<u16>,

    /// Height in terminal rows [default: 24, or less to fit the terminal]
    #[arg(short = 'H', long, value_parser = value_parser!(u16).range(i64::from(MIN_ROWS)..))]
    pub height: Option<u16>,

    /// Watch the garden sprout and bloom
    #[arg(short, long)]
    pub bloom: bool,

    /// Browse gardens full-screen with the keyboard
    #[arg(short, long, conflicts_with_all = ["width", "height"])]
    pub interactive: bool,

    /// Leave out the grass
    #[arg(long)]
    pub no_grass: bool,

    /// Colors the terminal supports
    #[arg(long, value_enum, default_value_t = ColorDepth::Truecolor)]
    pub colors: ColorDepth,
}
