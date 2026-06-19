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

/// A contested check: the actor's `skill + equipment` vs. the defender's (or
/// task's) Target Number.
#[derive(Clone, Copy, Debug)]
pub struct Contest {
    pub skill: i32,
    pub equipment: i32,
    pub tn: i32,
}

impl Contest {
    pub fn new(skill: i32, equipment: i32, tn: i32) -> Self {
        Self { skill, equipment, tn }
    }
}

/// The result of a [`resolve_contest`] roll.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct RollOutcome {
    /// The raw 3d6 (`3..=18`), before modifiers — what crit / fumble key off.
    pub dice: i32,
    /// `3d6 + skill + equipment`.
    pub total: i32,
    /// `total − TN`. Positive = succeeded by this much (the **degree of success**
    /// that scales the effect); negative = failed by this much.
    pub margin: i32,
    pub success: bool,
    pub crit: bool,
    pub fumble: bool,
}

/// Resolve `3d6 + skill + equipment` vs. `tn`.
///
/// Defaults (design-delta §13): succeed on **≥ TN**; a natural **18** auto-succeeds
/// (crit) and a natural **3** auto-fails (fumble), regardless of the modified total.
pub fn resolve_contest<R: RandomSource + ?Sized>(rng: &mut R, c: Contest) -> RollOutcome {
    let dice = rng.roll_3d6();
    let total = dice + c.skill + c.equipment;
    let margin = total - c.tn;
    let crit = dice == 18;
    let fumble = dice == 3;
    let success = crit || (!fumble && total >= c.tn);
    RollOutcome { dice, total, margin, success, crit, fumble }
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
    fn natural_eighteen_crits_even_against_a_wall() {
        let mut rng = ScriptedRng::from_d6([6, 6, 6]);
        let o = resolve_contest(&mut rng, Contest::new(0, 0, 99));
        assert_eq!(o.dice, 18);
        assert!(o.crit && o.success);
    }

    #[test]
    fn natural_three_fumbles_even_with_huge_mods() {
        let mut rng = ScriptedRng::from_d6([1, 1, 1]);
        let o = resolve_contest(&mut rng, Contest::new(50, 50, 1));
        assert_eq!(o.dice, 3);
        assert!(o.fumble && !o.success);
    }

    #[test]
    fn margin_is_the_degree_of_success() {
        // 3d6 = 9, +4 skill +2 equip = 15 vs TN 12 → margin +3, plain success.
        let mut rng = ScriptedRng::from_d6([3, 3, 3]);
        let o = resolve_contest(&mut rng, Contest::new(4, 2, 12));
        assert_eq!(o.total, 15);
        assert_eq!(o.margin, 3);
        assert!(o.success && !o.crit && !o.fumble);
    }

    #[test]
    fn ties_succeed() {
        // 3d6 = 10, +2 = 12 vs TN 12 → margin 0, success (≥ TN).
        let mut rng = ScriptedRng::from_d6([4, 3, 3]);
        let o = resolve_contest(&mut rng, Contest::new(2, 0, 12));
        assert_eq!(o.margin, 0);
        assert!(o.success);
    }

    #[test]
    fn skill_turns_a_loss_into_a_win() {
        // Same dice (3d6 = 6); skill is what clears the TN.
        let mut weak_rng = ScriptedRng::from_d6([2, 2, 2]);
        let weak = resolve_contest(&mut weak_rng, Contest::new(0, 0, 12));
        let mut skilled_rng = ScriptedRng::from_d6([2, 2, 2]);
        let skilled = resolve_contest(&mut skilled_rng, Contest::new(6, 0, 12));
        assert!(!weak.success && skilled.success);
    }

    // -- The reworked roll-under core --------------------------------------------------

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
