//! Scored objectives (design-delta §9, §13).
//!
//! A battle carries a list of [`Goal`]s in an [`Objectives`] container that tallies
//! **winnings** (rewards from met goals) and **losses** (penalties from failed
//! ones) and surfaces what's still **unachieved**. A fight can meet any number.
//!
//! [`ObjectiveStatus`] is *satisfied* (`Pending` or `Achieved`) until a goal's
//! explicit **fail condition** is met — being merely unachieved is not a failure.
//! The standard fight still drives *termination* (see [`Battle::outcome`]).

use crate::{Hex, Team, Unit};

/// The player's side by convention; the enemy is [`Team::B`].
pub const PLAYER: Team = Team::A;
const ENEMY: Team = Team::B;

/// Whether a scored objective has been met. *Satisfied* = not failed.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectiveStatus {
    /// Not yet achieved, but its fail condition hasn't fired either.
    Pending,
    Achieved,
    Failed,
}

impl ObjectiveStatus {
    /// `true` unless the goal has actually **failed** — an unachieved (`Pending`)
    /// goal is still satisfied.
    pub fn is_satisfied(self) -> bool {
        !matches!(self, ObjectiveStatus::Failed)
    }
}

/// A scored achievement. Returns `Failed` *only* when its fail condition is met.
pub trait Objective {
    fn status(&self, units: &[Unit], tick: u32, fight_over: bool) -> ObjectiveStatus;

    /// A **board hex the player should move toward** to make progress (Reach / Hold), or
    /// `None` for objectives with no position (Eliminate / Survive). The AI flows units
    /// onto it so positional objectives actually resolve in auto-play.
    fn focus(&self) -> Option<Hex> {
        None
    }
}

fn any_alive(units: &[Unit], team: Team) -> bool {
    units.iter().any(|u| u.is_alive() && u.team == team)
}

fn count_alive(units: &[Unit], team: Team) -> u32 {
    units.iter().filter(|u| u.is_alive() && u.team == team).count() as u32
}

/// The simple objective: **win the standard fight** (wipe the enemy). Its fail
/// condition is the **fight ending without victory** (a loss or a draw).
pub struct WinFight;
impl Objective for WinFight {
    fn status(&self, units: &[Unit], _tick: u32, fight_over: bool) -> ObjectiveStatus {
        match (any_alive(units, PLAYER), any_alive(units, ENEMY)) {
            (true, false) => ObjectiveStatus::Achieved, // victory
            _ if fight_over => ObjectiveStatus::Failed, // fight ended without victory
            _ => ObjectiveStatus::Pending,
        }
    }
}

/// Survive to round `rounds`. Fails only if the player is wiped.
pub struct Survive {
    pub rounds: u32,
}
impl Objective for Survive {
    fn status(&self, units: &[Unit], tick: u32, _fight_over: bool) -> ObjectiveStatus {
        if !any_alive(units, PLAYER) {
            ObjectiveStatus::Failed
        } else if tick >= self.rounds {
            ObjectiveStatus::Achieved
        } else {
            ObjectiveStatus::Pending
        }
    }
}

/// Get a unit onto `hex` (extract / heist / reach a spot). No fail condition —
/// you either reach it or it stays unachieved.
pub struct Reach {
    pub hex: Hex,
}
impl Objective for Reach {
    fn status(&self, units: &[Unit], _tick: u32, _fight_over: bool) -> ObjectiveStatus {
        if units.iter().any(|u| u.is_alive() && u.team == PLAYER && u.pos == self.hex) {
            ObjectiveStatus::Achieved
        } else {
            ObjectiveStatus::Pending
        }
    }
    fn focus(&self) -> Option<Hex> {
        Some(self.hex)
    }
}

/// Take the dive: **lose**, but leave the enemy at no more than `max_enemy_survivors`.
/// Fails if you win outright (botched the dive) or lose too badly.
pub struct MarginLoss {
    pub max_enemy_survivors: u32,
}
impl Objective for MarginLoss {
    fn status(&self, units: &[Unit], _tick: u32, _fight_over: bool) -> ObjectiveStatus {
        if any_alive(units, PLAYER) {
            return if any_alive(units, ENEMY) {
                ObjectiveStatus::Pending
            } else {
                ObjectiveStatus::Failed // won outright — botched the dive
            };
        }
        if count_alive(units, ENEMY) <= self.max_enemy_survivors {
            ObjectiveStatus::Achieved
        } else {
            ObjectiveStatus::Failed // lost too badly
        }
    }
}

/// Win the standard fight by round `by_round`. Fails on a loss or running late.
pub struct TimeAttack {
    pub by_round: u32,
}
impl Objective for TimeAttack {
    fn status(&self, units: &[Unit], tick: u32, _fight_over: bool) -> ObjectiveStatus {
        let won = any_alive(units, PLAYER) && !any_alive(units, ENEMY);
        if won && tick <= self.by_round {
            ObjectiveStatus::Achieved
        } else if !any_alive(units, PLAYER) || tick > self.by_round {
            ObjectiveStatus::Failed
        } else {
            ObjectiveStatus::Pending
        }
    }
}

/// **Capture / Hold**: take and keep the target `hex` — **controlled** when a
/// player unit occupies it and no living enemy shares it. Achieved once held *and*
/// either the round reaches `by_round` or the enemy is cleared. Fails if the
/// player is wiped, or the fight ends without control (you must take and hold it).
pub struct Hold {
    pub hex: Hex,
    pub by_round: u32,
}
impl Objective for Hold {
    fn status(&self, units: &[Unit], tick: u32, fight_over: bool) -> ObjectiveStatus {
        if !any_alive(units, PLAYER) {
            return ObjectiveStatus::Failed;
        }
        let on_hex = |team: Team| {
            units.iter().any(|u| u.is_alive() && u.team == team && u.pos == self.hex)
        };
        let controlled = on_hex(PLAYER) && !on_hex(ENEMY);
        let cleared = !any_alive(units, ENEMY);
        if controlled && (tick >= self.by_round || cleared) {
            ObjectiveStatus::Achieved
        } else if fight_over {
            ObjectiveStatus::Failed // the fight ended and you never held it
        } else {
            ObjectiveStatus::Pending
        }
    }
    fn focus(&self) -> Option<Hex> {
        Some(self.hex)
    }
}

/// A Clone-able **objective descriptor** — built into a boxed [`Objective`] when a
/// battle starts. The run layer carries one of these on each encounter (the trait
/// objects themselves aren't `Clone`, so this is the portable spec).
#[derive(Clone, Copy, Debug)]
pub enum ObjectiveKind {
    /// Wipe the enemy — the standard fight ([`WinFight`]).
    Eliminate,
    /// Last until round `rounds` ([`Survive`]).
    Survive(u32),
    /// Get a unit onto `hex` ([`Reach`]).
    Reach(Hex),
    /// Take and hold `hex` by round `1` ([`Hold`]).
    Hold(Hex, u32),
}

impl ObjectiveKind {
    /// Construct the boxed [`Objective`] for a fresh battle.
    pub fn build(self) -> Box<dyn Objective> {
        match self {
            ObjectiveKind::Eliminate => Box::new(WinFight),
            ObjectiveKind::Survive(rounds) => Box::new(Survive { rounds }),
            ObjectiveKind::Reach(hex) => Box::new(Reach { hex }),
            ObjectiveKind::Hold(hex, by_round) => Box::new(Hold { hex, by_round }),
        }
    }
}

/// An [`Objective`] paired with its stake: `reward` on `Achieved`, `penalty` on `Failed`.
pub struct Goal {
    pub objective: Box<dyn Objective>,
    pub reward: i32,
    pub penalty: i32,
}

impl Goal {
    pub fn new(objective: Box<dyn Objective>, reward: i32, penalty: i32) -> Self {
        Self { objective, reward, penalty }
    }
}

/// A battle's scored objectives: tally winnings / losses and surface the unmet.
#[derive(Default)]
pub struct Objectives {
    goals: Vec<Goal>,
}

impl Objectives {
    pub fn new(goals: Vec<Goal>) -> Self {
        Self { goals }
    }

    /// The status of every goal at the current state.
    pub fn report(&self, units: &[Unit], tick: u32, fight_over: bool) -> Vec<ObjectiveStatus> {
        self.goals.iter().map(|g| g.objective.status(units, tick, fight_over)).collect()
    }

    /// The board hex the player should flow toward — the first **unmet** positional goal's
    /// focus (Reach / Hold), or `None`. Drives objective-seeking movement in the sim.
    pub fn focus(&self, units: &[Unit], tick: u32, fight_over: bool) -> Option<Hex> {
        self.goals.iter().find_map(|g| {
            (g.objective.status(units, tick, fight_over) != ObjectiveStatus::Achieved)
                .then(|| g.objective.focus())
                .flatten()
        })
    }

    /// Sum of rewards from **achieved** goals.
    pub fn winnings(&self, units: &[Unit], tick: u32, fight_over: bool) -> i32 {
        self.tally(units, tick, fight_over, ObjectiveStatus::Achieved, |g| g.reward)
    }

    /// Sum of penalties from **failed** goals.
    pub fn losses(&self, units: &[Unit], tick: u32, fight_over: bool) -> i32 {
        self.tally(units, tick, fight_over, ObjectiveStatus::Failed, |g| g.penalty)
    }

    /// Net score: winnings − losses.
    pub fn net(&self, units: &[Unit], tick: u32, fight_over: bool) -> i32 {
        self.winnings(units, tick, fight_over) - self.losses(units, tick, fight_over)
    }

    /// Indices of goals still **pending** — unachieved, but not failed.
    pub fn unachieved(&self, units: &[Unit], tick: u32, fight_over: bool) -> Vec<usize> {
        self.goals
            .iter()
            .enumerate()
            .filter(|(_, g)| g.objective.status(units, tick, fight_over) == ObjectiveStatus::Pending)
            .map(|(i, _)| i)
            .collect()
    }

    fn tally(
        &self,
        units: &[Unit],
        tick: u32,
        fight_over: bool,
        want: ObjectiveStatus,
        value: impl Fn(&Goal) -> i32,
    ) -> i32 {
        self.goals
            .iter()
            .filter(|g| g.objective.status(units, tick, fight_over) == want)
            .map(value)
            .sum()
    }
}
