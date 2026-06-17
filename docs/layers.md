# Equipment layers — the character-generator / factor model

*How a `Character`'s effective profile is **composed** from a **base** (chassis)
plus **equipment layers** — where each layer is a **character generator** that emits
**factors** (modifiers), and the `Character` holds the factor list and **sums /
multiplies** it into effective stats. Generalises the hand-rolled cyberware fold
([`cyberware.md`](cyberware.md) Phases A–F) into one uniform model that also carries
weapons, armor, and behavior-corruption. ◆ = decision (overridable, per repo
convention). Status: 🔭 **planned** — the architecture to refactor toward; the
current code keeps the fold model until it lands.*

---

## 0. The principle ◆

Equipment is **not special-cased**, and the decorator **does not implement the unit
interface**. Instead, each piece of kit is a **character generator** ◆ — a decorator
that **returns a character** by contributing **factors** (modifiers). A pipeline of
generators (base → implant → weapon → armor → …) **builds the `Character`**; the
`Character` holds the resulting **list of factors** and **composes** it — summing
and multiplying — into its effective stats.

So the split is:
- **Generators** (the equipment layers) are **factor sources**: each emits its
  contributions (e.g. a cyberdeck → `Add(Link, 5)`, `Override(hack, deck)`).
- **The `Character`** owns the **factor `Vec`** and the **composition** (the
  sum/multiply rules below). The stat-query surface lives **on the `Character`**,
  computed from base + factors — *not* on each layer. There is **no separate
  orchestrator**; the `Character` `recompute()`s itself.

This is the architectural expression of two design throughlines:
- *"Chrome composes onto the body"* — every generator adds factors over what's
  beneath; the `Character` sums them cleanly.
- *"You program your units; the enemy hacks your script"* — **corruption is just
  another generator**: a spoof / Lockware emits an `Override` factor on behavior.
  No separate machinery.

---

## 1. The pieces — `Factor`, `CharacterGenerator`, `Character`

**`Factor`** — one contribution to one stat: `{ stat, kind, value }`, where `kind`
is `Add` · `Increased` · `More` · `Override` (§2). A generator emits a small `Vec`
of these.

**`CharacterGenerator`** (`chargen`) — the **one uniform type** for everything that
shapes a character: **gear, weapons, armor, augments/implants, consumables, buffs,
even a spoof** are all `chargen`-typed. `generate(character) → character` appends
its factors and returns the character. A generator carries **no math** — it only
*declares* the factors it contributes, gated/scaled by its condition
(Online/Degraded/Offline → full / half / none). That's its whole job.

**`Character`** — `{ base, factors: Vec<Factor> }` plus the **read surface** the
`sim` queries. **The `Character` does all the math:** every effective value is
**composed by it** from base + its factors (the §2 bucket fold) — generators never
compute, only the `Character` sums and multiplies.

> **Narrative vs. type ◆.** The domain language calls these things **modifiers** —
> they "modify the character." But the **type is `chargen`**: a modifier doesn't
> mutate the character, it **generates** the (composed) one via factors. So **every
> modifier is `chargen`** — *permanent* (gear / augments) **and** *transient*
> (buffs, debuffs like Lag/Breach, a spoof) alike; a transient modifier is just a
> generator whose factors are active for a duration. *(Boundary: pure
> **tick-effects** — DoTs, plating-shred — **act** each tick rather than modify
> stats, so they stay their own mechanism; a status may carry both a chargen
> modifier and a ticker.)*

| Group | Queries (on the `Character`) |
|---|---|
| **stats** | `link` · `firewall` · `immunity` · `initiative` · `max_integrity` · `armor_class` |
| **weapon** | `attack` (damage / type / pen / range / emp) |
| **behavior** | `movement` · `targeting` (the §7J profiles) |
| **capability** | `hack` (the netrunning loadout) |

The **base** is the chassis innate line; generators only ever **add factors** to
it — they never carry the query surface themselves.

---

## 2. Composition

```text
base ─▶ generate ─▶ generate ─▶ … ─▶ Character { base, factors[] }
        (implant)   (weapon)              │
                                          └─▶ recompute(): fold the factors per stat
```

Generators **append factors**; the `Character` then **folds its factor list per
stat**. For numeric stats it sums/multiplies by the buckets below; for behavior /
capability it takes the **last `Override`** (top wins). Folding is order-
independent for the numeric buckets (sum/product), so only `Override` cares about
order — last-applied generator wins.

### Combining numbers — the bucket model ◆ (additive vs. multiplicative)

*How the `Character` combines its factors is a **balance lever**, not an
implementation detail. Shipped ARPGs converged independently on a **bucket** model
(Path of Exile's `Added / Increased / More`; Diablo 4's additive-vs-`x%` buckets)
— adopt it.* Each `Factor`'s **kind** sets how it combines:

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
`movement`) the **last `Override` factor wins** (a spoof *replaces* targeting; not
a number).

**Composition formula** (PoE / D4 order, per stat) — but with `More` empty by
default it collapses to a plain additive fold:

```text
effective = (base + Σadd) × (1 + Σincreased) × Π(1 + moreᵢ)
          = (base + Σadd) × (1 + Σincreased)            // default: no More → just a fold
```

**Our calls ◆:**
- **Additive by default → the `Character` just folds its factors, runaway-proof.**
  Today's implant contributions are all flat `Add`, and `Increased` only *sums*, so
  the whole composition is a `Vec`-fold over the factor list.
- **`More` is off by default.** Genuine multiplication is the only runaway risk;
  don't ship a free-stacking `More` bucket. If a *signature* mechanic ever needs it
  (a marquee implant, the PAN-mesh synergy), it's **rare + hard-capped**, and still
  just one product step in the same fold.
- **Condition scales the *generator's* factors before the fold.** A Degraded
  generator ([`cyberware.md`](cyberware.md) §6) emits **half-value** factors (its
  `benefit_factor`); an Offline one emits none.
- **Per-stat buckets.** Link, Firewall, damage, … each fold independently.

---

## 3. Derived vs. live state ◆

| Composed (derived — recomputed from base + factors) | Live battle-state (on the `Character`, **not** composed) |
|---|---|
| the whole §1 query surface (stats / weapon / behavior / capability) | `pos`, **current** Integrity, the depletable **Barrier / Plating pools**, `statuses`, `alive`, and each generator's **condition** (Online/Degraded/Offline — *gates* the factors it emits) |

The split is the crux: the factor fold gives the **effective maxima / profile**;
the `Character` instance holds the **mutable fight state** that ticks down during a
battle.

---

## 4. Realization ◆ — a flat factor `Vec` on the `Character`

The generator-emits-factors model settles the earlier "decorator chain vs. fold"
question: there's **no query chain** at all.

- **Generators** run once at build / loadout (and on condition change) to emit
  `Factor`s into the `Character`'s flat `Vec<Factor>` — no per-method delegation,
  no `Box<dyn>` chain to walk.
- **`Character::recompute()`** folds that `Vec` per stat by the §2 buckets, caching
  the results; reruns only on a **loadout / condition change** (the dirty flag), so
  the deterministic hot loop just reads cached effective values.
- A breach / EMP that flips a generator's condition → re-emit its (now zero/halved)
  factors → `recompute()`. The generators stay **indexable** for exactly this.

The **public shape**: `character.link()` / `character.targeting()` read the
composed value; nothing outside cares that it came from a folded factor list.

---

## 5. Subsumes the cyberware fold

The current implant model ([`cyberware.md`](cyberware.md) A–F) becomes the **first
kind of generator**, with its meaning intact:

| Cyberware concept | In the generator/factor model |
|---|---|
| `Contribution` fold / `refold` | a generator emitting `Add`/`Increased` factors; effective = `Character::recompute()` |
| Condition (Online/Degraded/Offline) | gates/scales the **factors emitted** (Degraded = half, Offline = none), unchanged |
| benefit ↔ liability, hack-effects | the generator carries them; breach disables it → re-emit (zero) → recompute |
| EMP / PAN / Cascade | operate on the generator set (disable all / cascade), unchanged semantics |

Then it **extends**: **weapons** and **armor** become further generator kinds
(multi-weapon, layered armor), and **behavior-corruption** (spoof / Lockware) is a
generator that emits an `Override` factor on `movement` / `targeting`.

---

## 6. Layered defense — optional reach ◆

Defense is itself **layered** — an incoming hit routes **outermost → inner**
(Barrier → Plating → Integrity base), each absorbing the remainder by its pen-tier
rules. These defensive layers are **depletable pools** (live state §3), distinct
from the stat-factor generators above; a generator can *grant* a pool (subdermal
plating → +Plating), but the absorb sequence is its own ordered pass — *adopt
later*; the current pool-based `apply_damage` already works, so this is a clean-up,
not a blocker.

---

## 7. Migration plan ◆

| Step | Does | Touches |
|---|---|---|
| **L1** | define `Factor` + `CharacterGenerator` + the `Character`'s base + `Vec<Factor>` + a cached `recompute()` fold (Add/Increased buckets) | `sim` stat reads |
| **L2** | port **implants → generators** (Contribution/condition → emitted factors); keep breach / EMP / PAN / Cascade behavior | the implant model + ~10 tests re-expressed on the factor API |
| **L3** | **behavior factors** (movement / targeting compose from factors) → finishes combat **Phase 1** on this model; a smartgun emits an `Override(targeting)` | combat Phase 1 |
| **L4+** | **weapon** generators, **armor** generators, **corruption** generators (spoof/Lockware) | new content |

**Test impact:** the implant tests (install / disable / degrade / EMP / cascade)
re-express on the factor API — the breach/condition **semantics are unchanged**, so
the assertions port. Contest / objective / run tests are untouched: they read
*effective* stats, which still exist (just composed from factors now).

---

## 8. Why now ◆

Do **L1–L3 before finishing combat Phase 1** (behavior profiles): the movement /
targeting profiles *compose from factors on the `Character`*, so wiring them into
flat fields first would be throwaway. The Phase-1 enums are already committed and
slot straight in as `Override` factors. After L3, the "smartgun overrides targeting"
and
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
