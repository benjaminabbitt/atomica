//! Skills & chassis — the per-character roll-modifiers (`docs/stats.md`).
//!
//! **Skills are tiers *on* a governing attribute** (the rework): a unit's effective rating
//! at a skill = `governing attribute + skill tier` (untrained −3 … elite +2; competent 0).
//! The reworked roll is `3d6 ≤ (attribute + tier) × 2`. Every [`Chassis`] ships a low
//! baseline; XP-growth and chips build from there.

use crate::chargen::Stat;

/// A skill domain. Each is **governed by a primary attribute** ([`Skill::governs`]); the
/// effective rating is that attribute plus the unit's tier in the skill.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Skill {
    /// Hand-to-hand (Body).
    Melee,
    /// Ranged firearms (Dexterity).
    Gunnery,
    /// Netrunning (Intellect).
    Hacking,
    /// Field medicine (Intellect).
    Medical,
    /// **Active defense** — the opposed dodge roll (Dexterity).
    Evade,
    /// Moving unseen (Dexterity).
    Stealth,
    /// Heavy / support weapons (Body).
    Heavy,
    /// Gadgets, repair, demolitions (Intellect).
    Tech,
}

impl Skill {
    pub const ALL: [Skill; 8] = [
        Skill::Melee,
        Skill::Gunnery,
        Skill::Hacking,
        Skill::Medical,
        Skill::Evade,
        Skill::Stealth,
        Skill::Heavy,
        Skill::Tech,
    ];
    pub const COUNT: usize = Self::ALL.len();

    /// The **governing primary attribute** (`docs/stats.md`). Effective rating at this skill
    /// = this attribute + the unit's tier in it — so a high attribute lifts all its skills,
    /// and (e.g.) plating's −Dexterity drags every Dex skill down with it.
    pub fn governs(self) -> Stat {
        match self {
            Skill::Melee | Skill::Heavy => Stat::Body,
            Skill::Gunnery | Skill::Stealth | Skill::Evade => Stat::Dexterity,
            Skill::Hacking | Skill::Medical | Skill::Tech => Stat::Intellect,
        }
    }
}

/// A skill **proficiency tier** (`docs/stats.md`): a modifier on the governing attribute.
/// `competent` = your raw stat; training shifts ±; `untrained` is a steep −3.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SkillTier {
    Untrained,
    Exposed,
    Beginner,
    Competent,
    Expert,
    Elite,
}

impl SkillTier {
    /// The modifier this tier adds to the governing attribute (−3 … +2).
    pub fn modifier(self) -> i32 {
        match self {
            SkillTier::Untrained => -3,
            SkillTier::Exposed => -2,
            SkillTier::Beginner => -1,
            SkillTier::Competent => 0,
            SkillTier::Expert => 1,
            SkillTier::Elite => 2,
        }
    }
}

/// A unit's skill levels (one per [`Skill`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Skills {
    levels: [i32; Skill::COUNT],
}

impl Skills {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn level(&self, s: Skill) -> i32 {
        self.levels[s as usize]
    }

    pub fn set(&mut self, s: Skill, level: i32) -> &mut Self {
        self.levels[s as usize] = level;
        self
    }

    /// Builder-style set, for inline construction.
    pub fn with(mut self, s: Skill, level: i32) -> Self {
        self.levels[s as usize] = level;
        self
    }

    /// Builder: set a skill to a named proficiency [`SkillTier`] (the rework — the stored
    /// value is the tier modifier on the governing attribute).
    pub fn with_tier(mut self, s: Skill, tier: SkillTier) -> Self {
        self.levels[s as usize] = tier.modifier();
        self
    }

    /// Raise a skill (XP growth, §10).
    pub fn raise(&mut self, s: Skill, by: i32) {
        self.levels[s as usize] += by;
    }
}

/// The innate class (design §7J). Sets, among other things, the low skill floor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Chassis {
    Flesh,
    Augmented,
    Machine,
    Vehicle,
}

impl Chassis {
    /// Is this a **biological** body (flesh or augmented flesh) — a valid host for a
    /// **bio** contagion (a Virus)? `false` for pure `Machine` / `Vehicle` (no body to
    /// catch / spread it). Digital contagions key off Link, not this.
    pub fn is_biological(self) -> bool {
        matches!(self, Chassis::Flesh | Chassis::Augmented)
    }

    /// The chassis's **body coverage** — the fixed slice of the hit-location roll the *meat*
    /// (flesh / frame) occupies; each implant *extends* the domain on top (`combat.md` hit
    /// location). On a `1..100` scale (a plain body is `1..100`); a roll in this band wounds
    /// Integrity, beyond it strikes chrome. Bigger frames present more body.
    pub fn coverage(self) -> i32 {
        match self {
            Chassis::Flesh => 100,
            Chassis::Augmented => 90, // some of the meat is already chrome
            Chassis::Machine => 80,
            Chassis::Vehicle => 160, // a big target
        }
    }

    /// The **low** innate skill floor the chassis ships with (§10): a fresh
    /// recruit is competent-but-unremarkable; the gap to a veteran is earned.
    pub fn baseline_skills(self) -> Skills {
        match self {
            Chassis::Flesh => Skills::new().with(Skill::Melee, 2),
            Chassis::Augmented => Skills::new().with(Skill::Melee, 1).with(Skill::Hacking, 1),
            Chassis::Machine => Skills::new().with(Skill::Gunnery, 2),
            Chassis::Vehicle => Skills::new().with(Skill::Gunnery, 1),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skill_tiers_run_untrained_to_elite() {
        assert_eq!(SkillTier::Untrained.modifier(), -3);
        assert_eq!(SkillTier::Competent.modifier(), 0); // competent = your raw stat
        assert_eq!(SkillTier::Elite.modifier(), 2);
        // ...and they're a monotone ladder.
        let ladder: Vec<i32> = [
            SkillTier::Untrained,
            SkillTier::Exposed,
            SkillTier::Beginner,
            SkillTier::Competent,
            SkillTier::Expert,
            SkillTier::Elite,
        ]
        .map(SkillTier::modifier)
        .to_vec();
        assert_eq!(ladder, vec![-3, -2, -1, 0, 1, 2]);
    }

    #[test]
    fn each_skill_is_governed_by_an_attribute() {
        assert_eq!(Skill::Melee.governs(), Stat::Body);
        assert_eq!(Skill::Heavy.governs(), Stat::Body);
        assert_eq!(Skill::Gunnery.governs(), Stat::Dexterity);
        assert_eq!(Skill::Evade.governs(), Stat::Dexterity); // the opposed-defense skill
        assert_eq!(Skill::Hacking.governs(), Stat::Intellect);
        assert_eq!(Skill::Tech.governs(), Stat::Intellect);
    }

    #[test]
    fn chassis_baseline_is_low_and_native() {
        let flesh = Chassis::Flesh.baseline_skills();
        assert_eq!(flesh.level(Skill::Melee), 2); // native, low
        assert_eq!(flesh.level(Skill::Hacking), 0); // not its domain
        assert_eq!(Chassis::Machine.baseline_skills().level(Skill::Gunnery), 2);
    }

    #[test]
    fn skills_raise_from_the_floor() {
        let mut s = Chassis::Augmented.baseline_skills();
        assert_eq!(s.level(Skill::Hacking), 1);
        s.raise(Skill::Hacking, 3);
        assert_eq!(s.level(Skill::Hacking), 4);
    }
}
