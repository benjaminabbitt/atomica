//! Netrunning — the digital attack, resolved by the core **3d6 contest**.
//!
//! A hack is one unit projecting onto the net against another. **Link is the
//! digital bandwidth** (§7D): it caps how much of each side's digital stat
//! actually comes to bear, so the contest is
//!
//! ```text
//! 3d6 + min(Link, Hacking)   vs   min(Link, Firewall)
//! ```
//!
//! — your trained Hacking only lands up to what your Link can push, and the
//! defender's Firewall only engages up to *its* Link surface (§13). Both ends are
//! still hard-gated: **zero Link** means *no surface to reach* (the target is
//! air-gapped) or *no presence to reach with* (the attacker is dark), §7F. The
//! deck / equipment's digital strength flows through **Link** and **Hacking**
//! (cyberdeck → Link, skill chip → Hacking, §7D/§13), so there is no separate
//! roll add-on. On a success the hack lands its **payload** — a status: a tripped
//! hack-effect or a deployed program (§7F/§10.8) — with stacks scaling on the
//! **margin** (degree of success).
//!
//! The roll itself lives in [`resolve_contest`](crate::resolve_contest); the
//! Link-capping and payload application happen in
//! [`Battle::resolve_hack`](crate::Battle::resolve_hack).

use crate::{RollOutcome, StatusSpec};

/// How many margin points buy one extra payload stack (degree-of-success
/// scaling, §13). Placeholder tuning value — numbers are TBD.
const MARGIN_PER_STACK: i32 = 3;

/// A unit's hack loadout — the digital action it can take on its turn (§7F).
///
/// The hack's *strength* is the unit's own Link & Hacking (capped against each
/// other); this struct only says *what program* it runs and *how far*. The
/// `payload`'s own 9-axis spec governs how it behaves once it lands.
#[derive(Clone, Copy, Debug)]
pub struct Hack {
    /// Antenna reach in hexes the hack carries across (§7F beam / proximity).
    pub range: i32,
    /// The status landed on success — a tripped hack-effect or deployed program.
    pub payload: StatusSpec,
    /// Stacks at margin 0; the margin (degree of success) adds more.
    pub base_stacks: u32,
    /// Duration of the landed status.
    pub duration: u32,
}

impl Hack {
    pub fn new(range: i32, payload: StatusSpec, base_stacks: u32, duration: u32) -> Self {
        Self { range, payload, base_stacks, duration }
    }

    /// Stacks landed for a resolved `outcome`: base + a margin-scaled bonus + a
    /// crit bump, or `0` on failure (the §13 degree-of-success rule). The margin
    /// is floored at `0` so a crit-over-the-wall (negative margin) still lands base.
    pub fn stacks_for(&self, outcome: &RollOutcome) -> u32 {
        if !outcome.success {
            return 0;
        }
        let bonus = (outcome.margin.max(0) / MARGIN_PER_STACK) as u32;
        self.base_stacks + bonus + outcome.crit as u32
    }
}

/// The result of a [`Battle::resolve_hack`](crate::Battle::resolve_hack).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HackResult {
    /// The attacker carries no hack loadout — nothing to resolve.
    NoHack,
    /// Attacker has zero Link — it can't project onto the net (no roll, §7D).
    Offline,
    /// Target has zero Link — no surface to reach; immune (no roll, §7D/§7F).
    NoSurface,
    /// The contest was rolled. `stacks` is what landed (`0` ⇒ the hack whiffed).
    Rolled { outcome: RollOutcome, stacks: u32 },
}

impl HackResult {
    /// Did the hack land its payload?
    pub fn landed(&self) -> bool {
        matches!(self, HackResult::Rolled { stacks, .. } if *stacks > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{resolve_contest, Contest, ScriptedRng};

    fn hack() -> Hack {
        Hack::new(2, StatusSpec::lockware(), 1, 5)
    }

    #[test]
    fn margin_scales_the_stacks() {
        let mut rng = ScriptedRng::from_d6([3, 3, 3]); // 3d6 = 9
        let o = resolve_contest(&mut rng, Contest::new(4, 0, 10)); // 13 vs 10 → margin 3
        assert_eq!(o.margin, 3);
        assert_eq!(hack().stacks_for(&o), 2); // base 1 + 3/3
    }

    #[test]
    fn deeper_margin_lands_more() {
        let mut rng = ScriptedRng::from_d6([6, 6, 5]); // 3d6 = 17 (not a crit)
        let o = resolve_contest(&mut rng, Contest::new(4, 0, 10)); // 21 vs 10 → margin 11
        assert_eq!(hack().stacks_for(&o), 1 + 3); // base 1 + 11/3
    }

    #[test]
    fn a_whiff_lands_nothing() {
        let mut rng = ScriptedRng::from_d6([2, 2, 2]); // 3d6 = 6
        let o = resolve_contest(&mut rng, Contest::new(0, 0, 20)); // 6 vs 20 → fail
        assert!(!o.success);
        assert_eq!(hack().stacks_for(&o), 0);
    }

    #[test]
    fn a_crit_lands_despite_the_wall_and_adds_a_stack() {
        let mut rng = ScriptedRng::from_d6([6, 6, 6]); // natural 18 → crit
        let o = resolve_contest(&mut rng, Contest::new(0, 0, 99)); // negative margin
        assert!(o.crit && o.success);
        assert_eq!(hack().stacks_for(&o), 1 + 1); // base + crit bump (margin floored at 0)
    }
}
