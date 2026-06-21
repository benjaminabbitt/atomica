# Netrunning — system design & plan

*The digital-attack system: how hacks, worms, and spoofs resolve. Extends
[`../CHROME-AND-CODE.md`](../CHROME-AND-CODE.md) §7F and
[`design-delta-v0.26.md`](design-delta-v0.26.md) §13, on the core **2d10 roll-under**
([`stats.md`](stats.md)).
This doc fixes the **netrunning shapes** and records the resolution decisions; it
is the at-a-glance reference for the digital realm. **Numbers are TBD.** ◆ =
decision/synthesis. Status legend: **✅ built** (lives in `crates/sim`) ·
**🔭 planned** (designed, not implemented) · **⏳ tuning** (a feel number).*

---

## 1. The stat & skill line

A unit's whole netrunning profile is **two stats + one skill** (plus the bio
parallel). *Skills attack, stats defend* (§13) — so the offense is a **skill**
(Hacking) and the defenses are **stats** (Link gates, ICE walls).

| Name | Field | Type | Role | Status |
|---|---|---|---|---|
| **Link** | `unit.link` | int◆ | **Four jobs:** ① reachability **gate** both ways (`0` ⇒ immune target / offline attacker); ② **latency → digital initiative** (your own Link orders the digital pass; high = sooner); ③ the **connection channel** (a hack's bandwidth is the *weaker* endpoint's Link, `min`); ④ **antenna range** (`Unit::hack_reach` — a loud, high-Link runner projects far; a dark one barely reaches). The exposure dial. | ✅ (gate/init/channel/range); 🔭 exposure (worm-catch) |
| **ICE** | `unit.ice` | int | The **digital defense** — Intrusion Countermeasures Electronics rolls an *active defense* against a hack (opposed, [`stats.md`](stats.md) §4); also the resist for digital status gates (Crash/Lag/Lockware via `Resist::ICE`, §5). An attacker's **icebreaker breaks** it. **Link-blind** on defense. | ✅ |
| **Hacking** | `unit.skills[Hacking]` | tier | The **offensive skill** (a tier on Intellect, `stats.md` §3). No defensive net-skill exists — you buy ICE (the stat), not a skill. | ✅ |
| *Body (bio-resist)* | `unit.body` | int | The **bio** parallel — biological afflictions (Virus, poison, plague, organic toxins) resist against the **Body** attribute itself, not a separate stat. The bio track, not digital. | ✅ |

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
runner:    2d10 ≤ avg(effective Hacking, channel)     channel = min(Link_a, Link_t)
ICE:       2d10 ≤ ICE                                 (active defense; ≤ 0 = undefended)
breach lands = runner succeeds AND ICE fails
```

- **The connection channel ◆.** A hack runs over the link *between* the two
  units, and that channel is only as fat as its **weaker endpoint** —
  `min(Link_attacker, Link_target)` (the bottleneck). The attacker's effective
  rating **averages** its **effective Hacking** (Intellect + tier) with that channel,
  floored: `(hacking + channel) / 2`
  (`hack_rating`). Skill and channel each carry half the weight — a master runner
  on a thin pipe is dragged down but not gutted.
- **Skill attacks, the stat defends — as an opposed roll** ◆ (`stats.md` §4, the
  same shape as combat). The target's **net defense** rolls back: the breach lands
  only if the runner connects **and** the defense fails, so a stiffer defense
  **defends more often** (not "subtracts more"). An **undefended** surface (defense
  ≤ 0) skips the defense roll. **Link-blind on defense** — the target's Link enters
  the *attack* (the channel), never the wall, so a **darker target is harder to
  hack** while a **juicy high-Link target is easier**.
- **Active net defense ✅ — runners parry, and cover nodes** ◆ (`Battle::net_defense`).
  The defense is the **highest** of: the target's passive **ICE**; its own
  **Hacking**, if the target is itself a runner (it parries code with code); and the
  **Hacking of any allied runner covering it** — a living ally with a deck whose
  antenna reach spans the target. So netrunners are hard to hack (they defend at
  skill), and a runner can **actively defend a node it controls** — the live ICE on a
  [`Datamine`] vault. Kill the guarding runner and the node drops to its own wall.
- **Equipment arms the roll through the stats, not a separate term** ◆ — a
  cyberdeck raises **Link**, a skill-chip raises **Hacking**, an ICE implant
  raises **ICE**. There is no separate roll term — the stats *are* the contest
  (`resolve_opposed(rating, ICE)`).
- **Hard reachability gates** (§7D/§7F): zero-Link **target** ⇒ `NoSurface`
  (immune); zero-Link **attacker** ⇒ `Offline`. The locked immunity cliff —
  distinct from "Link affecting the math."
- **Margin = degree of success.** The breach lands when the runner rolls under
  rating **and** the ICE fails; a natural **2–3 crits**, a natural **19–20
  fumbles** the runner's leg ([`stats.md`](stats.md) §1). The runner's margin scales
  the payload: `stacks = base + margin / MARGIN_PER_STACK + crit` (placeholder
  `MARGIN_PER_STACK = 3`).

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

## 3. What a hack *does* — the payload

The **contest** is built, and (Phase C ✅) a success now **breaches a target
implant** via the severity ladder (`cyberware.md` §6) — **disable** → margin
**degrade** → crit **knockout** — firing the implant's `hack_effects`. Against a
target with **no chrome to trip**, it falls back to landing the deck's own
**payload** (Lockware). On top of the breach, the runner's **loaded programs** ride.

**Programs are a deck loadout ✅ (`Program` / `Unit::programs`).** A program is the
runner's software — a roster a fixer / Halcyon Cybernetics vendors. They live on the
*unit* (`programs`), and **loading one requires a cyberdeck** (`Unit::install_program`
is a no-op without a `Cyberdeck`-tagged rig — no deck, no programs). The resolver
(`run_programs`, §10.8) routes each by *when* it fires. Every offensive rider is
**gated to keep the §6 disable floor**: a *marginal* crack (margin 0) just disables;
a program needs a **solid** breach (margin ≥ one [`MARGIN_PER_STACK`]) to deploy —
except **Crash**, which is **crit-gated** like every knockout. The full roster is
built and tested:

| Program | Class | What it does | Gate |
|---|---|---|---|
| **Lockware** | payload | the generic lockout drain — an Internal DoT on a target with **no chrome** to exploit (the bread-and-butter) | success (no-chrome) |
| **Overheat** | rider (damage) | cooks the target's **heat-prone** (`EquipmentTag::HeatProne`) chrome — an Internal DoT scaling with the margin. Useless against a target running cool | solid + heat-prone |
| **Meltdown** | rider (damage) | a **heavy** burn (Internal DoT, far harder than Overheat) — the finisher | solid + heat-prone |
| **Crash** | rider (control) | a digital **stun** | **crit** |
| **Lag** | rider (control) | a `Slow` on the target's initiative | solid |
| **Breach** | rider (soften) | a **vulnerability** — incoming damage amped, so the squad's blows bite harder | solid |
| **Blind** | rider (soften) | a **Dexterity** debuff — its shots go wide | solid |
| **Decrypt** | rider (soften) | **melts** the target's **ICE** (a `Worm`-tagged corruption — cleansable) | solid |
| **Leech** | rider (soften) | drains the target's **Link** — thinner channel, slower digital initiative, edges toward dark | solid |
| **Worm** | rider (spread) | deploys a **contagious** ICE-melt (`Corruption::worm_swarm`) that rides the net to nearby surfaces | solid |
| **Cascade** | breach modifier | forces a **meshed** target's breach to trip **every** implant (not just on a crit) | solid + meshed |
| **Logicbomb** | breach modifier | force-fires the tripped chrome's degrade liabilities **past the disable floor** | success (chrome) |
| **Spoof** | rider (hijack) | corrupts the target's **targeting** script (`CORRUPTION` override) — it chases the wrong enemy | solid |
| **Misfire** | rider (hijack) | corrupts the target's **movement** routine — it backs off / scatters for a beat | solid |
| **Honeypot** | defensive | counter-ICE: a **repelled** intruder (a hack that fails) gets its deck fried (a lockout DoT) | on a failed hack vs the owner |
| **Ghost** | defensive | a stealth suite — the owner reads **darker**, a flat bonus to its `net_defense` | passive |
| **Antivirus** | defensive | a standing ward — strips **worm** corruption off the owner each tick (the `ward_phase`) | passive (per tick) |

Two **intrinsic** breach effects are always there, program or no:

| Effect | What it does | Status |
|---|---|---|
| **Trip a hack-effect** | breach an implant → fire its liability **on the owner**, by the severity ladder | ✅ built (`apply_breach`) |
| **Disable an implant** | knock a slot **Offline** (the ladder's floor) | ✅ built (`disable_implant`) |

**The netrunning doctrine ✅ (`NetDoctrine` / `Unit::with_doctrine`).** A loadout is only
half the runner — the other half is *how it's flown*. The **doctrine** is the digital
analog of the physical behavior profiles (`TargetingProfile` / `MovementProfile`): the
**code** the player programs into a deck, scripting **both** who the runner dives and which
program it leads with. Like the other profiles it's a behavior `Override`, so a hostile
**Spoof** (`CORRUPTION` priority) can corrupt a runner's *doctrine* too — the enemy hacks
your script. Each doctrine pairs a **target lean** (`hack_target_lean`) with a **rider
priority** (`rider_order`); on a breach the resolver deploys the **single** best-fit
applicable program (`run_programs` → `deploy_rider`) — a toolkit you pick from, not a suite
you dump — falling through the loadout when the lead doesn't fit. An **objective** focus (the
Datamine node) always outranks the doctrine's own lean.

| Doctrine | Dives | Leads with |
|---|---|---|
| **Disabler** (default) | the most-chromed enemy (more slots to trip) | Crash → Lag → Breach |
| **Burner** | **heat-prone** chrome (its burns bite there) | Meltdown → Overheat |
| **Saboteur** | the biggest gun | Breach → Decrypt → Worm → Blind |
| **Controller** | the biggest gun | Spoof → Misfire → Lag |
| **Defender** | enemy **runners** (kill the active defense) | Crash → Lag → Breach |

**A netrunning objective ✅ — the [`Datamine`] dive.** Beyond shooting: an
encounter can task the squad to **breach a bolted-down data node** (crack its
implant) rather than wipe the field. The runner is **objective-aware** (it
prioritizes hacking the node over poking grunts), the squad **won't slag the node**
it means to crack (an unarmed enemy on the objective hex is spared weapon-fire), and
a guarding netrunner **defends it actively** (above). The play: clear the ICE, then
crack the vault — a self-contained showcase of the digital realm.

**Severity scales with the roll ✅** (`cyberware.md` §6). A breach is not one
thing: a plain success **disables** the target implant (the floor — it goes
Offline), the **margin** magnifies its degrade-class hack-effects, and a **crit**
delivers the **knockout** (the stun class). The upshot — netrunning is reliable
**attrition**, not a reliable hard-disable.

---

## 4. The implant model 🔭 — the keystone

> **Designed in full in [`cyberware.md`](cyberware.md)** — the benefit ↔ liability
> symmetry, the value equation, slots/PAN, breach & repair, and the sim data
> model + build order. The summary below is the netrunning-facing view.

The single highest-leverage unbuilt piece: it turns hacks from "land a DoT" into
the **chrome-is-liability** core, and simultaneously gives **worms** their
payloads.

- **Most implants = a `(Link, ICE, Hack-effect)` bundle** (§7F). An implant's
  Link/ICE **sum into** the unit's stats; its **hack-effect** is a benefit
  the owner uses **and** a loaded liability that fires *on the owner* when the
  implant is breached (by a hack or a worm). **Exception ✅:** **inert physical
  cyberware** (subdermal plating) has no digital surface — Link 0, **no hack-effect**,
  and **unbreachable** by hack / worm / EMP (`Implant::is_digital` gates every vector).
- **Hack-effect roster** (the implant liability pool) — each maps to an existing
  or new status:

  | Implant (benefit) | Hack-effect (on owner) | Maps to |
  |---|---|---|
  | Reflex booster (+Init) | **Seizure** | Crash (stun) ✅ |
  | Smartgun (IFF-target) | **Misfire** | attack an ally/self 🔭 |
  | Subdermal plating (+def) | **— (none)** | inert physical ✅ — unbreachable |
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
| **ICE** | the digital defense roll — raise it with implants | ✅ |
| **Go dark / zero Link** | total digital immunity, total digital isolation | ✅ (the gate) |
| **Masking (low Link)** | smaller surface ⇒ harder to hack / lower worm-catch, less throughput | 🔭 (link-effect) |
| **White-hat mender** | cleanse Worm; restore ICE / Link | 🔭 |
| **EMP** | a **physical** attack that hits Link/cyberware and **bypasses ICE — no hack roll** (a pulse, not a contest); the counter to digital builds | ✅ (`cyberware.md` §6) |
| **Anti-Worm specialists** | Antivirus (eat stacks), Signals (lock/reverse IFF), Jammer, Honeypot, Quarantine | 🔭 |

**Link-effects** (the Link slot's flavor, §7F): Uplink / Relay·Mesh / Masking /
Spike / Leech — loadout choices that shape the Link number and its exposure. 🔭

---

## 6. Calibration — first pass ◆

**`Link` is `i32` ✅** — bandwidth tiers, migrated from `f32`.

**The anchor ◆.** The hack is an **opposed roll** (§2, `stats.md` §4). The runner's
leg is **2d10 roll-under** its rating (~even at **10–11**); the ICE rolls its
own **defense** at `2d10 ≤ ICE`. The breach lands when the runner connects
**and** the wall fails:

```text
P(breach) = P(2d10 ≤ rating) × P(2d10 > ICE)     rating = avg(effective Hacking, channel)
```

So ICE is a **probabilistic defense**: every point raises the chance it repels
the breach, but it can't make the target *unhittable* (the runner's own leg caps it
near the 2d10 ceiling). A pro runner (rating ~9–10) lands **~30%** through a modest
wall — viable, not free.

**First-cut bands ◆ (TBD), on the 2d10 scale:**

- **ICE** (defense roll — its chance to repel) — unprotected **0** (none) ·
  modest **4** (≈10%) · standard **6** (≈16%) · hardened **8** (≈26%) · bulwark
  **10+** (≈45%+).
- **effective Hacking** (Intellect + tier) — chip floor ~**10** · competent ~**12**
  · pro ~**14** · master ~**16**.
- **Link** (tiers) — 0 dark · 1–2 low · 3–4 mid · 5–6 high.

The **channel** = `min(Link_a, Link_t)` and the **rating** = `avg(effective
Hacking, channel)` lands ~**6–12** — a thin channel still drags a master down to a
mid rating. A pro runner (effective Hacking 14, channel 4 ⇒ rating 9) rolls over
unprotected chrome, is favored vs a modest wall (4), and an underdog vs a hardened
one (8+).

**Link gates *depth* as well as reach ◆.** Because the additive *averages* Hacking
with the channel (`min` of the two Links), a target's Link caps how much skill can
be brought against it: against a **dark** target (Link 1) even a master is held to
`avg(skill, 1) ≈ skill/2`. So **ICE is the hit-gate, Link is the depth-gate**
— a soft-but-dark mook (low ICE *and* low Link) is **easy to land but shallow**
(small margin ⇒ few payload stacks, little to own), and going dark defends against
*skill*, not just reach. The two dials give a clean 2×2 of target identities:

| | dark (low Link) | loud (high Link) |
|---|---|---|
| **soft** (low ICE) | easy, shallow — *mook* | easy, deep — *juicy* |
| **hard** (high ICE) | hard, shallow — *bunker* | hard but deep if cracked — *fortress* |

**Hacking is hard by construction ◆.** Landing a hack is only the *floor* — it
**disables** the implant. The two outcomes that *matter* are gated: the
**magnified liability** needs a strong **margin**, and the **knockout** (the stun
class) needs a **crit** (cyberware §6). So even when a hack lands, the severe
results are rare, and a stiff ICE repels a real fraction of breaches outright.
Netrunning rewards the **invested specialist against an exposed target**, not the
dabbler — soft mooks are easy to poke but shallow (the depth-gate above).

**Still open ⏳:**

| Knob | Question |
|---|---|
| **`MARGIN_PER_STACK`** (=3) | the margin→stacks curve; `base_stacks`; per-payload stack caps. |
| **Knockout gate** | crit-only is a **natural 2–3** (~3% on 2d10); widen to a **margin ≥ K** "decisive" tier (a strong runner overpowering a soft/exposed target)? |
| **Antenna range** | reach bands for the digital pass; beam (line) vs single delivery. |
| **Hack-effect severity** | how punishing each tripped liability is — the "chrome is a real-but-fair gamble" dial. |

---

## 7. Build order — the plan ◆

The road from "the contest works" to "the digital realm is whole":

1. ~~**Calibration + `Link → i32`**~~ ✅ — done (§6): Link is `i32`, ICE 11
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
