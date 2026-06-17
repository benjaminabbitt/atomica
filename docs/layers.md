# Equipment layers — the decorator architecture

*How a unit's effective profile is composed from a **base** (chassis) plus stacked
**equipment layers**, each implementing the **same interface** and **decorating**
the one below. Generalises the hand-rolled cyberware fold
([`cyberware.md`](cyberware.md) Phases A–F) into one uniform model that also carries
weapons, armor, and behavior-corruption. ◆ = decision (overridable, per repo
convention). Status: 🔭 **planned** — this is the architecture to refactor toward;
the current code keeps the fold model until it lands.*

---

## 0. The principle ◆

Equipment is **not special-cased**. A unit is a **base** stat line wrapped by an
ordered stack of **layers** (implants · weapons · armor · …), each implementing the
**same `Layer` interface** as the base and **decorating** it. The effective unit is
the result of querying the top of the stack.

**The character composes — no separate orchestrator ◆.** The unit/character owns
the **base** + a **`Vec` of layers** and `recompute()`s its effective stats by
**folding** them (sum the flat adds, sum the %-increments). Because the default
composition is **additive**, this is *just a fold* — not a heavyweight orchestrator
object. Cached + recomputed on loadout / condition change (the dirty-flag pattern,
see Precedents). The combination rules it folds by are below.

This is the architectural expression of two design throughlines:
- *"Chrome composes onto the body"* — every piece of kit is a layer over what's
  beneath, summing/overriding cleanly.
- *"You program your units; the enemy hacks your script"* — **corruption is just
  another layer**: a spoof / Lockware wraps your behavior interface and overrides
  it. No separate machinery.

---

## 1. The interface — `Layer`

The **derived** surface every layer (and the base) exposes — the read view the
`sim` queries:

| Group | Methods |
|---|---|
| **stats** | `link` · `firewall` · `immunity` · `initiative` · `max_integrity` · `armor_class` |
| **weapon** | `attack` (damage / type / pen / range / emp) |
| **behavior** | `movement` · `targeting` (the §7J profiles) |
| **capability** | `hack` (the netrunning loadout) |

- **Base** = the chassis innate values implements `Layer`.
- A **layer override** is `inner.x()` **augmented** (`link() = inner.link() + 5`) or
  **replaced** (`targeting() = Smart` over whatever's below; `hack() = Some(deck)`).

---

## 2. Composition

```text
base ─◄ implant ─◄ implant ─◄ weapon ─◄ armor ─◄ [corruption]
                                                   (top wins)
```

The effective query **folds/walks the stack from the base up**; **order matters**
for overrides — a spoof layer on top wins `targeting` over everything beneath.

### Combining numbers — the bucket model ◆ (additive vs. multiplicative)

*How layers' numbers combine is a **balance lever**, not an implementation detail.
Shipped ARPGs converged independently on a **bucket** model (Path of Exile's
`Added / Increased / More`; Diablo 4's additive-vs-`x%` buckets) — adopt it.* A
layer's contribution to a numeric stat declares its **kind**:

| Kind | Combine | Runs away? | Use |
|---|---|---|---|
| **Add** (flat) | summed: `Σadd` | no | the **default** — most gear (+5 Link, +6 Plating) |
| **Increased** (additive %) | **sum the % increments**, apply once: `1 + Σincreased` | **no** — diminishing relative returns | build-shaping % (a stim's +20%) |
| **More** (multiplicative %) | producted: `Π(1 + moreᵢ)` | **yes — the only runaway** | **off by default**; rare + capped if ever used |

> **Key point ◆:** in the `Increased` bucket the "multiplicative-looking" factors
> are **added together** (+20% and +30% → +50%, applied once) — so it's additive
> under the hood and **can't run away**. Only **`More`** genuinely multiplies
> separate factors, and **only `More` compounds into runaway**.

Plus **Override** (non-numeric): for behavior/capability (`targeting`, `hack`,
`movement`) the **top layer wins** (a spoof *replaces* targeting; not a number).

**Composition formula** (PoE / D4 order, per stat) — but with `More` empty by
default it collapses to a plain additive fold:

```text
effective = (base + Σadd) × (1 + Σincreased) × Π(1 + moreᵢ)
          = (base + Σadd) × (1 + Σincreased)            // default: no More → just a fold
```

**Our calls ◆:**
- **Additive by default → it's a fold, runaway-proof, no orchestrator.** Today's
  implant contributions are all flat `Add`, and `Increased` only *sums*, so the
  whole composition is a `Vec`-fold on the character.
- **`More` is off by default.** Genuine multiplication is the only runaway risk;
  don't ship a free-stacking `More` bucket. If a *signature* mechanic ever needs it
  (a marquee implant, the PAN-mesh synergy), it's **rare + hard-capped**, and still
  just one product step in the same fold.
- **Condition scales the *layer* before the fold.** A Degraded layer
  ([`cyberware.md`](cyberware.md) §6) delivers **half its contribution** (its
  `benefit_factor`), applied to that layer's adds/%s before they're summed.
- **Per-stat buckets.** Link, Firewall, damage, … each fold independently.

---

## 3. Derived vs. live state ◆

| Decorated (derived — recomputed from base + active layers) | Live battle-state (on the unit, **not** decorated) |
|---|---|
| the whole §1 surface (stats / weapon / behavior / capability) | `pos`, **current** Integrity, the depletable **Barrier / Plating pools**, `statuses`, `alive`, and each layer's **condition** (Online/Degraded/Offline — *gates* its contribution) |

The split is the crux: layers compute the **effective maxima / profile**; the unit
instance holds the **mutable fight state** that ticks down during a battle.

---

## 4. Realization ◆ — a layer `Vec`, folded

**Decision:** keep the exact `Layer` trait and decorator semantics, but store
equipment as an **ordered `Vec` of layers** and compute the effective surface by
**folding the trait over the base** — cached, recomputed on **loadout / condition
change** (not every read, for the deterministic hot loop).

- ✔ Same uniform interface + composition, **and** layers stay **indexable** (so a
  breach/EMP can flip one layer's condition without walking a boxed chain), with
  **no per-method delegation boilerplate**.
- **Alternative — literal `Box<dyn Layer>` chain:** the purest decorator, but every
  layer hand-delegates ~10 trait methods to `inner`, and mutating one mid-chain is
  awkward. Available if the purity is wanted; the `Vec` fold is the recommendation.

Either way the **public shape is identical**: `unit.link()` / `unit.targeting()`
read the composed value; nothing outside cares how it's stored.

---

## 5. Subsumes the cyberware fold

The current implant model ([`cyberware.md`](cyberware.md) A–F) becomes the **first
`Layer` kind**, with its meaning intact:

| Cyberware concept | In the layer model |
|---|---|
| `Contribution` fold / `refold` | a layer's stat overrides; effective = recompute over active layers |
| Condition (Online/Degraded/Offline) | the layer's gate + scale (Degraded = half), unchanged |
| benefit ↔ liability, hack-effects | the layer carries them; breach disables the layer → recompute |
| EMP / PAN / Cascade | operate on the layer set (disable all / cascade), unchanged semantics |

Then it **extends**: **weapons** and **armor** become further layer kinds
(multi-weapon, layered armor), and **behavior-corruption** (spoof / Lockware) is a
layer that overrides `movement` / `targeting`.

---

## 6. Layered defense — optional reach ◆

The same stack can model the design's **layered defense**: an incoming hit routes
**outermost → inner** (Barrier layer → Plating layer → Integrity base), each layer
**absorbing** the remainder by its pen-tier rules. That folds today's `apply_damage`
pipeline into the layer chain — *adopt later*; the current pool-based `apply_damage`
already works, so this is a clean-up, not a blocker.

---

## 7. Migration plan ◆

| Step | Does | Touches |
|---|---|---|
| **L1** | define the `Layer` trait + a `Base` (chassis stats); the unit's flat derived fields become a cached **`Derived`** recomputed from base + layers | `sim` stat reads |
| **L2** | port **implants → layers** (Contribution/condition → layer overrides); keep breach / EMP / PAN / Cascade behavior | the implant model + ~10 tests re-expressed on the layer API |
| **L3** | the **behavior layer** (movement / targeting join the interface) → finishes combat **Phase 1** on the layered model; a smartgun decorates `targeting` | combat Phase 1 |
| **L4+** | **weapon** layers, **armor** layers, **corruption** layers (spoof/Lockware) | new content |

**Test impact:** the implant tests (install / disable / degrade / EMP / cascade)
re-express on the layer API — the breach/condition **semantics are unchanged**, so
the assertions port. Contest / objective / run tests are untouched: they read
*effective* stats, which still exist (just composed differently).

---

## 8. Why now ◆

Do **L1–L3 before finishing combat Phase 1** (behavior profiles): the movement /
targeting profiles *belong in the `Layer` interface*, so wiring them into flat
fields first would be throwaway. The Phase-1 enums are already committed and slot
straight into the interface. After L3, the "smartgun decorates targeting" and
"spoof corrupts behavior" stories fall out for free.

---

## 9. Status

🔭 **planned.** The codebase keeps the cyberware **fold** model until L1/L2 land;
this doc is the **target**. Open to flip §4 to a literal `Box` chain if preferred.

---

## Precedents (the bucket model)

- **Path of Exile** — the canonical buckets: `Added` (flat) · `Increased/Reduced`
  (sum additively, one multiplier, diminishing) · `More/Less` (each its own
  multiplier, compounding, mostly from rare sources). [PoE Wiki — Stat](https://pathofexile.fandom.com/wiki/Stat)
- **Diablo 4** — additive (`Value%`, one bucket, sums) vs. multiplicative (`x%`,
  separate, multiplies); "invest across buckets beats stacking one." [Mobalytics — Damage Buckets](https://mobalytics.gg/diablo-4/guides/damage-buckets-deep-dive)
- **Implementation pattern** — `Statistic { base, current, modifiers[], altered }`;
  modifiers are `{ value, op: Add|Multiply }`; recompute only when `altered`
  (the cached dirty-flag the character's `recompute()` uses). [RefresherTowel — Modifiable Stats](https://refreshertowelgames.wordpress.com/2024/02/17/how-to-comfortably-deal-with-modifiable-stats/)
- **Design wisdom** — additive = legible, self-limiting (diminishing returns),
  easy to balance; multiplicative = powerful, compounding, must be rare. The
  bucket separation is the balance lever. [Paradox forums discussion](https://forum.paradoxplaza.com/forum/threads/additive-bonuses-vs-multiplicative-bonuses.1144836/)
