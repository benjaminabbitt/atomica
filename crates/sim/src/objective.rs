//! The injected win-condition seam (design-delta §9, §13).
//!
//! The orchestrator asks an [`Objective`] for the player-side outcome each tick,
//! so **Job Flights** (survive / reach / margin-loss / time-attack) reuse *one*
//! seam instead of a hardcoded "eliminate the enemy". Objectives are pure
//! functions of state — trivially testable, no RNG.

use crate::{Hex, Outcome, Team, Unit};

/// The player's side by convention; the enemy is [`Team::B`].
pub const PLAYER: Team = Team::A;
const ENEMY: Team = Team::B;

/// A battle's win-condition. `evaluate` returns the **player-side** outcome:
/// `Winner(PLAYER)` = objective met, `Winner(ENEMY)` = failed, plus `Draw` /
/// `Ongoing`.
pub trait Objective {
    fn evaluate(&self, units: &[Unit], tick: u32) -> Outcome;
}

fn any_alive(units: &[Unit], team: Team) -> bool {
    units.iter().any(|u| u.is_alive() && u.team == team)
}

fn count_alive(units: &[Unit], team: Team) -> u32 {
    units.iter().filter(|u| u.is_alive() && u.team == team).count() as u32
}

/// Wipe the enemy army — the standard Flight.
pub struct Eliminate;
impl Objective for Eliminate {
    fn evaluate(&self, units: &[Unit], _tick: u32) -> Outcome {
        match (any_alive(units, PLAYER), any_alive(units, ENEMY)) {
            (true, false) => Outcome::Winner(PLAYER),
            (false, true) => Outcome::Winner(ENEMY),
            (false, false) => Outcome::Draw,
            (true, true) => Outcome::Ongoing,
        }
    }
}

/// Last `rounds` ticks with a unit still standing.
pub struct Survive {
    pub rounds: u32,
}
impl Objective for Survive {
    fn evaluate(&self, units: &[Unit], tick: u32) -> Outcome {
        if !any_alive(units, PLAYER) {
            return Outcome::Winner(ENEMY);
        }
        if tick >= self.rounds {
            Outcome::Winner(PLAYER)
        } else {
            Outcome::Ongoing
        }
    }
}

/// Get a unit onto `hex` by round `by_round` (extract / heist / reach a spot).
pub struct Reach {
    pub hex: Hex,
    pub by_round: u32,
}
impl Objective for Reach {
    fn evaluate(&self, units: &[Unit], tick: u32) -> Outcome {
        let reached =
            units.iter().any(|u| u.is_alive() && u.team == PLAYER && u.pos == self.hex);
        if reached {
            return Outcome::Winner(PLAYER);
        }
        if !any_alive(units, PLAYER) || tick >= self.by_round {
            return Outcome::Winner(ENEMY);
        }
        Outcome::Ongoing
    }
}

/// Take the dive: **lose**, but leave the enemy at no more than `max_enemy_survivors`.
pub struct MarginLoss {
    pub max_enemy_survivors: u32,
}
impl Objective for MarginLoss {
    fn evaluate(&self, units: &[Unit], _tick: u32) -> Outcome {
        if any_alive(units, PLAYER) {
            // You're meant to lose — winning the fight outright fails the job.
            return if any_alive(units, ENEMY) {
                Outcome::Ongoing
            } else {
                Outcome::Winner(ENEMY)
            };
        }
        if count_alive(units, ENEMY) <= self.max_enemy_survivors {
            Outcome::Winner(PLAYER) // a convincing loss — job met
        } else {
            Outcome::Winner(ENEMY) // lost too badly
        }
    }
}

/// Eliminate the enemy by round `by_round`, else fail.
pub struct TimeAttack {
    pub by_round: u32,
}
impl Objective for TimeAttack {
    fn evaluate(&self, units: &[Unit], tick: u32) -> Outcome {
        match Eliminate.evaluate(units, tick) {
            Outcome::Ongoing if tick >= self.by_round => Outcome::Winner(ENEMY),
            other => other,
        }
    }
}
