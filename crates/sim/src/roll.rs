//! The core contested-roll mechanic: **`3d6 + skill + equipment` vs. a Target
//! Number** (design-delta §13).
//!
//! *Skills attack, stats defend* — the TN is the defender's resist (Firewall /
//! Immunity / control-resist) or a task difficulty, never a bonus to your own
//! roll. Pure and RNG-injected: drive it from a [`ScriptedRng`](crate::ScriptedRng)
//! to force any roll.

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
}
