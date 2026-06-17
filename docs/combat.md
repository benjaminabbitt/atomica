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
- **The tick loop** (`Battle::step`): **status** (DoTs / shred, may kill) → **woven**
  (one interleaved physical + digital order, §10.3) → **decay**.
- **Action**: units act in **effective-initiative order** (desc, id tiebreak); each
  non-stunned unit picks a target by its **targeting profile**, **moves** up to its
  `speed` through free hexes by its **movement profile**, **then attacks** if in range
  (§7J/§10.4, L3 + Phase 2) — programmable + spoofable, occupancy-aware.
- **Damage pipeline**: per-target over the attack's **footprint** (`Single` /
  `Blast` / `Beam`, **friendly fire on**), penetration tiers (External → Barrier,
  Contact → Plating, Internal → straight to Integrity), the **armor matrix** (type ×
  class), **Breach** vulnerability, the **softener** floor; layered Barrier → Plating
  → Integrity.
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
| **Movement profiles** (§7J) | Advance · Hold · Kite · Flank · Swarm · Disperse | **all six**, read each activation | ✅ |
| **Targeting profiles** (§7J) | Nearest · Lowest-Integrity · Highest-threat · Backline · Weakest-armor | **all five**, read each activation | ✅ |
| **Move stat + move-then-act** (§10.4) | move up to `move` hexes, then act | **`speed` hexes, then act** | ✅ |
| **Occupancy / pathing / boxed-in** (§10.5a) | occupied hexes block; no free hex ⇒ no move | **free-hex stepping + boxed-in** (greedy, no A*) | ✅ |
| **Woven initiative** (§7C/§10.3) | one interleaved physical+digital order | **one woven order** (Initiative + Link on one track) | ✅ |
| **AoE footprints + friendly fire** (§7G) | blast (radius) · beam (line/width); physical hits allies | **`Blast`/`Beam` wired, friendly fire on** | ✅ (width = 1) |
| **Range bands / reach** (§10.5) | gun bands · polearm reach | **`min_range..=range` band** (`usable_at`) | ✅ |
| **Multiple weapons / selection** | per-target weapon choice | **`weapons` + `weapon_at` (best in band)** | ✅ |
| **Smartgun / IFF targeting** (§7F) | smart profiles, fires on Link | — | 🔭 |
| **Death triggers** (§10.9) | Detonate · Legacy · Data-spill | none | 🔭 |
| **Board seam / two boards** (§7B) | ±½-hex seam, frontage pairings | single shared grid | 🔭 |
| **Heat** (§7D/§10.10) | thermal layer | — | 🔭 |
| **Morale / Resolve** (delta §4) | Resolve pool, Break (rout/berserk) | — | 🔭 |
| **Vehicles / multi-hex** (delta §5) | 2–3-hex occupancy, ram, crew | — | 🔭 |

---

## 3. Build order ◆ — the fight, phased

Sequenced so each phase is shippable and test-first, hardest-leverage first:

1. **Behavior profiles ✅** — **movement** + **targeting** profiles drive the action
   phase, read each activation (`Battle::select_target` / `movement_step`). Units are
   *programmable* (and thus hackable, §7J): the profiles **compose from the
   `Character`** ([`layers.md`](layers.md) L3) — `Unit::with_targeting` installs a
   `GEAR`-priority `Override`, and `Unit::spoof` a `CORRUPTION` one that wins, so "the
   enemy hacks your script" falls out. Default Nearest/Advance preserves the old
   baseline. *(Still Phase 1's single-hex step; the `move` stat is Phase 2.)*
2. **Movement model ✅** — a **`speed`** stat (move up to N hexes/activation),
   **move-then-act** (the unit closes by its movement profile, *then* attacks if in
   range, same activation), and **occupancy** (`occupied_by_other` blocks a hex;
   greedy free-hex stepping; **boxed in** ⇒ no move). Units no longer overlap;
   positioning is real. *(Greedy single-hex pathing — full A* around obstacles is a
   later refinement.)*
3. **AoE footprints + friendly fire ✅** — `Attack.footprint`: `Single` ·
   `Blast(radius)` (disc via `within`, centred on the target hex) · `Beam(length)`
   (line via `line`, along `direction_to` the target). `resolve_attack` runs the
   damage pipeline over `footprint_targets` — **every living unit in the area, allies
   included** (only the attacker is spared). *(Beam width is 1; multi-width is a later
   refinement.)*
4. **Woven initiative ✅** — `Battle::woven_order` builds **one descending track**:
   each living unit a **physical** activation keyed by effective Initiative and, if it
   can hack (deck + Link > 0), a **digital** one keyed by Link; ties = lower `id`, then
   physical before digital. `step` runs the single `woven_phase`; the old `action_phase`
   / `digital_phase` are now test-only. A high-Link runner hacks before a sluggish
   bruiser swings.
5. **Weapons & reach ◑** — **range bands** (`Attack.min_range..=range`, `usable_at`)
   and **multi-weapon selection** (`Unit.weapons` + `weapon_at` picks the
   highest-damage weapon whose band covers the distance) are built; a closing profile
   **stands off** once any weapon reaches. Polearm reach = a `2..=2` band. *Deferred:
   the **Smartgun/IFF** smart-targeting mod (fires on Link / IFF — ties to AR).*
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
