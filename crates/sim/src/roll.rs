//! The core contested-roll mechanic.
//!
//! **Core — `2d10` roll-*under* a target** (`docs/stats.md`): a check succeeds on `2d10 ≤
//! target`, where the `target` is the actor's effective skill (attribute + tier) less any
//! situational penalty — nothing is added to the dice. 2d10's flatter (triangular) curve
//! over 3d6 caps the extremes (no roll exceeds ~90%), so luck matters more per roll but the
//! balance is far less sensitive to any single stat. Attacks are **opposed**: the attacker
//! rolls to hit and the defender rolls an active defense ([`resolve_opposed`]); static
//! contests fold the defence in as a modifier ([`resolve_versus`]). RNG-injected: drive it
//! from a [`ScriptedRng`](crate::ScriptedRng).

use crate::RandomSource;

/// The result of a roll-under check ([`resolve_check`] / [`resolve_versus`] / a leg of
/// [`resolve_opposed`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RollOutcome {
    /// The raw 2d10 (`2..=20`) — what crit / fumble key off, and what the target is compared to.
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

/// Resolve a **roll-under** check: succeed on `2d10 ≤ target`. The core (`docs/stats.md`) —
/// the `target` is the actor's effective skill (attribute + tier) less any situational
/// penalty, so nothing is added to the dice. A natural **2–3** crits (auto-succeed), a
/// natural **19–20** fumbles (auto-fail); `margin = target − dice` (positive = the degree of
/// success). `total` carries the raw dice (no modifiers).
pub fn resolve_check<R: RandomSource + ?Sized>(rng: &mut R, target: i32) -> RollOutcome {
    let dice = rng.roll_2d10();
    let crit = dice <= 3; // 2d10: natural 2-3 ≈ 3%
    let fumble = dice >= 19; // natural 19-20 ≈ 3%
    let success = crit || (!fumble && dice <= target);
    RollOutcome { dice, total: dice, margin: target - dice, success, crit, fumble }
}

/// Resolve a **roll-under** skill check whose opposition is folded in as a **modifier** (the
/// GURPS pattern — no static TN): roll `2d10 ≤ rating − resist`, where `rating` is the actor's
/// effective skill (~10) and `resist` is the target's Firewall / Health / security rating as
/// a flat **penalty** (a few points), *not* a number to beat. `margin` is the degree of
/// success. (Active, two-sided defenses still go through [`resolve_opposed`].)
pub fn resolve_versus<R: RandomSource + ?Sized>(rng: &mut R, rating: i32, resist: i32) -> RollOutcome {
    resolve_check(rng, rating - resist)
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
/// hit (`2d10 ≤ attack`) and the defender rolls an **active defense** (`2d10 ≤ defense` — an
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
        // 2d10 = 10 vs target 10 → made it exactly (margin 0).
        let mut rng = ScriptedRng::from_d10([5, 5]);
        let o = resolve_check(&mut rng, 10);
        assert_eq!(o.dice, 10);
        assert_eq!(o.margin, 0);
        assert!(o.success && !o.crit && !o.fumble);
        // 2d10 = 11 vs target 10 → missed by 1.
        let mut rng = ScriptedRng::from_d10([5, 6]);
        let o = resolve_check(&mut rng, 10);
        assert!(!o.success);
        assert_eq!(o.margin, -1);
    }

    #[test]
    fn naturals_crit_low_and_fumble_high() {
        // Natural 3 crits even against an impossible (0) target...
        let mut rng = ScriptedRng::from_d10([1, 2]);
        let o = resolve_check(&mut rng, 0);
        assert!(o.dice == 3 && o.crit && o.success);
        // ...and a natural 19 fumbles even against a trivial (99) one.
        let mut rng = ScriptedRng::from_d10([10, 9]);
        let o = resolve_check(&mut rng, 99);
        assert!(o.dice == 19 && o.fumble && !o.success);
        // The 2d10 bands are 2–3 (crit) and 19–20 (fumble).
        let mut rng = ScriptedRng::from_d10([1, 1]); // 2
        assert!(resolve_check(&mut rng, 0).crit);
        let mut rng = ScriptedRng::from_d10([10, 10]); // 20
        assert!(resolve_check(&mut rng, 99).fumble);
    }

    #[test]
    fn versus_folds_resist_in_as_a_modifier() {
        // rating 14, resist 4 ⇒ target 14 − 4 = 10; 2d10 = 7 makes it by 3.
        let mut rng = ScriptedRng::from_d10([3, 4]);
        let o = resolve_versus(&mut rng, 14, 4);
        assert_eq!(o.margin, 3);
        assert!(o.success);
        // A stiffer resist drops the target below the same roll → a miss.
        let mut rng = ScriptedRng::from_d10([3, 4]);
        assert!(!resolve_versus(&mut rng, 14, 10).success); // target 4, 7 > 4
    }

    #[test]
    fn opposed_lands_only_when_the_attacker_beats_the_defense() {
        // Attacker rolls 8 ≤ 12 (hit); defender rolls 15 > 10 (fails to dodge) → lands.
        let mut rng = ScriptedRng::from_d10([4, 4, 7, 8]); // 8 then 15
        let o = resolve_opposed(&mut rng, 12, 10);
        assert!(o.attack.success && !o.defense.success && o.landed);
        // Same hit, but the defender rolls 7 ≤ 10 (dodges) → no blow.
        let mut rng = ScriptedRng::from_d10([4, 4, 3, 4]); // 8 then 7
        let o = resolve_opposed(&mut rng, 12, 10);
        assert!(o.attack.success && o.defense.success && !o.landed);
    }
}
