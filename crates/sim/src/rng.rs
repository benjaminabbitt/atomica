//! Deterministic PRNG.
//!
//! SplitMix64 — small, fast, dependency-free and reproducible across platforms
//! (no float/int width surprises between native and wasm). All randomness in the
//! simulation must flow through this so that a given seed yields a given battle.

/// A seeded, deterministic random number generator.
#[derive(Clone, Debug)]
pub struct Rng {
    state: u64,
}

impl Rng {
    /// Create a generator from a seed. The same seed always produces the same stream.
    pub fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    /// Next raw 64-bit value (SplitMix64).
    pub fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform `f32` in `[0.0, 1.0)`.
    pub fn next_f32(&mut self) -> f32 {
        // Take 24 high bits → exact division, no bias from float rounding.
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    /// Returns `true` with probability `p` (clamped to `[0, 1]`).
    ///
    /// This is the hook for the design's *stochastic* statuses: a resist value
    /// shifts `p` before the roll.
    pub fn chance(&mut self, p: f32) -> bool {
        self.next_f32() < p.clamp(0.0, 1.0)
    }

    /// Inclusive integer in `[lo, hi]`. Returns `lo` if `hi <= lo`.
    pub fn range(&mut self, lo: i32, hi: i32) -> i32 {
        if hi <= lo {
            return lo;
        }
        let span = (hi - lo + 1) as u64;
        lo + (self.next_u64() % span) as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_same_stream() {
        let mut a = Rng::new(42);
        let mut b = Rng::new(42);
        for _ in 0..1000 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn chance_is_bounded() {
        let mut r = Rng::new(7);
        assert!(!r.chance(0.0));
        assert!(r.chance(1.0));
    }
}
