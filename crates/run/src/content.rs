//! Authored **content** — rosters, archetypes, and scenarios — so the CLI (and tests)
//! can play a *real* run instead of an ad-hoc fixture. Magnitudes are illustrative (TBD;
//! see [`docs/rosters.md`](../../../docs/rosters.md)).
//!
//! **Player archetypes** ([`blade`] / [`runner`] / [`bulwark`] core, [`marksman`] /
//! [`sapper`] specialists) and **enemy archetypes** ([`mook`] up to [`sniper`],
//! [`grenadier`], [`swarmer`], [`breaker`], [`bomber`]) each lean on a different system —
//! armor matrix, range bands, AoE friendly fire, netrunning, behavior profiles, on-death
//! triggers — so a run exercises the engine broadly. The **scenarios** ([`gauntlet`] /
//! [`street_war`] / [`last_stand`]) vary the **objective** too (Eliminate vs Survive).
//!
//! Roster/enemy templates carry a placeholder id `0` and `Team::A`; [`deploy`](crate)
//! reassigns both at battle start, so only the stat line / loadout here matters.

use crate::{Encounter, GamePlan, RunPlan};
use atomica_sim::{
    ArmorClass, Attack, Chassis, DamageType, DeathTrigger, Footprint, Implant, MovementProfile,
    ObjectiveKind, PenTier, Skill, Team, TargetingProfile, Unit, EquipmentTags,
};

fn weapon(damage: f32, dtype: DamageType, pen: PenTier, range: i32) -> Attack {
    // Role from reach: a reach-1 weapon is Melee, anything longer is a Gunnery weapon —
    // and **ranged** (an inherent to-hit penalty that grows with distance).
    let (skill, tags) = if range > 1 {
        (Skill::Gunnery, EquipmentTags::RANGED)
    } else {
        (Skill::Melee, EquipmentTags::NONE)
    };
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
        tags,
    }
}

/// Builder: mark a weapon **awkward** (a rifle / polearm / heavy weapon) — clumsy
/// to-hit when an enemy is jammed up close (§7G).
fn awkward(mut a: Attack) -> Attack {
    a.tags = a.tags.with(EquipmentTags::AWKWARD);
    a
}

/// Builder: a **blast** weapon (grenade / rocket) — an AoE disc of `radius`. No `SMART`
/// tag means **no IFF**: the blast catches allies caught in the footprint too (§7G).
fn blast(mut a: Attack, radius: i32) -> Attack {
    a.footprint = Footprint::Blast(radius);
    a
}

/// Builder: an **EMP** weapon — a physical pulse that fries the target's digital chrome
/// *through* Firewall (§7I), the counter to chromed builds.
fn emp(mut a: Attack) -> Attack {
    a.emp = true;
    a
}

/// Builder: a **smartlinked** weapon — the `SMART` tag (IFF: its line / blast spares the
/// firer's team) plus a little inherent **accuracy** from the smartgun's targeting assist.
fn smart(mut a: Attack) -> Attack {
    a.accuracy += 1;
    a.smartlinked()
}

fn body(name: &str, hp: f32, init: f32) -> Unit {
    Unit::new(0, name, Team::A, Chassis::Augmented).with_integrity(hp).with_initiative(init)
}

// -- Player archetypes ------------------------------------------------------------

/// A **blade** — a fast melee bruiser. Slashing shreds the unarmored but glances off
/// Plate (×0.5), so the blade wants soft targets.
pub fn blade(name: &str) -> Unit {
    body(name, 68.0, 7.0)
        .with_skill(Skill::Melee, 8) // a duelist — lands the blade
        .with_evasion(16.0) // nimble elite — slips most incoming
        .with_attack(weapon(14.0, DamageType::Slashing, PenTier::Internal, 1))
}

/// A **netrunner** — a ranged sidearm plus a cyberdeck (hacks enemy chrome). Piercing
/// is also halved by Plate, so the runner leans on the breach, not the gun, vs armor.
pub fn runner(name: &str) -> Unit {
    let mut u = body(name, 60.0, 6.0)
        .with_skill(Skill::Gunnery, 6)
        .with_skill(Skill::Hacking, 5)
        .with_evasion(14.0)
        .with_attack(awkward(weapon(8.0, DamageType::Piercing, PenTier::Contact, 4))); // a rifle — clumsy in a clinch
    u.install(Implant::cyberdeck());
    u
}

/// A **bulwark** — an armored tank with a heavy maul. Bludgeoning is *amplified* vs
/// Plate (×1.5), so the slow bulwark is the answer to the hardened enemies. Heavy and
/// slow — it hits hard but is **easy to hit** (low Evasion).
pub fn bulwark(name: &str) -> Unit {
    body(name, 94.0, 5.0)
        .with_armor(ArmorClass::Plate)
        .with_skill(Skill::Melee, 6)
        .with_evasion(11.0)
        .with_attack(weapon(12.0, DamageType::Bludgeoning, PenTier::Contact, 1))
}

/// A **marksman** — a glass-cannon sharpshooter. A smartlinked long rifle (IFF, so it
/// won't tag a teammate in its lane) that picks the **back line** and **holds at standoff
/// range** (it stops closing the instant range-5 reaches); deadly at distance, clumsy and
/// fragile if something closes (low HP, AWKWARD). It *advances* rather than kites — two
/// mutual kiters would just flee each other to the tick cap.
pub fn marksman(name: &str) -> Unit {
    let rifle = smart(awkward(weapon(9.0, DamageType::Piercing, PenTier::Contact, 5)));
    body(name, 56.0, 6.0)
        .with_skill(Skill::Gunnery, 7)
        .with_evasion(15.0)
        .with_attack(rifle)
        .with_targeting(TargetingProfile::Backline)
}

/// A **sapper** — an EMP shock-trooper, the anti-chrome answer. A short-range pulse that
/// fries digital cyberware *through* Firewall (§7I); it hunts the **biggest threat**, so
/// it bee-lines the chromed heavies the rest of the squad struggles to crack.
pub fn sapper(name: &str) -> Unit {
    let shock = emp(weapon(6.0, DamageType::Bludgeoning, PenTier::Contact, 2));
    body(name, 66.0, 5.0)
        .with_skill(Skill::Gunnery, 5)
        .with_evasion(13.0)
        .with_attack(shock)
        .with_targeting(TargetingProfile::HighestThreat)
}

/// The **starter roster** — the core trio (one melee, one runner, one tank). The probe
/// tunes against this loadout, so it stays fixed.
pub fn starter_roster() -> Vec<Unit> {
    vec![blade("Katana"), runner("Glitch"), bulwark("Anvil")]
}

/// A **full strike team** — the core trio plus the two specialists (marksman, sapper), a
/// five-unit squad for the larger [`campaign`] scenarios.
pub fn full_squad() -> Vec<Unit> {
    vec![
        blade("Katana"),
        runner("Glitch"),
        bulwark("Anvil"),
        marksman("Hawkeye"),
        sapper("Surge"),
    ]
}

// -- Enemy archetypes -------------------------------------------------------------

/// A soft, lightly-armored **mook** (Mail) — cannon fodder the blade carves up. Hits
/// lightly; the threat is in numbers.
fn mook(name: &str) -> Unit {
    body(name, 74.0, 5.0)
        .with_armor(ArmorClass::Mail)
        .with_skill(Skill::Gunnery, 3)
        .with_evasion(14.0)
        .with_attack(weapon(4.0, DamageType::Piercing, PenTier::External, 2))
}

/// A hardened, **chromed** enemy in **Plate** — shrugs off the blade / gun (×0.5), so
/// it's a slog until the bulwark caves it in. Its **inert plating** can't be hacked,
/// but the netrunner can breach its **reflex booster** (digital smartware) for a Seizure.
fn enforcer(name: &str) -> Unit {
    let mut u = body(name, 104.0, 5.0)
        .with_armor(ArmorClass::Plate)
        .with_skill(Skill::Melee, 5)
        .with_evasion(12.0)
        .with_attack(weapon(5.0, DamageType::Bludgeoning, PenTier::Contact, 1));
    u.character.base_mut().link = 4.0; // a networked surface to hack at
    u.character.base_mut().firewall = 6.0;
    u.install(Implant::subdermal_plating()); // physical — bulwark's problem, not the runner's
    u.install(Implant::reflex_booster()); // digital smartware — the runner's breach target
    u
}

/// A **brute** — a tougher, up-armored mook (Plate). No chrome to breach and no plague;
/// just a meatier body than the rank-and-file, the muscle of a hardened position.
fn brute(name: &str) -> Unit {
    body(name, 94.0, 5.0)
        .with_armor(ArmorClass::Plate)
        .with_skill(Skill::Gunnery, 3)
        .with_evasion(13.0)
        .with_attack(weapon(5.0, DamageType::Piercing, PenTier::Contact, 2))
}

/// A **sniper** — a long-range nest that **holds position** and picks off the squad's
/// **biggest threat** from range 6, so the squad eats fire on the approach. Fragile up
/// close (AWKWARD, low HP): rush it down. (It Holds rather than kites — the board is
/// unbounded, so a fleeing shooter would never be cornered.)
fn sniper(name: &str) -> Unit {
    body(name, 58.0, 6.0)
        .with_skill(Skill::Gunnery, 5)
        .with_evasion(13.0)
        .with_attack(awkward(weapon(7.0, DamageType::Piercing, PenTier::Contact, 6)))
        .with_targeting(TargetingProfile::HighestThreat)
        .with_movement(MovementProfile::Hold)
}

/// A **grenadier** — lobs a **blast** (radius 1, no IFF). The AoE punishes a clustered
/// squad — and catches its *own* line if they bunch — so it rewards spreading out.
fn grenadier(name: &str) -> Unit {
    body(name, 70.0, 4.0)
        .with_armor(ArmorClass::Mail)
        .with_skill(Skill::Gunnery, 4)
        .with_evasion(12.0)
        .with_attack(blast(weapon(6.0, DamageType::Bludgeoning, PenTier::External, 3), 1))
}

/// A **swarmer** — a fast, fragile rusher (speed 2, **Swarm**) that hunts the **lowest
/// Integrity** target to finish the wounded. Trivial one-on-one; a threat in numbers.
fn swarmer(name: &str) -> Unit {
    body(name, 40.0, 7.0)
        .with_skill(Skill::Melee, 4)
        .with_evasion(15.0)
        .with_speed(2)
        .with_attack(weapon(5.0, DamageType::Slashing, PenTier::Internal, 1))
        .with_targeting(TargetingProfile::LowestIntegrity)
        .with_movement(MovementProfile::Swarm)
}

/// A **breaker** — an enemy netrunner with a cyberdeck: jacked in at the back (Holds), it
/// **hacks the squad's chrome** (the runner's deck — the sapper has none to lose). A
/// mirror of the player's runner, and its own deck is a breach target right back.
fn breaker(name: &str) -> Unit {
    let mut u = body(name, 56.0, 6.0)
        .with_skill(Skill::Hacking, 6)
        .with_skill(Skill::Gunnery, 4)
        .with_evasion(13.0)
        .with_attack(weapon(4.0, DamageType::Piercing, PenTier::External, 3))
        .with_targeting(TargetingProfile::HighestThreat)
        .with_movement(MovementProfile::Hold);
    u.install(Implant::cyberdeck());
    u
}

/// A **bomber** — **Swarms** in to die, then **detonates**: a parting blast (radius 1,
/// friendly fire) to everyone adjacent. Killing it at range, or not bunched, is the play.
fn bomber(name: &str) -> Unit {
    body(name, 50.0, 4.0)
        .with_armor(ArmorClass::Mail)
        .with_skill(Skill::Melee, 4)
        .with_evasion(12.0)
        .with_attack(weapon(4.0, DamageType::Bludgeoning, PenTier::Contact, 1))
        .with_movement(MovementProfile::Swarm)
        .with_on_death(DeathTrigger::Detonate {
            damage: 10.0,
            dtype: DamageType::Bludgeoning,
            pen: PenTier::Contact,
            radius: 1,
        })
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
                "Lockdown Zone",
                vec![brute("Heavy"), enforcer("Warden"), mook("Orderly")],
            ),
        ],
    )
}

/// A **street war** — a three-encounter run that leans on the *new* threats: a rushing
/// swarm that detonates, a ranged crossfire (sniper + AoE), and a netrunning duel. Built
/// for the [`full_squad`] (the specialists earn their keep here).
pub fn street_war() -> RunPlan {
    RunPlan::new(
        "Street War",
        vec![
            // A fast melee tide that ends with a bang — spread out or the bomber clusters you.
            Encounter::new(
                "Gang Rush",
                vec![
                    swarmer("Razor-1"),
                    swarmer("Razor-2"),
                    swarmer("Razor-3"),
                    swarmer("Razor-4"),
                    bomber("Boomer"),
                    bomber("Crash"),
                ],
            ),
            // Ranged pressure: a sniper picking the heavies, a grenadier punishing clusters.
            Encounter::new(
                "Crossfire",
                vec![sniper("Longshot"), grenadier("Lobber"), mook("Gun-1"), mook("Gun-2")],
            ),
            // The netrunning mirror — the breaker hacks your deck while the wall holds.
            Encounter::new(
                "Net Duel",
                vec![breaker("Daemon"), enforcer("Warden"), brute("Slab"), mook("Goon")],
            ),
        ],
    )
}

/// A **last stand** — a single **Survive** scenario: hold out against a relentless mixed
/// assault until the round count, passing by *lasting* rather than by a wipe (the
/// defensive objective, a different shape of win from Eliminate).
pub fn last_stand() -> RunPlan {
    RunPlan::new(
        "Last Stand",
        vec![Encounter::new(
            "Hold the Roof",
            vec![
                swarmer("Rush-1"),
                swarmer("Rush-2"),
                grenadier("Mortar"),
                bomber("Charge"),
                mook("Trooper"),
            ],
        )
        .with_objective(ObjectiveKind::Survive(8))],
    )
}

/// The **campaign** — the full [`crate::Game`] tier: the tuned gauntlet, then the harder
/// street war, then a last stand, with R&R between each. Field it with [`full_squad`].
pub fn campaign() -> GamePlan {
    GamePlan::new("Night City", vec![gauntlet(), street_war(), last_stand()])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Game, GameOutcome};

    /// Every authored archetype is armed and sits in the to-hit-relevant Evasion band —
    /// a smoke test that the loadout builders produce sane combatants.
    #[test]
    fn every_archetype_is_armed_and_in_band() {
        let roster: Vec<Unit> = full_squad()
            .into_iter()
            .chain([
                mook("m"),
                enforcer("e"),
                brute("b"),
                sniper("s"),
                grenadier("g"),
                swarmer("w"),
                breaker("k"),
                bomber("o"),
            ])
            .collect();
        for u in &roster {
            assert!(!u.weapons().is_empty(), "{} should be armed", u.name);
            let ev = u.evasion();
            assert!((10..=16).contains(&ev), "{} evasion {ev} out of band", u.name);
        }
    }

    /// The whole campaign (every new archetype, AoE / on-death / netrunning, the Survive
    /// objective) plays to a terminal outcome without panicking — and reproducibly.
    #[test]
    fn the_campaign_resolves_and_is_deterministic() {
        let play = |seed| {
            let mut g = Game::new(full_squad(), campaign(), seed);
            let report = g.play();
            assert_ne!(report.outcome, GameOutcome::Ongoing);
            (g.outcome(), g.roster().len())
        };
        assert_eq!(play(7), play(7));
    }

    /// Soundness: the authored scenarios are *beatable* — the full squad clears the
    /// campaign on at least one seed (content that's hard, not impossible).
    #[test]
    fn the_full_squad_can_win_the_campaign() {
        let won = (0..20u64).any(|seed| {
            let mut g = Game::new(full_squad(), campaign(), seed);
            g.play();
            g.outcome() == GameOutcome::Won
        });
        assert!(won, "no seed in 0..20 cleared the campaign — content too hard?");
    }
}
