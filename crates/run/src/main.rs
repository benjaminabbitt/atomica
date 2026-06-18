//! `atomica-cli` — a **headless run runner**. Plays an authored gauntlet (a series of
//! encounters fought as an attrition run) and streams the sim's **structured combat
//! log**, encounter by encounter.
//!
//! Usage:
//!   - `atomica-cli [text|logfmt|json] [seed] [scenario]` — play one seed, stream the
//!     trace (`text` human · `logfmt` structured · `json` one object/line).
//!   - `atomica-cli probe [N] [scenario]` — sweep `N` seeds (default 50) and print
//!     **balance stats** (clear rate, losses, ticks, hit/miss, netrunning, by-cause).
//!
//! `scenario` is `gauntlet` (default, the tuned core trio), `street` (the strike team's
//! street war), or `laststand` (a Survive hold-out). The event log streams to **stdout**;
//! headers / losses / verdict go to **stderr**.

use atomica_run::{content, Run, RunOutcome, RunPlan};
use atomica_sim::{CombatEvent, Unit};
use std::collections::BTreeMap;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args.first().map(String::as_str).unwrap_or("text");
    if mode == "probe" {
        let seeds = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(50);
        probe(seeds, args.get(2).map(String::as_str).unwrap_or("gauntlet"));
        return;
    }
    trace(
        mode,
        args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0xC0DE),
        args.get(2).map(String::as_str).unwrap_or("gauntlet"),
    );
}

/// Resolve a scenario name to its roster + run plan (defaults to the tuned gauntlet).
fn scenario(name: &str) -> (Vec<Unit>, RunPlan) {
    match name {
        "street" | "streetwar" => (content::full_squad(), content::street_war()),
        "laststand" | "last" => (content::full_squad(), content::last_stand()),
        "capture" | "hold" => (content::full_squad(), content::capture()),
        "extract" | "grab" => (content::full_squad(), content::extract_run()),
        "holdline" | "defend" => (content::full_squad(), content::hold_the_line()),
        "seize" | "flag" => (content::full_squad(), content::seize()),
        _ => (content::starter_roster(), content::gauntlet()),
    }
}

/// Play one seed of `scenario` and stream its structured combat log.
fn trace(format: &str, seed: u64, scenario_name: &str) {
    let (roster, plan) = scenario(scenario_name);
    let mut run = Run::new(roster, plan.encounters, seed).with_log();

    eprintln!("▶ {} — fielding {} units", plan.name, run.roster().len());
    while let Some(report) = run.fight_next() {
        eprintln!(
            "\n── {} ──  outcome={:?}  objective={:?}  survivors={}",
            report.encounter, report.outcome, report.objective, report.survivors
        );
        for rec in &report.events {
            let line = match format {
                "json" => rec.json(),
                "logfmt" => rec.logfmt(),
                _ => rec.to_string(),
            };
            println!("{line}");
        }
        if !report.losses.is_empty() {
            eprintln!("   ✝ lost: {}", report.losses.join(", "));
        }
    }

    let verdict = match run.outcome() {
        RunOutcome::Won => "RUN CLEARED",
        RunOutcome::Lost => "RUN FAILED",
        RunOutcome::Ongoing => "unresolved",
    };
    eprintln!("\n■ {verdict} — {} survivor(s) remain", run.roster().len());
}

/// Sweep `seeds` runs of `scenario_name` and print aggregate **balance** numbers — the
/// loop we tune against (deterministic per seed, so the sweep is reproducible).
fn probe(seeds: u64, scenario_name: &str) {
    let (mut clears, mut losses, mut survivors) = (0u64, 0u64, 0u64);
    let (mut hits, mut misses, mut hacks, mut breaches, mut spreads) = (0u64, 0u64, 0u64, 0u64, 0u64);
    let (mut tick_sum, mut encounters) = (0u64, 0u64);
    // What's actually killing the squad: every roster death bucketed by cause
    // ("weapon" for a lethal attack, the DoT's name — Virus / Bleed — for a status tick).
    let mut deaths_by_cause: BTreeMap<&'static str, u64> = BTreeMap::new();

    for seed in 0..seeds {
        let (roster, plan) = scenario(scenario_name);
        let mut run = Run::new(roster, plan.encounters, seed).with_log();
        while let Some(r) = run.fight_next() {
            encounters += 1;
            tick_sum += r.events.last().map_or(0, |e| e.tick as u64);
            // Player units deploy first, so ids `0..roster_size` are ours; a lethal event
            // on one of those ids is a roster loss we can attribute.
            let roster_size = (r.survivors + r.losses.len()) as u32;
            for e in &r.events {
                match &e.event {
                    CombatEvent::Attacked { target, killed: true, .. } if target.0 < roster_size => {
                        *deaths_by_cause.entry("weapon").or_default() += 1;
                    }
                    CombatEvent::Damaged { unit, cause, killed: true, .. } if unit.0 < roster_size => {
                        *deaths_by_cause.entry(cause).or_default() += 1;
                    }
                    _ => {}
                }
                match e.event.kind() {
                    "attacked" => hits += 1,
                    "missed" => misses += 1,
                    "hacked" => hacks += 1,
                    "breached" => breaches += 1,
                    "spread" => spreads += 1,
                    _ => {}
                }
            }
            losses += r.losses.len() as u64;
        }
        clears += (run.outcome() == RunOutcome::Won) as u64;
        survivors += run.roster().len() as u64;
    }

    let fielded = scenario(scenario_name).0.len();
    let attacks = hits + misses;
    let pct = |n: u64, d: u64| if d == 0 { 0.0 } else { 100.0 * n as f64 / d as f64 };
    let avg = |n: u64, d: u64| if d == 0 { 0.0 } else { n as f64 / d as f64 };
    println!("balance probe — {seeds} seeds of {scenario_name}");
    println!("  clear rate      : {:>5.0}%   ({clears}/{seeds})", pct(clears, seeds));
    println!("  avg losses/run  : {:>5.2}   (of {fielded} fielded)", avg(losses, seeds));
    println!("  avg survivors   : {:>5.2} / {fielded}", avg(survivors, seeds));
    println!("  avg ticks/enc   : {:>5.1}", avg(tick_sum, encounters));
    println!("  to-hit miss     : {:>5.0}%   ({misses} miss / {attacks} attacks)", pct(misses, attacks));
    println!("  netrunning      : {hacks} hacks, {breaches} breaches");
    println!("  contagion       : {spreads} spreads");
    let killed: u64 = deaths_by_cause.values().sum();
    if killed == 0 {
        println!("  losses by cause : (none)");
    } else {
        // Loudest cause first, so the balance lever is obvious at a glance.
        let mut by_cause: Vec<_> = deaths_by_cause.into_iter().collect();
        by_cause.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        println!("  losses by cause :");
        for (cause, n) in by_cause {
            println!("      {cause:<10} {n:>4}   ({:>3.0}%)", pct(n, killed));
        }
    }
}
