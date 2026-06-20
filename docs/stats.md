# Stats & the core roll — the resolution spine

*The canonical reference for **how a number becomes an outcome**: the dice, the
attributes, the skill tiers, and the three contest shapes every system resolves
through. Everything else (`combat.md`, `netrunning.md`, `corruption.md`,
`cyberware.md`) cites this file for the roll. **Numbers are illustrative / tuning
(TBD).** ◆ = decision. Status: **✅ built** · **◑ partial** · **🔭 planned**.*

---

## 1. The core roll — `2d10` roll-under ✅

Every check in the engine is one mechanic ([`crates/sim/src/roll.rs`](../crates/sim/src/roll.rs)):

```text
2d10 ≤ target          success on equal-or-under
margin = target − dice   the degree of success (drives stacks / severity)
```

- **No additive, no doubling, no base.** The `target` *is* the actor's effective
  number (skill, rating) less any situational penalty — nothing is added to the
  dice. This keeps one dice language everywhere.
- **Crit / fumble** key off the *natural* dice: a natural **2–3** crits
  (auto-succeed, even against an impossible target), a natural **19–20** fumbles
  (auto-fail, even against a trivial one). ≈3% each.

### Why 2d10 and not 3d6 ◆

2d10 is a **flatter (triangular)** curve than 3d6's bell (σ ≈ 4.1 vs 3.0). The
point is **lower balance sensitivity**: 2d10 **caps the extremes** — no roll
exceeds ~90% — so no single unit can become a near-invincible linchpin the whole
run pivots on, and a one-point stat change moves the outcome far less.

| roll ≤ target | 7 | 10 | 13 | 15 (elite) | 16 |
|---|---|---|---|---|---|
| **3d6** | 16% | 50% | 84% | **95%** | 98% |
| **2d10** | 21% | 45% | 72% | **85%** | 90% |

Measured on the sharpest sensitivity case (one unit's Evade → run clear %), 2d10
roughly **halved** the swing (worst single-point step 49 pts → 16) and removed the
cliffs (no setting auto-wipes or auto-sweeps). The cost: the flatter curve lifts
baseline miss (~49% → ~56%) — luck matters more per roll, by design.

---

## 2. Primary attributes — the GURPS ~10 scale ✅

Four characteristics, centred on **10 = average human** (range ≈ 8–14; combat
archetypes 10–13). They are the substrate the four **skill families** are tiers
*on*, and the combat/digital stats derive from.

| Attribute | Field | Governs (skills) | Feeds |
|---|---|---|---|
| **Body** | `unit.body` | Melee, Heavy | **Integrity = Body × `HP_PER_BODY`** ✅ (toughness *is* HP — one stat), melee damage |
| **Dexterity** | `unit.dexterity` | Gunnery, Stealth, **Evade** | **Evasion** ✅, **physical Initiative** ✅ — *dragged down by heavy plating (the armor tradeoff)* |
| **Intellect** | `unit.intellect` | Hacking, Medical, Tech | **digital Initiative** ✅ (net turn order — *speed of thought*) |
| **Will** | `unit.will` | (morale / spoof-resist) | 🔭 |

> **Firewall and Link are *granted*, not derived.** They come from gear / chassis (a cyberdeck
> lifts both), not from an attribute — Intellect governs the netrunning *skills* and the digital
> turn order, but the wall itself is equipment. This is deliberate: the net surface is something
> you *install*, not something you *are*.
>
> **Programs are the digital domain's *skills*.** Where a physical action is `attribute + skill-tier`,
> a digital one is `granted stat (Link / Firewall) + quality program` — you don't *train* onto the
> net surface, you *load better software* onto it. A quality program runs **on** the granted stat
> and acts as its skill-tier: **Ghost** raises net defense on top of Firewall (a defensive program,
> `netrunning.md` §10.8), the offensive riders run on the attacker's Link channel. So a fat granted
> stat with cheap software, or a thin one with premium programs, are two routes to the same edge —
> the same attribute-vs-skill trade, in installed form.

**Body and Integrity are one stat ✅.** Max Integrity (the HP pool) is **derived** —
`Body × HP_PER_BODY` (K = 6: an average Body-10 build carries ~60 HP; a bolted-down node
scales Body up to whatever pool it needs). So toughness and health aren't tracked separately:
a heavier unit (more HP) is *also* a harder melee hitter, and a Body stat-up implant (actuators,
the decentralized heart) fattens the HP pool directly. The cost the design accepts: a very
high-HP bruiser reliably lands its melee (Evasion, not a to-hit roll, is the defense).

**Initiative is action-typed ✅.** A unit's turn order derives from the attribute the *action*
uses — **Dexterity** for a physical activation (reflexes), **Intellect** for a digital one (a
quick mind dives sooner). Link still gates a hack's reach/channel/presence but no longer sets
the net turn order.

---

## 3. Skills — tiers on the governing attribute ✅

A skill is **not** an independent number; it's a **proficiency tier** *on* its
governing attribute ([`skills.rs`](../crates/sim/src/skills.rs)):

```text
effective skill = governing attribute + skill tier
```

The tiers run the GURPS default-to-master spread (**±4**):

| Tier | Untrained | Exposed | Beginner | Competent | Expert | Elite |
|---|---|---|---|---|---|---|
| **modifier** | **−4** | −3 | −2 | **0** | +2 | +4 |

- **`competent` = your raw attribute** (the 0 tier).
- **Untrained (−4) is the *default*** ◆ — every skill a unit hasn't trained sits
  there. This is load-bearing: a non-dodger's Evade is `Dex − 4`, a genuine but
  **secondary** save (see §4). Training raises specific skills off the floor.
- A high attribute lifts *all* its skills at once; plating's −Dexterity drags
  every Dex skill (Gunnery, Evade…) down with it.

So an elite duelist on Body 12 with Melee Elite (+4) attacks at effective **16**;
a rank-and-file mook with an untrained Dex-10 Evade defends at **6**.

---

## 4. Combat resolution — the opposed roll ✅

A blow is an **opposed** exchange ([`resolve_opposed`](../crates/sim/src/roll.rs),
[`combat.md`](combat.md)): the attacker rolls to hit *and* the target rolls an
active **Evade**. **The blow lands only if the attacker succeeds *and* the
defender fails.**

```text
attacker:  2d10 ≤ effective(weapon skill) + accuracy − range − cover
defender:  2d10 ≤ Evasion − Speed                    (Evasion = effective Evade = Dex + tier)
land = attacker succeeds AND defender fails
```

- **Evasion is just the Evade skill** ◆ — `Dexterity + Evade-tier`, defaulting to
  untrained (`Dex − 4`). Dodge is potent for a trained acrobat and a thin secondary
  save for everyone else; nothing special, no separate formula.
- **Speed — the Dodge penalty** ◆ (a stat on `Attack`, peer of `damage`). A
  fast attack is far harder to dodge than a slow one, so the weapon's **Speed**
  docks the defender's Evade: a swung blade is slow (Speed ~1, Dodge stays potent),
  a high-velocity round is fast (Speed ~3, Dodge barely helps). *(May also feed
  penetration later — 🔭.)*
- **Situational penalties** shrink the attacker's target: **range** (a projectile
  is harder the farther the shot), **awkward** (a rifle/polearm is clumsy jammed up
  close), and **cover** (the hex's bonus). See [`combat.md`](combat.md) §3.5.
- **Undefended ⇒ auto-hit.** If the target's (Speed-adjusted) Evade ≤ 0 and the
  shot is clear, the blow lands with no roll — trivial exchanges stay
  deterministic; the dice only matter once the target can actually dodge.
- **Hacking is opposed too** ◆ — a netrunner's breach has the same shape: the
  runner rolls `2d10 ≤ avg(effective Hacking, channel)` and the target's
  **Firewall** rolls an *active defense*; the breach lands only if the runner
  connects **and** the Firewall fails. An **undefended** surface (Firewall ≤ 0)
  needs no defense roll. See [`netrunning.md`](netrunning.md).

---

## 5. Static contests — the resist *modifier* ✅

A **contagion jump** or a **poison tick** has no active defender — it resolves
against a **passive threshold**, folded in as a **modifier (a flat penalty), not a
target number** ◆ ([`resolve_versus`](../crates/sim/src/roll.rs)):

```text
2d10 ≤ rating − resist
```

- `rating` is the actor's effective skill/potency (~10–15); `resist` is the
  target's **Firewall / Immunity / security rating** as a small penalty (a few
  points), *not* a number to beat. Every point of resist costs the actor a point of
  target. This is the GURPS skill-check pattern: *roll under your skill, penalized
  by the difficulty.*
- **No static TN anywhere** — the same `2d10 ≤ target` core; the defense is just a
  term inside `target`. (Hacking, which *does* have an active defender, is an
  opposed roll instead — §4.)
- Used by: **contagion** spread (`rating = virulence`, `resist = Immunity`,
  [`corruption.md`](corruption.md)) and the status **stochastic gate**
  (`rating = power + stacks`, `resist = the status's Resist`). Afflictions/contagions
  are authored on the same ~10 scale so the penalty bites meaningfully.

---

## 6. Derived & digital stats ✅

| Stat | From | Role |
|---|---|---|
| **Evasion** | `Dexterity + Evade-tier` | the active-defense roll (§4) |
| **Firewall** | `Intellect` + implants | digital **active defense** — rolls back against a hack (opposed, §4); ≤ 0 = undefended |
| **Immunity** | (bio track) | contagion resist penalty (§5) |
| **Link** | implants (cyberdeck…) | reachability gate · digital initiative · hack channel · **antenna range** ([`netrunning.md`](netrunning.md)) |
| **Initiative** | `Dexterity` + gear | physical activation order |
| **Integrity / Barrier / Plating** | Body + armor | the HP pools ([`combat.md`](combat.md)) — *not* modifiers; clamped pools |

---

## 7. Calibration — the Moderate target ⏳

The probe (`atomica-run probe`) tunes content against the fixed starter trio. The
current landing — squarely **Moderate** under 2d10:

| Scenario | Clear | Losses | To-hit miss |
|---|---|---|---|
| **gauntlet** (3-unit trio, the tuning target) | ~91% | ~1.1 / 3 | ~56% |
| **street** (5-unit squad) | ~100% | ~0.4 / 5 | ~56% |

Illustrative bands on the 2d10 scale:

- **Attributes** — weak 8 · average 10 · strong 12 · exceptional 13–14.
- **Skill tiers** — untrained −4 (default) · competent 0 · expert +2 · elite +4.
- **Effective combat skill** — fodder ~8–10 · professional ~13–15 · master ~16.
- **Firewall / Immunity (penalty)** — unprotected 0 · modest 4 · hardened 6–8.
- **Speed** — melee/thrown ~1 · firearm ~3.

The flatter curve means **bigger skill *gaps*** read as advantage (a +4 effective
edge is ~30 points of hit-rate), where 3d6 rewarded tighter ones. Tuning is now a
gentle knob rather than a cliff (§1).
