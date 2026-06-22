# Balance watch-list — emergent properties to monitor

*The model leans hard on **emergent** balance: the chassis rock-paper-scissors, the
build axes, and the dark/loud trades all fall out of the rules rather than being
hand-tuned ([`design-delta-v0.26.md`](design-delta-v0.26.md) §17, the three-realm
spine). That's a strength — but emergent properties drift, so the ones below are
**watch-items**, not settled numbers. Each names the property, why it's intended, the
**healthy band**, the **degenerate signal**, and the **dial** that moves it. Most can
only be confirmed once the sim/probe (`atomica-run probe`, [`stats.md`](stats.md) §7)
can report them against real content — they're recorded here so they aren't lost.*

---

## 1. Body overload — is "buy Body" the universal answer? ◆

**The property.** Folding the old Health/Immunity into **Body** ([`design-delta`](design-delta-v0.26.md)
§17) loaded one stat with **HP · melee damage · melee-hit reliability · bio-resist** — a
single Body-up implant fattens all four. The intended counterweight is **Dexterity**,
which carries the *active/finesse* half (to-hit, dodge, ranged, speed) — so the physical
realm is a **mass-vs-finesse** axis (durable bruiser vs fragile skirmisher), build-vs-build,
not a dominance.

**The seam.** Dex unloads Body **fully on the ranged path** (ranged damage is munition,
Body-independent) but only **partially on finesse-melee**: a `FINESSE` weapon moves the
*to-hit* to Dex, yet melee **damage** still takes the Body bump. So the clean zero-Body
build is the **gunner**; a Dex-duelist always keeps one foot in Body. Aim the monitoring
there.

| | |
|---|---|
| **Healthy** | Body-melee bruisers and Dex-gunners trade wins across content; neither is the default opener |
| **Degenerate** | "buy Body first" is universal; bruisers dominate win-rate / survival regardless of matchup |
| **Dial** | the `HP_PER_BODY` constant, the melee-damage Body-coefficient `k`, and how much `RANGED`/Evasion buys Dex builds back |

## 2. Machine-glass & the drone dark/loud equilibrium ◆

**The property.** Machines are **glass on the entire digital + mental side**: Nerve 0
(no Resolve → morale/intimidation-proof, but no composure → **most spoof-credulous**) and
high-Link (worm/EMP-exposed). This is **intended**, and it's **load-bearing**: machine-glass
is the *demand-generator* for the whole anti-digital toolkit (spoof, worm, jammer, EMP). If
nothing were juicy to hack, those tools wouldn't earn a roster slot and the digital realm
would be a dead system.

**The loop (mostly built).** Netrunners field drones; an enemy runner can spoof a drone's
IFF so it guns its owner's line (the *Hollowpoint* play). The drones' defense is the rigger's
own skill — but only via an **active, initiative-costed [Guard](netrunning.md)** (single
focus), so *threatening* the swarm taxes the rigger's tempo, and decapitating it drops every
drone to its own ≈zero wall. The drone-fielder's outs: **Guard** the key unit, raise drone
**ICE / segment the PAN**, or **air-gap** (immune, but AR-blind → dumb nearest-targeting).

| | |
|---|---|
| **Healthy** | drones expose often enough that counter-runners earn their slot, rarely enough that drone-builds survive without a full-time babysitter |
| **Degenerate** | air-gapping is painless ⇒ drones never expose ⇒ digital meta dies · **OR** any runner hard-counters drones ⇒ drone-builds unviable |
| **Master dial** | **the cost of going dark** (AR-blindness → dumb targeting) vs **the cost of staying loud** (spoof/worm/EMP hit-rate). One slider moves the whole "is a runner worth bringing?" question. |

---

## Also watching (lower priority)

- **Realm-escalation asymmetry.** The three realms resist off an attribute symmetrically
  (Body/ICE/Nerve) but **escalate differently**: digital *snowballs* (worm melts ICE), bio
  *wastes or burns* (Virus chips Body+HP / Poison flat), mental *depletes* (Stress → Resolve
  → Break). Intended texture — watch that it reads as variety, not inconsistency.
- **Link concentration.** Link does five jobs (gate · initiative · channel · reach · AR).
  Elegant, but it's a tuning chokepoint — a change to one job moves all five. No "good at
  hacking but not exposed" exists except skill (the intended glass-cannon identity).
- **Undefended → auto-hit density.** The `Evade/ICE ≤ 0 ⇒ auto-hit` fast-path keeps trivial
  exchanges deterministic and the sim fast; as builds scale and more units carry a trained
  defense, fewer auto-hits ⇒ more RNG ⇒ slower, swingier fights. Watch the share of
  exchanges that still resolve deterministically.

---

*See also: [`stats.md`](stats.md) §7 (per-roll calibration), [`netrunning.md`](netrunning.md)
§6 (the dark/loud 2×2), [`design-delta-v0.26.md`](design-delta-v0.26.md) §4.1 (the
behavior-corruption / chassis matrix).*
