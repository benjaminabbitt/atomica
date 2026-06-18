//! Authored **content** — a starter roster and a gauntlet — so the CLI (and tests) can
//! play a *real* run instead of an ad-hoc fixture. Magnitudes are illustrative (TBD;
//! see [`docs/rosters.md`](../../../docs/rosters.md)).
//!
//! Roster/enemy templates carry a placeholder id `0` and `Team::A`; [`deploy`](crate)
//! reassigns both at battle start, so only the stat line / loadout here matters.

use crate::{Encounter, GamePlan, RunPlan};
use atomica_sim::{
    ArmorClass, Attack, Chassis, Corruption, DamageType, Footprint, Implant, PenTier, Skill, Team,
    Unit,
};

fn weapon(damage: f32, dtype: DamageType, pen: PenTier, range: i32) -> Attack {
    // Role from reach: a reach-1 weapon is Melee, anything longer is Gunnery (and so
    // takes the range-difficulty penalty on its to-hit roll).
    let skill = if range > 1 { Skill::Gunnery } else { Skill::Melee };
    Attack {
        damage,
        dtype,
        pen,
        skill,
        accuracy: 0,
        range,
        min_range: 1,
        emp: false,
        footprint: Footprint::Single,
        smart: false,
    }
}

fn body(name: &str, hp: f32, init: f32) -> Unit {
    Unit::new(0, name, Team::A, Chassis::Augmented).with_integrity(hp).with_initiative(init)
}

// -- Player archetypes ------------------------------------------------------------

/// A **blade** — a fast melee bruiser. Slashing shreds the unarmored but glances off
/// Plate (×0.5), so the blade wants soft targets.
pub fn blade(name: &str) -> Unit {
    body(name, 68.0, 7.0).with_attack(weapon(14.0, DamageType::Slashing, PenTier::Internal, 1))
}

/// A **netrunner** — a ranged sidearm plus a cyberdeck (hacks enemy chrome). Piercing
/// is also halved by Plate, so the runner leans on the breach, not the gun, vs armor.
pub fn runner(name: &str) -> Unit {
    let mut u =
        body(name, 60.0, 6.0).with_attack(weapon(8.0, DamageType::Piercing, PenTier::Contact, 4));
    u.skills.set(Skill::Hacking, 5);
    u.install(Implant::cyberdeck());
    u
}

/// A **bulwark** — an armored tank with a heavy maul. Bludgeoning is *amplified* vs
/// Plate (×1.5), so the slow bulwark is the answer to the hardened enemies.
pub fn bulwark(name: &str) -> Unit {
    body(name, 94.0, 5.0)
        .with_armor(ArmorClass::Plate)
        .with_attack(weapon(12.0, DamageType::Bludgeoning, PenTier::Contact, 1))
}

/// The **starter roster** — one of each archetype.
pub fn starter_roster() -> Vec<Unit> {
    vec![blade("Katana"), runner("Glitch"), bulwark("Anvil")]
}

// -- Enemy archetypes -------------------------------------------------------------

/// A soft, lightly-armored **mook** (Mail) — cannon fodder the blade carves up. Hits
/// lightly; the threat is in numbers.
fn mook(name: &str) -> Unit {
    body(name, 74.0, 5.0)
        .with_armor(ArmorClass::Mail)
        .with_attack(weapon(3.0, DamageType::Piercing, PenTier::External, 2))
}

/// A hardened, **chromed** enemy in **Plate** — shrugs off the blade / gun (×0.5), so
/// it's a slog until the netrunner breaches its plating or the bulwark caves it in. A
/// punishing but slow-killing wall.
fn enforcer(name: &str) -> Unit {
    let mut u = body(name, 104.0, 5.0)
        .with_armor(ArmorClass::Plate)
        .with_attack(weapon(3.0, DamageType::Bludgeoning, PenTier::Contact, 1));
    u.character.base_mut().link = 4.0;
    u.character.base_mut().firewall = 6.0;
    u.install(Implant::subdermal_plating());
    u
}

/// A **carrier** — a tougher mook seeded with a virulent plague that spreads on contact.
fn carrier(name: &str) -> Unit {
    let mut u = body(name, 94.0, 5.0)
        .with_armor(ArmorClass::Mail)
        .with_attack(weapon(3.0, DamageType::Piercing, PenTier::External, 2));
    u.apply_modifier(Corruption::plague(4.0, 10, 8));
    u
}

// -- Plans ------------------------------------------------------------------------

/// The **gauntlet** — a three-encounter run of escalating threats (no R&R within).
pub fn gauntlet() -> RunPlan {
    RunPlan::new(
        "Sprawl Gauntlet",
        vec![
            Encounter::new(
                "Alley Ambush",
                vec![mook("Thug-1"), mook("Thug-2"), mook("Thug-3"), mook("Thug-4")],
            ),
            Encounter::new(
                "Corp Checkpoint",
                vec![enforcer("Enforcer"), mook("Guard-1"), mook("Guard-2"), mook("Guard-3")],
            ),
            Encounter::new(
                "Quarantine Zone",
                vec![carrier("Carrier"), enforcer("Warden"), mook("Orderly")],
            ),
        ],
    )
}

/// The **campaign** — two gauntlets with R&R between (the full [`crate::Game`] tier).
pub fn campaign() -> GamePlan {
    GamePlan::new(
        "Night City",
        vec![
            gauntlet(),
            RunPlan::new(
                "Deep Run",
                vec![
                    Encounter::new("Server Farm", vec![enforcer("Sentinel"), enforcer("Sentry")]),
                    Encounter::new("The Boss", vec![bulwark("Goliath"), carrier("Vector")]),
                ],
            ),
        ],
    )
}
