# CHROME AND CODE — documentation map

Async auto-battler / roguelike-deckbuilder. Cyberpunk, neo-Japan cyber-samurai.

## Read in this order
1. **[../CHROME-AND-CODE.md](../CHROME-AND-CODE.md)** — the at-a-glance spec (the locked shapes).
2. **[../status-effects-taxonomy.md](../status-effects-taxonomy.md)** — the survey-grounded design bible (v0.25): the design axes, master effect tables, the dual Virus/Worm contagions, board geometry, and the full first-pass rules.
3. **[design-delta-v0.26.md](design-delta-v0.26.md)** — post-v0.25 synthesis **+ decisions**: three-axis identity, factions & the live-summation economy, the mender triad, morale, vehicles, PAN/AR, Jobs, skills, the **2d10 roll-under** core mechanic (see [stats.md](stats.md)), shops, the run tree, and the §13 decisions log.
4. **[rosters.md](rosters.md)** — authored content: named corps / clans / law + the archetype roster.
5. **[netrunning.md](netrunning.md)** — the digital-attack system: the hack contest (`2d10 ≤ avg(Hacking, channel) − Firewall`, channel = weaker endpoint's Link), the stat/skill line, payloads, the implant model, and the build order. Marks **built vs planned**.
6. **[cyberware.md](cyberware.md)** — the augmentation economy: the implant bundle (benefit ↔ liability symmetry), the value equation that makes chrome worth its risk, slots/PAN, breach & repair, and the sim data model. The keystone the netrunning payloads plug into.
7. **[progression.md](progression.md)** — the meta-structure above one battle: the **Encounter < Run < Game** tiers, permadeath, R&R placement (none within a run, full between runs), and **objective-driven** encounters (the mission gate). Supersedes the delta's §15 within-run model.
8. **[stats.md](stats.md)** — the **resolution spine**: the **2d10 roll-under** core (and why 2d10 over 3d6 — lower balance sensitivity), the GURPS ~10 attributes, skill tiers (untrained −4 … elite +4, untrained-by-default), the **opposed** combat roll + the **Speed** dodge penalty, and the **resist-as-modifier** static contests. Everything else cites this for the roll.
9. **[combat.md](combat.md)** — the fight resolution: the **full phased build order (1–7) is now in** — behavior profiles, the move-then-act movement model + occupancy, AoE + friendly fire, woven initiative, weapons & range bands, death triggers, the board seam. The gap analysis tracks ✅ / ◑ vs the §10 design.
10. **[layers.md](layers.md)** — the **character architecture**, **built (L1–L3)**: generators (`chargen`) decorate a `Character` by **adding/removing `Modifier`s** (each linked to its spawning decorator id, referenceable + removable — the counterplay substrate); the `Character` composes effective stats via the additive-default **bucket** model, with a live **decorator active face** (events → reactions). Subsumes the cyberware fold *and* the status pool; drives the loop's behavior layer.

## Status
- **Design:** decided down to playtest numbers; remaining open items are tuning (⏳) — see delta §12/§13.
- **Code:** `crates/sim` (one battle) + `crates/run` (the **Encounter < Run < Game** progression) + `crates/game` (macroquad/egui front-end), built test-first from the delta's IoC plan. In: the RNG seam + 2d10 roll-under core ([stats.md](stats.md)), the full **netrunning + cyberware** systems ([netrunning.md](netrunning.md) / [cyberware.md](cyberware.md)), the **progression spine** ([progression.md](progression.md)), the **character architecture** ([layers.md](layers.md), L1–L3), and **the fight itself** ([combat.md](combat.md)) — the whole phased build order: behavior profiles, movement/occupancy, AoE/friendly-fire, woven initiative, weapons & range bands, death triggers, the board seam. **Next:** the cross-cutting layers (Morale / Vehicles — *Heat dropped for now*), the deferred ◑ polish, and the stat read-through migration.

## Build
`make test` (sim tests) · `make web` (wasm build) · `make run` (native window). See the root [../README.md](../README.md).
