//! **Structured combat events** — a typed, deterministic record of *what happened*
//! each tick: the substrate for logging, replay, and UI.
//!
//! The sim deliberately carries **no logging dependency** (it must stay pure and
//! deterministic — see the crate docs); it only *produces* these structured records.
//! A consumer routes them however it likes: render a human trace, forward each to a
//! `tracing` / JSON **structured-logging** backend (match on the typed fields), or
//! assert on them in a test. Capture is **opt-in** ([`Battle::with_log`]) and off by
//! default, so a normal auto-resolve pays nothing.

use crate::{DamageType, Hex, Outcome, UnitId};
use std::fmt;

/// How a breach reached its victim (`docs/cyberware.md` §6).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BreachVector {
    /// A netrunner's hack.
    Hack,
    /// A physical EMP pulse.
    Emp,
    /// A worm / logic-bomb.
    Worm,
}

impl BreachVector {
    fn as_str(self) -> &'static str {
        match self {
            BreachVector::Hack => "hack",
            BreachVector::Emp => "emp",
            BreachVector::Worm => "worm",
        }
    }
}

/// One thing that happened in a battle — **structured**, with typed fields a logger can
/// key on. Carries no formatting of its own: [`CombatEvent::kind`] is a stable machine
/// name (for filtering / routing) and [`fmt::Display`] gives a human line.
#[derive(Clone, Debug, PartialEq)]
pub enum CombatEvent {
    /// A unit stepped one hex toward its goal.
    Moved { unit: UnitId, from: Hex, to: Hex },
    /// A weapon struck a target. `amount` is the **Integrity** actually lost (after
    /// armor / pools), `killed` whether the blow was lethal.
    Attacked { attacker: UnitId, target: UnitId, dtype: DamageType, amount: f32, killed: bool },
    /// A hack resolved — the contest `success` / `crit` / `margin`.
    Hacked { attacker: UnitId, target: UnitId, success: bool, crit: bool, margin: i32 },
    /// An implant was breached (disabled), tagged with the vector that did it.
    Breached { target: UnitId, vector: BreachVector },
    /// A contagion jumped to a fresh victim (`family` = its corruption tag).
    Spread { from: UnitId, to: UnitId, family: &'static str },
    /// A unit died.
    Died { unit: UnitId },
    /// The battle reached a terminal [`Outcome`].
    Ended { outcome: Outcome },
}

impl CombatEvent {
    /// A stable machine name — the event's "type" for structured-log filtering / routing.
    pub fn kind(&self) -> &'static str {
        match self {
            CombatEvent::Moved { .. } => "moved",
            CombatEvent::Attacked { .. } => "attacked",
            CombatEvent::Hacked { .. } => "hacked",
            CombatEvent::Breached { .. } => "breached",
            CombatEvent::Spread { .. } => "spread",
            CombatEvent::Died { .. } => "died",
            CombatEvent::Ended { .. } => "ended",
        }
    }
}

impl fmt::Display for CombatEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CombatEvent::Moved { unit, from, to } => {
                write!(f, "#{} moved ({},{})→({},{})", unit.0, from.q, from.r, to.q, to.r)
            }
            CombatEvent::Attacked { attacker, target, dtype, amount, killed } => {
                write!(f, "#{} hit #{} for {amount:.1} {dtype:?}", attacker.0, target.0)?;
                if *killed {
                    write!(f, " (killed)")?;
                }
                Ok(())
            }
            CombatEvent::Hacked { attacker, target, success, crit, margin } => {
                let verb = if *crit {
                    "crit-hacked"
                } else if *success {
                    "hacked"
                } else {
                    "failed to hack"
                };
                write!(f, "#{} {verb} #{} (margin {margin})", attacker.0, target.0)
            }
            CombatEvent::Breached { target, vector } => {
                write!(f, "#{}'s implant breached ({})", target.0, vector.as_str())
            }
            CombatEvent::Spread { from, to, family } => {
                write!(f, "{family} spread #{}→#{}", from.0, to.0)
            }
            CombatEvent::Died { unit } => write!(f, "#{} died", unit.0),
            CombatEvent::Ended { outcome } => write!(f, "battle ended: {outcome:?}"),
        }
    }
}

/// A captured [`CombatEvent`] tagged with the tick it fired on.
#[derive(Clone, Debug, PartialEq)]
pub struct Record {
    pub tick: u32,
    pub event: CombatEvent,
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "t{:<3} {}", self.tick, self.event)
    }
}

/// The battle's **event log** — opt-in capture of the structured trace. Off by default
/// (`push` is a no-op, so emitting costs only the small enum value); enable it with
/// [`Battle::with_log`] and read the trace via [`Battle::events`].
#[derive(Default)]
pub struct EventLog {
    enabled: bool,
    records: Vec<Record>,
}

impl EventLog {
    /// Turn on capture.
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Is capture on?
    pub fn enabled(&self) -> bool {
        self.enabled
    }

    /// The captured trace, oldest first.
    pub fn records(&self) -> &[Record] {
        &self.records
    }

    /// Record `event` at `tick` (a no-op while disabled).
    pub fn push(&mut self, tick: u32, event: CombatEvent) {
        if self.enabled {
            self.records.push(Record { tick, event });
        }
    }
}
