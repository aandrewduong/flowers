mod ansi;
mod canvas;
mod cli;
mod color;
mod display;
mod flower;
mod foliage;
mod garden;
mod geometry;
mod interactive;
mod rng;
mod shape;

use std::io::{self, IsTerminal};
use std::process::ExitCode;

use clap::Parser;
use crossterm::terminal;

use cli::{Args, MIN_ROWS};
use display::Screen;
use flower::Species;
use garden::{Garden, Planting};
use rng::Rng;

/// Used when stdout isn't a terminal we can measure.
const FALLBACK_COLUMNS: usize = 80;
const DEFAULT_ROWS: usize = 24;
/// Terminal rows left free below the garden for the seed note and the next prompt.
const RESERVED_ROWS: usize = 2;
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

fn main() -> ExitCode {
    let args = Args::parse();
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        // Piping into `head` and friends closes stdout early; that isn't a failure.
        Err(err) if err.kind() == io::ErrorKind::BrokenPipe => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("flowers: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(args: &Args) -> io::Result<()> {
    let planting = planting_for(args);
    let seed = args
        .seed
        .unwrap_or_else(|| rng::fresh_seed(&mut rng::from_clock()));
    let last_seed = if args.interactive {
        interactive::run(planting, seed, args.colors)?
    } else {
        print_garden(args, &planting, seed)?;
        seed
    };
    report_seed(last_seed);
    Ok(())
}

fn print_garden(args: &Args, planting: &Planting, seed: u64) -> io::Result<()> {
    let terminal = terminal::size()
        .ok()
        .map(|(columns, rows)| (usize::from(columns), usize::from(rows)));
    let screen = screen_for(args, terminal);
    let garden = Garden::plan(
        planting,
        screen.pixel_width(),
        screen.pixel_height(),
        &mut Rng::new(seed),
    );

    let mut stdout = io::stdout().lock();
    // Redrawing in place only works when the whole garden fits on screen.
    let can_animate = stdout.is_terminal() && terminal.is_some_and(|(_, rows)| screen.rows < rows);
    if args.bloom && can_animate {
        display::play_bloom(&mut stdout, &garden, &screen)
    } else {
        display::show(&mut stdout, &garden, &screen)
    }
}

fn screen_for(args: &Args, terminal: Option<(usize, usize)>) -> Screen {
    let columns = args
        .width
        .map(usize::from)
        .or(terminal.map(|(columns, _)| columns))
        .unwrap_or(FALLBACK_COLUMNS);
    let rows = args.height.map(usize::from).unwrap_or_else(|| {
        let available =
            terminal.map_or(DEFAULT_ROWS, |(_, rows)| rows.saturating_sub(RESERVED_ROWS));
        available.clamp(usize::from(MIN_ROWS), DEFAULT_ROWS)
    });
    Screen {
        columns,
        rows,
        depth: args.colors,
    }
}

fn planting_for(args: &Args) -> Planting {
    Planting {
        count: args.count.map(usize::from),
        species: if args.kinds.is_empty() {
            Species::ALL.to_vec()
        } else {
            args.kinds.clone()
        },
        grass: !args.no_grass,
    }
}

/// Tells the user how to regrow this garden. Goes to stderr so piped art stays clean.
fn report_seed(seed: u64) {
    if io::stderr().is_terminal() {
        eprintln!("{DIM}seed {seed}{RESET}");
    } else {
        eprintln!("seed {seed}");
    }
}
