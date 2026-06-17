# CHROME AND CODE — documentation map

Async auto-battler / roguelike-deckbuilder. Cyberpunk, neo-Japan cyber-samurai.

## Read in this order
1. **[../CHROME-AND-CODE.md](../CHROME-AND-CODE.md)** — the at-a-glance spec (the locked shapes).
2. **[../status-effects-taxonomy.md](../status-effects-taxonomy.md)** — the survey-grounded design bible (v0.25): the design axes, master effect tables, the dual Virus/Worm contagions, board geometry, and the full first-pass rules.
3. **[design-delta-v0.26.md](design-delta-v0.26.md)** — post-v0.25 synthesis **+ decisions**: three-axis identity, factions & the live-summation economy, the mender triad, morale, vehicles, PAN/AR, Jobs, skills, the **3d6 + skill + equipment** roll mechanic, shops, the run tree, and the §13 decisions log.
4. **[rosters.md](rosters.md)** — authored content: named corps / clans / law + the archetype roster.
5. **[netrunning.md](netrunning.md)** — the digital-attack system: the hack contest (`3d6 + avg(Hacking, channel)` vs Firewall, channel = weaker endpoint's Link), the stat/skill line, payloads, the implant model, and the build order. Marks **built vs planned**.
6. **[cyberware.md](cyberware.md)** — the augmentation economy: the implant bundle (benefit ↔ liability symmetry), the value equation that makes chrome worth its risk, slots/PAN, breach & repair, and the sim data model. The keystone the netrunning payloads plug into.

## Status
- **Design:** decided down to playtest numbers; remaining open items are tuning (⏳) — see delta §12/§13.
- **Code:** `crates/sim` (one battle) + `crates/run` (the roguelike run layer — a persistent roster fighting a sequence of battles) + `crates/game` (macroquad/egui front-end), built test-first from the delta's IoC plan. In: the RNG seam + 3d6 contest, the full **netrunning + cyberware** systems (see [netrunning.md](netrunning.md) / [cyberware.md](cyberware.md)), and the **fighting run spine** (permadeath, win/loss). Economy/Rep, Jobs, shops, and the navigation tree are the next run-layer additions.

## Build
`make test` (sim tests) · `make web` (wasm build) · `make run` (native window). See the root [../README.md](../README.md).
