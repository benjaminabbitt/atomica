//! The à-la-carte status pool, modeled on the design's **9-axis schema**:
//! `effect · trigger · timing · decay · stacking · magnitude · behavior ·
//! targeting · resist`.
//!
//! A [`StatusSpec`] is the *definition* (the nine axes); a [`Status`] is a live
//! instance on a unit (spec + current stacks/duration). Magnitudes are
//! placeholders — numbers are TBD per the design; this fixes the shapes.
//!
//! Honored by the tick loop so far: `Trigger::Tick`, `Timing::TickStart`, the
//! `Decay`/`Stacking`/`Behavior`/`Resist` axes, and the `Effect`s below. The
//! remaining `Trigger`/`Timing`/`Targeting` variants are declared to document the
//! schema and are extension points.

use crate::{PenTier, Unit};

/// Axis: magnitude — how an amount is computed from the target.
#[derive(Clone, Copy, Debug)]
pub enum Magnitude {
    /// Fixed amount.
    Flat(f32),
    /// Fraction of max Integrity (anti-tank — *can* kill).
    PctMax(f32),
    /// Fraction of current Integrity (softener — *never* kills).
    PctCurrent(f32),
}

impl Magnitude {
    pub fn amount(self, unit: &Unit) -> f32 {
        match self {
            Magnitude::Flat(a) => a,
            Magnitude::PctMax(p) => p * unit.max_integrity,
            Magnitude::PctCurrent(p) => p * unit.integrity,
        }
    }

    /// `PctCurrent` is the "softener" magnitude that must never land a kill.
    pub fn can_kill(self) -> bool {
        !matches!(self, Magnitude::PctCurrent(_))
    }
}

/// Axis: behavior — deterministic vs rolled each tick (the 3d6 contest, §13).
#[derive(Clone, Copy, Debug)]
pub enum Behavior {
    Deterministic,
    /// Rolled each tick: `3d6 + power (+ stacks)` vs the target's resist (the
    /// `resist` axis picks which stat is the TN). Fires on success.
    Stochastic { power: i32 },
}

/// Axis: resist — which defensive stat shifts a stochastic roll.
#[derive(Clone, Copy, Debug)]
pub enum Resist {
    None,
    Immunity,
    Firewall,
}

/// Axis: stacking — how a fresh application combines with an existing instance.
#[derive(Clone, Copy, Debug)]
pub enum Stacking {
    /// Keep one instance; refresh its duration.
    Refresh,
    /// Accumulate stacks up to a cap.
    Stack { max: u32 },
}

/// Axis: decay — how the status wears off.
#[derive(Clone, Copy, Debug)]
pub enum Decay {
    /// Lose one tick of duration per tick.
    Duration,
    /// Lose one stack per tick.
    Stacks,
}

/// Axis: trigger — what causes the effect to evaluate. (Only `Tick` is honored.)
#[derive(Clone, Copy, Debug)]
pub enum Trigger {
    Tick,
    OnHit,
    OnDeath,
}

/// Axis: timing — when within a tick the effect resolves. (Only `TickStart`.)
#[derive(Clone, Copy, Debug)]
pub enum Timing {
    TickStart,
    TickEnd,
}

/// Axis: targeting — who receives the status (resolved at application time).
#[derive(Clone, Copy, Debug)]
pub enum Targeting {
    SelfOnly,
    Enemy,
    Ally,
    Area,
}

/// Axis: effect — what the status actually does. Carries its own magnitude where
/// relevant (the magnitude axis).
#[derive(Clone, Copy, Debug)]
pub enum Effect {
    /// Damage over time through a penetration tier (Burn=Contact, Bleed/Poison=Internal).
    Dot { magnitude: Magnitude, pen: PenTier },
    /// Crash — the unit cannot act while this is present.
    Stun,
    /// Lag — multiply effective Initiative by this factor (`< 1.0` slows).
    Slow(f32),
    /// Breach — multiply incoming damage by this factor (`> 1.0`).
    Vuln(f32),
    /// Corrode / Plating-shred — reduce the Plating layer each tick.
    PlatingShred(Magnitude),
}

/// The definition of a status — one value per design axis.
#[derive(Clone, Copy, Debug)]
pub struct StatusSpec {
    pub name: &'static str,
    pub effect: Effect,
    pub trigger: Trigger,
    pub timing: Timing,
    pub decay: Decay,
    pub stacking: Stacking,
    pub behavior: Behavior,
    pub targeting: Targeting,
    pub resist: Resist,
}

/// A live status instance on a unit.
#[derive(Clone, Copy, Debug)]
pub struct Status {
    pub spec: StatusSpec,
    pub stacks: u32,
    /// Ticks remaining (used when `decay == Duration`).
    pub duration: u32,
}

// --- Named statuses (placeholder magnitudes — TBD) ---------------------------
//
// These are concrete points in the 9-axis space, one per design family member,
// to exercise the pipeline. Values are illustrative, not balanced.
impl StatusSpec {
    /// Burn — Contact DoT, deterministic, stacks.
    pub fn burn() -> Self {
        Self {
            name: "Burn",
            effect: Effect::Dot { magnitude: Magnitude::Flat(2.0), pen: PenTier::Contact },
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Duration,
            stacking: Stacking::Stack { max: 5 },
            behavior: Behavior::Deterministic,
            targeting: Targeting::Enemy,
            resist: Resist::None,
        }
    }

    /// Bleed — Internal DoT, deterministic, stacks.
    pub fn bleed() -> Self {
        Self {
            name: "Bleed",
            effect: Effect::Dot { magnitude: Magnitude::Flat(3.0), pen: PenTier::Internal },
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Stacks,
            stacking: Stacking::Stack { max: 8 },
            behavior: Behavior::Deterministic,
            targeting: Targeting::Enemy,
            resist: Resist::None,
        }
    }

    /// Poison — Internal DoT, *stochastic*, resisted by Immunity, no spread.
    pub fn poison() -> Self {
        Self {
            name: "Poison",
            effect: Effect::Dot { magnitude: Magnitude::PctCurrent(0.08), pen: PenTier::Internal },
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Duration,
            stacking: Stacking::Refresh,
            behavior: Behavior::Stochastic { power: 3 },
            targeting: Targeting::Enemy,
            resist: Resist::Immunity,
        }
    }

    /// Crash — stun.
    pub fn crash() -> Self {
        Self {
            name: "Crash",
            effect: Effect::Stun,
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Duration,
            stacking: Stacking::Refresh,
            behavior: Behavior::Deterministic,
            targeting: Targeting::Enemy,
            resist: Resist::Firewall,
        }
    }

    /// Lag — slow (halve initiative).
    pub fn lag() -> Self {
        Self {
            name: "Lag",
            effect: Effect::Slow(0.5),
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Duration,
            stacking: Stacking::Refresh,
            behavior: Behavior::Deterministic,
            targeting: Targeting::Enemy,
            resist: Resist::Firewall,
        }
    }

    /// Breach — vulnerability (incoming damage +50%).
    pub fn breach() -> Self {
        Self {
            name: "Breach",
            effect: Effect::Vuln(1.5),
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Duration,
            stacking: Stacking::Refresh,
            behavior: Behavior::Deterministic,
            targeting: Targeting::Enemy,
            resist: Resist::None,
        }
    }

    /// Lockware — a netrunner's landed payload (§7F/§10.8): a deployed intrusion
    /// that drains the system as an Internal DoT. The hack roll already contested
    /// Firewall on landing, so it ticks **deterministically**; stacks (scaled by
    /// the hack's margin) deepen the drain.
    pub fn lockware() -> Self {
        Self {
            name: "Lockware",
            effect: Effect::Dot { magnitude: Magnitude::Flat(2.0), pen: PenTier::Internal },
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Duration,
            stacking: Stacking::Stack { max: 6 },
            behavior: Behavior::Deterministic,
            targeting: Targeting::Enemy,
            resist: Resist::Firewall,
        }
    }

    /// Corrode — plating-shred DoT.
    pub fn corrode() -> Self {
        Self {
            name: "Corrode",
            effect: Effect::PlatingShred(Magnitude::Flat(2.0)),
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Stacks,
            stacking: Stacking::Stack { max: 6 },
            behavior: Behavior::Deterministic,
            targeting: Targeting::Enemy,
            resist: Resist::None,
        }
    }
}
