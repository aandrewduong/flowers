//! What the interactive garden is showing, and how each command changes it.

use super::controls::Command;
use crate::flower::Species;
use crate::garden::{MAX_FLOWERS, Planting};
use crate::rng::{Rng, fresh_seed};

/// What the screen has to do after a command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// A garden nobody has seen yet: grow it from bare soil.
    Sprouted,
    /// Different settings, or a garden seen before: show it in full bloom straight away.
    Replanted,
    /// Grow the same garden again from bare soil.
    Replayed,
    Unchanged,
    Quit,
}

pub struct Session {
    planting: Planting,
    /// Seeds of every garden shown so far, oldest first.
    history: Vec<u64>,
    /// Index into `history` of the garden on screen.
    position: usize,
    seeds: Rng,
}

impl Session {
    /// `seeds` supplies the seeds for gardens grown during the session.
    pub fn new(planting: Planting, first_seed: u64, seeds: Rng) -> Self {
        Self {
            planting,
            history: vec![first_seed],
            position: 0,
            seeds,
        }
    }

    pub fn planting(&self) -> &Planting {
        &self.planting
    }

    pub fn seed(&self) -> u64 {
        self.history[self.position]
    }

    /// `flowers` is how many flowers the garden on screen holds; `+` and `-` count from there.
    pub fn apply(&mut self, command: Command, flowers: usize) -> Change {
        match command {
            Command::NewGarden => self.sprout(),
            Command::PreviousGarden => self.step_back(),
            Command::NextGarden => self.step_forward(),
            Command::ReplayBloom => Change::Replayed,
            Command::ToggleSpecies(species) => self.toggle(species),
            Command::AllSpecies => self.grow_all_species(),
            Command::MoreFlowers => self.set_count(flowers.saturating_add(1), flowers),
            Command::FewerFlowers => self.set_count(flowers.saturating_sub(1), flowers),
            Command::ToggleGrass => {
                self.planting.grass = !self.planting.grass;
                Change::Replanted
            }
            Command::Quit => Change::Quit,
        }
    }

    fn sprout(&mut self) -> Change {
        // Like a browser: growing something new after going back forgets what was ahead.
        self.history.truncate(self.position + 1);
        self.history.push(fresh_seed(&mut self.seeds));
        self.position = self.history.len() - 1;
        Change::Sprouted
    }

    fn step_back(&mut self) -> Change {
        if self.position == 0 {
            return Change::Unchanged;
        }
        self.position -= 1;
        Change::Replanted
    }

    fn step_forward(&mut self) -> Change {
        if self.position + 1 == self.history.len() {
            return self.sprout();
        }
        self.position += 1;
        Change::Replanted
    }

    fn toggle(&mut self, species: Species) -> Change {
        let growing = |candidate: Species| self.planting.species.contains(&candidate);
        let chosen: Vec<Species> = Species::ALL
            .into_iter()
            .filter(|&candidate| growing(candidate) != (candidate == species))
            .collect();
        // A garden needs at least one thing to grow.
        if chosen.is_empty() {
            return Change::Unchanged;
        }
        self.planting.species = chosen;
        Change::Replanted
    }

    fn grow_all_species(&mut self) -> Change {
        if Species::ALL
            .iter()
            .all(|species| self.planting.species.contains(species))
        {
            return Change::Unchanged;
        }
        self.planting.species = Species::ALL.to_vec();
        Change::Replanted
    }

    fn set_count(&mut self, wanted: usize, current: usize) -> Change {
        let count = wanted.clamp(1, usize::from(MAX_FLOWERS));
        if count == current {
            return Change::Unchanged;
        }
        self.planting.count = Some(count);
        Change::Replanted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn session() -> Session {
        let planting = Planting {
            count: None,
            species: Species::ALL.to_vec(),
            grass: true,
        };
        Session::new(planting, 1, Rng::new(7))
    }

    #[test]
    fn walks_back_and_forward_through_history() {
        let mut session = session();
        assert_eq!(session.apply(Command::PreviousGarden, 5), Change::Unchanged);

        assert_eq!(session.apply(Command::NewGarden, 5), Change::Sprouted);
        let second = session.seed();
        assert_ne!(second, 1);

        assert_eq!(session.apply(Command::PreviousGarden, 5), Change::Replanted);
        assert_eq!(session.seed(), 1);
        assert_eq!(session.apply(Command::NextGarden, 5), Change::Replanted);
        assert_eq!(session.seed(), second);
        assert_eq!(session.apply(Command::NextGarden, 5), Change::Sprouted);
        assert_ne!(session.seed(), second);
    }

    #[test]
    fn new_garden_after_going_back_forgets_what_was_ahead() {
        let mut session = session();
        session.apply(Command::NewGarden, 5);
        session.apply(Command::NewGarden, 5);
        session.apply(Command::PreviousGarden, 5);
        session.apply(Command::PreviousGarden, 5);
        session.apply(Command::NewGarden, 5);

        assert_eq!(session.history.len(), 2);
        assert_eq!(session.apply(Command::NextGarden, 5), Change::Sprouted);
    }

    #[test]
    fn toggling_twice_restores_species() {
        let mut session = session();
        assert_eq!(
            session.apply(Command::ToggleSpecies(Species::Rose), 5),
            Change::Replanted
        );
        assert!(!session.planting().species.contains(&Species::Rose));
        session.apply(Command::ToggleSpecies(Species::Rose), 5);
        assert_eq!(session.planting().species, Species::ALL.to_vec());
    }

    #[test]
    fn last_species_cannot_be_toggled_off() {
        let mut session = session();
        for species in Species::ALL.into_iter().skip(1) {
            session.apply(Command::ToggleSpecies(species), 5);
        }
        assert_eq!(session.planting().species, vec![Species::ALL[0]]);
        assert_eq!(
            session.apply(Command::ToggleSpecies(Species::ALL[0]), 5),
            Change::Unchanged
        );
        assert_eq!(session.apply(Command::AllSpecies, 5), Change::Replanted);
        assert_eq!(session.apply(Command::AllSpecies, 5), Change::Unchanged);
    }

    #[test]
    fn flower_count_stays_in_bounds() {
        let mut session = session();
        assert_eq!(session.apply(Command::FewerFlowers, 1), Change::Unchanged);
        assert_eq!(session.apply(Command::MoreFlowers, 1), Change::Replanted);
        assert_eq!(session.planting().count, Some(2));

        let most = usize::from(MAX_FLOWERS);
        assert_eq!(session.apply(Command::MoreFlowers, most), Change::Unchanged);
    }

    #[test]
    fn settings_changes_keep_the_same_seed() {
        let mut session = session();
        session.apply(Command::ToggleGrass, 5);
        session.apply(Command::MoreFlowers, 5);
        session.apply(Command::ToggleSpecies(Species::Tulip), 5);
        assert_eq!(session.seed(), 1);
        assert!(!session.planting().grass);
    }
}
