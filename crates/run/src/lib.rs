//! `atomica-run` — the roguelike **run** layer: a persistent roster marching
//! through a sequence of battles.
//!
//! **Scope (first pass): the fighting spine.** A [`Run`] carries the player's
//! **roster** across an ordered list of [`Encounter`]s, building and resolving
//! each as an [`atomica_sim::Battle`], banking the survivors, and tracking
//! casualties + win/loss. It is **deterministic** (seeded → reproducible, like
//! the sim it drives).
//!
//! What lives here vs the `sim`: the sim owns a *single* battle (units, ticks,
//! the dice); the run owns the **persistence between battles** — who survived,
//! who's gone, and whether the run is over. The navigation **tree**, economy /
//! Rep, Jobs, and shops are later layers; this is the combat loop they hang on.
//!
//! Rules of the spine (first pass):
//! - **Permadeath** — a unit that falls is removed from the roster for good.
//! - **The run ends only on a wipe** — losing units while still fielding an army
//!   advances you (you can bleed the roster across a winning run).
//! - **Rest between battles** — survivors redeploy at full Integrity with their
//!   chrome **repaired** (the downtime heal; the casualty / extraction economy of
//!   the design §9.4 is a later layer).

use atomica_sim::{Battle, Hex, Outcome, Team, Unit, UnitId};

/// Hard cap on ticks per battle (matches the sim's draw fallback).
const MAX_TICKS: u32 = 1000;
/// Depth column the enemy force deploys on (player on column 0).
const ENEMY_COLUMN: i32 = 8;

/// One planned battle: the enemy force the roster faces. (Team is assigned at
/// deploy time, so build the enemies however you like.)
pub struct Encounter {
    pub name: String,
    pub enemies: Vec<Unit>,
}

impl Encounter {
    pub fn new(name: impl Into<String>, enemies: Vec<Unit>) -> Self {
        Self { name: name.into(), enemies }
    }
}

/// Where the run stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RunOutcome {
    Ongoing,
    Won,
    Lost,
}

/// What one resolved battle did to the roster.
#[derive(Clone, Debug)]
pub struct BattleReport {
    pub encounter: String,
    /// The sim verdict (who held the field).
    pub outcome: Outcome,
    /// Names of the roster units lost this battle (permadeath).
    pub losses: Vec<String>,
    /// How many roster units remain.
    pub survivors: usize,
}

/// A run in progress: a persistent player **roster** marching through an ordered
/// list of [`Encounter`]s.
pub struct Run {
    roster: Vec<Unit>,
    encounters: Vec<Encounter>,
    index: usize,
    seed: u64,
    outcome: RunOutcome,
}

impl Run {
    /// Start a run. An empty roster is an instant loss; no encounters is an
    /// instant win.
    pub fn new(roster: Vec<Unit>, encounters: Vec<Encounter>, seed: u64) -> Self {
        let outcome = if roster.is_empty() {
            RunOutcome::Lost
        } else if encounters.is_empty() {
            RunOutcome::Won
        } else {
            RunOutcome::Ongoing
        };
        Self { roster, encounters, index: 0, seed, outcome }
    }

    pub fn outcome(&self) -> RunOutcome {
        self.outcome
    }

    /// The surviving roster (raw post-battle state; deployment refreshes it).
    pub fn roster(&self) -> &[Unit] {
        &self.roster
    }

    /// Index of the next unfought encounter.
    pub fn position(&self) -> usize {
        self.index
    }

    /// Resolve the **next** encounter: deploy the surviving roster against the
    /// enemy force, run the battle to completion (seeded → deterministic), bank
    /// the survivors (permadeath for the fallen), and advance — or end the run on
    /// a wipe. Returns the report, or `None` if the run is already over.
    pub fn fight_next(&mut self) -> Option<BattleReport> {
        if self.outcome != RunOutcome::Ongoing {
            return None;
        }
        let mut battle = self.build_battle(&self.encounters[self.index]);
        let outcome = battle.resolve(MAX_TICKS);

        let before: Vec<String> = self.roster.iter().map(|u| u.name.clone()).collect();
        let survivors: Vec<Unit> =
            battle.units.into_iter().filter(|u| u.team == Team::A && u.is_alive()).collect();
        let alive: std::collections::HashSet<&str> =
            survivors.iter().map(|u| u.name.as_str()).collect();
        let losses: Vec<String> =
            before.into_iter().filter(|n| !alive.contains(n.as_str())).collect();

        self.roster = survivors;
        let report = BattleReport {
            encounter: self.encounters[self.index].name.clone(),
            outcome,
            losses,
            survivors: self.roster.len(),
        };

        if self.roster.is_empty() {
            self.outcome = RunOutcome::Lost;
        } else {
            self.index += 1;
            if self.index >= self.encounters.len() {
                self.outcome = RunOutcome::Won;
            }
        }
        Some(report)
    }

    /// Fight through to the end (Won or Lost), collecting every report.
    pub fn resolve(&mut self) -> Vec<BattleReport> {
        let mut reports = Vec::new();
        while let Some(r) = self.fight_next() {
            reports.push(r);
        }
        reports
    }

    /// Build the sim battle: the roster on Team A, the encounter's enemies on
    /// Team B, deployed in opposing columns with fresh state and unique ids. The
    /// per-encounter seed is derived from the run seed so battles stay reproducible.
    fn build_battle(&self, encounter: &Encounter) -> Battle {
        let mut units = Vec::with_capacity(self.roster.len() + encounter.enemies.len());
        let mut next_id = 0u32;
        for (row, t) in self.roster.iter().enumerate() {
            units.push(deploy(t, &mut next_id, Team::A, Hex::new(0, row as i32)));
        }
        for (row, e) in encounter.enemies.iter().enumerate() {
            units.push(deploy(e, &mut next_id, Team::B, Hex::new(ENEMY_COLUMN, row as i32)));
        }
        let seed = self.seed ^ (self.index as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        Battle::new(units, seed)
    }
}

/// Clone a roster template into a fresh battle combatant: unique id, the given
/// team + position, full Integrity, repaired chrome, no statuses (the between-
/// battle rest).
fn deploy(template: &Unit, next_id: &mut u32, team: Team, pos: Hex) -> Unit {
    let mut u = template.clone();
    u.id = UnitId(*next_id);
    *next_id += 1;
    u.team = team;
    u.pos = pos;
    for idx in 0..u.implants.len() {
        u.repair_implant(idx); // un-brick between battles (Destroyed stays gone)
    }
    u.integrity = u.max_integrity;
    u.statuses.clear();
    u.alive = true;
    u
}

#[cfg(test)]
mod tests {
    use super::*;
    use atomica_sim::{Attack, Chassis, DamageType, PenTier};

    /// A melee combatant with an armor-bypassing hit (deterministic damage).
    fn fighter(name: &str, damage: f32, hp: f32, initiative: f32) -> Unit {
        Unit::new(0, name, Team::A, Chassis::Augmented)
            .with_integrity(hp)
            .with_initiative(initiative)
            .with_attack(Attack {
                damage,
                dtype: DamageType::Piercing,
                pen: PenTier::Internal,
                range: 1,
                emp: false,
            })
    }

    #[test]
    fn an_empty_roster_is_an_instant_loss() {
        let run = Run::new(vec![], vec![Encounter::new("X", vec![fighter("F", 5.0, 10.0, 5.0)])], 1);
        assert_eq!(run.outcome(), RunOutcome::Lost);
    }

    #[test]
    fn a_strong_roster_clears_the_run() {
        let roster = vec![fighter("Ace", 20.0, 80.0, 8.0), fighter("Bolt", 20.0, 80.0, 7.0)];
        let encounters = vec![
            Encounter::new("Gate", vec![fighter("Thug", 4.0, 18.0, 3.0)]),
            Encounter::new("Boss", vec![fighter("Heavy", 6.0, 28.0, 4.0)]),
        ];
        let mut run = Run::new(roster, encounters, 1);
        let reports = run.resolve();
        assert_eq!(run.outcome(), RunOutcome::Won);
        assert_eq!(reports.len(), 2);
        assert!(reports.iter().all(|r| matches!(r.outcome, Outcome::Winner(Team::A))));
        assert_eq!(run.roster().len(), 2); // crushed weak foes without a loss
    }

    #[test]
    fn a_weak_roster_is_wiped() {
        let roster = vec![fighter("Rookie", 3.0, 12.0, 4.0)];
        let encounters = vec![Encounter::new("Ambush", vec![fighter("Killer", 30.0, 120.0, 9.0)])];
        let mut run = Run::new(roster, encounters, 7);
        run.resolve();
        assert_eq!(run.outcome(), RunOutcome::Lost);
        assert!(run.roster().is_empty());
    }

    #[test]
    fn casualty_bookkeeping_reconciles() {
        // However the battle falls out, survivors + losses == the roster we sent.
        let roster =
            vec![fighter("A", 6.0, 20.0, 6.0), fighter("B", 6.0, 20.0, 5.0), fighter("C", 6.0, 20.0, 4.0)];
        let n = roster.len();
        let foe = vec![fighter("Brute", 18.0, 90.0, 9.0)];
        let mut run = Run::new(roster, vec![Encounter::new("Grinder", foe)], 5);
        let report = run.fight_next().unwrap();
        assert_eq!(report.survivors + report.losses.len(), n);
        assert_eq!(run.roster().len(), report.survivors);
    }

    #[test]
    fn fighting_past_the_end_returns_none() {
        let mut run = Run::new(
            vec![fighter("Solo", 30.0, 60.0, 9.0)],
            vec![Encounter::new("Only", vec![fighter("Mook", 2.0, 8.0, 1.0)])],
            2,
        );
        assert!(run.fight_next().is_some());
        assert_eq!(run.outcome(), RunOutcome::Won);
        assert!(run.fight_next().is_none()); // run's over — nothing more to fight
    }

    #[test]
    fn the_run_is_deterministic() {
        let setup = || {
            Run::new(
                vec![fighter("A", 12.0, 40.0, 6.0), fighter("B", 11.0, 40.0, 5.0)],
                vec![
                    Encounter::new("E1", vec![fighter("X", 12.0, 40.0, 7.0)]),
                    Encounter::new("E2", vec![fighter("Y", 14.0, 55.0, 6.0)]),
                ],
                42,
            )
        };
        let take = || {
            let mut r = setup();
            let reports = r.resolve();
            let losses: Vec<Vec<String>> = reports.iter().map(|x| x.losses.clone()).collect();
            (r.outcome(), r.roster().len(), losses)
        };
        assert_eq!(take(), take());
    }
}
