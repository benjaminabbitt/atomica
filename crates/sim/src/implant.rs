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

use crate::chargen::{Capability, Condition, Decorator, Factor, Stat, Tag};
use crate::{EquipmentTags, Hack, StatusSpec};

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

// `Condition` (the Online→Degraded→Offline→Destroyed ladder) now lives on the
// generic `Decorator` ([`chargen::Condition`], `docs/layers.md` L5) — re-exported by
// the crate root. An implant's `condition` field is its authored starting state.

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
    /// **Silhouette coverage** — this implant's share of the hit-location roll (`combat.md`):
    /// added to the chassis's coverage, so a heavily-chromed body presents more chrome to
    /// hit. Bulky pieces (plating) cover more than a tucked-away deck.
    pub coverage: i32,
    /// **Durability** — the implant's hit points. A physical blow that lands on it (the
    /// location roll) subtracts from this; at <50% it runs **Degraded** (half benefit), at
    /// 0 it's **Destroyed** (terminal). The Ripperdoc refills it on repair.
    pub max_hp: f32,
    /// **Equipment tags** (`docs/cyberware.md` §6) — the shared `const` flag set. Today:
    /// [`EquipmentTags::DIGITAL`] (networked chrome a breach can trip); its absence marks
    /// **inert physical** cyberware, immune to every breach vector. See [`Implant::is_digital`].
    pub tags: EquipmentTags,
}

impl Implant {
    /// Is this implant **digital** — does it present a surface a breach can trip? Defers
    /// to the [`DIGITAL`](EquipmentTags::DIGITAL) tag's rule ([`EquipmentTags::breachable`]).
    /// `false` ⇒ inert physical cyberware, immune to every breach vector; only physical wear.
    pub fn is_digital(&self) -> bool {
        self.tags.breachable()
    }

    /// A **cyberdeck** — grants the hack loadout and raises Link (the surface) +
    /// Firewall. Breached ⇒ **Lockout** (the deck bricks; the hack drops on the
    /// disable). The roster's keystone link to `netrunning.md`.
    pub fn cyberdeck() -> Self {
        Self {
            name: "Cyberdeck",
            contribution: Contribution { link: 5, firewall: 2, ..Default::default() },
            grant_hack: Some(Hack::new(6, StatusSpec::lockware(), 1, 6).with_overheat()), // runs the common Overheat program
            hack_effects: vec![StatusSpec::lockware()], // Lockout (placeholder)
            condition: Condition::Online,
            removable: true,
            coverage: 10, // small, tucked-away electronics
            max_hp: 18.0, // fragile
            tags: EquipmentTags::DIGITAL,
        }
    }

    /// **Subdermal plating** — +Plating. **Inert physical** chrome (no `DIGITAL` tag):
    /// unbreachable by hack / worm / EMP; only physical wear degrades it.
    pub fn subdermal_plating() -> Self {
        Self {
            name: "Subdermal Plating",
            contribution: Contribution { plating: 6.0, ..Default::default() },
            grant_hack: None,
            // **Inert physical armor** — no digital surface, so no breach liability: a
            // hacker / worm / EMP has nothing to trip here (§ "physical cyberware").
            hack_effects: vec![],
            condition: Condition::Online,
            removable: true,
            coverage: 150, // armor *covers* the body — it catches most blows (the widest band)
            max_hp: 60.0, // a deep buffer that wears slowly
            tags: EquipmentTags::NONE,
        }
    }

    /// **Skin weave** — light subdermal armor woven through the dermis: a small +Plating.
    /// **Inert physical** (no `DIGITAL` tag): EMP-/hack-proof, only physical wear. Like
    /// plating it *covers the body* (a wide hit-location band), but it's thinner — a smaller
    /// HP buffer that wears through sooner than dedicated plating.
    pub fn skin_weave() -> Self {
        Self {
            name: "Skin Weave",
            contribution: Contribution { plating: 3.0, ..Default::default() },
            grant_hack: None,
            hack_effects: vec![], // inert physical — no breach liability
            condition: Condition::Online,
            removable: true,
            coverage: 120, // covers the whole skin — a wide band, just under dedicated plating
            max_hp: 36.0, // thin: wears through sooner
            tags: EquipmentTags::NONE,
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
            coverage: 25, // spinal / limb wiring — a sizeable limb's worth
            max_hp: 24.0,
            tags: EquipmentTags::DIGITAL.with(EquipmentTags::VOLATILE), // runs hot
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
            coverage: 10,
            max_hp: 20.0,
            tags: EquipmentTags::DIGITAL,
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
            coverage: 10,
            max_hp: 16.0, // volatile
            tags: EquipmentTags::DIGITAL.with(EquipmentTags::VOLATILE), // runs hot
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
            coverage: 20, // visceral
            max_hp: 28.0,
            tags: EquipmentTags::DIGITAL.with(EquipmentTags::VOLATILE), // runs hot
        }
    }

    /// Project this implant as a [`Decorator`] on the layer architecture
    /// (`docs/layers.md` L2): each nonzero [`Contribution`] field becomes an `Add`
    /// [`Factor`], the [`Condition`] is carried on the decorator (it scales the
    /// factors — Degraded `0.5`, Offline `0.0`), and a granted deck loadout becomes a
    /// [`Capability::Hack`]. The breach ladder drives the implant by `set_condition`
    /// (degrade / disable / repair) on the `Character`'s gen.
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
        let mut d = Decorator::gear(Tag::Implant, factors).with_condition(self.condition);
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

    #[test]
    fn skin_weave_is_inert_physical_armor() {
        // Light armor chrome: a small Plating bump, and like plating it's inert physical —
        // no digital surface for a hack / worm / EMP to trip.
        assert!(!Implant::skin_weave().is_digital());
        let mut c = Character::new(chassis());
        c.install(Implant::skin_weave().to_decorator());
        assert_eq!(c.realize().plating(), 3.0);
    }

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
        c.set_condition(deck, Condition::Offline);
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
        c.set_condition(plate, Condition::Degraded);
        assert_eq!(c.realize().plating(), 3.0); // Degraded: half — wear, no liability
        c.set_condition(plate, Condition::Offline);
        assert_eq!(c.realize().plating(), 0.0); // Offline: none
        c.set_condition(plate, Condition::Online);
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
        c.set_condition(pump, Condition::Offline);
        c.clamp_integrity();
        assert_eq!(c.realize().max_integrity(), 30.0);
        assert_eq!(c.integrity, 30.0);

        // repair: capacity returns, but current does NOT refill (repair ≠ heal).
        c.set_condition(pump, Condition::Online);
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
        // EMP is the gen-op `condition_where(Implant, Offline)` — all chrome Offline at once
        // (docs/cyberware.md §5; the per-roll Cascade gating stays with the resolver).
        let mut c = Character::new(chassis());
        c.install(Implant::cyberdeck().to_decorator());
        c.install(Implant::subdermal_plating().to_decorator());
        c.install(Implant::metabolic_pump().to_decorator());
        assert_eq!(c.realize().link(), 5);
        assert!(c.realize().hack().is_some());

        c.condition_where(Tag::Implant, Condition::Offline); // pulse
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
        c.set_condition(id, Condition::Offline); // disable floor

        // fire the degrade-class liability (Bleed) at margin-scaled stacks.
        c.install(stim.hack_effects[0].to_decorator(2, 0));
        let before = c.integrity;
        c.dispatch(Event::TickStart, 1, &mut crate::SplitMix64::new(0));
        assert!(c.integrity < before); // the liability bites
    }
}
