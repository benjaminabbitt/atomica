# CHROME AND CODE — Design Reference

*Async auto-battler / roguelike-deckbuilder · cyberpunk, neo-Japan cyber-samurai. Names are placeholder. This is the at-a-glance spec; the full design doc (`status-effects-taxonomy.md`) holds the reasoning, the survey of precedents, and the open-question detail. **Numbers throughout are TBD** — this fixes the *shapes*, not the values.*

> **Post-v0.25 synthesis** (three-axis identity & factions, the mender triad + cross-pool healing, equipment-condition, morale/Resolve, vehicles, Rep/politics, **PAN** & **AR**, archetype roster) lives in [`docs/design-delta-v0.26.md`](docs/design-delta-v0.26.md).

---

## The spine — two realms

Every system mirrors across **two realms**, and the asymmetry between them *is* the game:

- **Physical** — **dumb & honest.** Attacks hit the hexes they're aimed at, friend or foe; nothing can be spoofed; you defend by armor and positioning.
- **Digital / Code** — **smart & corruptible.** Effects read friend/foe (IFF), target selectively, and reach through a unit's **Link**; you defend by ICE, going dark, and counter-programs — but all of it can be **hacked**.

Three recurring throughlines:
1. **"Loud = capable but exposed."** Exposure axes — **Link** (digital) and **clustering/position** (physical) — each buys power and costs safety. *(A thermal **Heat** axis was the third; dropped for now — see below.)*
2. **Chrome-is-liability.** More cyberware = more power *and* more attack surfaces (hackable, EMP-able, runs hot).
3. **You program your units; the enemy hacks your script.** Behavior is code, so the digital layer attacks *minds*, not just health.

## Core loop

Draft units (**chassis**) → equip them in the shop (**weapons + augments + software + behavior profiles**) → battles **auto-resolve** on a hex board per each unit's scripted profiles. Roguelike run structure; cyberware/augments are the major purchasable.

## Board

Flat-top **hex**. Columns = depth ranks; the **long edge = frontage**. Two boards meet at a vertical **seam** with a randomized **±½-hex offset** (item-controllable — sets cross-board front-line pairings). Center column = degree-6 hub; **corners safest**.

---

## Unit anatomy

| Stat | What it is |
|---|---|
| **Integrity** | the single HP pool — *all* damage ultimately reduces it |
| **Defense layers** | **Barrier/Shield** (External) → **Plating/Armor** (External/Contact; materials Composite / Reactive / Ablative) → **Integrity** (Internal). Pierce = "drop a tier." |
| **Physical Initiative** | turn order in the world; lowered by equipment **weight** + multitasking |
| **Digital Initiative = Link** | turn order on the net (the two tracks **interleave into one woven order**) |
| **Link** | net presence — digital Initiative + action throughput + Worm exposure + hack surface. Continuous, **equipment-set**; **zero Link = immune to all digital attack** |
| **Body** | one of the three primary attributes — carries HP/Integrity, melee damage, **and** biological resilience (resist vs **Virus** + bio afflictions). No separate Immunity stat. |
| **ICE** | resist vs **Worm** + hacks (digital). An attacker's icebreaker **breaks** it; a worm **melts** it. |
| ~~**Heat**~~ *(dropped for now)* | thermal layer — cut from scope, see below |

## Damage model

- **Penetration tier** (which defense applies): **Internal** (bypasses Barrier + Plating) / **Contact** (Plating mitigates) / **External** (hits Barrier).
- **Armor matrix** (3-tier): **Bludgeoning** (anti-rigid — beats Plate, soaked by Padding) / **Piercing** (universal penetrator — only Plate resists) / **Slashing** (anti-unarmored) vs **Padding / Mail / Plate**.
- **Magnitude:** flat · %max (anti-tank, can kill) · %current (softener, never kills).
- **Behavior:** deterministic · **stochastic** (rolled each tick; the resist shifts the roll).
- **Types ≠ statuses:** damage type governs *mitigation only*; statuses are a separate à-la-carte pool any weapon can apply.

## Status effects — the 9-axis schema

Every status = one value per axis: **effect · trigger · timing · decay · stacking · magnitude · behavior · targeting · resist.** Roster families:

| Family | Members |
|---|---|
| **Contagions** | Virus, Worm (see below) |
| **DoTs** | Burn (Contact) · Bleed (Internal) · Poison (Internal, stochastic, no spread) · Corrode (+Plating-shred) |
| **Control** | Crash (stun) · Lag (slow) · Lock (root) |
| **Amplify** | Breach (vuln) · Mark |
| **Strip** | ICE-strip · Body-sap · Plating-shred · Barrier-break |
| **Buffs** | Adaptive System (regen) · Vaccinated/Antimalware (cleanse) · Overclock (haste) · Hardening · Uplink/Spike |
| **Death-triggered** | Detonate · Legacy · Data-spill |

---

## The two contagions *(centerpiece)*

Both **Internal**, **stochastic**, **build over time**, each a **keyword family** (flagship = the spreading strain). Counterplay is **orthogonal**.

| | **Virus** (bio) | **Worm** (code) |
|---|---|---|
| **Spreads by** | hex **adjacency** (positional) | **interaction + ambient proximity** (Link-gated) |
| **Resisted by** | **Body** | **ICE** + anti-Worm specialists |
| **Infects** | flesh (organic + augmented); machines immune | any unit with **Link > 0** |
| **Cleanse →** | **Vaccinated** (decaying resist buff) | **Antimalware** |
| **Counterplay** | high Body + cleanse + **spread out** | ICE + **go low/zero Link** |
| **Strains** | Plague · Necrosis · Paralysis · Wasting · Delirium · Blight · Spore | Spreader · Glitch · Lockware · Logic-bomb · Cascade · Drainware · Spoofer |

- **Augmented units are double-exposed** (flesh *and* Link → both families).
- The **Worm reaches via Link** by 3 vectors (runner / interaction / proximity); it **melts ICE** as it builds. **Worms can trip cyberware hack-effects** by name (Logic-bomb) or all at once (Cascade).
- Signature trap: actions **lock at planning** (commit-time), so you can't perfectly dodge a Worm. **Low-Link melee is the safe carrier-killer.**
- **Corruption asymmetry.** Digital corruption **snowballs**: a Worm **melts ICE**, and thinner ICE lets the next Worm bite deeper — a runaway loop. Bio **splits** into two flavors: **Virus wastes** — it attacks **Body**, a chipping disease that drags HP/Integrity down (Body *is* the HP pool) and softens bio-resist for the next strain, all atop its fever DoT — while **Poison burns** — a flat **pure DoT**, resisted by Body, that touches no stat at all.
- Each strain needs ≥1 hard **runaway brake**; on death a carrier either **spills** stacks (Data-spill) or **purges** them.

## Augmentation economy

Both branches **enhance the physical body** — the difference is the **digital surface**.

| | **Cyberware** (chrome) | **Bioware** (bio-augment) |
|---|---|---|
| Profile | physical benefit **+ (Link, ICE, Hack-effect)** | physical benefit, **no Link** |
| Digital exposure | **hackable** (worms, hacks, hack-effect liability) | **none** — unhackable |
| EMP | **vulnerable** (hardware fries) | **immune** (no hardware) |
| Flesh exposure | host flesh still Virus-vulnerable | Virus-vulnerable + a **Reject** liability |
| Plays | **wired / out-tech** | **air-gapped / abstain** |

**Digital strength requires cyberware** (you can't be powerful on the net without exposure); physical strength comes either way. (Flesh resilience against bio still comes from **Body** regardless of branch.)

## Cyberware & netrunning *(the digital economy)*

- Every implant = a **(Link, ICE, Hack-effect)** bundle. The **hack-effect is a loaded liability** — when an enemy breaches the implant, it fires **on the owner**. Pool: Seizure · Misfire · Shed · Overload · Lockout · Blind · Overdose.
- **Netrunners** attack cyberware **via its Link, against its ICE** — their icebreaker **breaks** it → disable it, deploy a Worm, or trip its hack-effect.
- **IFF** (friend/foe) makes digital effects selective — but **spoofable** (Flip-hostile · Masquerade · Scramble · Ghost). **Code-only**: physical weapons aren't IFF-gated (the Smartgun mod is the one exception).
- **Link-effects** (the Link slot's flavor): Uplink · Relay/Mesh · Masking · Spike · Leech.
- **Inbuilt equipment**: some units ship with integral (often non-removable) chrome — a unit-identity source.

## Anti-Worm defense

The answer-half — mostly **loadouts/programs**, not classes. Two philosophies: **abstain** (flesh, low-Link) vs **out-tech** (code).

| Specialist | Counters |
|---|---|
| **Air-gapped blade** | the whole digital realm, by abstaining; safe carrier-killer |
| **Antivirus** | eats Worm stacks (enemy-side suppression) |
| **Patcher** | cleanses allies → Vaccinated / Antimalware |
| **Bulwark** | high ICE, extended to allies |
| **Signals officer** | locks IFF / reveals & reverses spoofs |
| **Jammer** | dead-zone — blocks beams, proximity, runner range |
| **Honeypot** | bait that draws and traps hacks |
| **Quarantine** | severs an infected ally's Link |

*Every counter costs a slot/unit/tempo — the arms race is the meta.* Cascade has no hard counter but **lean chrome** (by design).

## Weapons

**Guns (ranged):** Pistol · SMG · Rifle · Sniper/Railgun (armor-ignoring) · Shotgun (blast) · Heavy/LMG (Bludgeoning) · Energy/Plasma (Thermal + Burn) · Launcher (grenade platform).
**Melee (close — the air-gapped weapon):** Mono-katana · Monomolecular wire (armor-ignoring) · Vibro-blade (armor-soften) · War-maul (Bludgeoning) · Naginata (+reach) · Twin-blades (volume) · Shock-fist (Crash).
**Gun mods:** **Smartgun** (flagship — IFF auto-target → unlocks smart targeting profiles; **fires on digital Initiative**; lock-on; spoofable → Misfire) · EMP rounds · AP · suppressor.
**EMP:** *physical* attacks that fry **Link / antennae / cyberware**, **bypassing ICE** — the physical counter to digital builds.

**Delivery footprints:** single · **blast** (explosion, scales by **radius** — footprint 1 = 7 hexes, footprint 2 = 19) · **beam** (antenna, scales by **width** — footprint 1 = line, footprint 2 = 3-wide). **Friendly fire is on.**

**Attack matrix:**

| Attacker ↓ / Target → | **Physical** (Integrity, armor) | **Digital** (Link, cyberware) |
|---|---|---|
| **Physical** | guns / melee — vs armor | **EMP** — *ignores ICE* |
| **Digital** | worm / virus DoT (Internal) | hacks / worms — vs ICE |

## Units — classes & behavior

**Class = chassis** (innate); **role = loadout** (built in the shop).

| Chassis | Contagion exposure | Link floor |
|---|---|---|
| **Flesh** (organic) | Virus only | low/zero |
| **Augmented** (cyborg) | both — double-exposed | mid–high |
| **Machine** (drone) | Virus-immune; Worm only if Link > 0 | varies |

Units follow **scripted profiles** (auto-resolve), and **move only if not boxed in**:
- **Movement:** Advance · Hold · Kite/Retreat · Flank · Swarm · Disperse.
- **Targeting:** Nearest · Lowest-Integrity · Highest-threat · Backline/role · Weakest-armor.

Profiles are **code → hackable**: spoofs/Lockware/worms corrupt *behavior*, not just stats.

## Heat *(dropped for now)*

**Cut from scope** ◆ — parked, not deleted; revisit if a thermal axis is ever wanted.
The design (kept for the record): a gauge that builds from overclocking, energy/EMP
weapons, and heavy chrome, doing three things — **upside** (overclock + heat-scaling
abilities), **exposure** (thermal signature → more detectable), **risk** (overheat →
throttle / shutdown / Integrity); counterplay vent · coolant · play cool. It only ever
earned a slot as the *full* layer; as bare overclock it didn't — and for now it's out.

---

## Status of the design

**Locked (shapes):** the two realms; board; stat line (Integrity / layered defense / two interwoven Initiative tracks / Link / **Body**-as-bio-resist / ICE); the three primary attributes (Body / Dexterity / Intellect); penetration tiers + armor matrix; types ≠ statuses; the 9-axis status schema + 7 families; both contagion families; cyberware (Link/ICE/Hack-effect) + netrunning + IFF/spoof; anti-Worm specialists; bioware-vs-cyberware; weapons (guns/melee/EMP) + footprints + attack matrix; gun mods (Smartgun); chassis classes; movement + targeting profiles.

**Open (mostly numbers + roster fills):**
- All **values** — magnitudes, durations, caps, roll odds, ranges, weights, costs.
- ~~**Heat** in or out~~ — **resolved: dropped for now** (cut from scope; revisit later).
- Specific **bioware roster**; **gun-mod roster**; **chassis stat lines** + Link floors.
- **Resolution math** — netrunner hack-power vs ICE; spoof resolution; Worm catch odds.
- Which **Link-effects / movement profiles / targeting profiles** ship.
- **Runaway brakes** per contagion; death spill-vs-purge per strain.
- **Faction layer** (neo-Japan clans / dueling / honor) — aesthetic or mechanical.
