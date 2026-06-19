//! Netrunning — the digital attack, resolved by the core **3d6 contest**.
//!
//! A hack is one unit projecting onto the net against another. It runs over the
//! **connection** between them, whose bandwidth is the **weaker endpoint's Link**
//! (the bottleneck), and the attacker's rating averages its **Hacking** with that
//! channel:
//!
//! ```text
//! 3d6 + avg(Hacking, min(Link_attacker, Link_target))   vs   Firewall
//! ```
//!
//! — skill and channel each pull half the weight (a master runner on a thin pipe
//! is dragged down but not gutted). The target's Link feeds the **channel**, not
//! the wall, so a darker target is harder to hack while **defense stays the
//! Firewall alone** (Link-blind). Both ends are still hard-gated by reachability —
//! **zero Link** means *no surface to reach* (immune target) or *no presence to
//! reach with* (dark attacker), §7F. Equipment arms the roll through the stats
//! (cyberdeck → Link, skill chip → Hacking, §7D/§13), so there is no separate roll
//! add-on. On a success the hack lands its **payload** — a status (a tripped
//! hack-effect or a deployed program, §7F/§10.8) — with stacks scaling on the
//! **margin** (degree of success).
//!
//! Link's *other* job is **latency → digital initiative**: a unit's own Link sets
//! when it acts on the net (high Link = sooner), independent of the channel.
//!
//! The roll lives in [`resolve_versus`](crate::resolve_versus) (roll-under, Firewall as the
//! resist TN); the channel / rating and payload application happen in
//! [`Battle::resolve_hack`](crate::Battle::resolve_hack).

use crate::{RollOutcome, StatusSpec};

/// How many margin points buy one extra payload stack (degree-of-success
/// scaling, §13). Placeholder tuning value — numbers are TBD.
const MARGIN_PER_STACK: i32 = 3;

/// Extra stacks bought by the **degree of success** — the margin, floored by
/// [`MARGIN_PER_STACK`]. `0` at margin ≤ 0, so a marginal breach is a *pure
/// disable* (the §6 floor) with no liability fired.
pub fn margin_stacks(margin: i32) -> u32 {
    (margin.max(0) / MARGIN_PER_STACK) as u32
}

/// The attacker's effective hack rating: its **Hacking** averaged with the
/// **connection channel** (the weaker endpoint's Link bandwidth), floored —
/// `(hacking + channel) / 2`. Skill and channel each carry half the weight, so a
/// thin channel drags a master runner down without gutting it.
pub fn hack_rating(hacking: i32, channel: i32) -> i32 {
    (hacking + channel) / 2
}

/// A unit's hack loadout — the digital action it can take on its turn (§7F).
///
/// The hack's *strength* is the unit's own Hacking and the connection channel
/// (the weaker endpoint's Link); this struct only says *what program* it runs and
/// *how far*. The `payload`'s own 9-axis spec governs how it behaves once it lands.
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
        self.base_stacks + margin_stacks(outcome.margin) + outcome.crit as u32
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
    /// Did the hack succeed — i.e. **breach** the target? A success always at
    /// least *disables* a tripped implant (the §6 floor); `stacks` may still be 0
    /// (a marginal breach fires no liability).
    pub fn landed(&self) -> bool {
        matches!(self, HackResult::Rolled { outcome, .. } if outcome.success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{resolve_versus, ScriptedRng};

    fn hack() -> Hack {
        Hack::new(2, StatusSpec::lockware(), 1, 5)
    }

    #[test]
    fn rating_averages_skill_with_the_channel_floored() {
        assert_eq!(hack_rating(6, 4), 5);
        assert_eq!(hack_rating(6, 1), 3); // 3.5 → floor 3
        assert_eq!(hack_rating(9, 2), 5); // 5.5 → floor 5
        assert_eq!(hack_rating(0, 0), 0);
    }

    #[test]
    fn margin_scales_the_stacks() {
        // Roll-under versus: target = 21 + rating − firewall = 15; 3d6 = 12 ⇒ margin 3.
        let mut rng = ScriptedRng::from_d6([4, 4, 4]);
        let o = resolve_versus(&mut rng, 4, 10);
        assert_eq!(o.margin, 3);
        assert_eq!(hack().stacks_for(&o), 2); // base 1 + 3/3
    }

    #[test]
    fn deeper_margin_lands_more() {
        // Lower dice ⇒ deeper margin under the same target 15; 3d6 = 5 ⇒ margin 10.
        let mut rng = ScriptedRng::from_d6([1, 2, 2]);
        let o = resolve_versus(&mut rng, 4, 10);
        assert_eq!(o.margin, 10);
        assert_eq!(hack().stacks_for(&o), 1 + 3); // base 1 + 10/3
    }

    #[test]
    fn a_whiff_lands_nothing() {
        // target = 21 + 0 − 20 = 1; 3d6 = 6 misses (and isn't a 3–4 crit).
        let mut rng = ScriptedRng::from_d6([2, 2, 2]);
        let o = resolve_versus(&mut rng, 0, 20);
        assert!(!o.success);
        assert_eq!(hack().stacks_for(&o), 0);
    }

    #[test]
    fn a_crit_lands_despite_the_wall_and_adds_a_stack() {
        let mut rng = ScriptedRng::from_d6([1, 1, 1]); // natural 3 → roll-under crit
        let o = resolve_versus(&mut rng, 0, 99); // hopeless target, but a crit lands
        assert!(o.crit && o.success);
        assert_eq!(hack().stacks_for(&o), 1 + 1); // base + crit bump (margin floored at 0)
    }
}
