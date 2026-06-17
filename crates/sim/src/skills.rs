//! Skills & chassis — the per-character roll-modifiers (design-delta §10).
//!
//! *Skills attack, stats defend* (§13): a skill is the additive bonus on a
//! [`resolve_contest`](crate::resolve_contest) roll. Every [`Chassis`] ships a
//! **low baseline** in its native skills; XP-growth and chips build from there.

/// A skill domain. The relevant one modifies a contested roll.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Skill {
    Melee,
    Gunnery,
    Hacking,
    Medical,
}

impl Skill {
    pub const ALL: [Skill; 4] = [Skill::Melee, Skill::Gunnery, Skill::Hacking, Skill::Medical];
    pub const COUNT: usize = Self::ALL.len();
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
