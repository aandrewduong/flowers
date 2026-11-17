//! Writes gardens to the terminal, either finished or as a growing animation.

use std::io::{self, Write};
use std::thread;
use std::time::Duration;

use crate::ansi::{self, ColorDepth};
use crate::canvas::Canvas;
use crate::scene::{Atmosphere, Moment, Scene};

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

    /// The scene as it looks at `moment`, one string per terminal row.
    pub fn render(&self, scene: &Scene, atmosphere: &Atmosphere, moment: Moment) -> Vec<String> {
        let mut canvas = Canvas::new(self.pixel_width(), self.pixel_height(), SUPERSAMPLING);
        scene.paint(&mut canvas, atmosphere, moment);
        ansi::encode(&canvas.resolve(), self.depth)
    }
}

pub fn show(
    out: &mut impl Write,
    scene: &Scene,
    atmosphere: &Atmosphere,
    screen: &Screen,
) -> io::Result<()> {
    write_lines(out, &screen.render(scene, atmosphere, Moment::BLOOMED))
}

/// Grows the garden from bare soil to full bloom, redrawing it in place.
pub fn play_bloom(
    out: &mut impl Write,
    scene: &Scene,
    atmosphere: &Atmosphere,
    screen: &Screen,
) -> io::Result<()> {
    for frame in 0..=BLOOM_FRAMES {
        let moment = Moment {
            progress: frame as f32 / BLOOM_FRAMES as f32,
            time: FRAME_INTERVAL.as_secs_f32() * frame as f32,
        };
        let lines = screen.render(scene, atmosphere, moment);
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
    use crate::sky::Sky;
    use crate::weather::Weather;

    const STILL: Atmosphere = Atmosphere {
        sky: None,
        weather: Weather::Clear,
        wind: false,
        visitors: false,
    };

    fn at(time: f32) -> Moment {
        Moment {
            progress: 1.0,
            time,
        }
    }

    fn screen(columns: usize, rows: usize) -> Screen {
        Screen {
            columns,
            rows,
            depth: ColorDepth::Truecolor,
        }
    }

    fn plan(screen: &Screen, species: Vec<Species>, seed: u64) -> Scene {
        let planting = Planting {
            count: Some(4),
            species,
            grass: true,
        };
        Scene::plan(
            &planting,
            screen.pixel_width(),
            screen.pixel_height(),
            &mut Rng::new(seed),
        )
    }

    #[test]
    fn same_seed_regrows_the_same_garden() {
        let screen = screen(60, 16);
        let lively = Atmosphere {
            sky: Some(Sky::Dusk),
            weather: Weather::Rain,
            wind: true,
            visitors: true,
        };
        for atmosphere in [STILL, lively] {
            let render = || {
                let scene = plan(&screen, Species::ALL.to_vec(), 99);
                screen.render(&scene, &atmosphere, at(2.5))
            };
            assert_eq!(render(), render());
        }
    }

    #[test]
    fn a_still_garden_looks_the_same_at_any_time() {
        let screen = screen(60, 16);
        let scene = plan(&screen, Species::ALL.to_vec(), 99);
        assert!(!STILL.is_lively());
        assert_eq!(
            screen.render(&scene, &STILL, at(0.0)),
            screen.render(&scene, &STILL, at(7.0))
        );
    }

    #[test]
    fn everything_lively_moves_as_time_passes() {
        let screen = screen(60, 16);
        let scene = plan(&screen, Species::ALL.to_vec(), 99);
        let lively = [
            Atmosphere {
                wind: true,
                ..STILL
            },
            Atmosphere {
                visitors: true,
                ..STILL
            },
            Atmosphere {
                weather: Weather::Rain,
                ..STILL
            },
            Atmosphere {
                sky: Some(Sky::Night),
                ..STILL
            },
        ];
        for atmosphere in lively {
            assert!(atmosphere.is_lively(), "{atmosphere:?}");
            assert_ne!(
                screen.render(&scene, &atmosphere, at(0.0)),
                screen.render(&scene, &atmosphere, at(1.0)),
                "{atmosphere:?} stood still"
            );
        }
    }

    #[test]
    fn a_sky_leaves_no_cell_bare() {
        let screen = screen(40, 12);
        let scene = plan(&screen, Species::ALL.to_vec(), 3);
        for sky in Sky::ALL {
            let atmosphere = Atmosphere {
                sky: Some(sky),
                ..STILL
            };
            let picture = screen.render(&scene, &atmosphere, at(0.0)).concat();
            assert!(!picture.contains(' '), "{sky:?} left gaps");
        }
    }

    #[test]
    fn visitors_wait_for_the_garden_to_bloom() {
        let screen = screen(60, 16);
        let scene = plan(&screen, Species::ALL.to_vec(), 99);
        let visited = Atmosphere {
            visitors: true,
            ..STILL
        };
        let sprouting = Moment {
            progress: 0.5,
            time: 0.0,
        };
        assert_eq!(
            screen.render(&scene, &visited, sprouting),
            screen.render(&scene, &STILL, sprouting)
        );
        assert_ne!(
            screen.render(&scene, &visited, Moment::BLOOMED),
            screen.render(&scene, &STILL, Moment::BLOOMED)
        );
    }

    #[test]
    fn every_species_draws_something_at_every_stage() {
        let screen = screen(30, 10);
        for species in Species::ALL {
            let garden = plan(&screen, vec![species], 1);
            for progress in [0.0, 0.3, 0.7, 1.0] {
                let moment = Moment {
                    progress,
                    time: 0.0,
                };
                let lines = screen.render(&garden, &STILL, moment);
                assert_eq!(lines.len(), screen.rows, "{species:?} at {progress}");
            }
            let bloomed = screen.render(&garden, &STILL, Moment::BLOOMED).concat();
            assert!(
                bloomed.contains('▀') || bloomed.contains('▄'),
                "{species:?} drew nothing"
            );
        }
    }
}
