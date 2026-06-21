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

use crate::chargen::{
    Amount as GenAmount, Decorator, Event, Expiration, Factor, Flag, HookEffect, Resist as GenResist,
    Stat, Tag, Wear,
};
use crate::PenTier;

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
    /// Project onto the layer architecture's [`GenAmount`] (`docs/layers.md` L2b).
    pub fn to_gen(self) -> GenAmount {
        match self {
            Magnitude::Flat(a) => GenAmount::Flat(a),
            Magnitude::PctMax(p) => GenAmount::PctMax(p),
            Magnitude::PctCurrent(p) => GenAmount::PctCurrent(p),
        }
    }

    /// `PctCurrent` is the "softener" magnitude that must never land a kill.
    pub fn can_kill(self) -> bool {
        !matches!(self, Magnitude::PctCurrent(_))
    }
}

/// Axis: behavior — deterministic vs rolled each tick (the 2d10 contest, §13).
#[derive(Clone, Copy, Debug)]
pub enum Behavior {
    Deterministic,
    /// Rolled each tick: `2d10 ≤ power (+ stacks) − resist` (the
    /// `resist` axis picks which stat is the TN). Fires on success.
    Stochastic { power: i32 },
}

/// Axis: resist — which defensive stat shifts a stochastic roll.
#[derive(Clone, Copy, Debug)]
pub enum Resist {
    None,
    Health,
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

// A live status is now a **decorator** on the unit's `character` (`docs/layers.md`
// L2b): [`StatusSpec::to_decorator`] projects the spec, and [`Character::apply_status`]
// installs it with stacking-merge. The spec is the definition; the decorator the
// instance.

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

    /// Overheat — a hack's **thermal** payload (`netrunning.md`): forcing the system cooks the
    /// chrome from the inside, an **Internal** DoT (bypasses armor) that stacks with the breach
    /// margin. Deterministic, unresisted — this is netrunning's *damage*.
    pub fn overheat() -> Self {
        Self {
            name: "Overheat",
            effect: Effect::Dot { magnitude: Magnitude::Flat(2.0), pen: PenTier::Internal },
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Duration,
            stacking: Stacking::Stack { max: 5 },
            behavior: Behavior::Deterministic,
            targeting: Targeting::Enemy,
            resist: Resist::None,
        }
    }

    /// Meltdown — a hack's **heavy** thermal payload (`netrunning.md`): a premium burn program
    /// that melts a system far harder than [`Self::overheat`] (Flat 4 vs 2, Internal — straight
    /// to Integrity), the runner's *finisher* DoT. Like Overheat it's deterministic and unresisted
    /// (the breach already paid the Firewall) and stacks with the breach margin.
    pub fn meltdown() -> Self {
        Self {
            name: "Meltdown",
            effect: Effect::Dot { magnitude: Magnitude::Flat(4.0), pen: PenTier::Internal },
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Duration,
            stacking: Stacking::Stack { max: 5 },
            behavior: Behavior::Deterministic,
            targeting: Targeting::Enemy,
            resist: Resist::None,
        }
    }

    /// Poison — Internal DoT, *stochastic*, resisted by Health, no spread.
    pub fn poison() -> Self {
        Self {
            name: "Poison",
            effect: Effect::Dot { magnitude: Magnitude::PctCurrent(0.08), pen: PenTier::Internal },
            trigger: Trigger::Tick,
            timing: Timing::TickStart,
            decay: Decay::Duration,
            stacking: Stacking::Refresh,
            behavior: Behavior::Stochastic { power: 10 }, // GURPS-scaled affliction potency
            targeting: Targeting::Enemy,
            resist: Resist::Health,
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

    /// Project this status onto the layer architecture as a decaying [`Decorator`]
    /// (`docs/layers.md` L2b): the `effect` becomes either a `TickStart` **hook** (a
    /// DoT / shred — the active face) or a passive **flag**/**factor** (Stun / Vuln /
    /// Slow), the `decay` axis becomes the decorator's [`GenDecay`], and `stacks` /
    /// `duration` seed its lifetime.
    ///
    /// What does *not* port here (still the loop's job): the `behavior` axis's
    /// **stochastic** resist-roll (Poison gates on a 2d10-vs-resist roll that needs the
    /// RNG seam), `stacking` merge-on-reapply, and `targeting` — these enter when the
    /// live loop adopts the `Character` path.
    pub fn to_decorator(&self, stacks: u32, duration: u32) -> Decorator {
        let wear = match self.decay {
            Decay::Duration => Wear::ByDuration,
            Decay::Stacks => Wear::ByStacks,
        };
        let expiration = match self.decay {
            Decay::Duration => Expiration::Duration(duration),
            Decay::Stacks => Expiration::Permanent, // lifetime is the stack count
        };
        // Statuses are debuffs by default (the pool the loop applies to enemies). The
        // name is the decorator's merge / display label.
        let mut d = Decorator::status(Tag::Debuff, stacks, expiration, wear).with_label(self.name);
        match self.effect {
            Effect::Dot { magnitude, pen } => {
                d = d.with_hook(
                    Event::TickStart,
                    HookEffect::Damage {
                        amount: magnitude.to_gen(),
                        pen,
                        can_kill: magnitude.can_kill(),
                    },
                );
            }
            Effect::PlatingShred(mag) => {
                d = d.with_hook(
                    Event::TickStart,
                    HookEffect::ShredPlating { amount: mag.to_gen() },
                );
            }
            Effect::Stun => d = d.with_flag(Flag::Stun),
            Effect::Vuln(f) => d = d.with_flag(Flag::Vuln(f)),
            // Lag: a `More` factor on Initiative (×f); products with other slows.
            Effect::Slow(f) => d.factors.push(Factor::more(Stat::Initiative, f - 1.0)),
        }
        // The `behavior` axis: a stochastic status gates its hooks on a per-tick roll.
        if let Behavior::Stochastic { power } = self.behavior {
            let resist = match self.resist {
                Resist::None => GenResist::None,
                Resist::Firewall => GenResist::Firewall,
                Resist::Health => GenResist::Health,
            };
            d = d.with_gate(power, resist);
        }
        d
    }
}

#[cfg(test)]
mod l2b_tests {
    use super::*;
    use crate::chargen::{BaseLine, Character, Event};

    fn chassis() -> BaseLine {
        BaseLine {
            initiative: 6.0,
            body: 5.0, // Body 5 ⇒ 30 max Integrity
            plating: 10.0,
            ..Default::default()
        }
    }

    #[test]
    fn burn_is_a_tickstart_dot_against_the_pools() {
        // Burn: Contact DoT, Flat 2/stack. 3 stacks → 6 per tick, hits Plating first.
        let mut c = Character::new(chassis());
        c.install(StatusSpec::burn().to_decorator(3, 4));
        let r = c.dispatch(Event::TickStart, 1, &mut crate::SplitMix64::new(0));
        assert_eq!(r.len(), 1);
        assert_eq!(c.plating, 4.0); // 10 - (2*3) Contact
        assert_eq!(c.integrity, 30.0); // soaked by plating
    }

    #[test]
    fn bleed_decays_by_stacks_and_wears_off() {
        // Bleed: Internal DoT, Flat 3/stack, decays by stacks.
        let mut c = Character::new(chassis());
        c.install(StatusSpec::bleed().to_decorator(2, 0));
        c.dispatch(Event::TickStart, 1, &mut crate::SplitMix64::new(0));
        assert_eq!(c.integrity, 24.0); // 30 - 3*2, Internal straight through
        c.decay(); // 2 stacks -> 1
        c.dispatch(Event::TickStart, 2, &mut crate::SplitMix64::new(0));
        assert_eq!(c.integrity, 21.0); // -3*1
        c.decay(); // 1 -> 0, dropped
        assert!(c.generators().is_empty());
    }

    #[test]
    fn crash_stuns_passively() {
        let mut c = Character::new(chassis());
        assert!(!c.realize().stunned());
        c.install(StatusSpec::crash().to_decorator(1, 2));
        assert!(c.realize().stunned()); // gates the action phase
        // it's passive: dispatch produces no reaction.
        assert!(c.dispatch(Event::TickStart, 1, &mut crate::SplitMix64::new(0)).is_empty());
    }

    #[test]
    fn lag_folds_into_initiative_as_a_more_factor() {
        let mut c = Character::new(chassis()); // base init 6
        c.install(StatusSpec::lag().to_decorator(1, 2)); // Slow(0.5) → ×0.5
        assert_eq!(c.realize().initiative(), 3.0);
    }

    #[test]
    fn breach_exposes_the_vuln_multiplier() {
        let mut c = Character::new(chassis());
        assert_eq!(c.realize().vuln(), 1.0);
        c.install(StatusSpec::breach().to_decorator(1, 2)); // Vuln(1.5)
        assert_eq!(c.realize().vuln(), 1.5);
    }

    #[test]
    fn corrode_shreds_plating() {
        let mut c = Character::new(chassis()); // plating 10
        c.install(StatusSpec::corrode().to_decorator(2, 0)); // 2/stack, 2 stacks
        c.dispatch(Event::TickStart, 1, &mut crate::SplitMix64::new(0));
        assert_eq!(c.plating, 6.0); // 10 - 4
        assert_eq!(c.integrity, 30.0); // shred doesn't touch Integrity
    }

    #[test]
    fn poison_softener_never_kills() {
        // Poison: PctCurrent 0.08 Internal — the softener (can_kill = false).
        let mut c = Character::new(BaseLine { body: 5.0, ..Default::default() });
        c.apply_damage(0, 0, 29.0); // down to 1
        c.install(StatusSpec::poison().to_decorator(5, 3));
        c.dispatch(Event::TickStart, 1, &mut crate::SplitMix64::new(0));
        assert!(c.integrity > 0.0 && c.alive); // softener shrinks but never kills
    }
}
