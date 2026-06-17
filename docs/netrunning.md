# Netrunning — system design & plan

*The digital-attack system: how hacks, worms, and spoofs resolve. Extends
[`../CHROME-AND-CODE.md`](../CHROME-AND-CODE.md) §7F and
[`design-delta-v0.26.md`](design-delta-v0.26.md) §13 (the `3d6 + skill` mechanic).
This doc fixes the **netrunning shapes** and records the resolution decisions; it
is the at-a-glance reference for the digital realm. **Numbers are TBD.** ◆ =
decision/synthesis. Status legend: **✅ built** (lives in `crates/sim`) ·
**🔭 planned** (designed, not implemented) · **⏳ tuning** (a feel number).*

---

## 1. The stat & skill line

A unit's whole netrunning profile is **two stats + one skill** (plus the bio
parallel). *Skills attack, stats defend* (§13) — so the offense is a **skill**
(Hacking) and the defenses are **stats** (Link gates, Firewall walls).

| Name | Field | Type | Role | Status |
|---|---|---|---|---|
| **Link** | `unit.link` | int◆ | **Three jobs:** ① reachability **gate** both ways (`0` ⇒ immune target / offline attacker); ② **latency → digital initiative** (your own Link orders the digital pass; high = sooner); ③ the **connection channel** (a hack's bandwidth is the *weaker* endpoint's Link, `min`). The exposure dial. | ✅ (gate/init/channel); 🔭 exposure (worm-catch) |
| **Firewall** | `unit.firewall` | int | The **universal digital TN** — every digital contest rolls against it (hacks; the digital statuses Crash/Lag/Lockware via `Resist::Firewall`). **Link-blind** on defense. | ✅ |
| **Hacking** | `unit.skills[Hacking]` | int | The **sole offensive additive** on a digital roll. No defensive net-skill exists — you buy Firewall (the stat), not a skill. | ✅ |
| *Immunity* | `unit.immunity` | int | The **bio** parallel (Virus TN) — separate track, not digital. | ✅ |

**Link is an integer ✅.** It is only ever used as a gate (`> 0`), an ordering
key, and a channel floor (`min` of the two endpoints) — it carries no fractional
meaning, so it models cleanly as `i32` **bandwidth tiers** (migrated from `f32`).

**Chassis floors (✅).** Only **Augmented** ships innate Hacking (1); Flesh and
Machine have 0 — they **cannot hack without a skill-chip**. Faithful to "digital
strength requires cyberware." This makes the **skill-chip** (transferable,
capped-low, take-the-max vs character skill, §10) load-bearing — and it is
**🔭 not yet modeled** (no chip type exists).

---

## 2. The hack contest ✅

The core resolution — built in [`crates/sim/src/hack.rs`](../crates/sim/src/hack.rs)
+ `Battle::resolve_hack`:

```text
3d6 + avg(Hacking, channel)   vs   Firewall        channel = min(Link_a, Link_t)
```

- **The connection channel ◆.** A hack runs over the link *between* the two
  units, and that channel is only as fat as its **weaker endpoint** —
  `min(Link_attacker, Link_target)` (the bottleneck). The attacker's effective
  rating **averages** its Hacking with that channel, floored: `(Hacking + channel) / 2`
  (`hack_rating`). Skill and channel each carry half the weight — a master runner
  on a thin pipe is dragged down but not gutted.
- **Skill attacks, the stat defends.** The TN is the target's **Firewall, in
  full** — **Link-blind on defense.** The target's Link enters the *attack* (the
  channel), never the wall, so a **darker target is harder to hack** (thin
  channel) while a **juicy high-Link target is easier** (its exposure literally
  widens the attacker's pipe). *(The earlier `min(Link, Firewall)` softened the
  wall for low-Link units — backwards — and was dropped.)*
- **Equipment arms the roll through the stats, not a separate term** ◆ — a
  cyberdeck raises **Link**, a skill-chip raises **Hacking**, a Firewall implant
  raises **Firewall**. So `resolve_contest`'s `equipment` addend is `0` for hacks.
- **Hard reachability gates** (§7D/§7F): zero-Link **target** ⇒ `NoSurface`
  (immune); zero-Link **attacker** ⇒ `Offline`. The locked immunity cliff —
  distinct from "Link affecting the math."
- **Margin = degree of success.** `≥ TN` succeeds; **nat 18 crit**, **nat 3
  fumble**. The margin scales the payload: `stacks = base + margin / MARGIN_PER_STACK
  + crit` (placeholder `MARGIN_PER_STACK = 3`).

**Emergent identity ◆ — netrunners are glass cannons.** Skill is the *consistent*
buy (half-weight, can't be denied); **Link is situational** — your bandwidth only
widens the channel **up to the other endpoint's Link**, so it pays off most
against **connected** enemies (and for winning init). And high Link is, by design,
the most exposed state — a high-Link unit acts sooner, hacks connected enemies
hard, *and* is the easiest to hack back. "Loud = capable but exposed" carried by
one shared number. This *is* the `Null` archetype; the model produces it for free.

### The digital pass ✅

`Battle::digital_phase` runs after the physical action phase: every unit with a
hack loadout and `Link > 0` acts in **Link (digital-initiative) order**, hacking
the nearest reachable enemy (Link > 0) within its antenna **range**. A
Crash/Seizure **stun** freezes the net action too.

> **🔭 Planned:** the design's *fully interleaved* physical + digital initiative
> (one woven order, §10.3). Today they are two discrete phases — a deliberate
> first pass.

---

## 3. What a hack *does* — the payload 🔭

The **contest** is built; the **consequences** are mostly stubs. On success a
hack lands a **status payload** (margin-scaled stacks). Today the only digital
payloads are **Lockware** (an Internal DoT, `Resist::Firewall`), **Crash**
(Seizure-style stun), and **Lag**. The designed payload menu (§7F/§10.8) needs
substrate that doesn't exist yet:

| Payload | What it does | Needs (🔭) |
|---|---|---|
| **Trip a hack-effect** | fire the target implant's loaded liability **on its owner** (Seizure/Misfire/Shed/Overload/Lockout/Blind/Overdose) | the **implant model** (§4) |
| **Deploy a worm** | plant a spreading, re-rolling contagion strain | the **Worm contagion** family |
| **Spoof IFF** | Flip-hostile / Masquerade / Scramble / Ghost | an **IFF / targeting** layer |
| **Disable an implant** | knock a slot **Offline** | **equipment-condition** state |

**Severity scales with the roll ◆** (designed in [`cyberware.md`](cyberware.md)
§6). A breach is not one thing: a plain success **disables** the target implant
(the floor — it just goes Offline), the **margin** magnifies its hack-effect, and
a **crit** delivers the **knockout** (the stun class). So today's margin→stacks
scaling is the *magnification* half; the disable floor and the crit-gated knockout
arrive with the implant model (Phase C). The upshot — netrunning is reliable
**attrition**, not a reliable hard-disable.

---

## 4. The implant model 🔭 — the keystone

> **Designed in full in [`cyberware.md`](cyberware.md)** — the benefit ↔ liability
> symmetry, the value equation, slots/PAN, breach & repair, and the sim data
> model + build order. The summary below is the netrunning-facing view.

The single highest-leverage unbuilt piece: it turns hacks from "land a DoT" into
the **chrome-is-liability** core, and simultaneously gives **worms** their
payloads.

- **Every implant = a `(Link, Firewall, Hack-effect)` bundle** (§7F). An implant's
  Link/Firewall **sum into** the unit's stats; its **hack-effect** is a benefit
  the owner uses **and** a loaded liability that fires *on the owner* when the
  implant is breached (by a hack or a worm).
- **Hack-effect roster** (the implant liability pool) — each maps to an existing
  or new status:

  | Implant (benefit) | Hack-effect (on owner) | Maps to |
  |---|---|---|
  | Reflex booster (+Init) | **Seizure** | Crash (stun) ✅ |
  | Smartgun (IFF-target) | **Misfire** | attack an ally/self 🔭 |
  | Subdermal plating (+def) | **Shed** | Plating-shred ✅ (`corrode`) |
  | Metabolic pump (+regen) | **Overload** | Internal DoT ✅ (`lockware`-like) |
  | Cyberdeck (+Link/hacks) | **Lockout** | −Link / digital disable 🔭 |
  | Sensor suite (perception) | **Blind** | can't target / off-AR 🔭 |
  | Combat stim (+dmg/haste) | **Overdose** | self-DoT then Crash 🔭 |

- **PAN & Cascade** (§6 of the delta): implants are networked over a **PAN**; a
  breach can ride it to trip **every** hack-effect at once (**Cascade**).
  **Segmented PAN** contains it (no cross-implant synergy). A build commitment,
  not a toggle.
- **Worms trip these by name** (Logic-bomb) **or all at once** (Cascade) — so the
  worm roster and the implant roster are designed together.

---

## 5. Defense & counters 🔭

The answer-half. *Skills attack, stats defend*, so defense is mostly **stats +
loadout**, not classes.

| Counter | What | Status |
|---|---|---|
| **Firewall** | the digital TN — raise it with implants | ✅ (as TN) |
| **Go dark / zero Link** | total digital immunity, total digital isolation | ✅ (the gate) |
| **Masking (low Link)** | smaller surface ⇒ harder to hack / lower worm-catch, less throughput | 🔭 (link-effect) |
| **White-hat mender** | cleanse Worm; restore Firewall / Link | 🔭 |
| **EMP** | a **physical** attack that hits Link/cyberware and **bypasses Firewall — no hack roll** (a pulse, not a contest); the counter to digital builds | 🔭 |
| **Anti-Worm specialists** | Antivirus (eat stacks), Signals (lock/reverse IFF), Jammer, Honeypot, Quarantine | 🔭 |

**Link-effects** (the Link slot's flavor, §7F): Uplink / Relay·Mesh / Masking /
Spike / Leech — loadout choices that shape the Link number and its exposure. 🔭

---

## 6. Calibration — first pass ◆

**`Link` is `i32` ✅** — bandwidth tiers, migrated from `f32`.

**The even-odds anchor ◆.** 3d6 is symmetric about 10.5, so `P(3d6 ≥ 11) = 0.5`
*exactly*. ⇒ **Firewall 11 is the baseline:** a runner with no net advantage
(`avg = 0`) cracks it on a coin-flip, and every point of wall above 11 must be
bought back by the attack additive. The matched-contest line is the clean integer

```text
avg(Hacking, channel) = Firewall − 11
```

**First-cut bands ◆ (TBD):**

- **Firewall** (the TN) — soft **8–10** · even-odds ref **11** · standard **12–14**
  · hardened **15–16** · bulwark **17+**.
- **Link** (tiers) — 0 dark · 1–2 low · 3–4 mid · 5–6 high · 7–8 max.
- **Hacking** — 0 none · 1–2 chip floor · 3–4 competent · 5–6 pro · 7–8 master.

The additive `avg(Hacking, channel)` lands **0–8**. Even-odds additive per wall:
`11→0 · 13→2 · 15→4 · 17→6 · 19→8` — soft walls (8–10) sit below the floor, so a
net-even runner already beats them. A mid runner (Hacking 4, Link 5) rolls over
soft targets, is favored vs standard (13), a coin-flip vs hardened (15), and an
underdog vs a bulwark (17+).

**Link gates *depth* as well as reach ◆.** Because the additive *averages* Hacking
with the channel (`min` of the two Links), a target's Link caps how much skill can
be brought against it: against a **dark** target (Link 1) even a master is held to
`avg(skill, 1) ≈ skill/2`. So **Firewall is the hit-gate, Link is the depth-gate**
— a soft-but-dark mook (low Firewall *and* low Link) is **easy to land but shallow**
(small margin ⇒ few payload stacks, little to own), and going dark defends against
*skill*, not just reach. The two dials give a clean 2×2 of target identities:

| | dark (low Link) | loud (high Link) |
|---|---|---|
| **soft** (low FW) | easy, shallow — *mook* | easy, deep — *juicy* |
| **hard** (high FW) | hard, shallow — *bunker* | hard but deep if cracked — *fortress* |

**Hacking is hard by construction ◆.** Landing a hack is only the *floor* — it
**disables** the implant. The two outcomes that *matter* are gated: the
**magnified liability** needs a strong **margin**, and the **knockout** (the stun
class) needs a **crit** (cyberware §6). So even when a hack lands, the severe
results are rare, and meaningful targets sit at/above the even-odds wall (11+).
Netrunning rewards the **invested specialist against an exposed target**, not the
dabbler — soft mooks are easy to poke but shallow (the depth-gate above).

**Still open ⏳:**

| Knob | Question |
|---|---|
| **`MARGIN_PER_STACK`** (=3) | the margin→stacks curve; `base_stacks`; per-payload stack caps. |
| **Knockout gate** | crit-only is **nat 18** (~0.5%) — likely too rare; widen to a **margin ≥ K** "decisive" tier (a strong runner overpowering a soft/exposed target)? |
| **Antenna range** | reach bands for the digital pass; beam (line) vs single delivery. |
| **Hack-effect severity** | how punishing each tripped liability is — the "chrome is a real-but-fair gamble" dial. |

---

## 7. Build order — the plan ◆

The road from "the contest works" to "the digital realm is whole":

1. ~~**Calibration + `Link → i32`**~~ ✅ — done (§6): Link is `i32`, Firewall 11
   is the even-odds baseline, first-cut bands set.
2. **Implant model → hack-effect roster** 🔭 — the keystone (§4); gives hacks teeth
   and worms their payloads. *(Delta Phase 6/7.)*
3. **Equipment-condition** (Online→Degraded→Offline→Destroyed) 🔭 — what "disable"
   and "field repair" act on. *(Phase 2/6.)*
4. **IFF / spoof** 🔭 — the targeting layer + the spoof toolkit. *(Phase 5/8.)*
5. **Worm contagion** 🔭 — the deploy-worm payload + spread channel. *(Phase 7/8.)*
6. **EMP · White-hat · Link-effects · PAN/AR** 🔭 — the counters and the
   intra/perception tiers. *(Phase 7/8.)*

None of it breaks the crate split: the `sim` owns resolution; factions/economy
stay engine-blind.
