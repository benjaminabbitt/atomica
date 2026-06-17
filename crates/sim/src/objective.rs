//! Scored objectives (design-delta §9, §13).
//!
//! A Flight carries a **list** of objectives and can meet **any number** of them
//! independently — the node's standard [`WinFight`] (awarded for winning the
//! fight) plus any bonus goals (survive / reach / margin-loss / time-attack).
//! Rewards (run-layer) are *commensurate* with what's achieved.
//!
//! The standard fight still drives *termination* (see [`Battle::outcome`]);
//! objectives are scored alongside. They are pure functions of state — no RNG.

use crate::{Hex, Team, Unit};

/// The player's side by convention; the enemy is [`Team::B`].
pub const PLAYER: Team = Team::A;
const ENEMY: Team = Team::B;

/// Whether a scored objective has been met.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ObjectiveStatus {
    Pending,
    Achieved,
    Failed,
}

/// A scored achievement on a Flight.
pub trait Objective {
    /// Evaluate against the state. `fight_over` = the standard fight has ended
    /// (one army wiped / cap), so deadline-style goals can resolve.
    fn status(&self, units: &[Unit], tick: u32, fight_over: bool) -> ObjectiveStatus;
}

fn any_alive(units: &[Unit], team: Team) -> bool {
    units.iter().any(|u| u.is_alive() && u.team == team)
}

fn count_alive(units: &[Unit], team: Team) -> u32 {
    units.iter().filter(|u| u.is_alive() && u.team == team).count() as u32
}

/// The node's baseline: **win the standard fight** (wipe the enemy).
pub struct WinFight;
impl Objective for WinFight {
    fn status(&self, units: &[Unit], _tick: u32, fight_over: bool) -> ObjectiveStatus {
        match (any_alive(units, PLAYER), any_alive(units, ENEMY)) {
            (true, false) => ObjectiveStatus::Achieved, // enemy wiped → won
            (false, _) => ObjectiveStatus::Failed,      // player wiped → lost
            (true, true) if fight_over => ObjectiveStatus::Failed, // timed out, both alive
            _ => ObjectiveStatus::Pending,
        }
    }
}

/// Last `rounds` ticks with a unit still standing.
pub struct Survive {
    pub rounds: u32,
}
impl Objective for Survive {
    fn status(&self, units: &[Unit], tick: u32, fight_over: bool) -> ObjectiveStatus {
        if any_alive(units, PLAYER) && tick >= self.rounds {
            ObjectiveStatus::Achieved
        } else if !any_alive(units, PLAYER) || fight_over {
            ObjectiveStatus::Failed // died, or the fight ended before the deadline
        } else {
            ObjectiveStatus::Pending
        }
    }
}

/// Get a unit onto `hex` (extract / heist / reach a spot) before the fight ends.
pub struct Reach {
    pub hex: Hex,
}
impl Objective for Reach {
    fn status(&self, units: &[Unit], _tick: u32, fight_over: bool) -> ObjectiveStatus {
        let reached =
            units.iter().any(|u| u.is_alive() && u.team == PLAYER && u.pos == self.hex);
        if reached {
            ObjectiveStatus::Achieved
        } else if fight_over {
            ObjectiveStatus::Failed
        } else {
            ObjectiveStatus::Pending
        }
    }
}

/// Take the dive: **lose**, but leave the enemy at no more than `max_enemy_survivors`.
pub struct MarginLoss {
    pub max_enemy_survivors: u32,
}
impl Objective for MarginLoss {
    fn status(&self, units: &[Unit], _tick: u32, _fight_over: bool) -> ObjectiveStatus {
        if any_alive(units, PLAYER) {
            // You're meant to lose — winning the fight outright fails it.
            return if any_alive(units, ENEMY) {
                ObjectiveStatus::Pending
            } else {
                ObjectiveStatus::Failed
            };
        }
        if count_alive(units, ENEMY) <= self.max_enemy_survivors {
            ObjectiveStatus::Achieved // a convincing loss
        } else {
            ObjectiveStatus::Failed // lost too badly
        }
    }
}

/// Win the standard fight by round `by_round`.
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
