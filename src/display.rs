//! Writes gardens to the terminal, either finished or as a growing animation.

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::ansi::{self, ColorDepth};
use crate::canvas::Canvas;
use crate::garden::Garden;

/// Samples per pixel along each axis; 3 gives smooth edges at little cost.
const SUPERSAMPLING: usize = 3;
/// Each terminal row shows two pixel rows.
pub const PIXELS_PER_ROW: usize = 2;
pub const BLOOM_FRAMES: usize = 45;
pub const FRAME_INTERVAL: Duration = Duration::from_millis(40);

/// The block of terminal cells a garden is drawn into.
pub struct Screen {
    pub columns: usize,
    pub rows: usize,
    pub depth: ColorDepth,
}

impl Screen {
    pub fn pixel_width(&self) -> usize {
        self.columns
    }

    pub fn pixel_height(&self) -> usize {
        self.rows * PIXELS_PER_ROW
    }

    /// The garden `progress` (0–1) of the way to full bloom, one string per terminal row.
    pub fn render(&self, garden: &Garden, progress: f32) -> Vec<String> {
        let mut canvas = Canvas::new(self.pixel_width(), self.pixel_height(), SUPERSAMPLING);
        garden.paint(&mut canvas, progress);
        ansi::encode(&canvas.resolve(), self.depth)
    }
}

pub fn show(out: &mut impl Write, garden: &Garden, screen: &Screen) -> io::Result<()> {
    write_lines(out, &screen.render(garden, 1.0))
}

/// Grows the garden from bare soil to full bloom, redrawing it in place.
pub fn play_bloom(out: &mut impl Write, garden: &Garden, screen: &Screen) -> io::Result<()> {
    for frame in 0..=BLOOM_FRAMES {
        let lines = screen.render(garden, frame as f32 / BLOOM_FRAMES as f32);
        if frame > 0 {
            // Move the cursor back up over the previous frame.
            write!(out, "\x1b[{}A", lines.len())?;
        }
        write_lines(out, &lines)?;
        out.flush()?;
        if frame < BLOOM_FRAMES {
            thread::sleep(FRAME_INTERVAL);
        }
    }
    Ok(())
}

fn write_lines(out: &mut impl Write, lines: &[String]) -> io::Result<()> {
    for line in lines {
        writeln!(out, "{line}")?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flower::Species;
    use crate::garden::Planting;
    use crate::rng::Rng;

    fn screen(columns: usize, rows: usize) -> Screen {
        Screen {
            columns,
            rows,
            depth: ColorDepth::Truecolor,
        }
    }

    fn plan(screen: &Screen, species: Vec<Species>, seed: u64) -> Garden {
        let planting = Planting {
            count: Some(4),
            species,
            grass: true,
        };
        Garden::plan(
            &planting,
            screen.pixel_width(),
            screen.pixel_height(),
            &mut Rng::new(seed),
        )
    }

    #[test]
    fn same_seed_regrows_the_same_garden() {
        let screen = screen(60, 16);
        let first = screen.render(&plan(&screen, Species::ALL.to_vec(), 99), 1.0);
        let second = screen.render(&plan(&screen, Species::ALL.to_vec(), 99), 1.0);
        assert_eq!(first, second);
    }

    #[test]
    fn every_species_draws_something_at_every_stage() {
        let screen = screen(30, 10);
        for species in Species::ALL {
            let garden = plan(&screen, vec![species], 1);
            for progress in [0.0, 0.3, 0.7, 1.0] {
                let lines = screen.render(&garden, progress);
                assert_eq!(lines.len(), screen.rows, "{species:?} at {progress}");
            }
            let bloomed = screen.render(&garden, 1.0).concat();
            assert!(
                bloomed.contains('▀') || bloomed.contains('▄'),
                "{species:?} drew nothing"
            );
        }
    }
}
