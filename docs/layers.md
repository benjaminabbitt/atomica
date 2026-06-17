# Character architecture — generators, modifiers, composition

*The **architecture** for building a `Character`: **generators** (`chargen`) decorate
it by **adding / removing `Modifier`s**; the `Character` holds a **referenceable
modifier set** (with **summation**, **tags**, **add/remove** — not just a stat
formula); its **accessors compose** the effective stats. Every modifier **links back
to the decorator id that spawned it**, and any component can **reference and remove
another's** — the counterplay substrate. Generalises the hand-rolled cyberware fold
([`cyberware.md`](cyberware.md) Phases A–F) to also carry weapons, armor, buffs, and
behavior-corruption. ◆ = decision (overridable, per repo convention). Status: 🔭
**planned** — the architecture to refactor toward; the current code keeps the fold
until it lands.*

---

## 0. The principle ◆

Equipment is **not special-cased**, and the decorator **does not implement the unit
interface**. Instead, each piece of kit is a **character generator** (`chargen`) ◆ —
a decorator that **emits a `Character`**: given the character so far, it **adds (and
may remove) `Modifier`s** on it and returns it. A pipeline of generators
(base → implant → weapon → armor → …) **builds the `Character`**, accumulating its
**referenceable modifier set**; the `Character`'s **accessors** then **sum and
operate** over those modifiers (the §2 math) to answer each query.

So the split is three clean roles:
- **`chargen` (decorators) emit a `Character`** — their *output is the character*,
  not a bag of modifiers. Their job is to **decorate** it: **add their modifiers**
  (each stamped with the decorator's id), and **remove** others' where they counter.
- **The `Character` holds** the base + the **referenceable modifier set** (add /
  remove / look-up by id, `source`, or `tag`).
- **The `Character`'s accessors do the math** — `link()` / `attack()` / … **sum the
  factors and run the operations** (sum / multiply / pick-override). There is **no
  separate orchestrator**; the calculation lives in the accessors.

This is the architectural expression of two design throughlines:
- *"Chrome composes onto the body"* — each generator decorates the character with
  more factors; the accessors sum them cleanly.
- *"You program your units; the enemy hacks your script"* — **corruption is just
  another generator**: a spoof / Lockware decorates the character with an `Override`
  on behavior. No separate machinery.

---

## 1. The pieces — `Modifier`, `CharacterGenerator`, `Character`

**`Modifier` — the standard interface ◆.** Every modifying component a generator
puts on a character implements **one `Modifier` interface**, so the `Character` holds
them **uniformly** and — crucially — can **find and remove** them. Each carries
identity so it's **referenceable**:
- **`id`** — a stable handle;
- **`source`** — a **link to the id of the `chargen` (decorator) that spawned it**,
  so removing/expiring a decorator drops exactly the modifiers it spawned (a deck
  going Offline, a buff ending);
- **`tag`** — a category (`Worm` · `Virus` · `Buff` · `Spoof` · …) for matching.

Kinds of `Modifier`: a numeric **`Factor`** `{ stat, kind: Add/Increased/More, value }`
(§2) · an **`Override`** (behavior / capability) · and other modifying components (a
granted pool, a hook). All share the interface — **referenceable, removable**.

**`CharacterGenerator`** (`chargen`) — the **one uniform type** for everything that
shapes a character (gear · weapons · armor · augments · consumables · buffs · a
spoof). `generate(character) → character` **decorates** it: it **adds** its modifiers
and **may remove** others' (a Vaccinated cleanse removes `Virus`-tagged modifiers; a
Ripperdoc removes a breach's disable), then **returns the character**. Its output *is
a character*, not modifiers.

**A decorator is a stateful component ◆**, not a one-shot factory. Beyond the
modifiers it contributes, it carries:
- **`expiration`** — its lifecycle: `Permanent` (gear) · `Duration(n)` · `Until(cond)`.
  On **expiry** the decorator is removed and its modifiers drop with it (matched by
  `source` id).
- an **event handler** — it **receives events** and reacts. On a battle event —
  `TickStart` · `OnHit` · `OnDeath` · `OnBreach` · … (the taxonomy's §6.6 trigger
  set) — it can tick its expiration, deal a DoT, fire a death-trigger, add/remove
  modifiers, or **spread** (a contagion adds a decorator to a neighbour).

So a decorator has a **passive face** (the modifiers the `Character` composes into
stats — *no math of its own*) and an **active face** (its lifecycle + event
reactions). Static gear is just a `Permanent` decorator with no reactions.

**This subsumes the status system ◆.** The 9-axis status schema *is* a decorator:
`trigger` → which events it handles · `timing` → event order · `decay` → `expiration`
· `effect`/`magnitude` → its modifiers + reactions. So **statuses and equipment are
one component type** — a buff is a `Duration` decorator emitting factors; a **DoT** a
decorator that damages on `TickStart`; an implant a `Permanent` one.

**`Character`** — holds `base` + the **referenceable modifier set**, plus the
**accessors** the `sim` queries. Mutating API: `add(m) → id` · `remove(id)` ·
`remove_where(pred)` (match by `source` / `tag`). **All math lives in the
accessors:** `link()` / `attack()` / … **sum the relevant `Factor`s and run the
operations** (§2) over the base. Generators never compute; only the accessors do.
*(Caching behind a dirty flag is a pure optimization, §4 — it doesn't move the math.)*

> **Removal is the counterplay substrate ◆.** Because every modifier is
> referenceable + removable, the design's whole **answer-half *is* removal**:
> **cleanse / Antimalware / Antivirus** strip contagion modifiers (by `tag`); the
> **Ripperdoc** removes a breach's disable; **Signals / counter-spoof** removes an
> `Override`; **dispel** removes a buff; **Quarantine** severs Link. Counterplay =
> one component **referencing and removing another's modifiers** — one mechanism for
> the whole mender / anti-Worm / signals layer (taxonomy §7H).

> **Narrative vs. type ◆.** The domain language calls these things **modifiers** —
> they "modify the character." But the **type is `chargen`**: a modifier doesn't
> mutate the character, it **generates** the (composed) one. So **every modifier is
> `chargen`** — *permanent* (gear / augments) **and** *transient* (buffs, debuffs
> like Lag/Breach, a spoof) alike; a transient one is just a decorator with a
> `Duration` `expiration`. *(A pure **tick-effect** — a DoT, plating-shred — is the
> same decorator's **active face** reacting on `TickStart`, not a separate
> mechanism.)*

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
                                          └─▶ accessor link()/attack()/…: sum factors per stat
```

Generators **add factors**; the `Character`'s **accessors fold the relevant factors
per stat** on query. For numeric stats they sum/multiply by the buckets below; for
behavior /
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
  generator ([`cyberware.md`](cyberware.md) §6) adds **half-value** factors (its
  `benefit_factor`); an Offline one adds none.
- **Per-stat buckets.** Link, Firewall, damage, … each fold independently.

---

## 3. Derived vs. live state ◆

| Composed (derived — recomputed from base + factors) | Live battle-state (on the `Character`, **not** composed) |
|---|---|
| the whole §1 query surface (stats / weapon / behavior / capability) | `pos`, **current** Integrity, the depletable **Barrier / Plating pools**, `statuses`, `alive`, and each generator's **condition** (Online/Degraded/Offline — *gates* the factors it adds) |

The split is the crux: the factor fold gives the **effective maxima / profile**;
the `Character` instance holds the **mutable fight state** that ticks down during a
battle.

---

## 4. Realization ◆ — a flat, referenceable modifier set on the `Character`

The generator-decorates-the-character model settles the earlier "decorator chain vs.
fold" question: there's **no query chain** at all.

- **Generators** run at build / loadout (and on condition change) to **add** (and
  sometimes **remove**) `Modifier`s in the `Character`'s flat, **keyed** modifier set
  (`id → Modifier`, indexed by `source` / `tag` for removal) — no per-method
  delegation, no `Box<dyn>` chain to walk.
- **The `Character`'s accessors** sum the relevant factors per stat (§2). Caching
  the summed values behind a **dirty flag** — recomputed only on **loadout /
  condition change** — is a pure optimization for the deterministic hot loop; it
  doesn't move the math out of the accessors.
- A breach / EMP that flips a generator's condition → re-add its (now zero/halved)
  factors → mark dirty. The generators stay **indexable** for exactly this.

The **public shape**: `character.link()` / `character.targeting()` read the
composed value; nothing outside cares that it came from a folded factor list.

---

## 5. Subsumes the cyberware fold

The current implant model ([`cyberware.md`](cyberware.md) A–F) becomes the **first
kind of generator**, with its meaning intact:

| Cyberware concept | In the generator/factor model |
|---|---|
| `Contribution` fold / `refold` | a generator **adding** `Add`/`Increased` factors; effective = the `Character`'s accessors summing them |
| Condition (Online/Degraded/Offline) | gates/scales the **factors it adds** (Degraded = half, Offline = none), unchanged |
| benefit ↔ liability, hack-effects | the generator carries them; breach disables it → re-add (zero) → mark dirty |
| EMP / PAN / Cascade | operate on the generator set (disable all / cascade), unchanged semantics |

Then it **extends**: **weapons** and **armor** become further generator kinds
(multi-weapon, layered armor), and **behavior-corruption** (spoof / Lockware) is a
generator that adds an `Override` factor on `movement` / `targeting`.

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
| **L1** | the architecture: **`Modifier`** interface (`id` / `source`→decorator-id / `tag`; `Factor` kind) + the **decorator** (`generate` add/**remove**, **`expiration`**, **event handler**) + the **`Character`** (base + keyed modifier set + `add`/`remove`/`remove_where` + **accessors** that sum factors; dirty-flag cache optional) | `sim` stat reads |
| **L2** | port **implants → decorators** (Contribution/condition → factors); keep breach / EMP / PAN / Cascade behavior | the implant model + ~10 tests re-expressed |
| **L2b** | port the **status pool → decorators** — `trigger`→events, `decay`→`expiration`, DoTs→`TickStart` reactions; unifies statuses + equipment | the `Status` system + its tests |
| **L3** | **behavior factors** (movement / targeting compose from factors) → finishes combat **Phase 1** on this model; a smartgun adds an `Override(targeting)` | combat Phase 1 |
| **L4+** | **weapon** decorators, **armor** decorators, **corruption** decorators (spoof/Lockware) | new content |

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
  modifiers are `{ value, op: Add|Multiply }`; the accessor recomputes only when
  `altered` (the optional dirty-flag cache behind the `Character`'s accessors). [RefresherTowel — Modifiable Stats](https://refreshertowelgames.wordpress.com/2024/02/17/how-to-comfortably-deal-with-modifiable-stats/)
- **Design wisdom** — additive = legible, self-limiting (diminishing returns),
  easy to balance; multiplicative = powerful, compounding, must be rare. The
  bucket separation is the balance lever. [Paradox forums discussion](https://forum.paradoxplaza.com/forum/threads/additive-bonuses-vs-multiplicative-bonuses.1144836/)
