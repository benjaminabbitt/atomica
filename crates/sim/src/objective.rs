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
///
/// Objectives may be **stateful** (capture progress, an item picked up, a search): the
/// battle calls [`Objective::tick`] once a round to advance that state, and **latches** —
/// a success condition met at any tick stays met, so a courier can grab-and-go or a holder
/// can mop up the field afterwards without losing credit when the fight ends on a wipe.
pub trait Objective {
    /// Advance internal state for the round (called each tick, after movement/combat).
    /// Stateless objectives (Eliminate / Survive / Reach) need no override.
    fn tick(&mut self, _units: &[Unit], _tick: u32) {}

    fn status(&self, units: &[Unit], tick: u32, fight_over: bool) -> ObjectiveStatus;

    /// A **board hex the player should move toward** to make progress — the *current phase's*
    /// target (Reach / Hold / the item then the exit), or `None` for a placeless objective
    /// (Eliminate / Survive). The AI flows units onto it so it resolves in auto-play.
    fn focus(&self) -> Option<Hex> {
        None
    }

    /// What **share of the active squad** should pursue the [`focus`](Self::focus), as a
    /// percent — the nearest that many (at least 1) flow to it, the rest hold the line (so a
    /// fleeing enemy still gets hunted). A fraction of the *live* squad, so it scales down as
    /// units fall. Default 50%.
    fn seeker_pct(&self) -> u32 {
        50
    }
}

fn any_alive(units: &[Unit], team: Team) -> bool {
    units.iter().any(|u| u.is_alive() && u.team == team)
}

fn count_alive(units: &[Unit], team: Team) -> u32 {
    units.iter().filter(|u| u.is_alive() && u.team == team).count() as u32
}

/// Is a living unit of `team` standing on `hex`?
fn on_hex(units: &[Unit], team: Team, hex: Hex) -> bool {
    units.iter().any(|u| u.is_alive() && u.team == team && u.pos == hex)
}

/// Does the player **control** `hex` — a player on it, no living enemy sharing it?
fn controlled(units: &[Unit], hex: Hex) -> bool {
    on_hex(units, PLAYER, hex) && !on_hex(units, ENEMY, hex)
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
    fn seeker_pct(&self) -> u32 {
        25 // a courier or two — not the whole squad
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

/// **Capture / Hold**: take and keep the target `hex` — **controlled** when a player unit
/// occupies it and no living enemy shares it. **Latches** Achieved the first round it's
/// held *and* the round reaches `by_round` or the enemy is cleared — so the holder may then
/// leave to mop up without losing it. Fails if the player is wiped, or the fight ends
/// without ever holding it.
pub struct Hold {
    pub hex: Hex,
    pub by_round: u32,
    done: bool,
}
impl Hold {
    pub fn new(hex: Hex, by_round: u32) -> Self {
        Self { hex, by_round, done: false }
    }
}
impl Objective for Hold {
    fn tick(&mut self, units: &[Unit], tick: u32) {
        if controlled(units, self.hex) && (tick >= self.by_round || !any_alive(units, ENEMY)) {
            self.done = true;
        }
    }
    fn status(&self, units: &[Unit], _tick: u32, fight_over: bool) -> ObjectiveStatus {
        latched(self.done, units, fight_over)
    }
    fn focus(&self) -> Option<Hex> {
        Some(self.hex)
    }
}

/// The shared **latched** verdict for a stateful objective: once `done`, it stays Achieved;
/// otherwise it Fails on a wipe or a fight that ended unachieved, else stays Pending.
fn latched(done: bool, units: &[Unit], fight_over: bool) -> ObjectiveStatus {
    if done {
        ObjectiveStatus::Achieved
    } else if !any_alive(units, PLAYER) || fight_over {
        ObjectiveStatus::Failed
    } else {
        ObjectiveStatus::Pending
    }
}

/// **Capture and hold for N turns**: control `hex` for `turns` (cumulative) rounds. Each
/// round of control ticks the counter; clearing the enemy while you stand on it also seals
/// it. Latches once the quota is met.
pub struct CaptureHold {
    pub hex: Hex,
    pub turns: u32,
    held: u32,
    done: bool,
}
impl Objective for CaptureHold {
    fn tick(&mut self, units: &[Unit], _tick: u32) {
        if controlled(units, self.hex) {
            self.held += 1;
            if self.held >= self.turns || !any_alive(units, ENEMY) {
                self.done = true;
            }
        }
    }
    fn status(&self, units: &[Unit], _tick: u32, fight_over: bool) -> ObjectiveStatus {
        latched(self.done, units, fight_over)
    }
    fn focus(&self) -> Option<Hex> {
        Some(self.hex)
    }
}

/// **Capture the flag (sticky) and hold**: touch `hex` once and it's **captured for good**
/// (even if you're driven off); then hold it `turns` rounds (or clear the field) to seal
/// the win. The grab can't be undone — only the *hold* remains to finish.
pub struct CaptureFlag {
    pub hex: Hex,
    pub turns: u32,
    captured: bool,
    held: u32,
    done: bool,
}
impl Objective for CaptureFlag {
    fn tick(&mut self, units: &[Unit], _tick: u32) {
        if on_hex(units, PLAYER, self.hex) {
            self.captured = true; // sticky — once grabbed, always grabbed
        }
        if self.captured && controlled(units, self.hex) {
            self.held += 1;
        }
        if self.captured && (self.held >= self.turns || !any_alive(units, ENEMY)) {
            self.done = true;
        }
    }
    fn status(&self, units: &[Unit], _tick: u32, fight_over: bool) -> ObjectiveStatus {
        latched(self.done, units, fight_over)
    }
    fn focus(&self) -> Option<Hex> {
        Some(self.hex)
    }
}

/// **Get the item and escape**: a two-phase courier run — reach the `item` hex (picked up,
/// sticky), then carry it to the `exit`. Latches on extraction, so wiping the enemy first
/// isn't required (and a runner can dash out under fire). The focus moves: the item, then
/// the exit.
pub struct Extract {
    pub item: Hex,
    pub exit: Hex,
    has_item: bool,
    done: bool,
}
impl Objective for Extract {
    fn tick(&mut self, units: &[Unit], _tick: u32) {
        if on_hex(units, PLAYER, self.item) {
            self.has_item = true;
        }
        if self.has_item && on_hex(units, PLAYER, self.exit) {
            self.done = true;
        }
    }
    fn status(&self, units: &[Unit], _tick: u32, fight_over: bool) -> ObjectiveStatus {
        latched(self.done, units, fight_over)
    }
    fn focus(&self) -> Option<Hex> {
        Some(if self.has_item { self.exit } else { self.item })
    }
    fn seeker_pct(&self) -> u32 {
        25 // a lone courier grabs and runs; the rest screen
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
    /// Take and hold `hex`, sealed at round `by_round` or on a clear ([`Hold`]).
    Hold(Hex, u32),
    /// Control `hex` for `turns` cumulative rounds ([`CaptureHold`]).
    CaptureHold(Hex, u32),
    /// Touch `hex` (sticky capture) then hold `turns` rounds ([`CaptureFlag`]).
    Flag(Hex, u32),
    /// Grab the item at `item`, then carry it to `exit` ([`Extract`]).
    Extract { item: Hex, exit: Hex },
}

impl ObjectiveKind {
    /// Construct the boxed [`Objective`] for a fresh battle (fresh state per battle).
    pub fn build(self) -> Box<dyn Objective> {
        match self {
            ObjectiveKind::Eliminate => Box::new(WinFight),
            ObjectiveKind::Survive(rounds) => Box::new(Survive { rounds }),
            ObjectiveKind::Reach(hex) => Box::new(Reach { hex }),
            ObjectiveKind::Hold(hex, by_round) => Box::new(Hold::new(hex, by_round)),
            ObjectiveKind::CaptureHold(hex, turns) => {
                Box::new(CaptureHold { hex, turns, held: 0, done: false })
            }
            ObjectiveKind::Flag(hex, turns) => {
                Box::new(CaptureFlag { hex, turns, captured: false, held: 0, done: false })
            }
            ObjectiveKind::Extract { item, exit } => {
                Box::new(Extract { item, exit, has_item: false, done: false })
            }
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

    /// Advance every goal's internal state for the round (capture progress, item pickup).
    pub fn tick(&mut self, units: &[Unit], tick: u32) {
        for g in &mut self.goals {
            g.objective.tick(units, tick);
        }
    }

    /// The status of every goal at the current state.
    pub fn report(&self, units: &[Unit], tick: u32, fight_over: bool) -> Vec<ObjectiveStatus> {
        self.goals.iter().map(|g| g.objective.status(units, tick, fight_over)).collect()
    }

    /// The board hex the player should flow toward and **what share of the squad** should —
    /// the first **unmet** positional goal's focus + its seeker percent, or `None`. Drives
    /// nearest-N objective-seeking movement in the sim (N = that percent of the live squad).
    pub fn focus(&self, units: &[Unit], tick: u32, fight_over: bool) -> Option<(Hex, u32)> {
        self.goals.iter().find_map(|g| {
            if g.objective.status(units, tick, fight_over) == ObjectiveStatus::Achieved {
                return None;
            }
            g.objective.focus().map(|h| (h, g.objective.seeker_pct()))
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
