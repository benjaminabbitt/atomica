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

use crate::chargen::{Capability, Decorator, Factor, Stat, Tag};
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

    /// Project this implant as a [`Decorator`] on the layer architecture
    /// (`docs/layers.md` L2): each nonzero [`Contribution`] field becomes an `Add`
    /// [`Factor`], the [`Condition`] becomes the decorator's **`scale`**
    /// ([`benefit_factor`](Condition::benefit_factor) — Degraded `0.5`, Offline `0.0`),
    /// and a granted deck loadout becomes a [`Capability::Hack`]. The breach ladder
    /// then drives the implant by `set_scale` (degrade / disable / repair) on the
    /// `Character`'s gen; the liabilities it fires (`hack_effects`) join once statuses
    /// become decorators (L2b).
    pub fn to_decorator(&self) -> Decorator {
        let c = self.contribution;
        let mut factors = Vec::new();
        if c.link != 0 {
            factors.push(Factor::add(Stat::Link, c.link as f32));
        }
        if c.firewall != 0 {
            factors.push(Factor::add(Stat::Firewall, c.firewall as f32));
        }
        if c.plating != 0.0 {
            factors.push(Factor::add(Stat::Plating, c.plating));
        }
        if c.initiative != 0.0 {
            factors.push(Factor::add(Stat::Initiative, c.initiative));
        }
        if c.damage != 0.0 {
            factors.push(Factor::add(Stat::Damage, c.damage));
        }
        if c.max_integrity != 0.0 {
            factors.push(Factor::add(Stat::MaxIntegrity, c.max_integrity));
        }
        let mut d = Decorator::gear(Tag::Implant, factors).with_scale(self.condition.benefit_factor());
        if let Some(h) = self.grant_hack {
            d = d.with_grant(Capability::Hack(h));
        }
        d
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chargen::{Event, Tag};
    use crate::{BaseLine, Character};

    /// A blank chassis whose innate Firewall (9) is the netrunning baseline, so a
    /// cyberdeck's +2 lands the unit at the even-odds wall (11).
    fn chassis() -> BaseLine {
        BaseLine {
            firewall: 9.0,
            max_integrity: 30.0,
            initiative: 5.0,
            damage: 10.0,
            ..Default::default()
        }
    }

    #[test]
    fn cyberdeck_folds_the_surface_and_grants_the_hack() {
        let mut c = Character::new(chassis());
        c.install(Implant::cyberdeck().to_decorator());
        let r = c.realize();
        assert_eq!(r.link(), 5); // 0 base + 5
        assert_eq!(r.firewall(), 11); // 9 base + 2
        assert!(r.hack().is_some()); // the deck grants the loadout
    }

    #[test]
    fn breach_disabling_the_deck_unfolds_it_and_drops_the_hack() {
        let mut c = Character::new(chassis());
        let deck = c.install(Implant::cyberdeck().to_decorator());
        // disable floor (§6): Offline → scale 0.
        c.set_scale(deck, Condition::Offline.benefit_factor());
        let r = c.realize();
        assert_eq!(r.link(), 0); // surface gone
        assert_eq!(r.firewall(), 9); // wall back to base
        assert!(r.hack().is_none()); // deck bricked → no hack
    }

    #[test]
    fn degrade_halves_the_benefit_and_repair_restores_it() {
        let mut c = Character::new(chassis());
        let plate = c.install(Implant::subdermal_plating().to_decorator()); // +6 plating
        assert_eq!(c.realize().plating(), 6.0); // Online: full
        c.set_scale(plate, Condition::Degraded.benefit_factor());
        assert_eq!(c.realize().plating(), 3.0); // Degraded: half — wear, no liability
        c.set_scale(plate, Condition::Offline.benefit_factor());
        assert_eq!(c.realize().plating(), 0.0); // Offline: none
        c.set_scale(plate, Condition::Online.benefit_factor());
        assert_eq!(c.realize().plating(), 6.0); // Ripperdoc repair — same decorator
    }

    #[test]
    fn max_integrity_implant_fills_at_deploy_then_chunks_on_breach() {
        let mut c = Character::new(chassis()); // base max 30
        let pump = c.install(Implant::metabolic_pump().to_decorator()); // +8 max
        assert_eq!(c.realize().max_integrity(), 38.0);
        c.fill(); // deploy at full
        assert_eq!(c.integrity, 38.0);

        // breach the pump: max drops, current chunks to the new cap (§3a).
        c.set_scale(pump, Condition::Offline.benefit_factor());
        c.clamp_integrity();
        assert_eq!(c.realize().max_integrity(), 30.0);
        assert_eq!(c.integrity, 30.0);

        // repair: capacity returns, but current does NOT refill (repair ≠ heal).
        c.set_scale(pump, Condition::Online.benefit_factor());
        c.clamp_integrity();
        assert_eq!(c.realize().max_integrity(), 38.0);
        assert_eq!(c.integrity, 30.0);
    }

    #[test]
    fn a_full_rig_folds_every_implant() {
        let mut c = Character::new(chassis());
        for im in [
            Implant::cyberdeck(),       // +5 link, +2 fw, hack
            Implant::subdermal_plating(), // +6 plating
            Implant::reflex_booster(),    // +3 init
            Implant::combat_stim(),       // +4 dmg, +1 init
            Implant::metabolic_pump(),    // +8 max
        ] {
            c.install(im.to_decorator());
        }
        c.fill();
        let r = c.realize();
        assert_eq!(r.link(), 5);
        assert_eq!(r.firewall(), 11);
        assert_eq!(r.plating(), 6.0);
        assert_eq!(r.initiative(), 9.0); // 5 base + 3 + 1
        assert_eq!(r.damage(), 14.0); // 10 base + 4
        assert_eq!(r.max_integrity(), 38.0);
        assert!(r.hack().is_some());
        assert_eq!(c.integrity, 38.0);
    }

    #[test]
    fn emp_fries_all_chrome_at_once() {
        // EMP is the gen-op `scale_where(Implant, 0)` — every implant Offline at once
        // (docs/cyberware.md §5; the per-roll Cascade gating stays with the resolver).
        let mut c = Character::new(chassis());
        c.install(Implant::cyberdeck().to_decorator());
        c.install(Implant::subdermal_plating().to_decorator());
        c.install(Implant::metabolic_pump().to_decorator());
        assert_eq!(c.realize().link(), 5);
        assert!(c.realize().hack().is_some());

        c.scale_where(Tag::Implant, 0.0); // pulse
        let r = c.realize();
        assert_eq!(r.link(), 0); // surface gone
        assert_eq!(r.plating(), 0.0); // plating gone
        assert_eq!(r.max_integrity(), 30.0); // resilience gone
        assert!(r.hack().is_none()); // deck bricked
    }

    #[test]
    fn breach_fires_a_liability_as_a_decorator() {
        // The §6 ladder, on the Character path: disable the implant (scale 0), then
        // install its hack_effect as a status decorator — the liability now ticks.
        let mut c = Character::new(chassis());
        let stim = Implant::combat_stim(); // Overdose: Bleed + Crash
        let id = c.install(stim.to_decorator());
        c.fill();
        c.set_scale(id, Condition::Offline.benefit_factor()); // disable floor

        // fire the degrade-class liability (Bleed) at margin-scaled stacks.
        c.install(stim.hack_effects[0].to_decorator(2, 0));
        let before = c.integrity;
        c.dispatch(Event::TickStart, 1);
        assert!(c.integrity < before); // the liability bites
    }
}
