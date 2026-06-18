//! `atomica-cli` — a **headless battle runner**. Builds a demo encounter, plays it to a
//! decision, and streams the sim's **structured combat log**.
//!
//! Usage: `atomica-cli [text|logfmt|json] [seed]`
//!   - `text`   (default) — a human trace, one event per line.
//!   - `logfmt` — `tick=… event=… k=v …` for structured-log pipelines.
//!   - `json`   — one JSON object per line (jq-friendly).
//!
//! The log streams to **stdout**; a one-line summary + final roster go to **stderr**,
//! so `atomica-cli json | jq …` stays clean.

use atomica_sim::{
    Attack, Battle, Chassis, Corruption, DamageType, Footprint, Hex, Implant, Outcome, PenTier,
    Skill, Team, Unit,
};

/// Draw fallback — stop stepping after this many ticks (matches the sim's own cap).
const MAX_TICKS: u32 = 1000;

/// A single-target weapon profile.
fn weapon(damage: f32, dtype: DamageType, pen: PenTier, range: i32) -> Attack {
    Attack { damage, dtype, pen, range, min_range: 1, emp: false, footprint: Footprint::Single, smart: false }
}

/// A small demo encounter that exercises the whole pipeline: melee + ranged attacks,
/// a netrunner breaching chrome, and a contagion spreading.
fn demo_units() -> Vec<Unit> {
    use DamageType::{Bludgeoning, Piercing, Slashing};
    use PenTier::{Contact, External, Internal};

    // Team A — a melee blade and a netrunner.
    let katana = Unit::new(0, "Katana", Team::A, Chassis::Augmented)
        .at(Hex::new(0, 0))
        .with_initiative(7.0)
        .with_attack(weapon(14.0, Slashing, Internal, 1));
    let mut runner = Unit::new(1, "Runner", Team::A, Chassis::Augmented)
        .at(Hex::new(0, 2))
        .with_initiative(5.0)
        .with_attack(weapon(8.0, Piercing, Contact, 4));
    runner.skills.set(Skill::Hacking, 5);
    runner.install(Implant::cyberdeck()); // grants the hack loadout + Link/Firewall

    // Team B — a hardened, deck-bearing tank (a slot to breach) and a plague-carrying mook.
    let mut bulwark = Unit::new(2, "Bulwark", Team::B, Chassis::Augmented)
        .at(Hex::new(6, 0))
        .with_initiative(4.0)
        .with_attack(weapon(9.0, Bludgeoning, Contact, 1));
    bulwark.character.base_mut().link = 4.0;
    bulwark.character.base_mut().firewall = 6.0;
    bulwark.install(Implant::subdermal_plating());
    let mut mook = Unit::new(3, "Mook", Team::B, Chassis::Augmented)
        .at(Hex::new(6, 1)) // adjacent to Bulwark — proximity spread can reach
        .with_initiative(6.0)
        .with_attack(weapon(7.0, Piercing, External, 3));
    mook.apply_modifier(Corruption::plague(4.0, 12, 8)); // a virulent plague to watch spread

    vec![katana, runner, bulwark, mook]
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let format = args.first().map(String::as_str).unwrap_or("text");
    let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0xC0DE_u64);

    let mut battle = Battle::new(demo_units(), seed).with_log();
    let mut ticks = 0;
    while matches!(battle.outcome(), Outcome::Ongoing) && ticks < MAX_TICKS {
        battle.step();
        ticks += 1;
    }

    for r in battle.events() {
        let line = match format {
            "json" => r.json(),
            "logfmt" => r.logfmt(),
            _ => r.to_string(),
        };
        println!("{line}");
    }

    eprintln!(
        "— {} events over {} ticks; outcome: {:?}",
        battle.events().len(),
        battle.tick,
        battle.outcome()
    );
    for u in &battle.units {
        let state = if u.is_alive() { format!("{:.0} hp", u.integrity()) } else { "down".to_string() };
        eprintln!("  {:?} {:<7} {state}", u.team, u.name);
    }
}
