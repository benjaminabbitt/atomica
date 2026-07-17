# Mapping — the battlefield position graph

*The battlefield's spatial model: an authored **graph of positions** (nodes + edges), **replacing
the hex board** (`combat.md` §0's `hex.rs`/`board.rs`/`terrain.rs` spatial role). Units occupy
**nodes** and move along **edges**; there is **no free grid and no arbitrary pathfinding**. Ancestor:
Darkest Dungeon's rank system, generalized to a **2.5D graph** with elevation, per-mode movement, and
dynamic topology. ◆ = decision. Status: **🔭 planned** (this is a rework of the built ✅ hex layer) ·
**⏳ tuning**. Numbers are TBD.*

> **Not the run-map.** Two graph layers exist and are distinct: the **run-map** (Slay-the-Spire
> navigation *between* battles, `design-delta` §15) and this **battle-position graph** (*within* one
> battle). Same data shape, different scope.

---

## 1. Two graphs on the same nodes ◆

A position graph is really **two overlaid relations** — keep them separate:

| | **Movement graph** | **Engagement graph** |
|---|---|---|
| An edge means | you can *travel* A→B | you can *reach* A→B (shoot / see / melee) |
| Weight | move **cost** (per mode) + optional **skill-gate** | **range** (a distance number) |
| Modified by | elevation (climbs), movement mode (foot / vehicle) | elevation, cover, concealment (LOS) |
| Directional? | yes (asymmetric up/down; one-way drops) | usually symmetric |

They differ on purpose: you can **shoot across a gap you can't cross**, or hold **high ground that
sees everywhere but is a hard climb**. Model only one and you fight it forever.

**Engagement is *weighted* ◆** — it carries an inter-node **range** number, not just adjacency. One
range scale drives **everything spatial**: melee reach, weapon range, **minimum range**, and **AoE
size** (§4). "Adjacency" is not a separate concept — it's just *range ≤ the relevant threshold*.

---

## 2. Nodes ◆

A node is a **position**, and — like a [`Character`](layers.md) — a **base profile + a live modifier
stack** (§6), realized on demand. It carries:

- **Elevation** (`z`) — feeds engagement LOS/range (see over cover, shoot down) and movement (climbs).
- **Capacity** ◆ — **per-node**: how many units it holds (a chokepoint `1`, a courtyard `N`). Authored.
- **Cover / Concealment / Visibility** — three *distinct* tactical axes (§6).
- **Hazard** — per-tick damage to its occupants (fire, radiation).
- **Tags** — cover / chokepoint / **objective** / **exit** (extraction) / spawn.
- **World position** — `(x, y, z)` for the **camera / renderer** (§8). The sim ignores it; the data
  carries it so the presentation layer can place units and frame the action.

Everything but elevation and world-position can be **added or stripped at runtime** (§6).

---

## 3. Edges ◆

A movement edge A→B carries:

- **Per-mode cost** — a cost *per movement mode* (`foot`, `vehicle`, later `flight`): a road is cheap
  for wheels, a ladder is **foot-only**, rubble is slow for both. `∞` / absent ⇒ that mode can't take
  it. **This is the vehicle integration** — vehicle movement is just a different cost column (`§5`).
- **Skill-gate** (optional) — a hard traversal (climb / leap / ford) needs a **check** first: an
  attribute + skill roll (Dexterity to vault, Body/Grit to haul up). **Fail** ⇒ don't move / take a
  fall to a lower node / take damage. Wires movement to the character sheet (skills, `stats.md`).
- **Direction** — asymmetric or one-way (a drop you can't climb back).

The engagement graph's edges are separate (a range + an authored LOS relation, §4).

---

## 4. Combat on the graph ◆

Everything spatial reads the **engagement range** between the actor's node and the target's:

- **Melee** — same node (range 0) *or* a node within the weapon's (short) range. No special adjacency.
- **Ranged** — target any node within `[min_range, range]` **and** with a live engagement edge (LOS).
  **Minimum range** falls straight out — a mortar / launcher **can't hit too-close nodes**; it lobs
  *over* them.
- **AoE = a radius on the engagement graph ◆** — an area attack has a **size** (in range units) and
  hits the **target node + every node within that range**. This dissolves the hex-footprint problem:
  - **Close nodes share AoE fate** — cluster in a tight room (short inter-node ranges) and one grenade
    catches the lot; spread across distant nodes and you're safe. **Node spacing is the author's
    AoE-tuning knob.**
  - **Large explosions** are just a big radius; **no grid to reason about**. Fidelity is **by node**
    ◆ (a node is in or out — no sub-node precision needed).
  - **Beams** stay the one awkward case — "nodes along a direction / path"; rarer than blasts, spec later.
- **LOS is authored, not computed ◆** — the map author **draws the engagement edges** (who can reach
  whom, at what range); dynamic node modifiers (smoke, a dropped wall, elevation) **toggle / re-weight**
  them. No raycasting — fits the "authored maps, no arbitrary computation" ethos, and makes "smoke cuts
  this sightline" a one-line node effect.

Cover / concealment then modify the resolved attack (§6).

---

## 5. Dynamic topology ◆ (the graph changes mid-battle)

Nodes **and** edges can be **added or removed during combat** — the graph is live, not static:

- **Remove** — a bridge collapses (drop an edge / node), a building falls. A removed node's
  **occupants spill** to a connected node; **no connected node ⇒ they're Downed** (§9.4) — the same
  "no free hex ⇒ downed" rule as vehicle crew-spill (`design-delta` §5).
- **Add** — a **breach** blows a wall (new edge between two rooms), a deployable creates a position, a
  vehicle rams a new path.
- **Stability** — NodeIds are **never reused** (removed ones tombstone) so a unit's `pos: NodeId` and
  authored references stay sound.

This makes destructible terrain, breaching, and collapsing structures **first-class**, not scripted
one-offs.

---

## 6. Node state & modifiers — reuse the decorator engine ◆

Node properties are **composed exactly like unit stats** (`layers.md` L1–L6): a base + a stack of
decorators, realized on demand. So the *same* engine that composes a character composes a node — **do
not build a second modifier system.** Sources:

- **Deployables** — a **smoke grenade** stacks a timed node decorator (`+concealment`, `−visibility`),
  decaying like any status; mines / hazard-drops add `hazard`; a flare adds `visibility`.
- **Units** — a **vehicle** projects a **cover** decorator onto its node (and maybe neighbours) while
  parked — **mobile, dynamic cover**.
- **Damage** — an attack **strips a node's cover** decorator (blow up the wall) — destructible cover.

**The three tactical axes (pin the taxonomy) ◆** — conflated constantly, but they do *different* things:

| Axis | Effect on the occupant | Example |
|---|---|---|
| **Cover** | reduces incoming ranged **damage / to-hit** — *stops bullets* | wall, parked vehicle |
| **Concealment** | reduces **targetability** — harder to acquire; *doesn't* stop bullets | smoke, foliage |
| **Visibility** | how far / well *it* sees & is seen **out** (LOS range, exposure) | high ground high; smoke low |

**Smoke** is the sharp case: **+concealment, −visibility (both ways), zero cover** — hidden but not
protected, and half-blind. A **wall** is cover *and* concealment *and* an LOS blocker. Three separate
values, not one "cover" number, is what makes smoke, high ground, and a vehicle-shield each behave right.

*(⏳ the numeric hooks — cover as damage-% vs to-hit-penalty; concealment as a targetability penalty vs
a hard range cap — are TBD.)*

---

## 7. Reinterpreting the built systems

Everything spatial in the current sim re-expresses on the graph:

| Built on hexes (`combat.md`) | On the position graph |
|---|---|
| `pos: Hex`, `distance`, `neighbors` | `pos: NodeId`; **engagement range**; movement edges |
| **Movement profiles** (Advance / Kite / Flank / Hold / Swarm / Disperse) | a light **Dijkstra over ~10–30 nodes** — Advance = least-cost path toward the target node; Kite = a node keeping you in *your* range but out of theirs; Flank = a node with a cover / elevation edge on the target. *(Not "arbitrary pathfinding" — trivial on a tiny graph.)* |
| **Footprints** (blast / beam) | AoE = engagement-range radius (§4) |
| **Terrain** (zones / cover / hazards / blockers) | **node attributes** + the modifier stack (§6) |
| **Objectives** (Hold / Capture / Reach / **Extract**) | **node-native** — hold node X, reach the **exit** node; **extraction** = a vehicle reaches an exit node along vehicle-edges (§5 / `design-delta` §9.4). This graph is the **enabler for vehicles + extraction**. |
| **Board seam / frontage** (`board.rs`) | the front line *is* which nodes hold engagement edges across the middle |

---

## 8. Camera & visualization ◆

Because every node has an **authored world position** and every edge a **drawable path**, the graph is
a natural fit for a **stylized, cinematic presentation**: the camera frames and cuts **node-to-node**,
elevation reads at a glance, cover / concealment are **visible node states**, and a move animates
**along its edge**. The authored (not procedural) layout is the point — hand-crafted, readable, easy to
light. The sim stays presentation-blind (it reads the graph; the renderer reads the world positions),
so this is a clean seam, not a coupling.

---

## 9. Migration & factoring 🔭

This **replaces** the hex spatial layer ◆ — a large rework, phased (the built combat is otherwise
intact):

- **New:** `graph.rs` (nodes, edges, the two relations, Dijkstra, runtime add/remove); `NodeId`
  positions; the node modifier stack (reusing the `Character` composition engine); range-based
  targeting / AoE; per-mode movement.
- **Superseded:** `hex.rs`, `board.rs`, and `terrain.rs`'s spatial role (their *content* — cover /
  hazard / zones — becomes node attributes).
- **Unchanged:** the resolution core (rolls, damage pipeline, status pool, morale, Death's Door,
  netrunning) — they read *positions* and *ranges*, which the graph still supplies. Only the spatial
  substrate under them changes.
- **Engine-blind stays blind:** factions / economy never knew about hexes and won't know about nodes.

Phase order (rough): ① graph data model + NodeId positions + range → ② movement (edges, per-mode cost,
Dijkstra, skill-gates) → ③ targeting / AoE / min-range / authored LOS → ④ node modifier stack (cover /
concealment / visibility, smoke, vehicle-cover, destructible) → ⑤ dynamic topology (add / remove,
spill) → ⑥ camera metadata.

---

## 10. Open ⏳

- **Numeric hooks** for cover / concealment / visibility (§6) — the exact combat modifiers.
- **Beams** on a graph (§4) — the node-line delivery.
- **Vehicle footprint** — does a vehicle occupy one node or span several (a multi-node body, echoing
  the §5 multi-hex occupancy)?
- **Movement-mode set** — foot / vehicle / flight; do flyers ignore elevation gates?
- **Skill-gate consequences** (§3) — fall damage / drop-a-node / just-fail, per gate type.
- **Authoring format** — how maps declare nodes, both edge sets, per-node attributes, and world
  positions (content in `atomica-content` / `atomica-run`).
