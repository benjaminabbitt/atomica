//! `atomica-cli` — a **headless run runner**. Plays an authored gauntlet (a series of
//! encounters fought as an attrition run) and streams the sim's **structured combat
//! log**, encounter by encounter.
//!
//! Usage: `atomica-cli [text|logfmt|json] [seed]`
//!   - `text`   (default) — a human trace, one event per line.
//!   - `logfmt` — `tick=… event=… k=v …` for structured-log pipelines.
//!   - `json`   — one JSON object per line (jq-friendly).
//!
//! The event log streams to **stdout**; encounter headers, losses, and the run verdict
//! go to **stderr**, so `atomica-cli json | jq …` stays a clean record stream.

use atomica_run::{content, Run, RunOutcome};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let format = args.first().map(String::as_str).unwrap_or("text");
    let seed = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(0xC0DE_u64);

    let plan = content::gauntlet();
    let mut run = Run::new(content::starter_roster(), plan.encounters, seed).with_log();

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
