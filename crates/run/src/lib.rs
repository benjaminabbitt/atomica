//! `atomica-run` — the roguelike progression layers above a single battle.
//!
//! **Three nested tiers, each a series of the one below:**
//!
//! - **[`Encounter`]** — one combat, resolved as an [`atomica_sim::Battle`].
//! - **[`Run`]** — a *series of encounters* fought as an **attrition gauntlet**:
//!   **no R&R within a run**, so Integrity damage and chrome condition persist
//!   from one combat to the next. (Every run is a series; some have length one —
//!   a single encounter is just a run of one.)
//! - **[`Game`]** — a *series of runs* with **R&R between them** (full heal +
//!   chrome repair). The campaign: gauntlet, recover, gauntlet, …
//!
//! All three are **deterministic** (seeded → reproducible, like the sim they
//! drive) and share the spine's rules: **permadeath** (the fallen are gone for
//! good) and **end-on-wipe** (you advance as long as you still field an army,
//! bleeding the roster as you go). The navigation **tree**, economy / Rep, Jobs,
//! shops, and async match-making are later layers on top of this combat
//! progression.

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

/// A planned **run**: a *named* series of [`Encounter`]s (length ≥ 1) — the unit
/// of work a [`Game`] plays, resting between one and the next. (Every run is a
/// series; some have length one.)
pub struct RunPlan {
    pub name: String,
    pub encounters: Vec<Encounter>,
}

impl RunPlan {
    pub fn new(name: impl Into<String>, encounters: Vec<Encounter>) -> Self {
        Self { name: name.into(), encounters }
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

/// What one played run did — the named run plus its per-combat [`BattleReport`]s.
#[derive(Clone, Debug)]
pub struct RunReport {
    pub run: String,
    pub battles: Vec<BattleReport>,
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

    /// Consume the run and take its surviving roster (their post-gauntlet state —
    /// carried damage and chrome condition intact). The [`Game`] tier applies R&R
    /// to these before the next run.
    pub fn into_survivors(self) -> Vec<Unit> {
        self.roster
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

/// Where a [`Game`] stands.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameOutcome {
    Ongoing,
    Won,
    Lost,
}

/// A whole game — the campaign: a persistent roster carried through a **series of
/// runs**, with **R&R between runs** (full heal + chrome repair). Each run is an
/// attrition gauntlet (no R&R *within*); the game is the series of gauntlets.
///
/// Mirrors [`Run`] one tier up: a run is a series of [`Encounter`]s with no rest
/// between; a game is a series of runs *with* rest between. Ends `Lost` when a run
/// wipes the army, `Won` when every run is cleared.
pub struct Game {
    roster: Vec<Unit>,
    runs: Vec<RunPlan>,
    index: usize,
    seed: u64,
    outcome: GameOutcome,
}

impl Game {
    /// Start a game from a roster and the ordered [`RunPlan`]s to play. An empty
    /// roster is an instant loss; no runs is an instant win.
    pub fn new(roster: Vec<Unit>, runs: Vec<RunPlan>, seed: u64) -> Self {
        let outcome = if roster.is_empty() {
            GameOutcome::Lost
        } else if runs.is_empty() {
            GameOutcome::Won
        } else {
            GameOutcome::Ongoing
        };
        Self { roster, runs, index: 0, seed, outcome }
    }

    pub fn outcome(&self) -> GameOutcome {
        self.outcome
    }

    /// The persistent roster (rested between runs, worn within them).
    pub fn roster(&self) -> &[Unit] {
        &self.roster
    }

    /// Index of the next unplayed run.
    pub fn position(&self) -> usize {
        self.index
    }

    /// Play the next **run** to its end (an attrition gauntlet, no rest within),
    /// then — if the army survives — **R&R** (full heal + chrome repair) before the
    /// next run. Returns the named [`RunReport`], or `None` if the game is over.
    pub fn play_run(&mut self) -> Option<RunReport> {
        if self.outcome != GameOutcome::Ongoing {
            return None;
        }
        let name = self.runs[self.index].name.clone();
        let encounters = std::mem::take(&mut self.runs[self.index].encounters);
        let roster = std::mem::take(&mut self.roster);
        let run_seed = self.seed ^ (self.index as u64).wrapping_mul(0xD1B5_4A32_D192_ED03);
        let mut run = Run::new(roster, encounters, run_seed);
        let battles = run.resolve();

        let mut survivors = run.into_survivors();
        if survivors.is_empty() {
            self.outcome = GameOutcome::Lost;
        } else {
            rest_and_recuperate(&mut survivors); // R&R between runs
            self.index += 1;
            if self.index >= self.runs.len() {
                self.outcome = GameOutcome::Won;
            }
        }
        self.roster = survivors;
        Some(RunReport { run: name, battles })
    }

    /// Play through to the end (Won or Lost), collecting each run's report.
    pub fn play(&mut self) -> Vec<RunReport> {
        let mut all = Vec::new();
        while let Some(r) = self.play_run() {
            all.push(r);
        }
        all
    }
}

/// **R&R between runs** (the meta-tier rest): survivors heal to full Integrity and
/// the Ripperdoc repairs their chrome — Degraded / Offline implants come back
/// Online (Destroyed stays gone, terminal). Transient statuses clear.
fn rest_and_recuperate(roster: &mut [Unit]) {
    for u in roster.iter_mut() {
        for idx in 0..u.implants.len() {
            u.repair_implant(idx);
        }
        u.integrity = u.max_integrity;
        u.statuses.clear();
    }
}

/// Clone a roster template into a fresh battle combatant: unique id, the given
/// team + position, and cleared transient statuses. **No R&R within a run** —
/// carried Integrity damage and chrome condition persist across the sequence
/// (healing / repair is between *runs*, the meta tier).
fn deploy(template: &Unit, next_id: &mut u32, team: Team, pos: Hex) -> Unit {
    let mut u = template.clone();
    u.id = UnitId(*next_id);
    *next_id += 1;
    u.team = team;
    u.pos = pos;
    u.statuses.clear(); // transient combat effects don't carry between combats
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
    fn damage_persists_across_combats_no_rnr() {
        // A unit wounded in combat 1 enters combat 2 still hurt — no rest heals it.
        let roster = vec![fighter("Vet", 9.0, 50.0, 6.0)];
        let encounters = vec![
            Encounter::new("E1", vec![fighter("Foe1", 8.0, 22.0, 5.0)]),
            Encounter::new("E2", vec![fighter("Foe2", 8.0, 22.0, 5.0)]),
        ];
        let mut run = Run::new(roster, encounters, 11);
        run.fight_next().unwrap(); // combat 1
        let after_1 = run.roster()[0].integrity;
        assert!(after_1 < 50.0); // took damage and carries it (deploy no longer heals)
        run.fight_next().unwrap(); // combat 2 — fought on from the wounded state
        assert_eq!(run.outcome(), RunOutcome::Won);
        assert!(run.roster()[0].integrity < after_1); // even more worn — attrition
    }

    #[test]
    fn rnr_heals_between_runs() {
        // Contrast with within-run attrition: a unit worn down in run 1 returns to
        // full Integrity for run 2 — R&R happens *between* runs.
        let roster = vec![fighter("Vet", 9.0, 50.0, 6.0)];
        let runs = vec![
            RunPlan::new("First", vec![Encounter::new("R1", vec![fighter("F1", 8.0, 22.0, 5.0)])]),
            RunPlan::new("Second", vec![Encounter::new("R2", vec![fighter("F2", 4.0, 10.0, 3.0)])]),
        ];
        let mut game = Game::new(roster, runs, 11);
        game.play_run().unwrap(); // run 1 wounds the Vet...
        assert_eq!(game.roster()[0].integrity, 50.0); // ...but R&R restored it before run 2
        game.play_run().unwrap();
        assert_eq!(game.outcome(), GameOutcome::Won);
    }

    #[test]
    fn a_run_may_be_a_single_encounter() {
        // Every run is a series; some have length one.
        let roster = vec![fighter("Solo", 30.0, 60.0, 9.0)];
        let runs =
            vec![RunPlan::new("Sortie", vec![Encounter::new("OneShot", vec![fighter("Mook", 2.0, 8.0, 1.0)])])];
        let mut game = Game::new(roster, runs, 2);
        let report = game.play_run().unwrap();
        assert_eq!(report.run, "Sortie"); // the wrapper's name flows through
        assert_eq!(report.battles.len(), 1); // a single combat
        assert_eq!(game.outcome(), GameOutcome::Won);
    }

    #[test]
    fn a_game_is_lost_when_a_run_wipes_the_army() {
        let roster = vec![fighter("Rookie", 3.0, 12.0, 4.0)];
        let runs =
            vec![RunPlan::new("Doomed", vec![Encounter::new("Doom", vec![fighter("Killer", 30.0, 120.0, 9.0)])])];
        let mut game = Game::new(roster, runs, 7);
        game.play();
        assert_eq!(game.outcome(), GameOutcome::Lost);
        assert!(game.roster().is_empty());
    }

    #[test]
    fn the_game_is_deterministic() {
        let setup = || {
            Game::new(
                vec![fighter("A", 12.0, 40.0, 6.0), fighter("B", 11.0, 40.0, 5.0)],
                vec![
                    RunPlan::new("One", vec![Encounter::new("R1E1", vec![fighter("X", 12.0, 40.0, 7.0)])]),
                    RunPlan::new(
                        "Two",
                        vec![
                            Encounter::new("R2E1", vec![fighter("Y", 10.0, 30.0, 6.0)]),
                            Encounter::new("R2E2", vec![fighter("Z", 12.0, 45.0, 6.0)]),
                        ],
                    ),
                ],
                42,
            )
        };
        let take = || {
            let mut g = setup();
            g.play();
            (g.outcome(), g.roster().len())
        };
        assert_eq!(take(), take());
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
