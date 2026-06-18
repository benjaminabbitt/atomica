# Character architecture — generators, modifiers, composition

*The **architecture** for building a `Character`: **generators** (`chargen`) decorate
it by **adding / removing `Modifier`s**; the `Character` holds a **referenceable
modifier set** (with **summation**, **tags**, **add/remove** — not just a stat
formula); its **accessors compose** the effective stats. Every modifier **links back
to the decorator id that spawned it**, and any component can **reference and remove
another's** — the counterplay substrate. Generalises the hand-rolled cyberware fold
([`cyberware.md`](cyberware.md) Phases A–F) to also carry weapons, armor, buffs, and
behavior-corruption. ◆ = decision (overridable, per repo convention). Status: ✅
**built — L1–L6** (see §7/§9); the `Character` gen is the single source of truth —
stats, weapons, armor, behavior and corruption all compose, the flat stat line and the
hand-rolled fold are gone. **L7+** (Cascade/breach ladder, Smartgun, contagion) is next.*

---

## 0. The principle ◆

Equipment is **not special-cased**, and the decorator **does not implement the unit
interface**. Instead, each piece of kit is a **character generator** (`chargen`) ◆ —
a stateful decorator that **adds (and may remove) `Modifier`s**. `chargen` is **the
modification interface** — *every* change to a character flows through it. The
**`Character` wraps the chargen set** ◆: it *owns* the ordered decorators (the
persistent modifier state) **plus the live pools** (§3), and exposes
**`realize()`** — the composed view, produced on demand by running the decorators
over the `base` (base → implant → weapon → armor → …) into a **referenceable modifier
set**, whose accessors **sum and operate** (the §2 math) to answer each query.

So reads take one of two paths:
- **Composed stats / weapon / behavior** → `self.realize().link()` — through the
  folded modifier set.
- **Live pools** (current Integrity, Barrier / Plating, pos, alive) → **straight off
  the `Character`** — they're path-dependent state, not composed (§3).

And the three roles stay clean:
- **`chargen` (decorators) are the modification interface** — they **add their
  modifiers** (each stamped with the decorator's id) and **remove** others' where they
  counter. Stateful: they hold an `expiration` and **receive events** (which may
  modify them). **Applying a DoT / status / breach mutates the gen** — adds or changes
  a decorator in the set, which dirties `realize`.
- **The `Character` wraps the gen + pools** — the persistent thing that saves, ticks,
  and takes events; `realize()` is its on-demand composed view (composed **fresh each
  call** — no cache, §4), looked-up by id / `source` / `tag`.
- **The realized view's accessors do the math** — `link()` / `attack()` / … **sum the
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
them **uniformly** and — crucially — can **find and remove** them. The modifier is a
**transient projection** (regenerated each `realize`), so its durable referent is its
**`source`**, not an id of its own:
- **`source`** — a **link to the id of the `chargen` (decorator) that spawned it**,
  so removing/expiring a decorator drops exactly the modifiers it spawned (a deck
  going Offline, a buff ending);
- **`tag`** — a category (`Worm` · `Virus` · `Buff` · `Spoof` · …) for matching.

Kinds of `Modifier`: a numeric **`Factor`** `{ stat, kind: Add/Increased/More, value }`
(§2) · an **`Override`** (behavior / capability) · and other modifying components (a
granted pool, a hook). All share the interface — **referenceable, removable**.

**`CharacterGenerator`** (`chargen`) is **the modification interface** ◆ — the *one*
interface through which a character is ever modified. Everything that shapes a
character — gear · weapons · armor · augments · consumables · buffs · a spoof · and
a decorator reacting to an event — does so **through `chargen`**.
`generate(character) → character` **decorates** it: **adds** its modifiers and **may
remove** others' (a Vaccinated cleanse removes `Virus`-tagged modifiers; a Ripperdoc
removes a breach's disable), then **returns the character**. Its output *is a
character*, not modifiers.

**A decorator is a stateful component ◆**, not a one-shot factory. Beyond the
modifiers it contributes, it carries:
- **`expiration`** — its lifecycle: `Permanent` (gear) · `Duration(n)` · `Until(cond)`.
  On **expiry** the decorator is removed and its modifiers drop with it (matched by
  `source` id).
- an **event handler** — it **receives events** and reacts. On a battle event —
  `TickStart` · `OnHit` · `OnDeath` · `OnBreach` · … (the taxonomy's §6.6 trigger
  set) — it may **modify itself** (bump its stacks, refresh / decrement its
  `expiration`, change the modifiers it carries) **and** act outward through
  `chargen` (deal a DoT, fire a death-trigger, add/remove modifiers, or **spread** —
  a contagion adds a decorator to a neighbour).

So a decorator has a **passive face** (the modifiers the `Character` composes into
stats — *no math of its own*) and an **active face** (its mutable lifecycle + event
reactions). Static gear is just a `Permanent` decorator with no reactions.

**This subsumes the status system ◆** — *built* (`StatusSpec::to_decorator`, L2b).
The 9-axis status schema *is* a decorator: `effect` → a `TickStart` **hook** (DoT /
shred — the active face) *or* a passive **flag** / **factor** (Stun → `Flag::Stun`,
Breach → `Flag::Vuln`, Lag → a `More` factor on Initiative); `decay` → the
decorator's `Wear` (by-duration / by-stacks) + `stacks`; `magnitude` → an `Amount`
(Flat / PctMax / PctCurrent, the softener) resolved at dispatch. So **statuses and
equipment are one component type** — a buff is a `Duration` decorator emitting
factors; a **DoT** one that damages on `TickStart`; an implant a `Permanent` one.
*(Still the loop's job, not the decorator's: the **stochastic** resist-roll behaviour,
`stacking` merge-on-reapply, and `targeting` — they enter when the loop adopts the
`Character` path.)*

**`Character` — wraps the gen, owns the pools ◆.** The `Character` is the durable
object: it **wraps the chargen set** (`base` + ordered decorators — the persistent
modifier state) **and holds the live pools** (current Integrity / Barrier / Plating,
`pos`, `alive` — §3). It exposes **`realize()`** — the composed view, produced on
demand by running the decorators over the base (each `generate` adds its modifiers)
into the referenceable modifier set, then queried through the **accessors**. So reads
split: **`self.realize().link()`** for composed stats, **`self.integrity`** for a
pool. Events and the mutating API (`add`/`remove`/`remove_where` by
`id`/`source`/`tag`) act on the **gen** — applying a DoT, status, or breach mutates a
decorator — and the next `realize()` reflects them. **All composed math lives in the
accessors:** `link()` / `attack()` / … **sum the relevant `Factor`s and run the
operations** (§2) over the base — generators never compute. *(`realize()` composes
**fresh each call**: a dirty-flag cache was **intentionally eliminated** — the fold is
cheap and a cache only buys invalidation bugs. Revisit only if it ever profiles hot.)*

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
behavior / capability it takes the **highest-priority `Override`**. The gen is a
**priority-ordered vec** (sorted by `(priority, install-order)`), so only `Override`
cares about order, and it's settled by **priority, not install luck** — a
`CORRUPTION`-priority spoof beats `GEAR` however the loadout was equipped. Numeric
buckets are order-independent (sum/product). **Removal (`removes`) is a standing ward
— order-independent**: a decorator strips matching modifiers from the whole set,
whether the infection arrived before or after it.

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

| Composed (`realize()` — recomputed from base + factors) | Live battle-state (on the `Character`, **not** composed) |
|---|---|
| the whole §1 query surface (stats / weapon / behavior / capability) | `pos`, **current** Integrity, the depletable **Barrier / Plating pools**, `alive`, and each generator's **condition** (Online/Degraded/Offline — *gates* the factors it adds) |

The split is the crux: the factor fold gives the **effective maxima / profile**;
the `Character` holds the **mutable fight state** that ticks down during a battle.

### 3a. The dividing-line rule ◆

What goes in the fold vs. on the `Character`:

> **Pure function of the current gen → composed** (`realize`); `max_integrity =
> base + Σ factors`, no history. **Path-dependent → live state** (a pool/field);
> `current_integrity` depends on the *sequence* of hits / heals / clamps / deaths.

The two *feel* alike — both are sums — which is the trap. The cut is **order and
thresholds**: a hit that crosses 0 fires death triggers a later heal can't un-fire;
pools clamp at 0 and max on *every* op, so `clamp(clamp(x−a)+b) ≠ clamp(x−a+b)`. A
fold has no order and no thresholds, so **path-dependent quantities can't live in
it** — HP loss is a pool, not a modifier.

### 3b. The three kinds of transient ◆

Everything tempting to "event-source" is one of these — **none is a modifier-log**:

| Kind | Examples | Where | Semantics |
|---|---|---|---|
| **Pools** | Integrity · Barrier · Plating · (Resolve) | live-state on `Character` | current fill of a composed max; **clamp** on change; threshold → death/break |
| **Decorators w/ expiration** | buffs · DoTs · stacks · charges | the §1 gen set | stack count + `expiration` = the decorator's mutable self-state, mutated by events |
| **Per-tick scratch** | "damage taken this tick" · "was hit" | ephemeral | recomputed each activation, fed to reactions, then discarded |

### 3c. Damage is an event against the pool, not a decorator ◆

The instinct to make damage a decorator "with a reference to its event" is right about
the **reference**, wrong about the **decorator** — current HP is a pool (§3a). So:

- **Damage is a `DamageEvent { tick, source, kind, amount }`** — applied to the pool
  immediately (ordered, clamped, threshold-checked) and emitted on an **event stream**.
  *The event* carries the attribution (kill credit, Data-spill `source`, lifesteal,
  thorns) — reactions read the stream **within the tick**; it's telemetry/output, never
  a second source of truth (the sim already replays from the seed).
- **Cause is a decorator; effect is an event.** A **Bleed** is a *decorator*
  (duration / stacks / removable); each tick it **spawns a `DamageEvent`** that hits the
  pool. The decorator persists; the −3 this tick is an event.
- **The one HP change that *is* a decorator:** a **reversible / expiring /
  identity-removable** one — a −5-max curse for 3 turns, an absorb **shield** pool a
  decorator *grants*. It expires, it's removable by id, it modifies *capacity*. The
  **running tally of damage taken is never a modifier** — it's the pool's current value.

---

## 4. Realization ◆ — `Character` wraps the gen; `realize()` is the composed view

The `Character`-wraps-`chargen` model settles the earlier "decorator chain vs. fold"
question: there's **no query chain** at all.

- **The `Character` is the durable thing** — it wraps the **gen** (the ordered
  `chargen` decorators, + `base`, each indexed by `id` so any component can reference /
  remove another) **and the live pools** (§3). This is what saves, ticks, and takes
  events.
- **`realize()` is the composed view, on demand** — running the decorators over the
  `base` produces a flat, **keyed** modifier set (`id → Modifier`, indexed by `source`
  / `tag` for removal); its **accessors** then sum the relevant factors per stat (§2).
  No per-method delegation, no `Box<dyn>` chain to walk.
- **No cache — intentionally eliminated** — `realize()` composes the view **fresh each
  call**. The fold is cheap for the gen sizes in play, and a dirty-flag cache only adds
  invalidation surface (every gen mutation, condition change, and event would have to
  remember to dirty it) for no measured win — so it was deliberately left out. The
  accessors stay the single home for the math; revisit a cache only if it profiles hot.

The **public shape**: `character.realize().link()` reads a composed value,
`character.integrity` a pool; nothing outside cares that the former came from a freshly
folded factor list.

---

## 5. Subsumes the cyberware fold

The current implant model ([`cyberware.md`](cyberware.md) A–F) becomes the **first
kind of generator**, with its meaning intact:

| Cyberware concept | In the generator/factor model |
|---|---|
| `Contribution` fold | an implant **decorator adding** `Add`/`Increased` factors; effective = the `Character`'s accessors summing them — *built* (`Implant::to_decorator`; the old `refold` is gone) |
| Condition (Online/Degraded/Offline/Destroyed) | the decorator's **`condition`** (Online `1.0`, Degraded `0.5`, Offline/Destroyed `0.0`) multiplies its factors; at `0.0` it's gated off entirely — *built* (`set_condition`, in-place, keeps identity; Destroyed terminal) |
| benefit ↔ liability, hack-effects | the generator carries them; breach **degrades** it (`set_condition`) — identity kept, so repair restores; install/breach `resize_pools` by the Δ in composed maxima |
| deck **grants** the hack | a `Capability::Hack` on the decorator; highest-priority active grant wins, drops when Offline — *built* |
| EMP / PAN / Cascade | operate on the generator set (`condition_where(Implant, Offline)` / cascade), unchanged semantics |

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
| **L1 ✅** | the architecture: **`Modifier`** (`source`→decorator-id / `tag`; `Factor { Add/Increased/More }` · `Override`) + the **`Decorator`** (priority · **`condition`** (the Online→Degraded→Offline→Destroyed ladder) · `expiration` · factors · overrides · **`grants` `Capability`** · `removes` ward · `label` · `gate`) + the **`Character`** wrapping the **priority-ordered gen** + **pools**: `install`/`remove`/`remove_where`/`set_condition` on the gen, **`realize()`** → modifier set whose **accessors** fold the bucket model; pools (`apply_pool_damage`/`heal`) read/written direct (`crates/sim/src/chargen.rs`) | parallel to `Unit` |
| **L2 ✅** (structural) | **`Implant::to_decorator`**: `Contribution` → `Add` factors, `Condition` carried on the decorator, deck → `grants: Capability::Hack`; breach/degrade/repair drive `set_condition`; `fill()` is the deploy step (max-rise never refills, §3a). The **breach-liability firing / EMP / Cascade / mesh-synergy** need the status pool → **deferred to L2b** | `implant.rs` `to_decorator` + 5 tests; `Unit` fold still drives the loop |
| **L2b ✅** | the **active face** (`Event` · `Hook`/`HookEffect` · `Reaction` · `Character::dispatch`) + **`StatusSpec::to_decorator`** (DoT/shred → `TickStart` hooks; Stun/Vuln → `Flag`; Lag → `More`; `decay`→`Wear`; `magnitude`→`Amount`) + generalized `decay()`. Completes L2's behavioral half on the gen: **EMP** = `condition_where(Implant, Offline)`, breach **fires liabilities as decorators**. *Deferred to L5:* stochastic resist-roll, stacking-merge, Cascade crit-gating | `chargen.rs` events + `status.rs` `to_decorator` + 9 tests |
| **L3 ✅** | **behavior composes from the gen** — `Realized::targeting()/movement()` (last/ highest-priority `Override`); `Unit` embeds a behavior `Character`, the action phase reads it (`select_target` / `movement_step`), `with_targeting`/`with_movement` program it, `spoof` corrupts it. Finishes combat **Phase 1** | `lib.rs` action phase + 5 tests |
| **L4 ✅** (stat read-through) | the loop reads stats **through the gen**: `Unit` composed accessors `link()`/`firewall()`/`immunity()`/`initiative()`/`max_integrity()`; the loop (hack TN/gates, woven order, resist TN, init) calls them. `apply_modifier` installs a stat `Factor` that composes live — a firewall debuff lands a hack, a buff reorders initiative. *(Transitional flat-fields-as-base seam **closed by L5**.)* | `lib.rs` accessors + loop reads + 2 tests |
| **L5 ✅** (flat-base retirement — the big bang) | the flat stat line is **gone**. The authored base lives in `Character.base` (`BaseLine`); the **live pools** (Integrity / Barrier / Plating / `alive`) live on the `Character`. **Implants** are decorators (`Unit.implants: Vec<InstalledImplant>{spec, gen}`; condition on the decorator; `refold` deleted; install/breach `resize_pools` by the Δ in composed maxima). **Statuses** are decorators (`add_status` installs/merges by `label`; `status_phase` → `Character::dispatch`; passive Stun/Slow/Vuln compose; `decay_phase` → `decay()`; the `Status` struct, `Magnitude::amount`, `resist_tn`, the free `apply_damage`, and `Defense` all deleted). Picks up the L2b deferrals: **stochastic resist-roll** (`Decorator::gate`, RNG into `dispatch`) and **stacking-merge** (`Character::apply_status`). The gen is now the **single source** | `chargen` + `lib.rs` + `status.rs` + game/run; ~190 sites, 142+14 tests |
| **L6 ✅** (content decorators) | **weapons** retire onto the gen as `Capability::Weapon(Attack)` grants (`realize().weapons()` is the loadout; a chrome arm grants one like a deck grants `Hack`; `weapon_at`/`rearm`/`with_attack` read & edit it). **Armor class** composes as `Override::Armor` (base in `BaseLine.armor`; gear wins last; `Unit::armor_class()`). **Corruption** is first-class content (`corruption.rs`): `Worm`/`Virus` debuff-source decorators tagged for **tag-targeted cleanse** (`Remove::Tag` wards), alongside the existing `spoof` behavior-corruption | `chargen` + `lib.rs` + `corruption.rs` + game; 6 tests |
| **L7+** (remaining threads) | the deferrals: **Cascade crit-gating** and the breach **per-effect §6 severity ladder** (`implant.rs`); the **Smartgun/IFF** smart-targeting mod; and the **contagion families** (Virus/Worm **spreading** via Data-spill) — a system of its own | `lib.rs` combat + `implant.rs` |

**Test impact (as built):** the implant tests (install / disable / degrade / EMP /
cascade) re-expressed on the factor API — the breach/condition **semantics are
unchanged**, so the assertions ported. Contest / objective / run tests were untouched:
they read *effective* stats, which still exist (just composed from factors now).

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

✅ **built — L1 through L6.** The `Character` (gen + `BaseLine` + pools) is the single
source of truth: every stat, the weapon loadout, armor class, and behavior all compose
through `realize()`; the live pools sit on the `Character`; implants, statuses, weapons,
armor, buffs and corruption are **all decorators**. The flat stat line and the parallel
fold / status-pool engines are **gone** (no `refold`, no `Unit.statuses`, no `Defense`,
no flat `attack` / `armor_class`). **Next (L7+):** Cascade crit-gating + the breach
per-effect §6 ladder, the Smartgun/IFF mod, and the spreading **contagion** families.

---

## Precedents (the bucket model)

- **Path of Exile** — the canonical buckets: `Added` (flat) · `Increased/Reduced`
  (sum additively, one multiplier, diminishing) · `More/Less` (each its own
  multiplier, compounding, mostly from rare sources). [PoE Wiki — Stat](https://pathofexile.fandom.com/wiki/Stat)
- **Diablo 4** — additive (`Value%`, one bucket, sums) vs. multiplicative (`x%`,
  separate, multiplies); "invest across buckets beats stacking one." [Mobalytics — Damage Buckets](https://mobalytics.gg/diablo-4/guides/damage-buckets-deep-dive)
- **Implementation pattern** — `Statistic { base, current, modifiers[], altered }`;
  modifiers are `{ value, op: Add|Multiply }`; the accessor recomputes only when
  `altered` (its dirty-flag cache). *We take the bucket model but **not** the cache —
  `realize()` recomputes fresh (§4).* [RefresherTowel — Modifiable Stats](https://refreshertowelgames.wordpress.com/2024/02/17/how-to-comfortably-deal-with-modifiable-stats/)
- **Design wisdom** — additive = legible, self-limiting (diminishing returns),
  easy to balance; multiplicative = powerful, compounding, must be rare. The
  bucket separation is the balance lever. [Paradox forums discussion](https://forum.paradoxplaza.com/forum/threads/additive-bonuses-vs-multiplicative-bonuses.1144836/)
