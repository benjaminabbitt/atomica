//! Board geometry — the two-board **seam** (`docs/combat.md` Phase 7, taxonomy §7B).
//!
//! Flat-top hexes: **columns (`q`) are depth ranks**, **forward runs `+q`** toward
//! the enemy, and the long edge (`r`) is **frontage**. The two armies meet at a
//! vertical seam where one board is staggered **±½ hex** along the frontage — rolled
//! at start, settable by item. We model this on the **single shared integer grid**
//! (no second coordinate system): the half-hex stagger is realised as *which of the
//! two forward neighbours* a front hex engages across the seam.
//!
//! A flat-top hex at `(q, r)` always has **two** forward (`+q`) neighbours —
//! `(q+1, r)` and one diagonal. The [`SeamOffset`] picks the diagonal: `Down` →
//! `(q+1, r-1)` (the natural axial pairing), `Up` → `(q+1, r+1)`. So **each front hex
//! engages two enemy front hexes**, and which two is the offset — exactly the §7B
//! "seam offset sets the cross-board front-line pairings". `Up` makes a pair the raw
//! grid would call distance-2 into a melee engagement (the seam closes the stagger).

use crate::{Hex, RandomSource};

/// The half-hex stagger at the seam (§7B) — which diagonal a front hex pairs to.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum SeamOffset {
    /// Second forward neighbour is `(q+1, r-1)` — the natural axial pairing.
    #[default]
    Down,
    /// Second forward neighbour is `(q+1, r+1)` — the board shifted half a hex up.
    Up,
}

impl SeamOffset {
    /// The `r`-delta of the staggered forward neighbour.
    fn delta(self) -> i32 {
        match self {
            SeamOffset::Down => -1,
            SeamOffset::Up => 1,
        }
    }
}

/// The battle board: just the seam stagger (the single shared grid holds positions).
#[derive(Clone, Copy, Debug, Default)]
pub struct Board {
    pub offset: SeamOffset,
}

impl Board {
    pub fn new(offset: SeamOffset) -> Self {
        Self { offset }
    }

    /// Roll the seam offset at start (§7B) — `Up`/`Down` from one RNG draw.
    pub fn roll<R: RandomSource>(rng: &mut R) -> Self {
        let offset = if rng.next_u64() & 1 == 0 { SeamOffset::Down } else { SeamOffset::Up };
        Self { offset }
    }

    /// The two enemy front hexes a hex at `from` engages across the seam — its two
    /// forward (`+q`) neighbours under the offset (§7B "edge-adjacent to two").
    pub fn frontage_pairs(self, from: Hex) -> [Hex; 2] {
        [Hex::new(from.q + 1, from.r), Hex::new(from.q + 1, from.r + self.offset.delta())]
    }

    /// Do `a` and `b` **engage across the seam** — are they front-line forward-pairs
    /// (one column apart, paired by the offset)? Order-independent. This is what lets
    /// an `Up`-staggered pair (raw grid distance 2) clash in melee.
    pub fn engages(self, a: Hex, b: Hex) -> bool {
        self.frontage_pairs(a).contains(&b) || self.frontage_pairs(b).contains(&a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn down_offset_pairs_the_natural_axial_diagonal() {
        let b = Board::new(SeamOffset::Down);
        let pairs = b.frontage_pairs(Hex::new(0, 0));
        assert!(pairs.contains(&Hex::new(1, 0)));
        assert!(pairs.contains(&Hex::new(1, -1)));
        // both are genuine grid neighbours (distance 1).
        assert!(pairs.iter().all(|h| h.distance(Hex::new(0, 0)) == 1));
    }

    #[test]
    fn up_offset_pairs_the_other_diagonal() {
        let b = Board::new(SeamOffset::Up);
        let pairs = b.frontage_pairs(Hex::new(0, 0));
        assert!(pairs.contains(&Hex::new(1, 0)));
        assert!(pairs.contains(&Hex::new(1, 1)));
        // (1,1) is grid distance 2 — the seam stagger closes it.
        assert_eq!(Hex::new(0, 0).distance(Hex::new(1, 1)), 2);
    }

    #[test]
    fn engagement_is_order_independent() {
        let b = Board::new(SeamOffset::Up);
        let (a, c) = (Hex::new(0, 0), Hex::new(1, 1));
        assert!(b.engages(a, c));
        assert!(b.engages(c, a));
        // a non-pair across two columns does not engage.
        assert!(!b.engages(a, Hex::new(2, 0)));
    }

    #[test]
    fn roll_is_deterministic_per_rng() {
        use crate::SplitMix64;
        let a = Board::roll(&mut SplitMix64::new(42)).offset;
        let b = Board::roll(&mut SplitMix64::new(42)).offset;
        assert_eq!(a, b);
    }
}
