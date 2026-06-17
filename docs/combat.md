# Combat — the fight resolution (current vs designed)

*The `sim`'s **battle** resolution: what one fight actually does, tick by tick.
Today it runs a deliberately **minimal** loop; the full design (taxonomy
§7B board · §7C initiative · §7G footprints · §7I weapons · §7J movement/targeting
· **§10 full rules**) is much richer. This is the **deepest** system and the next
coding focus — the auto-battler's soul ("you program your units; the enemy hacks
your script"). ◆ = decision. Status: **✅ built** · **◑ partial** · **🔭 planned**.*

---

## 0. What's built ✅

In [`crates/sim`](../crates/sim/src/lib.rs) today:

- **Hex board + math** ([`hex`](../crates/sim/src/hex.rs)) — axial flat-top:
  distance, neighbours, `within` (blast disc), `ring`, `line` (beam), `step_toward`.
- **The tick loop** (`Battle::step`): **status** (DoTs / shred, may kill) →
  **action** (physical) → **digital** (hacks) → **decay**.
- **Action**: units act in **effective-initiative order** (desc, id tiebreak);
  each non-stunned unit attacks the **nearest** enemy in range, else **steps one
  hex** toward it.
- **Damage pipeline**: penetration tiers (External → Barrier, Contact → Plating,
  Internal → straight to Integrity), the **armor matrix** (type × class), **Breach**
  vulnerability, the **softener** floor; layered Barrier → Plating → Integrity.
- **Digital pass**: Link-ordered hacks (see [`netrunning.md`](netrunning.md)).
- **Statuses** (the 9-axis pool) and **objectives** ([`progression.md`](progression.md)).

That's enough to *resolve* a fight deterministically — but it's "attack nearest /
walk forward," not the designed tactical combat.

---

## 1. The target — the full activation (§10.3–10.5)

The designed round:

1. **Build one woven order** — every unit contributes a **physical** activation
   (ranked by physical Initiative) *and* a **digital** one (ranked by Link),
   **interleaved into a single order** (§7C/§10.3).
2. **Each activation:**
   - **start-of-activation statuses** — DoTs / contagion ticks fire (can kill
     before it acts);
   - **move** — up to the unit's **move** stat toward its **movement profile**'s
     goal, pathing only through **free** hexes (boxed-in ⇒ no move);
   - **act** — pick a target via the **targeting profile**, resolve the attack over
     its **footprint** (single / blast / beam, **friendly fire on** for physical),
     then `penetration → defense → magnitude → apply → on-hit statuses → death`.
3. **Cleanup** — decay / duration ticks; elimination check; **death triggers**.

---

## 2. Gap analysis — built vs the fight

| System | Designed | Now | Status |
|---|---|---|---|
| **Movement profiles** (§7J) | Advance · Hold · Kite · Flank · Swarm · Disperse | only "advance to nearest" | 🔭 |
| **Targeting profiles** (§7J) | Nearest · Lowest-Integrity · Highest-threat · Backline · Weakest-armor | only Nearest | 🔭 |
| **Move stat + move-then-act** (§10.4) | move up to `move` hexes, then act | 1 hex/tick *or* attack | 🔭 |
| **Occupancy / pathing / boxed-in** (§10.5a) | occupied hexes block; no free hex ⇒ no move | units can overlap | 🔭 |
| **Woven initiative** (§7C/§10.3) | one interleaved physical+digital order | two discrete phases | 🔭 |
| **AoE footprints + friendly fire** (§7G) | blast (radius) · beam (line/width); physical hits allies | single-target | 🔭 (hex math ✅) |
| **Range bands / reach** (§10.5) | gun bands · polearm reach | one `range` value | ◑ |
| **Multiple weapons / selection** | per-target weapon choice | one attack profile | 🔭 |
| **Smartgun / IFF targeting** (§7F) | smart profiles, fires on Link | — | 🔭 |
| **Death triggers** (§10.9) | Detonate · Legacy · Data-spill | none | 🔭 |
| **Board seam / two boards** (§7B) | ±½-hex seam, frontage pairings | single shared grid | 🔭 |
| **Heat** (§7D/§10.10) | thermal layer | — | 🔭 |
| **Morale / Resolve** (delta §4) | Resolve pool, Break (rout/berserk) | — | 🔭 |
| **Vehicles / multi-hex** (delta §5) | 2–3-hex occupancy, ram, crew | — | 🔭 |

---

## 3. Build order ◆ — the fight, phased

Sequenced so each phase is shippable and test-first, hardest-leverage first:

1. **Behavior profiles** — **movement** + **targeting** profiles on the unit, read
   each activation. *The keystone:* it makes units *programmable* (and thus
   hackable, §7J), and turns "walk to nearest" into real tactics. Pure logic,
   deterministic, no new spatial rules.
2. **Movement model** — a **move** stat (move up to N/turn), **move-then-act** per
   activation, and **occupancy / pathing** (free-hex pathing, boxed-in). Units stop
   overlapping; positioning becomes real.
3. **AoE footprints + friendly fire** — wire **blast** (`within`) and **beam**
   (`line`) into attacks; **physical AoE hits allies**. The hex math is already
   there — this is attack-resolution plumbing + a footprint on the weapon.
4. **Woven initiative** — collapse the two phases into **one interleaved order**
   (physical Initiative + Link), the §10.3 model. Affects timing/tie-breaks.
5. **Weapons & reach** — range bands, polearm reach, multi-weapon selection, the
   **Smartgun/IFF** smart-targeting mod (ties to AR).
6. **Death triggers** — Detonate / Legacy / Data-spill on removal (§10.9); feeds
   contagion **Data-spill** later.
7. **Board geometry** — the two-board **seam** (±½-hex) and frontage pairings (§7B).

**Cross-cutting layers** (their own systems, slot in later): **Heat** (§7D),
**Morale/Resolve** (delta §4), **Vehicles** (delta §5, the multi-hex one — the
biggest engine change).

---

## 4. Why it's the highest complexity

- **Behavior is emergent** — profiles × positioning × AoE × friendly fire produce
  the interactions that *are* the game; small rule changes ripple.
- **Spatial coupling** — pathing, occupancy, footprints, reach, and board geometry
  all interact, and must stay **deterministic** (every tie-break seeded/ordered).
- **It's the substrate** — objectives ([`progression.md`](progression.md)),
  contagion spread, AR targeting, and the digital realm ([`netrunning.md`](netrunning.md))
  all read combat state, so its shape constrains everything above it.

Begin at **Phase 1 (behavior profiles)** — highest design leverage, lowest spatial
risk, and the thing that makes a unit a *program*.
