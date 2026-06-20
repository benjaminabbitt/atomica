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
//! Each encounter is fought on an authored **board** ([`atomica_sim::Terrain`]) — a bounded
//! arena with blockers (route around), cover (harder to hit), and hazards (burn whoever
//! stands there). Bounds corner the kiters; the maps give the skirmish/cover AI something
//! to use. The deploy edges (player column 0, enemy column 6) stay clear; terrain lives in
//! the middle.
//!
//! Roster/enemy templates carry a placeholder id `0` and `Team::A`; [`deploy`](crate)
//! reassigns both at battle start, so only the stat line / loadout here matters.

use crate::{Encounter, GamePlan, RunPlan};
use atomica_sim::{
    ArmorClass, Attack, Chassis, DamageType, DeathTrigger, Footprint, FoundAction, Hex, Implant,
    MovementProfile, NetDoctrine, ObjectiveKind, PenTier, Program, Skill, Team, TargetingProfile,
    Terrain, Tile, Unit, EquipmentTag, EquipmentTags,
};

fn weapon(damage: f32, dtype: DamageType, pen: PenTier, range: i32) -> Attack {
    // Role from reach: a reach-1 weapon is Melee, anything longer is a Gunnery weapon —
    // and **ranged** (an inherent to-hit penalty that grows with distance).
    let (skill, tags) = if range > 1 {
        (Skill::Gunnery, EquipmentTags::RANGED)
    } else {
        (Skill::Melee, EquipmentTags::NONE)
    };
    // Speed (the Dodge penalty, `docs/stats.md`): a fast round (3) vs a slow swing (1).
    let speed = if range > 1 { 3 } else { 1 };
    Attack {
        damage,
        speed,
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
    a.tags = a.tags.with(EquipmentTag::Awkward);
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
    // Average (GURPS 10) baseline across the four attributes (`docs/stats.md`); archetypes
    // bump their signature stat and layer skill tiers on top. Evasion = Dexterity + Evade-tier,
    // and Evade defaults to untrained (−4), so a non-dodger sits at Dex − 4 — a secondary save.
    Unit::new(0, name, Team::A, Chassis::Augmented)
        .with_integrity(hp)
        .with_initiative(init)
        .with_body(10.0)
        .with_dexterity(10.0)
        .with_intellect(10.0)
        .with_will(10.0)
}

// -- Player archetypes ------------------------------------------------------------

/// A **blade** — a fast melee bruiser. Slashing shreds the unarmored but glances off
/// Plate (×0.5), so the blade wants soft targets. Runs **skin weave** — light subdermal
/// armor that soaks blows into its own HP (a buffer that wears through under fire).
pub fn blade(name: &str) -> Unit {
    let mut u = body(name, 68.0, 7.0)
        .with_body(12.0)
        .with_dexterity(11.0)
        .with_skill(Skill::Melee, 3) // master duelist ⇒ effective Melee 15
        .with_skill(Skill::Evade, 1) // nimble, but no acrobat ⇒ Evasion 11 + 1 = 12
        .with_attack(weapon(14.0, DamageType::Slashing, PenTier::Internal, 1));
    u.install(Implant::skin_weave());
    u
}

/// A **netrunner** — a ranged sidearm plus a cyberdeck (hacks enemy chrome). Piercing
/// is also halved by Plate, so the runner leans on the breach, not the gun, vs armor.
pub fn runner(name: &str) -> Unit {
    let mut u = body(name, 60.0, 6.0)
        .with_dexterity(11.0)
        .with_intellect(12.0)
        .with_skill(Skill::Gunnery, 2) // expert shot ⇒ effective 13
        .with_skill(Skill::Hacking, 3) // ace netrunner ⇒ effective 15
        // Evade untrained: Dex 11 − 4 ⇒ Evasion 7
        .with_attack(awkward(weapon(8.0, DamageType::Piercing, PenTier::Contact, 4))) // a rifle — clumsy in a clinch
        .with_doctrine(NetDoctrine::Burner); // dives heat-prone chrome, leads Overheat (else softens)
    u.install(Implant::cyberdeck());
    u.install(Implant::neural_net()); // a cognition co-processor — sharper Hacking, stiffer defense
    u.install_program(Program::Lockware); // the deck's basic breach program…
    u.install_program(Program::Overheat); // …the common Overheat program…
    u.install_program(Program::Breach); // …and a softener: a breach exposes the target to the squad
    u
}

/// A **bulwark** — an armored tank with a heavy maul. Bludgeoning is *amplified* vs
/// Plate (×1.5), so the slow bulwark is the answer to the hardened enemies. Heavy and
/// slow — it hits hard but is **easy to hit** (low Evasion).
pub fn bulwark(name: &str) -> Unit {
    body(name, 94.0, 5.0)
        .with_armor(ArmorClass::Plate)
        .with_body(13.0)
        .with_dexterity(9.0)
        .with_skill(Skill::Melee, 1) // a seasoned maul ⇒ effective 14
        // heavy and slow: Evade untrained, Dex 9 − 4 ⇒ Evasion 5, easy to hit
        .with_attack(weapon(12.0, DamageType::Bludgeoning, PenTier::Contact, 1))
}

/// A **marksman** — a glass-cannon sharpshooter. A smartlinked long rifle (IFF, so it
/// won't tag a teammate in its lane) that picks the **back line** and **skirmishes** to its
/// range-5 standoff: closes when out of range, backs off when crowded, fires from the sweet
/// spot. Deadly at distance, clumsy and fragile if something closes (low HP, AWKWARD).
pub fn marksman(name: &str) -> Unit {
    let rifle = smart(awkward(weapon(9.0, DamageType::Piercing, PenTier::Contact, 5)));
    body(name, 56.0, 6.0)
        .with_dexterity(13.0)
        .with_skill(Skill::Gunnery, 2) // elite sharpshooter ⇒ effective 15
        // Evade untrained: Dex 13 − 4 ⇒ Evasion 9
        .with_attack(rifle)
        .with_targeting(TargetingProfile::Backline)
        .with_movement(MovementProfile::Kite)
}

/// A **sapper** — an EMP shock-trooper, the anti-chrome answer. A short-range pulse that
/// fries digital cyberware *through* Firewall (§7I); it hunts the **biggest threat**, so
/// it bee-lines the chromed heavies the rest of the squad struggles to crack.
pub fn sapper(name: &str) -> Unit {
    let shock = emp(weapon(6.0, DamageType::Bludgeoning, PenTier::Contact, 2));
    body(name, 66.0, 5.0)
        .with_dexterity(11.0)
        .with_skill(Skill::Gunnery, 0) // competent ⇒ effective 11
        // Evade untrained: Dex 11 − 4 ⇒ Evasion 7
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

/// A **heist team** for a [`datamine`] dive — the core trio plus the **sapper** (its EMP cracks
/// chrome through Firewall, a second way at the node), but **no marksman**: its `Backline`
/// targeting would shell the backmost unit (the objective node itself) and slag the data.
pub fn heist_team() -> Vec<Unit> {
    vec![blade("Katana"), runner("Glitch"), bulwark("Anvil"), sapper("Surge")]
}

// -- Enemy archetypes -------------------------------------------------------------

/// A soft, lightly-armored **mook** (Mail) — cannon fodder the blade carves up. Hits
/// lightly; the threat is in numbers.
fn mook(name: &str) -> Unit {
    body(name, 74.0, 5.0)
        .with_armor(ArmorClass::Mail)
        .with_skill(Skill::Gunnery, -2) // a poor shot ⇒ effective 8
        // Evade untrained: baseline Dex 10 − 4 ⇒ Evasion 6, easy to carve up
        .with_attack(weapon(4.0, DamageType::Piercing, PenTier::External, 2))
}

/// A hardened, **chromed** enemy in **Plate** — shrugs off the blade / gun (×0.5), so
/// it's a slog until the bulwark caves it in. Its **inert plating** can't be hacked,
/// but the netrunner can breach its **reflex booster** (digital smartware) for a Seizure.
fn enforcer(name: &str) -> Unit {
    let mut u = body(name, 104.0, 5.0)
        .with_armor(ArmorClass::Plate)
        .with_body(11.0)
        .with_skill(Skill::Melee, 2) // a hardened bruiser ⇒ effective 13
        // Evade untrained: Dex 10 − 4 ⇒ Evasion 6
        .with_attack(weapon(5.0, DamageType::Bludgeoning, PenTier::Contact, 1));
    u.character.base_mut().link = 4.0; // a networked surface to hack at
    u.character.base_mut().firewall = 6.0; // hardened: rolls an active defense vs hackers (stats.md §4)
    u.install(Implant::subdermal_plating()); // physical — bulwark's problem, not the runner's
    u.install(Implant::reflex_booster()); // digital smartware — the runner's breach target
    u
}

/// A **brute** — a tougher, up-armored mook (Plate). No chrome to breach and no plague;
/// just a meatier body than the rank-and-file, the muscle of a hardened position.
fn brute(name: &str) -> Unit {
    body(name, 94.0, 5.0)
        .with_armor(ArmorClass::Plate)
        .with_skill(Skill::Gunnery, -2) // a poor shot ⇒ effective 8
        // Evade untrained: Dex 10 − 4 ⇒ Evasion 6
        .with_attack(weapon(5.0, DamageType::Piercing, PenTier::Contact, 2))
}

/// A **sniper** — a long-range **skirmisher** that keeps its range-6 standoff (backs off
/// when crowded, but now gets **cornered against the board edge**) and picks off the
/// squad's **biggest threat**, so the squad eats fire on the approach. Fragile up close
/// (AWKWARD, low HP): rush it into a wall and gut it.
fn sniper(name: &str) -> Unit {
    body(name, 58.0, 6.0)
        .with_dexterity(12.0)
        .with_skill(Skill::Gunnery, 2) // a real marksman ⇒ effective 14
        // Evade untrained: Dex 12 − 4 ⇒ Evasion 8; fragile up close
        .with_attack(awkward(weapon(7.0, DamageType::Piercing, PenTier::Contact, 6)))
        .with_targeting(TargetingProfile::HighestThreat)
        .with_movement(MovementProfile::Kite)
}

/// A **grenadier** — lobs a **blast** (radius 1, no IFF). The AoE punishes a clustered
/// squad — and catches its *own* line if they bunch — so it rewards spreading out.
fn grenadier(name: &str) -> Unit {
    body(name, 70.0, 4.0)
        .with_armor(ArmorClass::Mail)
        .with_skill(Skill::Gunnery, -2) // a poor shot ⇒ effective 8
        // Evade untrained: Dex 10 − 4 ⇒ Evasion 6
        .with_attack(blast(weapon(6.0, DamageType::Bludgeoning, PenTier::External, 3), 1))
}

/// A **swarmer** — a fast, fragile rusher (speed 2, **Swarm**) that hunts the **lowest
/// Integrity** target to finish the wounded. Trivial one-on-one; a threat in numbers.
fn swarmer(name: &str) -> Unit {
    body(name, 40.0, 7.0)
        .with_dexterity(11.0)
        .with_skill(Skill::Melee, 0) // a rusher's slash ⇒ effective 11
        .with_skill(Skill::Evade, -2) // quicker than a grunt ⇒ Evasion 11 − 2 = 9
        .with_speed(2)
        .with_attack(weapon(5.0, DamageType::Slashing, PenTier::Internal, 1))
        .with_targeting(TargetingProfile::LowestIntegrity)
        .with_movement(MovementProfile::Swarm)
}

/// A **breaker** — an enemy netrunner with a cyberdeck: it **hacks the squad's chrome**
/// (the runner's deck — the sapper has none to lose) while **skirmishing** at its sidearm's
/// range. A mirror of the player's runner, and its own deck is a breach target right back.
fn breaker(name: &str) -> Unit {
    let mut u = body(name, 56.0, 6.0)
        .with_intellect(12.0)
        .with_skill(Skill::Hacking, 2) // expert runner ⇒ effective 14
        .with_skill(Skill::Gunnery, 0) // a rank-and-file sidearm ⇒ effective 10
        // Evade untrained: Dex 10 − 4 ⇒ Evasion 6
        .with_attack(weapon(4.0, DamageType::Piercing, PenTier::External, 3))
        .with_targeting(TargetingProfile::HighestThreat)
        .with_movement(MovementProfile::Kite)
        .with_doctrine(NetDoctrine::Controller); // ICE: dives the biggest gun, leads Spoof
    u.install(Implant::cyberdeck());
    u.install(Implant::neural_net()); // mirrors the player runner's cognition edge
    u.install_program(Program::Lockware); // a mirror of the player runner's loadout…
    u.install_program(Program::Overheat);
    u.install_program(Program::Spoof); // …plus ICE of its own: it corrupts the squad's targeting…
    u.install_program(Program::Ghost); // …and runs dark, harder to crack back
    u
}

/// A **bomber** — **Swarms** in to die, then **detonates**: a parting blast (radius 1,
/// friendly fire) to everyone adjacent. Killing it at range, or not bunched, is the play.
fn bomber(name: &str) -> Unit {
    body(name, 50.0, 4.0)
        .with_armor(ArmorClass::Mail)
        .with_skill(Skill::Melee, -2) // a poor strike ⇒ effective 8
        // Evade untrained: Dex 10 − 4 ⇒ Evasion 6
        .with_attack(weapon(4.0, DamageType::Bludgeoning, PenTier::Contact, 1))
        .with_movement(MovementProfile::Swarm)
        .with_on_death(DeathTrigger::Detonate {
            damage: 10.0,
            dtype: DamageType::Bludgeoning,
            pen: PenTier::Contact,
            radius: 1,
        })
}

// -- Boards -----------------------------------------------------------------------
//
// Maps are sized so the deploy columns fit: the player lands on column 0 (rows top-down),
// the enemy on column 6, on an 8×6 arena (cols 0..=7, rows 0..=5). Terrain lives in the
// **middle** columns (1..=5) — the deploy edges stay clear ground.

/// A blank bounded arena sized for deployment (cols 0..=7, rows 0..=5).
fn arena() -> Terrain {
    Terrain::arena(8, 6)
}

/// Stamp `Blocked` walls (impassable, route around) at the given `(q, r)` hexes.
fn walls(t: Terrain, hexes: &[(i32, i32)]) -> Terrain {
    t.fill(hexes.iter().map(|&(q, r)| Hex::new(q, r)), Tile::Blocked)
}

/// Stamp `Cover(tn)` (harder to hit its occupant) at the given hexes.
fn cover(t: Terrain, tn: i32, hexes: &[(i32, i32)]) -> Terrain {
    t.fill(hexes.iter().map(|&(q, r)| Hex::new(q, r)), Tile::Cover(tn))
}

/// Stamp a `Hazard` field (Internal damage each tick — the AI steps around it) at the hexes.
fn hazard(t: Terrain, damage: f32, hexes: &[(i32, i32)]) -> Terrain {
    let tile = Tile::Hazard { damage, dtype: DamageType::Piercing, pen: PenTier::Internal };
    t.fill(hexes.iter().map(|&(q, r)| Hex::new(q, r)), tile)
}

/// **Alley** — scattered crates (cover) to fight over; a soft, open brawl map.
fn alley() -> Terrain {
    cover(arena(), 2, &[(3, 1), (3, 4), (4, 2), (2, 3)])
}

/// **Checkpoint** — a wall across column 3 with a single **gate** at row 2 (pathing
/// funnels through it), cover flanking the gate.
fn checkpoint() -> Terrain {
    let t = walls(arena(), &[(3, 0), (3, 1), (3, 3), (3, 4), (3, 5)]);
    cover(t, 3, &[(2, 2), (4, 2)])
}

/// **Quarantine** — a toxic spill burning the center; fighting flows around it.
fn quarantine() -> Terrain {
    let t = hazard(arena(), 6.0, &[(3, 2), (3, 3), (4, 2), (4, 3)]);
    cover(t, 2, &[(2, 1), (5, 4)])
}

/// **Open ground** with a little hard cover — room for a swarm to close.
fn yard() -> Terrain {
    let t = walls(arena(), &[(3, 2), (4, 3)]);
    cover(t, 2, &[(2, 1), (2, 4), (5, 2)])
}

/// **Firing lanes** — cover staggered down both sides so the snipers nest and the squad
/// has something to advance behind.
fn firing_lanes() -> Terrain {
    cover(arena(), 3, &[(5, 1), (5, 4), (4, 2), (2, 1), (2, 4), (3, 3)])
}

/// **Server room** — racks (walls) carving two lanes, cover at the mouths.
fn server_room() -> Terrain {
    let t = walls(arena(), &[(3, 1), (4, 1), (3, 4), (4, 4)]);
    cover(t, 3, &[(2, 2), (5, 3)])
}

/// **Rooftop** — cover ringing the player's hold point (col 1) with a hazard strip on the
/// approach to channel the assault.
fn rooftop() -> Terrain {
    let t = cover(arena(), 3, &[(1, 1), (1, 3), (2, 2)]);
    hazard(t, 5.0, &[(4, 1), (4, 2), (4, 4)])
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
            )
            .on(alley()),
            Encounter::new(
                "Corp Checkpoint",
                vec![enforcer("Enforcer"), mook("Guard-1"), mook("Guard-2"), mook("Guard-3")],
            )
            .on(checkpoint()),
            Encounter::new(
                "Lockdown Zone",
                vec![brute("Heavy"), enforcer("Warden"), mook("Orderly")],
            )
            .on(quarantine()),
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
            )
            .on(yard()),
            // Ranged pressure: a sniper picking the heavies, a grenadier punishing clusters.
            Encounter::new(
                "Crossfire",
                vec![sniper("Longshot"), grenadier("Lobber"), mook("Gun-1"), mook("Gun-2")],
            )
            .on(firing_lanes()),
            // The netrunning mirror — the breaker hacks your deck while the wall holds.
            Encounter::new(
                "Net Duel",
                vec![breaker("Daemon"), enforcer("Warden"), brute("Slab"), mook("Goon")],
            )
            .on(server_room()),
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
        .on(rooftop())
        .with_objective(ObjectiveKind::Survive(8))],
    )
}

/// A **capture** run — a **Hold** objective: fight through the gate and take the node, then
/// keep it. The teeth are positional (Hold *fails* if the fight ends without control), so
/// it exercises the objective-seeking AI — the squad flows to the point and holds it rather
/// than just hunting the enemy. On the checkpoint map, the hold hex sits past the gate.
pub fn capture() -> RunPlan {
    RunPlan::new(
        "Node Capture",
        // Closing defenders only — a *kiting* enemy would never be hunted while the squad
        // fixates on the node, so the fight wouldn't end. They come to contest the point.
        vec![Encounter::new(
            "Uplink",
            vec![enforcer("Guard"), brute("Bruiser"), mook("Sentry-1"), mook("Sentry-2")],
        )
        .on(alley())
        .with_objective(ObjectiveKind::Hold(Hex::new(4, 2), 6))],
    )
}

/// **Smash & Grab** — an **Extract** objective: punch in to the data core, grab it, and run
/// it back to the extraction point. Nearest-N means one courier peels off for the item
/// while the squad screens — and *hunts the overwatch sniper*, which a totalizing pull
/// would have left plinking forever.
pub fn extract_run() -> RunPlan {
    RunPlan::new(
        "Smash & Grab",
        vec![Encounter::new(
            "Data Core",
            vec![enforcer("Ward"), sniper("Eye"), mook("Net-1"), mook("Net-2")],
        )
        .on(yard())
        .with_objective(ObjectiveKind::Extract { item: Hex::new(6, 2), exit: Hex::new(0, 2) })],
    )
}

/// A **data node** — a bolted-down networked terminal, the [`datamine`] prize. Tanky and
/// immobile, with a fat surface (Link) and its own wall; you **crack it by hacking** (breaching
/// its implant), not by shelling it. The guarding netrunner stiffens its defense (`net_defense`).
fn data_node(name: &str) -> Unit {
    let mut u = body(name, 320.0, 3.0)
        .with_armor(ArmorClass::Plate)
        .with_speed(0) // bolted to the floor
        .with_movement(MovementProfile::Hold);
    u.character.base_mut().link = 5.0; // a fat surface to dive
    u.character.base_mut().firewall = 2.0; // a thin own-wall; the guarding runner stiffens it
    u.install(Implant::firewall_suite()); // a digital implant — breaching it (Offline) cracks the node
    u.disarm(); // a terminal, not a combatant — it never attacks
    u
}

/// A **data heist** — breach the bolted-down [`data_node`] to extract its data (the netrunning
/// objective). A guarding **breaker** defends the node *actively*, so the play is **clear the
/// ICE, then crack the node**: drop the enemy runner to kill the active defense, then dive. The
/// node deploys first (enemy row 0 ⇒ hex `(6, 0)`).
pub fn datamine() -> RunPlan {
    RunPlan::new(
        "Data Heist",
        vec![Encounter::new(
            "Black Vault",
            vec![
                data_node("Server"), // row 0 ⇒ the node hex below
                breaker("ICE").with_movement(MovementProfile::Hold), // guards the node (holds, does not kite)
                enforcer("Sentinel"),
                mook("Sec-1"),
                mook("Sec-2"),
            ],
        )
        .on(yard())
        .with_objective(ObjectiveKind::Datamine(Hex::new(6, 0)))],
    )
}

/// **Hold the Line** — a **CaptureHold**: take the junction and hold it four cumulative
/// rounds against a closing assault (clearing the field early also seals it).
pub fn hold_the_line() -> RunPlan {
    RunPlan::new(
        "Hold the Line",
        vec![Encounter::new(
            "Junction",
            vec![
                brute("Ram-1"),
                brute("Ram-2"),
                swarmer("Dog-1"),
                swarmer("Dog-2"),
                mook("Gun"),
            ],
        )
        .on(alley())
        .with_objective(ObjectiveKind::CaptureHold(Hex::new(4, 2), 4))],
    )
}

/// **Seize the Relay** — a sticky **Flag**: grab the forward relay (once touched it's yours,
/// even if you're driven off) then hold the ground three rounds to lock it in.
pub fn seize() -> RunPlan {
    RunPlan::new(
        "Seize the Relay",
        vec![Encounter::new(
            "Relay",
            vec![enforcer("Keeper"), brute("Slab"), mook("Tech-1"), mook("Tech-2")],
        )
        .on(firing_lanes())
        .with_objective(ObjectiveKind::Flag(Hex::new(5, 2), 3))],
    )
}

/// **Recon Sweep** — a **Search**: sweep three caches, and the right one reveals a stash to
/// capture and hold three rounds ("search N, find the correct, then do the above"). The
/// squad fans through the block while the rest screen the closing guards.
pub fn recon() -> RunPlan {
    RunPlan::new(
        "Recon Sweep",
        vec![Encounter::new(
            "Search the Block",
            vec![
                brute("Heavy"),
                enforcer("Guard"),
                swarmer("Dog-1"),
                swarmer("Dog-2"),
                mook("Gun"),
            ],
        )
        .on(yard())
        .with_objective(ObjectiveKind::search(
            &[Hex::new(3, 1), Hex::new(3, 4), Hex::new(4, 2)],
            2,
            FoundAction::Capture(3),
        ))],
    )
}

/// The **campaign** — the full [`crate::Game`] tier: the tuned gauntlet, the harder street
/// war, then a tour of objective types (capture, extract, hold-the-line, seize, recon) and
/// a last stand, with R&R between each. Field it with [`full_squad`].
pub fn campaign() -> GamePlan {
    GamePlan::new("Night City", vec![
        gauntlet(),
        street_war(),
        capture(),
        extract_run(),
        hold_the_line(),
        seize(),
        recon(),
        last_stand(),
    ])
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
            // Evasion = Dexterity + Evade-tier (GURPS scale; Evade usually untrained at −4): a
            // sane band runs from a heavy (Dex 9 − 4) up to a trained, agile dodger.
            assert!((4..=12).contains(&ev), "{} evasion {ev} out of band", u.name);
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
