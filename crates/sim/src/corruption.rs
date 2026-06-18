//! Corruption — **hostile decorators**, the inverse of buffs (`docs/layers.md` L6).
//!
//! A **Worm** rots the *digital* surface (Firewall), a **Virus** the *biological* one
//! (Immunity); a **Spoof** corrupts *behavior* (the [`Unit::spoof`](crate::Unit::spoof)
//! `CORRUPTION`-priority override, already wired). Each is just a decorator with a
//! hostile `Factor`/`Override` — what makes it *corruption* is its [`Tag`]
//! (`Worm`/`Virus`/`Spoof`), which a **cleanse** ward strips by family
//! ([`Remove::Tag`]). That tag-targeted counterplay is the point: a generic dispel
//! can't tell a worm from a buff, but an antivirus removes *only* viruses.
//!
//! Scope: the corruptions and their cleanses as content. **Spreading** (a corruption
//! propagating via Data-spill — the contagion families) is a separate later layer;
//! here a corruption lands on exactly the unit it's applied to.

use crate::chargen::{
    Amount, Contagion, Decorator, Event, Factor, HookEffect, Remove, Stat, Tag, Vector,
};
use crate::PenTier;

/// A library of named **corruption** decorators and their **cleanse** wards (content;
/// magnitudes are illustrative — TBD per the design).
pub struct Corruption;

impl Corruption {
    /// **ICE-breaker worm** — a digital corruption (`Tag::Worm`) that tanks Firewall by
    /// `firewall` for `turns`, prying the target's wall open so a follow-up hack lands.
    pub fn worm(firewall: f32, turns: u32) -> Decorator {
        Decorator::timed(Tag::Worm, turns, vec![Factor::add(Stat::Firewall, -firewall)])
            .with_label("Worm")
    }

    /// **Bio virus** — a biological corruption (`Tag::Virus`): a **fever DoT** of `dot`
    /// each tick (Internal — it bypasses armor) that also rots Immunity by `immunity`
    /// for `turns` (softening the host for the next strain). The DoT is the combat bite;
    /// the Immunity rot is the snowball.
    pub fn virus(immunity: f32, dot: f32, turns: u32) -> Decorator {
        Decorator::timed(Tag::Virus, turns, vec![Factor::add(Stat::Immunity, -immunity)])
            .with_label("Virus")
            .with_hook(
                Event::TickStart,
                HookEffect::Damage { amount: Amount::Flat(dot), pen: PenTier::Internal, can_kill: true },
            )
    }

    /// **Plague** — a *contagious* virus (`docs/corruption.md`): the [`Self::virus`]
    /// fever / Immunity-rot that also **spreads by proximity** (within 1 hex), each jump
    /// a contest of `virulence` vs the victim's Immunity. Friend or foe — keep the
    /// infected isolated, because the fever rides along with it.
    pub fn plague(immunity: f32, dot: f32, virulence: i32, turns: u32) -> Decorator {
        Self::virus(immunity, dot, turns)
            .with_contagion(Contagion { virulence, resist: Stat::Immunity, vector: Vector::Proximity(1) })
    }

    /// **Worm swarm** — a *contagious* worm: the [`Self::worm`] Firewall-rot that also
    /// **rides the net** to any unit with a live digital surface (`Link > 0`), each jump
    /// a contest of `virulence` vs the victim's Firewall. The digital pandemic.
    pub fn worm_swarm(firewall: f32, virulence: i32, turns: u32) -> Decorator {
        Self::worm(firewall, turns)
            .with_contagion(Contagion { virulence, resist: Stat::Firewall, vector: Vector::Net })
    }

    /// **Antivirus** — a standing ward (gear) that suppresses every `Tag::Virus`
    /// corruption while installed (the bio counterplay).
    pub fn antivirus() -> Decorator {
        Decorator::gear(Tag::Gear, vec![]).with_remove(Remove::Tag(Tag::Virus))
    }

    /// **Firewall patch** — a standing ward that suppresses every `Tag::Worm`
    /// corruption while installed (the digital counterplay).
    pub fn firewall_patch() -> Decorator {
        Decorator::gear(Tag::Gear, vec![]).with_remove(Remove::Tag(Tag::Worm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chargen::{BaseLine, Character};

    fn base() -> BaseLine {
        BaseLine { firewall: 12.0, immunity: 10.0, ..BaseLine::default() }
    }

    #[test]
    fn a_worm_rots_firewall_until_patched() {
        let mut c = Character::new(base());
        assert_eq!(c.realize().firewall(), 12);
        c.install(Corruption::worm(5.0, 3));
        assert_eq!(c.realize().firewall(), 7); // wall pried open — a hack lands easier
        // A firewall patch is a ward: it strips Worm-tagged modifiers while installed.
        let patch = c.install(Corruption::firewall_patch());
        assert_eq!(c.realize().firewall(), 12); // worm suppressed
        c.remove(patch);
        assert_eq!(c.realize().firewall(), 7); // worm bites again once the patch is gone
    }

    #[test]
    fn a_plague_is_a_contagious_virus() {
        // The contagious variant is the virus debuff plus a Contagion the phase reads.
        let mut c = Character::new(base());
        c.install(Corruption::plague(4.0, 2.0, 6, 5));
        assert_eq!(c.realize().immunity(), 6); // still rots Immunity like a plain virus
        assert_eq!(c.active_contagions().len(), 1); // and it's a spread source
        // The plain virus is *not* contagious — single-target corruption.
        let mut d = Character::new(base());
        d.install(Corruption::virus(4.0, 2.0, 5));
        assert!(d.active_contagions().is_empty());
    }

    #[test]
    fn an_antivirus_strips_only_viruses_not_buffs() {
        let mut c = Character::new(base());
        c.install(Corruption::virus(4.0, 2.0, 3)); // Immunity 10 → 6
        // a friendly Buff on the same stat — the antivirus must NOT touch it.
        c.install(Decorator::timed(Tag::Buff, 9, vec![Factor::add(Stat::Immunity, 3.0)]));
        assert_eq!(c.realize().immunity(), 9); // 10 − 4 + 3
        c.install(Corruption::antivirus());
        assert_eq!(c.realize().immunity(), 13); // virus gone, buff kept (10 + 3)
    }
}
