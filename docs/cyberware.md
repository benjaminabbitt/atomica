# Cyberware — system design

*The augmentation economy: what an implant **gives**, what it **costs**, and why
chroming up is a gamble worth taking. Extends [`../CHROME-AND-CODE.md`](../CHROME-AND-CODE.md)
§7F and [`design-delta-v0.26.md`](design-delta-v0.26.md) §1–3 (identity,
equipment-condition, menders), §6 (PAN), §13 (gear / hidden points). Pairs with
[`netrunning.md`](netrunning.md): **hacks are the *attack* on cyberware; this doc
is the cyberware.** ◆ = decision/synthesis. Status: **🔭 planned** unless marked
**✅ built**. Numbers are TBD.*

---

## 0. Thesis — chrome is a gamble ◆

Every implant is an **unconditional benefit** wrapped around a **conditional
liability** and a **standing exposure**:

- **Benefit** — the capability you buy it for. *Always on.*
- **Liability** — its hack-effect, which fires **only when the implant is
  breached** (hacked, wormed, or EMP'd). *Sometimes.*
- **Exposure** — the Link surface, the weight, the EMP-able hardware. *Always
  paid.*

The system only works if **benefit out-values the costs** — otherwise nobody
chromes up, and the entire digital realm (which we just built, `netrunning.md`)
has *no targets*. So the first job of cyberware design is the **value equation
(§3)**, not the liability. We already built the liability half; this doc designs
the half that makes it worth running.

The recurring law (delta §1): **more chrome = more power *and* more attack
surface.** Lean = safe-but-weak; loaded = strong-but-exploitable. The meta's
netrunning/EMP pressure is the dial that decides which wins.

---

## 1. Implant anatomy — the bundle ◆

Each implant carries six things, plus run-layer tags:

| Part | What | Direction |
|---|---|---|
| **Benefit** | the capability — a stat, a granted ability/loadout, a skill, a profile unlock | ✅ unconditional |
| **Link** | net-presence contribution: digital initiative **+ the hackable surface** (§7D) | + capability, **+ exposure** |
| **Firewall** | digital defense it adds to the unit | + defense |
| **Weight** | mass → **−physical Initiative** (§7C) | − tempo |
| **Hack-effect(s)** | one **or more** named liabilities that fire *on the owner* when the implant is breached (loaded chrome carries several) | **conditional risk** |
| **Condition** | `Online → Degraded → Offline → Destroyed` (delta §3.1) | live state |

Run-layer tags (engine-blind): **affiliation** (generic / corp / clan, §2.6),
**illicit** flag (→ Notoriety), **price**, and **removable?** — *inbuilt* implants
are non-removable and **are the unit's identity** (§7F, the archetype signature).

So a unit's whole digital stat line (Link, Firewall) and much of its physical one
(Plating, Initiative, granted skills/abilities) is the **sum of its installed
implants** over a chassis base — not hand-authored numbers. That sum *is* the
loadout (delta §13).

---

## 2. The core symmetry — the benefit IS the liability, inverted ◆

The design's elegant lever: **an implant's hack-effect is its own benefit turned
against the owner.** A reflex booster makes you fast → breached, it **Seizes**
(you freeze). Plating protects → breached, it **Sheds** (armor drops). A deck lets
you hack → breached, it **Locks out** (you're bricked off the net).

This does three things at once:
1. **Telegraphed & thematic** — the liability is legible from the benefit, so
   buying chrome is an *informed* gamble (§7F).
2. **Commensurate value, structurally** — magnitude tracks magnitude: a **bigger
   benefit hangs a bigger liability.** You cannot buy a huge edge without hanging a
   huge sign on it. *This is the answer to "hacks must carry commensurate value":
   the implant's power and its risk are the same number, mirrored.*
3. **Worms & netrunners share a payload roster** — every hack-effect is both the
   owner's liability *and* a hacker's/worm's prize (§7F, netrunning §4).

**Symmetry is the default, not a law ◆.** Most implants mirror (benefit ↔ inverted
benefit); **some carry an orthogonal liability** instead — a deck whose breach
**Sheds** your plating — **where it makes sense** thematically or mechanically
(it's a deliberate content choice, not just an escape hatch). What's mechanically
load-bearing isn't the symmetry but the **severity class** of the hack-effect:
whether a breach merely **disables**, fires a **magnified liability**, or delivers
a **knockout** (§6) — and that is gated by the *roll*, not the implant.

**Loaded chrome carries multiple hack-effects ◆.** An implant is a **list** of
liabilities, not one — and the most powerful chrome carries **several** (its
bigger benefit hangs a bigger, multi-pronged sign). A war-deck might breach into
**Lockout** (stun, crit-gated) *and* a **Drain** DoT (margin-scaled) *and* an
orthogonal **Shed**. Which of them fire is the severity ladder (§6), applied
**per effect**: the degrade-class ones scale with margin, the stun-class ones are
crit-gated, and a **Cascade**/worm can trip the whole list at once.

### The implant roster (first pass ◆)

Benefit (unconditional) ↔ hack-effect (on breach). Magnitudes are band-tier
sketches (TBD); the benefit and its corruption are sized to mirror.

| Implant | Slot | Benefit (always on) | Link | FW | Wt | Hack-effect (on breach) |
|---|---|---|---|---|---|---|
| **Reflex booster** | reflex | +Physical Initiative | low | — | — | **Seizure** — Crash / skip |
| **Cyberdeck** | deck | grants the **hack loadout** + Hacking + Link | **high** | + | — | **Lockout** — −Link, digital disabled |
| **Firewall suite** | security | **+Firewall** (the wall) | low | **high** | — | **Breach** — −Firewall, vuln |
| **Subdermal plating** | armor | **+Plating** | — | low | **heavy** | **Shed** — Plating offline |
| **Smartgun link** | targeting | IFF **smart-targeting** profiles + accuracy | mid | — | — | **Misfire** — attack ally / self |
| **Metabolic pump** | bio-sys | +regen (Adaptive System) | — | — | — | **Overload** — Internal DoT (runs hot) |
| **Sensor suite** | optics | +AR range, reveal stealth | mid | — | — | **Blind** — off-AR → dumb targeting |
| **Combat stim** | stim | +damage / Overclock (haste) | — | — | — | **Overdose** — self-DoT → Crash |
| **Skill chip** | chip | **+1 skill** (capped low, §10) | low | — | — | **Scramble** — skill denied |

The **Cyberdeck** is the keystone link to the netrunning system: a unit hacks
*because it carries a deck implant*. Its benefit grants the `Hack` loadout we
already built (`netrunning.md` §2); its Link is why netrunners are exposed glass
cannons; its **Lockout** is the cost of having a deck breached.

**Built so far (Phase A/B ✅, `implant.rs`):** Cyberdeck, Subdermal Plating,
Reflex Booster, Firewall Suite, **Combat Stim** (a multi-effect **Overdose** —
self-DoT + Crash, exercising the per-effect ladder), Metabolic Pump. **Pending a
subsystem (🔭):** Smartgun (targeting profiles), Sensor suite (AR), Skill chip
(the §10 *take-the-max* rule, vs the current additive fold) — each waits on its
own layer. The `Contribution` folds Link / Firewall / Plating / Initiative /
damage / max-Integrity today; regen, targeting, and AR benefits come with those
systems.

---

## 3. The value equation ◆ — why you chrome up anyway

Let an implant give benefit **B** every battle and a liability **L** that fires
with probability **p** (where `p` rises with your **Link exposure** and the
enemy's **net/EMP pressure**). Net value per battle:

```text
value  ≈  B  −  p·L  −  (exposure + weight + price)
```

Because **B is unconditional and L is conditional**, the implant is **+value
whenever breaches are rare** and a **trap when they're common** — even though,
by §2, `L ≈ B + a sting` (the inverted benefit plus a hit). So:

- **Low net/EMP meta** (`p → 0`): `value ≈ B − costs` → chrome is a clear win;
  load up.
- **Heavy net/EMP meta** (`p → 1`): `value ≈ −sting − costs` → chrome is a
  liability; go lean or air-gapped (bioware, §4).

**Lean vs loaded is self-limiting ◆.** Stack *n* implants and the **benefit grows
linearly (`nB`)** but the **liability grows super-linearly**, because more chrome:
1. **sums more Link** → higher `p` (bigger surface, higher worm-catch / hack odds);
2. adds **more independent breach targets** (each implant is its own trip);
3. widens **Cascade** (a meshed-PAN breach trips *all n* at once, §5).

So there's an **optimal chrome level that falls as enemy pressure rises** — the
build-level "loud = capable but exposed." No hard cap needed; the math caps it.

**Hidden points (delta §13).** Each implant is tagged with a net **power-point**
value (`benefit − exposure`), invisible to the player, used by the run layer to
budget shop offers and normalise async-PvP matchups. The value equation above is
what those points encode.

---

## 4. Cost axes — what chrome exposes you to

| Axis | Cost | Counter / fork |
|---|---|---|
| **Link** | the hackable surface (worm-catch, hack channel — `netrunning.md`) — *also* digital capability | Masking / go dark (§7F link-effects) |
| **EMP** ✅ | **physical, bypasses Firewall, no roll** — fries **every** active implant **Offline** and fires their **degrade-class** liabilities (no stun knockout — EMP is blunt). The hard counter; the more chrome, the more an EMP ruins | hardening; **bioware** (no hardware) |
| **Weight** | mass → **−physical Initiative** — heavy chrome = slow body | lean loadouts |
| **Heat** *(if adopted)* | chrome runs hot → closer to overheat **+ brighter AR signature** (§7D) | vent / coolant |

**The bioware fork ◆.** Bioware buys the *same physical-benefit class* (stats,
regen, resilience) with **no Link, no hack surface, EMP-immune** — but **no
digital upside** and a **Reject** liability (malfunction on Virus, the bio mirror
of a hack-effect). So the augmentation choice is **wired / out-tech (cyberware)**
vs **air-gapped / abstain (bioware)** — and *digital strength requires cyberware*
(you can't be strong on the net without the surface). Augmented units run both →
double-exposed (§7F).

---

## 5. Slots, PAN & loadout

- **Slots ◆.** A unit has **implant slots**, count by chassis (Augmented many ·
  Flesh few/none · Machine varies). **Inbuilt** implants occupy slots
  non-removably and define the archetype (§7F). You buy *more* chrome into the
  free slots.
- **PAN — mesh vs segment ✅ (delta §6, build commitment).** `Unit.pan` over the
  installed implants:
  - **Meshed** (default) → **synergy** (modelled as a netrunning **throughput**
    bonus, `mesh_synergy`: +1 to the hack rating per active implant beyond the
    first, capped), **but a crit Cascades** — `apply_breach` breaches *every*
    active implant, not just the targeted slot.
  - **Segmented** → isolated → a breach is **contained** to the one slot (a crit
    still knocks out that implant, but no Cascade), **and no synergy**.
  - Chosen at **loadout, not in-battle** (delta §13) — a build identity. *(The
    synergy as netrunning throughput is a first-cut; richer set-effects are
    content.)*
- **Skill chips are implants** — a capped, **transferable, salvageable** skill
  floor (§10): the fungible counterpart to a character's earned skill. Slot a
  hacking chip on a bruiser for basic netrunning; it dies *recoverable* (unlike
  earned skill).

---

## 6. Breach & repair

An implant is **breached** three ways:

| Vector | How | Defended by |
|---|---|---|
| **Hack** | a netrunner trips the implant (a hack success, `netrunning.md` §3) | Firewall, low Link |
| **Worm** | **Logic-bomb** trips one by name · **Cascade** trips all (meshed PAN) | Firewall, segment PAN |
| **EMP** ✅ | a physical pulse (`Attack.emp` → `apply_emp`) — **bypasses Firewall**, fries every active implant Offline + fires degrade liabilities | hardening, bioware |

### Breach outcome — severity scales with the roll ◆

A breach is **not one thing**; its severity is the **degree of success** (margin)
of the hack that caused it. This is the answer to *"the knockout stuff may be crit
only"* and *"hacking should be hard":*

| Roll quality | Outcome on the owner | Frequency |
|---|---|---|
| **Success (floor)** | **Disable** — the implant goes **Offline**, its benefit lost (repairable). The least-bad case. | the common success |
| **+ margin** | **Magnified liability** — the implant's hack-effect fires, **scaled by margin** (the degrade class: Shed, Overload, Breach, Blind, Misfire…). | needs a solid hit |
| **Crit** | **Knockout** — the **incapacitate class** (a stun: Seizure-Crash, Lockout, Overdose→Crash), or **Cascade** on a meshed PAN (trip *all* implants). | crit-gated; rare |

So **disable is the default best case** (for the victim — you just lose the gear),
a **magnified liability is the common case** when a runner lands it well, and the
**turn-skipping knockouts are crit-only**. Netrunning is reliable **attrition**,
not a reliable hard-disable. *Rule of thumb ◆: any hack-effect that **stuns**
(removes the action) is **crit-gated**; everything else **scales with margin**.*
*(Knockout-gate width — strict nat-18 vs a margin ≥ K "decisive" tier — is a knob,
netrunning §6.)*

When an implant carries **several** hack-effects (§2), the ladder runs **per
effect**: one breach can disable the slot, land its margin-scaled liabilities, and
— on a crit — fire its stun-class one(s) on top. A **Cascade**/worm trips the
whole list regardless of margin (that's what makes it the finisher).

**Condition ladder ✅** (delta §3.1): `Online → Degraded → Offline → Destroyed`,
with each tier delivering a **fraction of the benefit** — Online **full**,
Degraded **half** (`benefit_factor`), Offline / Destroyed **none**. Physical wear
(`degrade_implant`) steps it **one tier down** (reduced benefit, *no liability* —
wear is not a breach); a **breach** knocks it straight **Offline** (firing the
ladder); **Destroyed is terminal** (salvage on death). A degraded **deck still
hacks**, just on a thinner Link surface. *(Engine note: condition changes fold the
`round(new·factor) − round(old·factor)` delta, so a half-tier round-trips exactly
— no integer drift.)*

**Repair — the Ripperdoc ✅** (chrome mender, delta §3.2): `repair_implant`
un-bricks Offline / restores Degraded gear to **Online** (refolding the full
benefit); Destroyed is beyond it. Today it's the bare operation — the in-battle
mender *unit* (targeting, cross-pool at the high end) is later content.

---

## 7. The sim data model & build order ◆

> **Architecture note:** the implant model below (the shipped fold) is being
> generalised into the **equipment-layer decorator** architecture
> ([`layers.md`](layers.md)) — implants become the first `Layer` kind, alongside
> weapons / armor / behavior-corruption, all over one interface. The semantics
> here (benefit↔liability, condition, EMP, PAN/Cascade) carry over unchanged.

**Engine shape.** `atomica-sim` gains an `Implant`; a `Unit` composes its stat
line from a **chassis base + the sum of its Online implants** (the loadout
derivation delta §13 promised). The flat fields we have today (`link`, `firewall`,
`defense.plating`, `initiative`, `skills`, `hack`) become **derived**, not
hand-set.

```rust
pub struct Implant {
    pub name: &'static str,
    pub slot: Slot,                 // Reflex | Deck | Security | Armor | ...
    pub link: i32,                  // folded into Unit.link
    pub firewall: i32,              // folded into Unit.firewall
    pub benefit: Benefit,             // +Plating | +Init | grant Hack | grant Skill | ...
    pub hack_effects: Vec<StatusSpec>,// one or more liabilities; the §6 ladder runs per effect
    pub condition: Condition,         // Online → Degraded → Offline → Destroyed
    pub removable: bool,              // inbuilt = false
}
// Unit { … , implants: Vec<Implant> }  →  derive stats from base + Σ Online
```

A hack/worm/EMP that **breaches** an implant: set `condition = Offline` and
**re-derive** the unit's stats (the benefit drops out — the disable floor), then
apply its `hack_effects` per the §6 severity ladder (margin-scaled degrade,
crit-gated stun; Cascade fires all). The Ripperdoc reverses the disable.

**Build order** (this is netrunning §7 step 2, unpacked):

| Phase | Deliverable | Notes |
|---|---|---|
| **A** ✅ | `Implant` + **stat derivation** — `Unit::install` folds a `Contribution` (Link/Firewall/plating/init) into the line; `disable`/`repair` un/refold (the disable floor). Cyberdeck grants the `Hack` loadout | **built** (`implant.rs`): cyberdeck / subdermal-plating / reflex-booster presets; breach *trigger* is Phase C |
| **B** ✅ | the **benefit roster** as content (§2 table) | **built**: deck · plating · reflex · firewall · stim (multi-effect Overdose) · pump; smartgun / sensor / skill-chip wait on targeting / AR / §10 |
| **C** ✅ | **trip-on-breach** — a hack success targets an implant (`first_active_implant`) and applies the **severity ladder** (§6): success ⇒ **disable**, margin ⇒ **degrade**, crit ⇒ **knockout** (stun) | **built** (`Battle::apply_breach`): reuses the margin/crit roll outputs; chromeless targets fall back to the deck payload. Closes the netrunning loop |
| **D** ✅ | **condition + Ripperdoc** — Degraded (half benefit) / Offline / Destroyed (terminal) ladder + `degrade_implant` / `repair_implant` | **built**: condition-scaled fold (round-trips exact); the mender *unit* is later content |
| **E** ✅ | **EMP** — physical, Firewall-bypassing pulse (`Attack.emp`); fries **all** active implants Offline + fires degrade liabilities (no stun) | **built** (`apply_emp`): reuses `disable_implant`; bioware/flesh immune |
| **F** ✅ | **PAN** mesh/segment + **Cascade** — `Unit.pan`; meshed crit Cascades to all implants + a `mesh_synergy` throughput bonus; segmented contains | **built**: closes the cyberware loop |

Phase **A** is the load-bearing refactor (the stat line becomes derived);
everything after is content + one mechanic each. None of it breaks the crate
split — factions/economy stay engine-blind; the `sim` owns derivation + breach.
