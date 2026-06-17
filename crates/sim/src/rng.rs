//! Deterministic PRNG and the [`RandomSource`] seam.
//!
//! The simulation depends on the `RandomSource` **trait**, never a concrete RNG,
//! so tests inject a [`ScriptedRng`] and force every roll (inversion of control
//! for TDD). Production uses [`SplitMix64`] — small, fast, and reproducible across
//! native and wasm.

use std::collections::VecDeque;

/// The randomness seam. Implementors supply only [`RandomSource::next_u64`]; the
/// higher-level draws (floats, chances, dice) derive from it with shared defaults.
pub trait RandomSource {
    /// The single required primitive.
    fn next_u64(&mut self) -> u64;

    /// Uniform `f32` in `[0.0, 1.0)`.
    fn next_f32(&mut self) -> f32 {
        // 24 high bits → exact division, no float-rounding bias.
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// `true` with probability `p` (clamped to `[0, 1]`). Hook for stochastic statuses.
    fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p.clamp(0.0, 1.0)
    }

    /// Inclusive integer in `[lo, hi]`. Returns `lo` if `hi <= lo`.
    fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i32
    }

    /// A single d6, `1..=6`.
    fn d6(&mut self) -> i32 {
        (self.next_u64() % 6) as i32 + 1
    }

    /// The core mechanic's dice: **3d6** (`3..=18`) — a tight bell curve so skill
    /// dominates and luck is a small nudge (design-delta §13).
    fn roll_3d6(&mut self) -> i32 {
        self.d6() + self.d6() + self.d6()
    }
}

/// Production RNG (SplitMix64): dependency-free, reproducible across platforms.
#[derive(Clone, Debug)]
pub struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }
}

impl RandomSource for SplitMix64 {
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
}

/// A scripted RNG for tests: yields a queued sequence so every roll is *asserted*,
/// not hoped for. Panics if drained — rolling past the script is a test bug.
#[derive(Clone, Debug, Default)]
pub struct ScriptedRng {
    queue: VecDeque<u64>,
}

impl ScriptedRng {
    /// Queue raw `u64` draws.
    pub fn new(values: impl IntoIterator<Item = u64>) -> Self {
        Self { queue: values.into_iter().collect() }
    }

    /// Queue d6 **faces** (`1..=6`) directly — e.g. `from_d6([6, 6, 6])` forces 3d6 = 18.
    pub fn from_d6(faces: impl IntoIterator<Item = i32>) -> Self {
        Self::new(faces.into_iter().map(|f| (f.clamp(1, 6) - 1) as u64))
    }
}

impl RandomSource for ScriptedRng {
    fn next_u64(&mut self) -> u64 {
        self.queue.pop_front().expect("ScriptedRng drained: rolled more than scripted")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a = SplitMix64::new(42);
        let mut b = SplitMix64::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn chance_is_bounded() {
        let mut r = SplitMix64::new(7);
        assert!(!r.chance(0.0));
        assert!(r.chance(1.0));
    }

    #[test]
    fn scripted_d6_faces_come_back_exact() {
        let mut r = ScriptedRng::from_d6([1, 4, 6]);
        assert_eq!(r.d6(), 1);
        assert_eq!(r.d6(), 4);
        assert_eq!(r.d6(), 6);
    }

    #[test]
    fn scripted_3d6_sums() {
        let mut r = ScriptedRng::from_d6([6, 6, 6]);
        assert_eq!(r.roll_3d6(), 18);
    }
}
