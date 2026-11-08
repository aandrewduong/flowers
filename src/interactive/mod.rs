//! A full-screen garden you browse with the keyboard.

mod controls;
mod session;
mod status;

use std::io::{self, IsTerminal, Write};

use crossterm::cursor::{Hide, MoveTo, Show};
use crossterm::event::{self, Event};
use crossterm::style::Print;
use crossterm::terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{execute, queue};

use crate::ansi::ColorDepth;
use crate::cli::{MIN_COLUMNS, MIN_ROWS};
use crate::display::{BLOOM_FRAMES, FRAME_INTERVAL, Screen};
use crate::garden::{Garden, Planting};
use crate::rng::{self, Rng};
use session::{Change, Session};
use status::STATUS_ROWS;

const TOO_SMALL: &str = "Make the terminal a little bigger to see the garden. (q quits)";

/// Runs until the user quits, and returns the seed of the last garden on screen.
pub fn run(planting: Planting, first_seed: u64, depth: ColorDepth) -> io::Result<u64> {
    let mut out = io::stdout();
    if !out.is_terminal() {
        return Err(io::Error::other(
            "interactive mode needs a terminal to draw in",
        ));
    }
    let _screen = FullScreen::enter(&mut out)?;
    let mut session = Session::new(planting, first_seed, rng::from_clock());
    let mut view = View::new(terminal::size()?, depth, &session);
    loop {
        view.draw(&mut out, &session)?;
        match next_event(view.is_blooming())? {
            None => view.advance_bloom(),
            Some(Event::Key(key)) => {
                let Some(command) = controls::command_for(key) else {
                    continue;
                };
                match session.apply(command, view.flowers()) {
                    Change::Quit => break,
                    change => view.update(change, &session),
                }
            }
            Some(Event::Resize(columns, rows)) => view.resize((columns, rows), &session),
            Some(_) => {}
        }
    }
    Ok(session.seed())
}

/// Waits for the next terminal event, or returns `None` when it's time for the next
/// animation frame.
fn next_event(animating: bool) -> io::Result<Option<Event>> {
    if animating && !event::poll(FRAME_INTERVAL)? {
        return Ok(None);
    }
    event::read().map(Some)
}

/// Switches to a raw, full-screen terminal for as long as it lives. Restores the
/// terminal when dropped, including while unwinding from a panic.
struct FullScreen;

impl FullScreen {
    fn enter(out: &mut impl Write) -> io::Result<Self> {
        terminal::enable_raw_mode()?;
        // From here on, dropping the guard undoes raw mode even if the next step fails.
        let guard = Self;
        execute!(out, EnterAlternateScreen, Hide)?;
        Ok(guard)
    }
}

impl Drop for FullScreen {
    fn drop(&mut self) {
        // There's no caller left to report to, and stopping early would leave the terminal
        // half-restored, so each step is attempted regardless of the others.
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
        let _ = terminal::disable_raw_mode();
    }
}

/// The garden on screen and how far it has grown.
struct View {
    terminal: (u16, u16),
    depth: ColorDepth,
    /// `None` while the terminal is too small to hold a garden.
    bed: Option<(Screen, Garden)>,
    /// Frame of the bloom animation; `BLOOM_FRAMES` once fully grown.
    frame: usize,
}

impl View {
    fn new(terminal: (u16, u16), depth: ColorDepth, session: &Session) -> Self {
        let mut view = Self {
            terminal,
            depth,
            bed: None,
            frame: 0,
        };
        view.replant(session);
        view
    }

    fn update(&mut self, change: Change, session: &Session) {
        match change {
            Change::Sprouted => {
                self.replant(session);
                self.frame = 0;
            }
            Change::Replanted => {
                self.replant(session);
                self.frame = BLOOM_FRAMES;
            }
            Change::Replayed => self.frame = 0,
            Change::Unchanged | Change::Quit => {}
        }
    }

    /// Regrows the same garden at the new size, carrying on with any bloom in progress.
    fn resize(&mut self, terminal: (u16, u16), session: &Session) {
        self.terminal = terminal;
        self.replant(session);
    }

    fn replant(&mut self, session: &Session) {
        let columns = usize::from(self.terminal.0);
        let rows = usize::from(self.terminal.1).saturating_sub(STATUS_ROWS);
        let fits = columns >= usize::from(MIN_COLUMNS) && rows >= usize::from(MIN_ROWS);
        self.bed = fits.then(|| {
            let screen = Screen {
                columns,
                rows,
                depth: self.depth,
            };
            let mut rng = Rng::new(session.seed());
            let garden = Garden::plan(
                session.planting(),
                screen.pixel_width(),
                screen.pixel_height(),
                &mut rng,
            );
            (screen, garden)
        });
    }

    fn is_blooming(&self) -> bool {
        self.frame < BLOOM_FRAMES
    }

    fn advance_bloom(&mut self) {
        self.frame = (self.frame + 1).min(BLOOM_FRAMES);
    }

    fn flowers(&self) -> usize {
        self.bed
            .as_ref()
            .map_or(0, |(_, garden)| garden.flower_count())
    }

    fn draw(&self, out: &mut impl Write, session: &Session) -> io::Result<()> {
        let Some((screen, garden)) = &self.bed else {
            queue!(out, Clear(ClearType::All), MoveTo(0, 0), Print(TOO_SMALL))?;
            return out.flush();
        };
        let progress = self.frame as f32 / BLOOM_FRAMES as f32;
        let mut rows = 0..;
        // Garden lines span the full width, so they overwrite the previous frame as they go.
        for (line, row) in screen.render(garden, progress).iter().zip(rows.by_ref()) {
            queue!(out, MoveTo(0, row), Print(line))?;
        }
        let status_lines =
            status::status_lines(session, garden.flower_count(), screen.columns, self.depth);
        for (line, row) in status_lines.iter().zip(rows) {
            queue!(
                out,
                MoveTo(0, row),
                Clear(ClearType::CurrentLine),
                Print(line)
            )?;
        }
        out.flush()
    }
}
