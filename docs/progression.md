# Progression — encounters, runs, games (the meta-structure)

*The roguelike layers **above a single battle**: how combats chain into runs, runs
into a game, and where rest, objectives, and stakes live. Built in
[`crates/run`](../crates/run/src/lib.rs) (`atomica-run`) on the `sim`'s battle +
[`Objective`](../crates/sim/src/objective.rs) seam. **Supersedes the delta's §15
run-tree** on the within-run model (no shop/rest nodes inside a run; R&R is between
runs). ◆ = decision. Status: **✅ built** · **🔭 planned**. Numbers TBD.*

---

## 0. The three tiers ◆

Each tier is a **series of the one below**, and every level is a **named object**:

| Tier | Is | Type(s) | Rest between elements? | Status |
|---|---|---|---|---|
| **Encounter** | one combat | `Encounter` → `sim::Battle` | — | ✅ |
| **Run** | a series of encounters — an **attrition gauntlet** | `RunPlan` / `Run` | **no** (no R&R within) | ✅ |
| **Game** | a series of runs — the campaign | `GamePlan` / `Game` | **yes** (R&R) | ✅ |

> Every run is a series; **some have length one** — a single encounter is just a
> run of one.

**Named hierarchy, in and out:**

```text
plan:    GamePlan ── RunPlan ── Encounter        (what you set up)
report:  GameReport ─ RunReport ─ BattleReport   (what came back)
```

---

## 1. The spine rules ◆

| Rule | Meaning | Status |
|---|---|---|
| **Permadeath** | a unit that falls is gone for good | ✅ |
| **End-on-wipe** | you advance as long as you still field an army — you can *bleed the roster across a winning run* | ✅ |
| **No R&R within a run** | a run is an attrition gauntlet: carried **Integrity** damage and **chrome condition** persist combat→combat (only transient statuses reset) | ✅ |
| **R&R between runs** | survivors fully heal + the Ripperdoc repairs chrome (Degraded/Offline → Online; **Destroyed stays gone**) before the next run | ✅ |
| **Deterministic** | seeded → reproducible (each tier derives child seeds), like the sim it drives | ✅ |

**Two downtime scales** (both decided, both built):

| Scale | When | What |
|---|---|---|
| *(none)* | within a run, combat→combat | nothing — pure attrition |
| **R&R** | between runs | full heal + chrome repair (`rest_and_recuperate`) |

---

## 2. Objective-driven encounters ✅

An encounter is defined by an **objective**, not just "wipe the enemy"
([`sim::ObjectiveKind`], carried on `Encounter`, default `Eliminate`):

| Objective | Means | Posture | Status |
|---|---|---|---|
| **Eliminate** (`WinFight`) | wipe the enemy | both | ✅ |
| **Survive(N)** | last N rounds (satisfied unless wiped) | defend | ✅ |
| **Reach(hex)** | get a unit onto a hex | attack | ✅ |
| **Hold(hex, by)** | take + keep a hex (uncontested) by a round / by clearing | both (capture) | ✅ |
| **TimeAttack / MarginLoss** | win by round N / take-the-dive | — | ✅ (sim, unwired) |
| **Extract** | reach a hex **and leave the board** | attack | 🔭 |
| **Escort / Protect** | a named VIP survives | both | 🔭 |

**The mission gate ◆.** An encounter is **passed only if the army survives *and*
the objective stays *satisfied*** (not Failed). So **"win the fight" and "complete
the mission" can diverge** — wipe the enemy but fail to hold the node and the run
is *still lost*. ([`ObjectiveStatus::is_satisfied`] = not-Failed: a `Pending`
objective is still satisfied, so a defend-Survive you end alive counts.)

---

## 3. Win/loss flow ◆

| Tier | Advances when | Ends when |
|---|---|---|
| **Run** | an encounter is passed (army survives + objective satisfied) | **Lost** on a wipe *or* a failed objective · **Won** when all encounters clear |
| **Game** | a run is Won → **R&R** → next run | **Lost** if any run is Lost · **Won** when all runs clear |

The `Game` **defers to the run's outcome** — so a mission failure that leaves
survivors still ends the campaign (it's not just "is the roster empty").

---

## 4. Attack / Defend posture 🔭

A run carries a **posture** that selects the objective framing — it needs **no
engine change**, just *which objective Team A carries*:

| Posture | Objectives |
|---|---|
| **Attack** | Capture (Hold) · Extract · Assassinate · Breach-through |
| **Defend** | Hold · Survive-N · Repel |

The *same* battle read from two sides (attacker must take the node; defender must
keep it). Because the game is an **auto-battler**, **defense auto-resolves** from
the scripted roster — your agency is loadout + positioning + the objective plan,
not live control. *(Planned: a `Posture` tag on `RunPlan`.)*

---

## 5. Async PvE — the meta tier 🔭

"**PvE fights between player runs**": your **attack run** is paired against a
**stored snapshot of another player's defense**, AI-resolved (PvE for you, async
for them). Two sources, **one code path** — the run just consumes an enemy roster:

- **Player snapshots** when available (true async).
- **Seeded "ghost" rosters** as the fallback / PvE content.

The delta's **hidden-points** (§13) normalise matchups so a veteran-heavy roster
and a fresh one meet fairly. *(All `atomica-run` + a snapshot store; the `sim` is
untouched.)*

---

## 6. R&R / between-runs economy 🔭

R&R is **full heal + chrome repair** today (`rest_and_recuperate`). The between-
runs tier is where the rest of the meta-economy lands: **recruit · re-spec
loadouts · shop · salvage / insurance** (delta §9.4) · **set your defense**. The
delta's §9 **Jobs** are objective contracts — they map directly onto
`ObjectiveKind` encounters with rewards.

---

## 7. Code map

| Layer | Owns | Where |
|---|---|---|
| `sim::Battle` | one fight (units, ticks, dice) + the `Objective` seam | `crates/sim` |
| `Encounter` | enemies + an `ObjectiveKind` | `crates/run` |
| `RunPlan` / `Run` | a named series of encounters; attrition; pass-gating | `crates/run` |
| `GamePlan` / `Game` | a named series of runs; R&R; campaign outcome | `crates/run` |

**Built ✅:** the three tiers + named plans/reports, the spine rules, objective-
driven encounters (Eliminate/Survive/Reach/Hold), the mission gate, R&R between
runs, determinism. **Planned 🔭:** Attack/Defend posture, Extract/Escort
objectives, the async snapshot tier + matchmaking, the between-runs economy, the
navigation **tree** (branching routes, vs today's linear list).

---

## 8. Relation to the delta

This **simplifies delta §15**: there are **no shop/rest nodes inside a run** — a
run is purely the combat sequence, and all recovery/economy is **between runs**.
The delta's **§9 Jobs / objective types** and **§13 async-PvP normalisation**
carry forward unchanged (Jobs → objective encounters; hidden points → matchmaking).
The navigation **tree** (§15) becomes a *between-runs* map of which run to take
next, not a within-run node graph.

---

## 9. Next — the fight (the high-complexity core)

Everything above is the *scaffolding around* combat. The combat resolution itself
— movement & targeting profiles, the woven physical+digital initiative, AoE
footprints + friendly fire, the full damage pipeline — is the **deepest** system
and the next coding focus. Its current state vs the full design and the build plan
live in **[`combat.md`](combat.md)**.
