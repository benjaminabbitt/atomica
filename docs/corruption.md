# Corruption — system design & plan

*The **hostile-decorator** family: viruses, worms, and spoofs — the inverse of
buffs — and how a corruption **spreads** (contagion). Extends
[`netrunning.md`](netrunning.md) (worms as a breach vector) and the decorator
model in [`layers.md`](layers.md) (L6 content + the L7 contagion phase). ◆ =
decision/synthesis. Status legend: **✅ built** (lives in `crates/sim`) ·
**🔭 planned** · **⏳ tuning** (a feel number). Numbers are TBD.*

---

## 1. What a corruption is

A **corruption** is just a [`Decorator`] with a *hostile* payload, tagged by
**family** so counterplay can target it. What makes it corruption — versus a
debuff — is the tag (`Virus` / `Worm` / `Spoof`), not the effect: a generic
dispel can't tell a worm from a buff, but a **tag-ward cleanse** strips *only*
its family. The three families and their surfaces:

| Family | Tag | Corrupts | Defends / resists | Cleanse |
|---|---|---|---|---|
| **Virus** ✅ | `Virus` | the **biological** surface (Immunity) | Immunity | `antivirus` (`Remove::Tag(Virus)`) |
| **Worm** ✅ | `Worm` | the **digital** surface (Firewall) · trips chrome (§ breach) | Firewall, segment PAN | `firewall_patch` (`Remove::Tag(Worm)`) |
| **Spoof** ✅ | `Spoof` | **behavior** (a `CORRUPTION`-priority targeting override) | — | a counter-spoof ward |

Content lives in `corruption.rs` ([`Corruption`]): `virus` / `worm` (the
stat-rot debuffs — the **virus** also carries a **fever DoT**, Internal so it bypasses
armor: the combat bite, while the Immunity rot is the snowball), `plague` /
`worm_swarm` (their **contagious** variants), and the `antivirus` / `firewall_patch`
wards. Behavior corruption is
[`Unit::spoof`](../crates/sim/src/lib.rs) (already wired). A worm is also a
**breach vector** — see [`cyberware.md`](cyberware.md) §6 and
`Battle::worm_breach` (logic-bomb trips one implant; Cascade trips all on a
meshed PAN).

## 2. Contagion — contested spread ✅

A corruption marked **contagious** carries a [`Contagion`] `{ virulence, resist,
vector }`. Each round the **contagion phase** (`Battle::contagion_phase`, after
the action phase) tries to **jump** every active contagion to fresh victims:

1. **Reach** — candidates along the [`Vector`]:
   - `Proximity(n)` — *biological*: any unit within `n` hexes (a plague needs contact).
   - `Net` — *digital*: any unit with a live surface (`Link > 0`), **distance-independent**
     (a worm doesn't care where you stand).
2. **Contest** ◆ — a `3d6 + virulence` roll vs the victim's **resist** stat as TN
   (Immunity for a plague, Firewall for a worm). A hardened immune system / wall
   beats a weak strain; a virulent one takes hold. (`resolve_contest`, the same
   `3d6` mechanic as hacks.)
3. **Land** — on a win the **whole decorator copies itself** onto the victim
   (`install` re-stamps a fresh `GenId`): the contagion is **self-replicating**,
   and the new carrier spreads it onward next round.

**Guards** keep it controlled:

- **No re-infection** — a unit already carrying that corruption (by label), or one
  freshly caught this phase, is skipped (no stacking, no double-land).
- **One hop per round** — jumps land *after* the scan, so a strain advances at
  most one ring of victims per round (controlled exponential, not instant
  saturation).
- **Friend or foe** ◆ — a contagion doesn't read uniforms; it spreads to *any*
  unit in range. Your own planted plague will ravage your line if you cluster —
  positioning is the counterplay, alongside the cleanse wards and the resist stat.

## 3. Open threads 🔭

- **Mutation / decay of virulence** as a strain spreads (weakening or hardening
  per hop).
- **Cross-family interaction** — a worm that *opens* a unit to a virus, a spoof
  that rides a Data-spill.
- **Data-spill on death** as a contagion seed (the [`combat.md`](combat.md) §10.9
  death trigger feeding a strain into the survivors).

[`Decorator`]: ../crates/sim/src/chargen.rs
[`Contagion`]: ../crates/sim/src/chargen.rs
[`Vector`]: ../crates/sim/src/chargen.rs
[`Corruption`]: ../crates/sim/src/corruption.rs
