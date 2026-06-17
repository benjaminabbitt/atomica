//! Macroquad + egui front-end for CHROME AND CODE.
//!
//! This layer is deliberately thin: it owns a [`Battle`] from `atomica-sim`, draws
//! a snapshot of it each frame, and drives ticks from a step button / auto-play
//! timer. All game rules live in the sim crate.

use atomica_sim::{
    ArmorClass, Attack, BaseLine, Battle, Character, Chassis, DamageType, Defense, Footprint, Hex,
    Implant, Outcome, Pan, Skill, StatusSpec, Team, Unit, UnitId,
};
use egui_macroquad::egui;
use macroquad::prelude::*;

/// Pixels per hex (centre-to-corner). Flat-top orientation.
const HEX_SIZE: f32 = 34.0;
/// Seconds between auto-play ticks.
const AUTO_INTERVAL: f32 = 0.6;

/// Convert an axial hex to a screen pixel, given a board origin (flat-top layout).
fn hex_to_pixel(h: Hex, origin: Vec2) -> Vec2 {
    let x = HEX_SIZE * 1.5 * h.q as f32;
    let y = HEX_SIZE * 3f32.sqrt() * (h.r as f32 + h.q as f32 / 2.0);
    origin + vec2(x, y)
}

/// A tiny demo encounter so the window shows something real on first run.
fn demo_battle() -> Battle {
    let mk = |id: u32, name: &str, team, q, r, dmg, init, range, dtype, pen, armor_class| Unit {
        id: UnitId(id),
        name: name.to_string(),
        team,
        pos: Hex::new(q, r),
        integrity: 40.0,
        max_integrity: 40.0,
        defense: Defense { barrier: 6.0, plating: 6.0 },
        armor_class,
        chassis: Chassis::Augmented,
        skills: Chassis::Augmented.baseline_skills(),
        initiative: init,
        speed: 1,
        link: 0,
        firewall: 0,
        immunity: 0,
        attack: Attack {
            damage: dmg,
            dtype,
            pen,
            range,
            min_range: 1,
            emp: false,
            footprint: Footprint::Single,
        },
        weapons: Vec::new(),
        hack: None,
        implants: Vec::new(),
        pan: Pan::Meshed,
        statuses: Vec::new(),
        character: Character::new(BaseLine::default()),
        alive: true,
    };
    use atomica_sim::PenTier::*;
    use ArmorClass::*;
    use DamageType::*;
    let mut units = vec![
        mk(0, "Katana", Team::A, 0, 0, 14.0, 7.0, 1, Slashing, Internal, Padding),
        mk(1, "Runner", Team::A, 0, 2, 9.0, 5.0, 4, Piercing, Contact, Mail),
        mk(2, "Bulwark", Team::B, 5, 0, 7.0, 4.0, 1, Bludgeoning, Contact, Plate),
        mk(3, "SMG", Team::B, 5, 2, 8.0, 6.0, 3, Piercing, External, Mail),
    ];
    // Wire the Runner as a netrunner by **installing a cyberdeck** — the implant
    // grants the hack loadout and folds in its Link (5) + Firewall (the derived
    // stat line, docs/cyberware.md §7). Its Hacking is a character skill.
    units[1].skills.set(Skill::Hacking, 4);
    units[1].install(Implant::cyberdeck());
    // The enemy line shows the netrunning spread (§ calibration): Bulwark is a
    // hardened, connected "fortress" (deep if cracked); SMG a soft, low-Link
    // "mook" (easy to land but the thin channel keeps it shallow).
    units[2].link = 5;
    units[2].firewall = 15; // hardened + connected
    units[2].attack.emp = true; // an EMP maul — frying the Runner's deck on contact
    units[3].link = 2;
    units[3].firewall = 9; // soft + dark
    // Seed a couple of statuses so the pipeline is visible on first run.
    units[2].add_status(StatusSpec::burn(), 6, 3);
    units[3].add_status(StatusSpec::lag(), 6, 1);
    Battle::new(units, 0xC0DE)
}

fn team_color(team: Team, alive: bool) -> Color {
    let base = match team {
        Team::A => Color::new(0.30, 0.80, 0.95, 1.0), // cyan
        Team::B => Color::new(0.95, 0.35, 0.45, 1.0), // red
    };
    if alive { base } else { Color::new(0.35, 0.38, 0.42, 1.0) }
}

#[macroquad::main("CHROME AND CODE")]
async fn main() {
    let mut battle = demo_battle();
    let mut auto = false;
    let mut timer = 0.0f32;

    loop {
        clear_background(Color::new(0.043, 0.055, 0.078, 1.0)); // near-black blue

        // --- advance the sim --------------------------------------------------
        let outcome = battle.outcome();
        if auto && matches!(outcome, Outcome::Ongoing) {
            timer += get_frame_time();
            if timer >= AUTO_INTERVAL {
                timer = 0.0;
                battle.step();
            }
        }

        // --- draw the board ---------------------------------------------------
        let origin = vec2(120.0, screen_height() * 0.5 - 60.0);

        // Faint grid behind the units.
        for q in -1..=6 {
            for r in -1..=4 {
                let p = hex_to_pixel(Hex::new(q, r), origin);
                draw_hexagon(p.x, p.y, HEX_SIZE, 1.0, true, Color::new(0.16, 0.2, 0.28, 1.0), BLANK);
            }
        }

        for u in &battle.units {
            let p = hex_to_pixel(u.pos, origin);
            let col = team_color(u.team, u.is_alive());
            draw_hexagon(p.x, p.y, HEX_SIZE * 0.82, 2.0, true, col, Color::new(col.r, col.g, col.b, 0.18));
            draw_text(&u.name, p.x - HEX_SIZE * 0.7, p.y - 4.0, 18.0, col);

            // Integrity bar.
            let frac = (u.integrity / u.max_integrity).clamp(0.0, 1.0);
            let bw = HEX_SIZE * 1.3;
            let bx = p.x - bw / 2.0;
            let by = p.y + 6.0;
            draw_rectangle(bx, by, bw, 5.0, Color::new(0.1, 0.1, 0.12, 1.0));
            draw_rectangle(bx, by, bw * frac, 5.0, col);
        }

        // --- egui control panel ----------------------------------------------
        egui_macroquad::ui(|ctx| {
            egui::Window::new("Battle").default_pos((screen_width() - 260.0, 20.0)).show(ctx, |ui| {
                ui.label(format!("Tick: {}", battle.tick));
                ui.label(match outcome {
                    Outcome::Ongoing => "Status: fighting".to_string(),
                    Outcome::Winner(t) => format!("Winner: Team {t:?}"),
                    Outcome::Draw => "Result: draw".to_string(),
                });
                ui.separator();

                ui.horizontal(|ui| {
                    if ui.add_enabled(matches!(outcome, Outcome::Ongoing), egui::Button::new("Step")).clicked() {
                        battle.step();
                    }
                    ui.checkbox(&mut auto, "Auto");
                    if ui.button("Reset").clicked() {
                        battle = demo_battle();
                        auto = false;
                        timer = 0.0;
                    }
                });

                ui.separator();
                for u in &battle.units {
                    if !u.is_alive() {
                        ui.label(format!("{:?}  {:<8} (down)", u.team, u.name));
                        continue;
                    }
                    let statuses: String = u
                        .statuses
                        .iter()
                        .map(|s| {
                            if s.stacks > 1 {
                                format!("{}×{}", s.spec.name, s.stacks)
                            } else {
                                s.spec.name.to_string()
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let deck = if u.hack.is_some() { "⚡" } else { " " };
                    ui.label(format!(
                        "{:?}  {:<7}{} {:>4.0}/{:<3.0}  [{:?}]  L{:<2} {}",
                        u.team, u.name, deck, u.integrity, u.max_integrity, u.armor_class, u.link,
                        statuses
                    ));
                }
            });
        });
        egui_macroquad::draw();

        next_frame().await;
    }
}
