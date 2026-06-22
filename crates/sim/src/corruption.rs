//! Corruption — **hostile decorators**, the inverse of buffs (`docs/layers.md` L6).
//!
//! A **Worm** rots the *digital* surface (Ice), a **Virus** the *biological* one
//! (Health); a **Spoof** corrupts *behavior* (the [`Unit::spoof`](crate::Unit::spoof)
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
    /// **ICE-breaker worm** — a digital corruption (`Tag::Worm`) that tanks Ice by
    /// `ice` for `turns`, prying the target's wall open so a follow-up hack lands.
    pub fn worm(ice: f32, turns: u32) -> Decorator {
        Decorator::timed(Tag::Worm, turns, vec![Factor::add(Stat::Ice, -ice)])
            .with_label("Worm")
    }

    /// **Bio virus** — a biological corruption (`Tag::Virus`): a **fever DoT** of `dot`
    /// each tick (Internal — it bypasses armor) that **attacks Body** by `body` for `turns` (a
    /// wasting bite). Because Body *is* the HP pool and the bio-resist, the attack drags Integrity
    /// down *and* softens the host for the next strain. The DoT is the immediate bite; the Body
    /// attack is the slow wasting.
    pub fn virus(body: f32, dot: f32, turns: u32) -> Decorator {
        Decorator::timed(Tag::Virus, turns, vec![Factor::add(Stat::Body, -body)])
            .with_label("Virus")
            .with_hook(
                Event::TickStart,
                HookEffect::Damage { amount: Amount::Flat(dot), pen: PenTier::Internal, can_kill: true },
            )
    }

    /// **Plague** — a *contagious* virus (`docs/corruption.md`): the [`Self::virus`]
    /// fever / Body-attack that also **spreads by proximity** (within 1 hex), each jump
    /// a contest of `virulence` vs the victim's **Body**. Friend or foe — keep the
    /// infected isolated, because the fever rides along with it.
    pub fn plague(body: f32, dot: f32, virulence: i32, turns: u32) -> Decorator {
        Self::virus(body, dot, turns)
            .with_contagion(Contagion { virulence, resist: Stat::Body, vector: Vector::Proximity(1) })
    }

    /// **Worm swarm** — a *contagious* worm: the [`Self::worm`] Ice-rot that also
    /// **rides the net** to any unit with a live digital surface (`Link > 0`), each jump
    /// a contest of `virulence` vs the victim's Ice. The digital pandemic.
    pub fn worm_swarm(ice: f32, virulence: i32, turns: u32) -> Decorator {
        Self::worm(ice, turns)
            .with_contagion(Contagion { virulence, resist: Stat::Ice, vector: Vector::Net })
    }

    /// **Antivirus** — a standing ward (gear) that suppresses every `Tag::Virus`
    /// corruption while installed (the bio counterplay).
    pub fn antivirus() -> Decorator {
        Decorator::gear(Tag::Gear, vec![]).with_remove(Remove::Tag(Tag::Virus))
    }

    /// **Ice patch** — a standing ward that suppresses every `Tag::Worm`
    /// corruption while installed (the digital counterplay).
    pub fn ice_patch() -> Decorator {
        Decorator::gear(Tag::Gear, vec![]).with_remove(Remove::Tag(Tag::Worm))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chargen::{BaseLine, Character};

    fn base() -> BaseLine {
        BaseLine { ice: 12.0, body: 10.0, ..BaseLine::default() }
    }

    #[test]
    fn a_worm_rots_ice_until_patched() {
        let mut c = Character::new(base());
        assert_eq!(c.realize().ice(), 12);
        c.install(Corruption::worm(5.0, 3));
        assert_eq!(c.realize().ice(), 7); // wall pried open — a hack lands easier
        // An ICE patch is a ward: it strips Worm-tagged modifiers while installed.
        let patch = c.install(Corruption::ice_patch());
        assert_eq!(c.realize().ice(), 12); // worm suppressed
        c.remove(patch);
        assert_eq!(c.realize().ice(), 7); // worm bites again once the patch is gone
    }

    #[test]
    fn a_plague_is_a_contagious_virus() {
        // The contagious variant is the virus debuff plus a Contagion the phase reads.
        let mut c = Character::new(base());
        c.install(Corruption::plague(4.0, 2.0, 6, 5));
        assert_eq!(c.realize().body(), 6); // attacks Body (10 → 6) — wasting, drags HP with it
        assert_eq!(c.active_contagions().len(), 1); // and it's a spread source
        // The plain virus is *not* contagious — single-target corruption.
        let mut d = Character::new(base());
        d.install(Corruption::virus(4.0, 2.0, 5));
        assert!(d.active_contagions().is_empty());
    }

    #[test]
    fn an_antivirus_strips_only_viruses_not_buffs() {
        let mut c = Character::new(base());
        c.install(Corruption::virus(4.0, 2.0, 3)); // attacks Body 10 → 6
        // a friendly Buff on the same stat — the antivirus must NOT touch it.
        c.install(Decorator::timed(Tag::Buff, 9, vec![Factor::add(Stat::Body, 3.0)]));
        assert_eq!(c.realize().body(), 9); // 10 − 4 + 3
        c.install(Corruption::antivirus());
        assert_eq!(c.realize().body(), 13); // virus gone, buff kept (10 + 3)
    }
}
