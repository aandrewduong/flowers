//! Deterministic randomness, so a seed always regrows the same garden.

use std::ops::Range;
use std::time::{SystemTime, UNIX_EPOCH};

const GOLDEN_GAMMA: u64 = 0x9E37_79B9_7F4A_7C15;
/// An f32 mantissa holds 24 bits, so that many random bits map onto [0, 1) evenly.
const UNIT_BITS: u32 = 24;
/// Seeds picked for the user stay short enough to retype.
const FRESH_SEED_LIMIT: u64 = 1_000_000;

/// SplitMix64: tiny, fast, and good enough for scattering petals.
pub struct Rng {
    state: u64,
}

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(GOLDEN_GAMMA);
        mix(self.state)
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        to_unit(self.next_u64())
    }

    pub fn range(&mut self, range: Range<f32>) -> f32 {
        range.start + (range.end - range.start) * self.unit()
    }

    pub fn integer(&mut self, range: Range<usize>) -> usize {
        let span = (range.end - range.start).max(1) as u64;
        range.start + (self.next_u64() % span) as usize
    }

    pub fn chance(&mut self, probability: f32) -> bool {
        self.unit() < probability
    }

    /// Either 1 or -1, for picking a side.
    pub fn sign(&mut self) -> f32 {
        if self.chance(0.5) { 1.0 } else { -1.0 }
    }

    pub fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.integer(0..items.len())]
    }
}

/// A seed for a garden nobody has asked for by number.
pub fn fresh_seed(rng: &mut Rng) -> u64 {
    rng.next_u64() % FRESH_SEED_LIMIT
}

/// A generator that differs from run to run.
pub fn from_clock() -> Rng {
    // Some clocks only tick in microseconds, so mix the whole timestamp rather than
    // trusting its low digits to vary.
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_nanos() as u64);
    Rng::new(mix(nanos))
}

/// Stateless hash in [0, 1). Shaders use it for per-petal variation, where the answer
/// must depend only on which petal is being drawn, not on the order pixels are visited.
pub fn hash_unit(key: u64) -> f32 {
    to_unit(mix(key))
}

fn mix(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn to_unit(bits: u64) -> f32 {
    (bits >> (u64::BITS - UNIT_BITS)) as f32 / (1u64 << UNIT_BITS) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_same_sequence() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn unit_stays_in_half_open_range() {
        let mut rng = Rng::new(7);
        for _ in 0..10_000 {
            let value = rng.unit();
            assert!((0.0..1.0).contains(&value), "{value} escaped [0, 1)");
        }
    }

    #[test]
    fn integer_stays_in_range() {
        let mut rng = Rng::new(3);
        for _ in 0..1_000 {
            assert!((16..25).contains(&rng.integer(16..25)));
        }
    }
}
