//! Cyberware implants — the augmentation bundle (`docs/cyberware.md`).
//!
//! An implant is an unconditional **benefit** + a standing **exposure**
//! (Link / Firewall / weight) + a conditional **liability** (its
//! [`hack_effects`](Implant::hack_effects), fired on breach — the §6 severity
//! ladder, wired in a later phase). A unit's effective stat line is its chassis
//! **base plus the sum of its active implants**: installing an implant *folds*
//! its [`Contribution`] into the line; a breach that knocks it Offline *unfolds*
//! it again (the "disable" floor of the breach ladder).
//!
//! Phase A scope: the data model + stat derivation. The breach *trigger* (a hack
//! tripping an implant and applying its `hack_effects`) and PAN / slots come in
//! later phases — the fields are here, the wiring is not.

use crate::{Hack, StatusSpec};

/// A unit's **Personal Area Network** mode (`docs/cyberware.md` §5, delta §6) — a
/// loadout commitment, not an in-battle toggle.
///
/// - **Meshed** (default): cross-implant **synergy** + full throughput, but a
///   breach can **Cascade** to *every* implant at once.
/// - **Segmented**: implants isolated → a breach is **contained** (Cascade-proof),
///   but no synergy.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Pan {
    #[default]
    Meshed,
    Segmented,
}

/// Live state of an implant (`docs/cyberware.md` §6): `Online → Degraded →
/// Offline → Destroyed`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Condition {
    Online,
    Degraded,
    Offline,
    Destroyed,
}

impl Condition {
    /// Does the implant deliver (some of) its benefit right now? (Online or
    /// Degraded; an Offline / Destroyed implant contributes nothing.)
    pub fn is_active(self) -> bool {
        matches!(self, Condition::Online | Condition::Degraded)
    }

    /// Fraction of its benefit the implant delivers now (`docs/cyberware.md` §6):
    /// Online full, **Degraded half**, Offline / Destroyed none.
    pub fn benefit_factor(self) -> f32 {
        match self {
            Condition::Online => 1.0,
            Condition::Degraded => 0.5,
            Condition::Offline | Condition::Destroyed => 0.0,
        }
    }

    /// One step down the wear ladder: `Online → Degraded → Offline → Destroyed`
    /// (Destroyed is terminal).
    pub fn degraded(self) -> Condition {
        match self {
            Condition::Online => Condition::Degraded,
            Condition::Degraded => Condition::Offline,
            Condition::Offline | Condition::Destroyed => Condition::Destroyed,
        }
    }
}

/// The stat deltas an implant folds into its owner while active. Link / Firewall
/// are the digital surface (exposure + defense); plating / initiative / damage /
/// max-Integrity the physical benefit. Weight folds in as **negative** initiative
/// at build time.
#[derive(Clone, Copy, Debug, Default)]
pub struct Contribution {
    pub link: i32,
    pub firewall: i32,
    pub plating: f32,
    pub initiative: f32,
    pub damage: f32,
    pub max_integrity: f32,
}

/// A cyberware implant (`docs/cyberware.md` §1): a bundle of stat contributions,
/// an optional granted capability, and its conditional liabilities.
#[derive(Clone, Debug)]
pub struct Implant {
    pub name: &'static str,
    /// Stat deltas folded into the owner while active.
    pub contribution: Contribution,
    /// A granted netrunning loadout — present on a **cyberdeck**. A unit hacks
    /// *because* it carries a deck; breach the deck (Offline) and the hack drops.
    pub grant_hack: Option<Hack>,
    /// The liabilities fired on the owner when breached — **one or more** (loaded
    /// chrome carries several). The §6 ladder runs per effect (later phase).
    pub hack_effects: Vec<StatusSpec>,
    pub condition: Condition,
    /// Inbuilt implants are non-removable identity (§7F).
    pub removable: bool,
}

impl Implant {
    /// A **cyberdeck** — grants the hack loadout and raises Link (the surface) +
    /// Firewall. Breached ⇒ **Lockout** (the deck bricks; the hack drops on the
    /// disable). The roster's keystone link to `netrunning.md`.
    pub fn cyberdeck() -> Self {
        Self {
            name: "Cyberdeck",
            contribution: Contribution { link: 5, firewall: 2, ..Default::default() },
            grant_hack: Some(Hack::new(6, StatusSpec::lockware(), 1, 6)),
            hack_effects: vec![StatusSpec::lockware()], // Lockout (placeholder)
            condition: Condition::Online,
            removable: true,
        }
    }

    /// **Subdermal plating** — +Plating. Breached ⇒ **Shed** (plating-shred).
    pub fn subdermal_plating() -> Self {
        Self {
            name: "Subdermal Plating",
            contribution: Contribution { plating: 6.0, ..Default::default() },
            grant_hack: None,
            hack_effects: vec![StatusSpec::corrode()], // Shed
            condition: Condition::Online,
            removable: true,
        }
    }

    /// **Reflex booster** — +Physical Initiative. Breached ⇒ **Seizure** (Crash).
    pub fn reflex_booster() -> Self {
        Self {
            name: "Reflex Booster",
            contribution: Contribution { initiative: 3.0, ..Default::default() },
            grant_hack: None,
            hack_effects: vec![StatusSpec::crash()], // Seizure
            condition: Condition::Online,
            removable: true,
        }
    }

    /// **Firewall suite** — +Firewall (the wall) + a little Link surface. Breached
    /// ⇒ **Breach** (incoming-damage vulnerability).
    pub fn firewall_suite() -> Self {
        Self {
            name: "Firewall Suite",
            contribution: Contribution { firewall: 4, link: 1, ..Default::default() },
            grant_hack: None,
            hack_effects: vec![StatusSpec::breach()], // Breach (vuln)
            condition: Condition::Online,
            removable: true,
        }
    }

    /// **Combat stim** — +damage and a touch of haste. Breached ⇒ **Overdose**: a
    /// self-DoT *and* a Crash — a **multi-effect** liability (the degrade fires on
    /// margin, the stun only on a crit, §6).
    pub fn combat_stim() -> Self {
        Self {
            name: "Combat Stim",
            contribution: Contribution { damage: 4.0, initiative: 1.0, ..Default::default() },
            grant_hack: None,
            hack_effects: vec![StatusSpec::bleed(), StatusSpec::crash()], // Overdose
            condition: Condition::Online,
            removable: true,
        }
    }

    /// **Metabolic pump** — +max Integrity (resilience). Breached ⇒ **Overload**:
    /// an Internal DoT (it runs hot).
    pub fn metabolic_pump() -> Self {
        Self {
            name: "Metabolic Pump",
            contribution: Contribution { max_integrity: 8.0, ..Default::default() },
            grant_hack: None,
            hack_effects: vec![StatusSpec::bleed()], // Overload (Internal DoT)
            condition: Condition::Online,
            removable: true,
        }
    }
}
