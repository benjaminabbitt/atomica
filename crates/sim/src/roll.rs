//! The core contested-roll mechanic.
//!
//! **Reworked core — `3d6` roll-*under* a target** (`docs/stats.md`): a check succeeds on
//! `3d6 ≤ target`, where the `target` already folds in `(stat + skill-tier) × 2` and any
//! situational penalty — nothing is added to the dice. Attacks are **opposed**: the
//! attacker rolls to hit and the defender rolls an active defense ([`resolve_opposed`]).
//!
//! The legacy **roll-high** `3d6 + skill vs TN` ([`resolve_contest`]) is still wired into
//! combat / netrunning during the migration and will be retired once every site moves to
//! the roll-under core. RNG-injected: drive it from a [`ScriptedRng`](crate::ScriptedRng).

use crate::RandomSource;

/// The result of a roll-under check ([`resolve_check`] / [`resolve_versus`] / a leg of
/// [`resolve_opposed`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RollOutcome {
    /// The raw 3d6 (`3..=18`) — what crit / fumble key off, and what the target is compared to.
    pub dice: i32,
    /// The raw dice again (roll-under adds nothing to the dice) — kept for log symmetry.
    pub total: i32,
    /// `target − dice`. Positive = made it by this much (the **degree of success** that scales
    /// the effect); negative = failed by this much.
    pub margin: i32,
    pub success: bool,
    pub crit: bool,
    pub fumble: bool,
}

/// Resolve a **roll-under** check: succeed on `3d6 ≤ target`. The reworked core
/// (`docs/stats.md`) — the `target` already folds in `(stat + skill-tier) × 2` and any
/// situational penalty, so nothing is added to the dice. A natural **3–4** crits
/// (auto-succeed), a natural **17–18** fumbles (auto-fail); `margin = target − dice`
/// (positive = the degree of success). `total` carries the raw dice (no modifiers).
pub fn resolve_check<R: RandomSource + ?Sized>(rng: &mut R, target: i32) -> RollOutcome {
    let dice = rng.roll_3d6();
    let crit = dice <= 4;
    let fumble = dice >= 17;
    let success = crit || (!fumble && dice <= target);
    RollOutcome { dice, total: dice, margin: target - dice, success, crit, fumble }
}

/// 3d6 spans `3..=18`; `MIN + MAX = 21` is the pivot that converts a roll-*high* contest to
/// the odds-identical roll-*under* target — the bridge for static-TN (non-opposed) checks.
const DICE_PIVOT: i32 = 21;

/// Resolve a **roll-under** skill-vs-resist contest: `rating` (a skill / virulence / status
/// power) tries to overcome a static `resist` TN (Firewall / Immunity / task difficulty — a
/// passive threshold, *not* an active defender; opposed defenses go through [`resolve_opposed`]).
/// Odds-identical to the old `3d6 + rating ≥ resist`, recast roll-under so the whole engine
/// speaks one dice language; `margin` is the roll-under degree of success.
pub fn resolve_versus<R: RandomSource + ?Sized>(rng: &mut R, rating: i32, resist: i32) -> RollOutcome {
    resolve_check(rng, DICE_PIVOT + rating - resist)
}

/// The result of an [`resolve_opposed`] attack-vs-defense exchange.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Opposed {
    /// The attacker's to-hit check (vs the attack target).
    pub attack: RollOutcome,
    /// The defender's active-defense check (vs the defense target).
    pub defense: RollOutcome,
    /// Did the blow land — attacker **succeeded** *and* defender **failed** to defend?
    pub landed: bool,
}

/// Resolve an **opposed** exchange (the reworked combat resolution): the attacker rolls to
/// hit (`3d6 ≤ attack`) and the defender rolls an **active defense** (`3d6 ≤ defense` — an
/// Evade / Parry / Block, or Firewall vs a hack). The blow lands only if the attacker
/// **succeeds and the defender fails**. The attacker rolls first (deterministic order);
/// both rolls are returned for logging.
pub fn resolve_opposed<R: RandomSource + ?Sized>(rng: &mut R, attack: i32, defense: i32) -> Opposed {
    let attack = resolve_check(rng, attack);
    let defense = resolve_check(rng, defense);
    let landed = attack.success && !defense.success;
    Opposed { attack, defense, landed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ScriptedRng;

    #[test]
    fn roll_under_succeeds_at_or_below_target() {
        // 3d6 = 10 vs target 10 → made it exactly (margin 0).
        let mut rng = ScriptedRng::from_d6([4, 3, 3]);
        let o = resolve_check(&mut rng, 10);
        assert_eq!(o.dice, 10);
        assert_eq!(o.margin, 0);
        assert!(o.success && !o.crit && !o.fumble);
        // 3d6 = 11 vs target 10 → missed by 1.
        let mut rng = ScriptedRng::from_d6([5, 3, 3]);
        let o = resolve_check(&mut rng, 10);
        assert!(!o.success);
        assert_eq!(o.margin, -1);
    }

    #[test]
    fn naturals_crit_low_and_fumble_high() {
        // Natural 3 crits even against an impossible (0) target...
        let mut rng = ScriptedRng::from_d6([1, 1, 1]);
        let o = resolve_check(&mut rng, 0);
        assert!(o.dice == 3 && o.crit && o.success);
        // ...and a natural 18 fumbles even against a trivial (99) one.
        let mut rng = ScriptedRng::from_d6([6, 6, 6]);
        let o = resolve_check(&mut rng, 99);
        assert!(o.dice == 18 && o.fumble && !o.success);
        // The bands are 3–4 (crit) and 17–18 (fumble).
        let mut rng = ScriptedRng::from_d6([2, 1, 1]); // 4
        assert!(resolve_check(&mut rng, 0).crit);
        let mut rng = ScriptedRng::from_d6([6, 6, 5]); // 17
        assert!(resolve_check(&mut rng, 99).fumble);
    }

    #[test]
    fn versus_recasts_a_skill_vs_resist_contest_roll_under() {
        // rating 4 vs resist 10 ⇒ target 21 + 4 − 10 = 15; 3d6 = 12 makes it by 3.
        let mut rng = ScriptedRng::from_d6([4, 4, 4]);
        let o = resolve_versus(&mut rng, 4, 10);
        assert_eq!(o.margin, 3);
        assert!(o.success);
        // A stiffer resist drops the target below the same roll → a miss.
        let mut rng = ScriptedRng::from_d6([4, 4, 4]);
        assert!(!resolve_versus(&mut rng, 4, 20).success); // target 5, 12 > 5
    }

    #[test]
    fn opposed_lands_only_when_the_attacker_beats_the_defense() {
        // Attacker rolls 8 ≤ 12 (hit); defender rolls 15 > 10 (fails to dodge) → lands.
        let mut rng = ScriptedRng::from_d6([3, 3, 2, 6, 6, 3]); // 8 then 15
        let o = resolve_opposed(&mut rng, 12, 10);
        assert!(o.attack.success && !o.defense.success && o.landed);
        // Same hit, but the defender rolls 7 ≤ 10 (dodges) → no blow.
        let mut rng = ScriptedRng::from_d6([3, 3, 2, 3, 2, 2]); // 8 then 7
        let o = resolve_opposed(&mut rng, 12, 10);
        assert!(o.attack.success && o.defense.success && !o.landed);
    }
}
