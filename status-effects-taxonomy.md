# Status-Effect & Modifier Taxonomy — Async Battlers & Roguelike Deckbuilders

**Working draft v0.25** · Comprehensive survey + design schema. Reported mechanics are sourced (§9). Anything marked ◆ is my synthesis/analysis, **not** a game fact.

> **Continued in [`docs/design-delta-v0.26.md`](docs/design-delta-v0.26.md)** — post-v0.25 decisions: three-axis identity (chassis × faction × role), the faction economy, the Doctor/Ripperdoc/White-hat mender triad + cross-pool healing, equipment-condition, the Resolve/morale layer, vehicles, Rep/Notoriety politics, and the **PAN** & **AR** digital sub-layers.

## Contents
- §0 Conventions & flags
- §1 Headline findings
- §2 The design axes (the real schema) — A Stacking · B Decay · C Trigger · D Interaction · E Timing · F Targeting/position
- §3 Damage types & elements (types ≠ statuses; flat vs % magnitude; stochastic behavior)
- §4 Master tables — DoTs · amps · reducers
- §5 Master tables — defense · sustain · offense buffs
- §6 Master tables — tempo/control · targeting/position · death/threshold · wards/cleanse · **triggers (6.6)**
- §7 Your custom viruses — **Virus** & **Worm** (dual contagion)
- §7A Your **defense-penetration** model (Internal / Contact / External; armor materials; Body/ICE)
- §7B **Board geometry** (flat-top hex, forward on the long side, meshed seam)
- §7C **Clock & initiative**
- §7D **Secondary resource — Link** (connectivity)
- §7E **Theme — cyberpunk / neo-Japan cyber-samurai**
- §7F **Cyberware, netrunning & the digital economy**
- §7G **The status-effect system — composed by axes**
- §7H **Anti-Worm specialists — the digital defense**
- §7I **Weapons — guns & melee**
- §7J **Units — classes, behavior & movement**
- §8 Open questions / to-verify
- §9 Sources
- §10 **Full rules (first pass)**

---

## 0. Conventions & flags

Per-cell confidence: **✓✓** 2+ independent sources, non-identical wording · **✓** one solid source · **⚠** inferred / needs verification · **◆** my synthesis, not a reported fact.

**Two clocks.** "Frequency" means different things depending on the game's clock:
- **Real-time tick** (cooldown/seconds): *The Bazaar*, *Backpack Battles*. DoTs tick on seconds; "speed" effects modify cooldown rate.
- **Turn-based** (turn/round/floor): *Slay the Spire*, *Wildfrost*, *Monster Train*, *Across the Obelisk*, *Darkest Dungeon*. DoTs tick at start or end of turn; tempo is manipulated via action counters / extra activations, not literal speed.

Games surveyed: The Bazaar, Backpack Battles (async battlers); Slay the Spire (1 & 2), Wildfrost, Monster Train (1 & 2), Across the Obelisk, Darkest Dungeon (1 & 2) (roguelike/positional deckbuilders & RPGs). Reference vocabularies pulled from Hearthstone (CCG), Path of Exile (ARPG), WoW/Guild Wars/MTG where they're the clearest precedent — labeled as such.

---

## 1. Headline findings

1. **"Decay" is a per-game decision, not a property of the effect.** ◆ Your model (burn self-decays, poison persists) matches *The Bazaar* exactly, but is **inverted** in the deckbuilders: StS Poison, Wildfrost Shroom, MT Frostbite, ATO Burn/Bleed all self-decay. Decay is a free lever you assign per effect.

2. **Stacking has ~5 modes, and games formalize them.** ◆ Slay the Spire 2's own classifier: **Intensity** (additive magnitude), **Duration** (−1/turn, fixed magnitude), **Counter** (charges consumed on trigger), **Does-Not-Stack** (binary on/off). Add the hybrid **Both** (Poison = magnitude + countdown) and your proposed **Build-up** (negative decay) and you have the full menu (§2A).

3. **Your "bruise" and "shock" are the same family** (incoming-damage amplifier); they differ only on decay model, damage-type filter, and synergy hook (§4.2).

4. **Correction to v0.1:** spread-to-adjacent **does** exist in-genre. *Across the Obelisk* **Spark** hits the afflicted unit's sides each turn; **Dark** builds to 25 then explodes onto sides. So a building, spreading "disease" has direct genre precedent (§3, §7).

5. **"True damage" vs "HP loss" is a load-bearing split.** ◆ ATO and StS both separate damage that respects Block/armor/resistance (Burn, Spark) from raw HP-loss that ignores it (Bleed, Poison, Mark-of-HP-loss). Which side a DoT sits on is a core identity choice (§2D).

---

## 2. The design axes (the real schema) ◆

Framework is mine; data points are sourced in §3–6.

### Axis A — Stacking semantics (what the number *means*)

| Mode | Number means | Decay coupling | Examples |
|---|---|---|---|
| **Intensity** | magnitude | usually none | StS Strength ✓, Bazaar Poison ✓✓, MT Spikes ✓✓, ATO Sharp ✓ |
| **Duration** | turns/seconds left (magnitude fixed) | −1/turn built in | StS Vulnerable/Weak/Intangible ✓✓, MT Stealth ✓ |
| **Counter (charges)** | uses left; spent on trigger | consumed, not timed | StS Buffer/Artifact ✓, HS Divine Shield ✓✓, DD Aegis ✓✓ |
| **Binary / does-not-stack** | on/off, one persistent effect | none (until removed) | StS Barricade ✓✓, HS Taunt ✓✓, ATO Invulnerable ✓✓ |
| **Both (self-consuming)** | magnitude *and* a countdown | applies then −1 | StS Poison ✓✓, Shroom ✓✓, Frostbite ✓✓, ATO Burn/Bleed ✓✓ |
| **Build-up (negative decay)** | magnitude that *grows* / accumulates to a threshold | +N/turn or +N/event | ATO Dark (→explode at 25) ✓✓; **your Disease** ◆ |

*Slay the Spire's Poison is its only debuff with both intensity and duration stacking ✓✓ — that hybrid is what makes "stack it sky-high and it's still lethal as it ticks down" work.*

### Axis B — Decay model

| Model | Behavior | Examples |
|---|---|---|
| **None** | full strength until removed/cleansed | Bazaar Poison ✓✓, MT Spikes ✓✓, StS Strength ✓, MT Reap ✓✓, Wildfrost Bom ✓ |
| **Self-consuming DoT** | applies payload, then −1 | Bazaar Burn ✓✓, StS Poison ✓✓, Shroom ✓✓, Frostbite ✓✓, ATO Burn/Bleed/Chill/Spark/Dark ✓✓ |
| **Duration countdown** | −1/turn whether or not it "fires" | StS Vulnerable/Weak/Intangible ✓✓, MT Stealth ✓, Wildfrost Snow ✓, PoE Shock ✓✓, ATO Mark/Crack ✓ |
| **Counter / consume-on-trigger** | spent by a qualifying event | StS Buffer ✓, HS Divine Shield ✓✓, DD Aegis ✓✓, BB Reflect ✓✓ |
| **Per-hit consume** | amp/effect spent by the next hit | Wildfrost Demonize (per-hit) ✓ |
| **Decaying buff** | a *buff* that still ticks down | MT Rage −1/turn ✓✓, MT Regen −1/turn ✓✓, ATO Bless/Fury/Powerful ✓✓ |
| **Build-up (negative decay)** | grows over time / to a threshold | ATO Dark ✓✓; **your Disease** ◆ |

Cross-game contrast worth internalizing ◆: **permanence is also a free choice.** StS Strength is permanent within combat ✓; MT Rage does the same job (+attack) but decays −1/turn ✓✓. Same effect, opposite durability.

### Axis C — Trigger / tick model

| Trigger | Fires when… | Examples |
|---|---|---|
| Per-turn (start) | top of afflicted unit's turn | StS Poison ✓✓ (kills *before* the enemy acts), ATO Burn/Bleed/Spark ✓✓, DD Bleed/Blight ✓✓ |
| Per-turn (end) | end of turn/round | Shroom ✓✓, Frostbite ✓✓, MT Regen ✓✓ (unit acts *first*), ATO Bless/Powerful ✓✓ |
| Per-second | real-time interval | Bazaar Burn 2×/s ✓✓, Bazaar Poison 1×/s ✓✓, BB Poison/Regen every 2s ✓✓ |
| On being attacked | reactive thorns / counter | MT Spikes ✓✓, BB Spikes ✓✓, DD Riposte ✓✓ |
| On hit landed | attacker-side trigger | Lifesteal/Vampirism ✓✓, HS Poisonous (instakill) ✓✓ |
| On being healed | heal-triggered | MT Rejuvenate ✓ |
| On counter = 0 | Wildfrost's per-unit action counter | Wildfrost units; Snow freezes it ✓ |
| On card/spell played | Incant, etc. | MT Incant ✓ |
| On summon / on play | enters board | HS Battlecry/Rush ✓✓ |
| On death | death / recursion | MT Endless/Burnout ✓, HS Reborn/Deathrattle ✓✓ |
| On threshold | accumulation hits a cap | ATO Dark explodes at 25 ✓✓, HS Overkill (excess dmg) ✓ |
| Passive / continuous | always-on modifier, no "tick" | Slow/Haste, BB Cold/Heat, Blind, type-resistances ✓✓ |

### Axis D — Interaction layer (what it respects vs. bypasses)

The ATO damage pipeline is the clearest model ✓✓: **base → +/− modifiers (auras/curses/items) → Block → percent resistance (by damage type)**.

| Layer | Respects | Examples |
|---|---|---|
| **True damage** | Block **and** type resistance | most attacks; ATO Burn/Spark/Dark; Bazaar Burn (halved by Shield) ✓✓ |
| **HP loss** | nothing — straight to HP | ATO Bleed/Mark ✓✓, StS Poison (through Block) ✓✓, Bazaar Poison (ignores Shield) ✓✓ |
| **Type-filtered** | only one damage category | StS Vulnerable/Weak (attacks only, not poison/orbs) ✓✓; Lock-On (orbs only) ✓ |
| **Pierce keywords** | negate a defense | MT Piercing (negates Armor) ✓, Shieldbreaker Puncture (removes Guard) ✓✓ |

Two *resistance* axes exist and are different ◆: **damage-type resistance** (ATO % resist by element; Insulate/Courage grant +30%) vs **status resistance** (DD Bleed/Blight/Stun resist = chance to avoid being *afflicted*; Wildfrost Resist = caps stacks).

### Axis E — Resolution timing & ordering ◆

- **Start-of-turn DoTs** (StS Poison, ATO Bleed) can remove a unit before it acts. **End-of-turn DoTs** (Frostbite, Shroom) let it act first — a documented downside (a Monster Train player notes Frostbite "can't kill an enemy before it attacks").
- **Ordering between effects matters.** DD Restoration (HoT) is defined to trigger *before* Bleed/Blight, so it can save a unit on Death's Door from a DoT death ✓✓. Decide your HoT-before-DoT or DoT-before-HoT rule explicitly.

### Axis F — Targeting & position control ◆

Only meaningful if your board has positions. Archetypes (sourced in §6.2):
- **Forced-target (Taunt):** this unit becomes the only/priority legal target.
- **Redirect-protect (Guard):** this unit takes hits aimed at a *specific* ally.
- **Focus amp (Mark / Lock-On):** marked target takes more from synergy attacks.
- **Untargetable (Stealth / Elusive):** can't be targeted (by all, or by spells only) until a break condition.
- **Forced move (Move / knockback):** shuffle a unit out of its effective slot.
- **Adjacency-spread (Spark / Dark / Disease):** effect reaches the afflicted unit's neighbors.

---

## 3. Damage types & elements

### 3.1 The type taxonomy ◆ (composite of D&D-style physical + ARPG/ATO elemental/mystical)

| Category | Types | Canonical "feel" / coupling |
|---|---|---|
| **Physical** | Slashing, Piercing, Bludgeoning(Blunt) | Slashing → **Bleed**; Piercing → **armor-ignore**; Blunt → **stun / sunder / Crack** |
| **Elemental** | Fire, Cold/Ice, Lightning | Fire → **Burn**; Cold → **Chill/Freeze/Slow**; Lightning → **Shock/Spark** |
| **Mystical/Other** | Shadow/Dark, Holy/Light, Mind/Psychic, Arcane/Chaos, Nature/Poison | Shadow → **curse/Decay**; Holy → anti-undead/heal; Mind → **stress/insanity**; Nature → **Poison/Disease** |

Most deckbuilders (StS, Wildfrost) are **typeless** — they make each *status* its own axis instead of typing the damage. The two clearest typed systems in-genre are **Across the Obelisk** (full Physical/Elemental/Mystical with % resistances) and, lightly, **Monster Train** (the `Piercing` keyword).

### 3.2 Type → status coupling — the "types encode the effects" model ✓✓ (ATO is the worked example)

| Element / type | Status it applies | Mechanic (ATO unless noted) | Conf. |
|---|---|---|---|
| Fire | **Burn** | DoT (1 Fire/charge, start of turn, −1); blockable; **lowers fire resist −0.5%/charge**; negated by Wet | ✓✓ |
| Cold | **Chill** | −1 speed/5 charges (turn-order hit); **lowers cold resist −1%/charge**; −1/turn | ✓✓ |
| Lightning | **Spark** | DoT that **hits the sides (adjacent)** at turn start; +1 lightning dmg/3 charges | ✓✓ |
| Shadow | **Dark** | **builds; explodes at 25** (self + sides); lowers shadow resist; −1/turn | ✓✓ |
| Slashing/Phys | **Bleed** | HP-loss DoT (1/charge, start of turn, −1); **ignores Block** | ✓✓ |
| Blunt | **Crack** | **+1 blunt damage taken/charge** (a type-specific Vulnerable); −1/turn | ✓ |
| (general) | **Mark** | +1 *all* damage taken/charge | ✓ |
| Water (enabler) | **Wet** | +lightning damage taken; **neutralizes Burn** | ✓✓ |

The self-amplifying loop ◆: each element-status also **lowers the matching resistance**, so fire makes you take more fire, lightning more lightning. That's how ATO turns an element into a snowball without a separate "amp" effect.

### 3.3 Resistance / counterplay patterns ✓✓

- **Percent resistance by type** (ATO): each unit has per-type resist %; Insulate (+30% fire/cold/lightning), Courage (+30% holy/shadow/mind).
- **Pierce** (MT `Piercing`): a damage keyword that *negates Armor* — type-as-defense-bypass.
- **Elemental countering** (ATO Wet ⟷ Burn): one element can cancel another. Powerful but, per community feedback, can make the cancelled element feel awkward to use.
- **True-damage vs HP-loss** (ATO, StS): Burn/Spark are blockable+resistible; Bleed/Poison ignore Block. Choose per DoT.

---

### 3.4 Damage magnitude model — flat vs percent ◆

The concept here is **percent-based / health-ratio damage**. League formalizes the categories: scaling off **max**, **current**, **missing**, or **bonus** HP.

| Model | Per-tick payload | Behavior | Use | Example |
|---|---|---|---|---|
| **Flat** | fixed number | same every tick | predictable; weak vs big HP | StS/Bazaar poison ✓✓ |
| **% max HP** | X% of max | flat absolute; **can kill**; anti-tank equalizer | "the fatty-killer" | Risk of Rain 2 Acrid (~10% max over time) ✓ |
| **% current HP** | X% of current | **shrinks as HP drops; never lethal alone** (asymptotic) | softener / wither | "% current" DoTs ✓ |
| **% missing HP** | scales as target weakens | ramps toward low HP | execute flavor | League missing-HP scalings ✓ |

Key interactions ◆: **% damage and flat armor are near-orthogonal** (League: percent modifiers stack multiplicatively with resistances) — a flat-armor unit isn't protected from % HP damage, which is *why* % HP is the anti-tank tool. For **Disease**: **building + % max HP** ≈ a ramping Pokémon **Toxic** (1/16, 2/16, … of max HP) — strong and snowbally, so the Internal resist (Body/ICE, §7A) is the brake. A **% current HP** disease would soften-but-never-finish (needs a separate finisher). Burst DoTs (Burn) usually stay **flat**; persistent equalizers (Poison/Disease) are where % earns its place.

### 3.5 Stochastic behavior — rolled, Body-modified effects ◆

Not a category — a *behavior* some effects have. **Virus/Worm** and toxin-DoTs resolve each tick via a **roll**, and the **Internal resist (Body for bio / ICE for digital) modifies that roll** (shifts outcomes toward the mild end / lowers rolled value or tier-odds). A deterministic DoT like **Fire** does *not* roll — it deals a fixed Burn per tick. Three **independent** properties:

- **Tier** (§7A) — External / Contact / Internal → which defense applies.
- **Over-time?** — a DoT or a one-shot. Fire, Poison, Disease all tick; a slash hit lands once.
- **Stochastic?** — does each tick *roll* (Poison, Disease — the Internal resist, Body for bio, modifies the roll) or land a fixed number (Fire)?

Worked examples (the axes don't move together):
- **Fire / Burn** = Contact + DoT + **deterministic** → mitigated by **Plating** (armor).
- **Poison / Corruption** = Internal + DoT + **stochastic** → the **Internal resist** (Body if bio, ICE if digital) modifies the roll.
- **Virus / Worm** = Internal + building DoT + **stochastic** → the resist modifies the T1/T2/T3 roll (§7).
- **Direct True-damage burst** = Internal + one-shot + deterministic → flat resist %.

### 3.6 Types ≠ statuses — the decoupled model ◆ (decided)

**Rejected:** "each damage type *is* its status." The survey showed why — at scale (Warframe ~13 type→status pairs; Cyberpunk 2077's 4; PoE's elemental ailments) the coupling reliably produces **type bloat** (too many to learn) and **meta collapse** (play converges on 2–3 dominant type combos, e.g. Viral+Heat). So the two systems are **kept separate**:

- **Damage types** govern **mitigation & penetration only** — *which defense applies*. A small set: the penetration tier (External / Contact / Internal, §7A) × a resistance class (Kinetic, Thermal, **bio**, **digital**). That's all a "type" decides.
- **Statuses** are a **separate keyword pool** — Virus, Worm, Corruption, Overload, Lag, Breach, Burn… — applied **à la carte** by specific weapons, programs, or cards. *Any* source can carry *any* status.

A mono-katana deals **Melee/Kinetic Contact** damage *and* may plant a **Breach** — but Kinetic ≠ Breach; the weapon couples them, the type system doesn't. Benefits: no type bloat, no forced type→status combo meta, and full design freedom (re-skin a status onto any delivery). The earlier roster table is retired; statuses live in the master tables (§4–§6), types in the penetration model (§7A).

*Survey kept for the record:* Warframe (wiki.warframe.com), Cyberpunk 2077 (cited, not copied), PoE — all demonstrate the coupling and its failure modes (§9).

## 4. Master effect tables — damage & amplifiers

> Payloads are base/typical; exact numbers vary by item/card/upgrade and patch. Sources in §9.

### 4.1 Damage-over-time (DoT)

| Effect | Game | Payload | Cadence | Decay | Bypass | Conf. |
|---|---|---|---|---|---|---|
| Burn | The Bazaar | dmg = stack | 2×/sec | **−1 per tick** | halved by Shield | ✓✓ |
| Poison | The Bazaar | dmg = stack | 1×/sec | **none** | ignores Shield | ✓✓ |
| Poison | Slay the Spire | dmg = stack at turn start | start of turn | **−1/turn** | through Block | ✓✓ |
| Shroom | Wildfrost | dmg = stack | end of turn | **−1/turn** | (vs card HP) | ✓✓ |
| Frostbite | Monster Train | 1 dmg × stack | end of turn | **−1/turn** | (+Piercing → ignores Armor) | ✓✓ |
| Decay | Monster Train 2 | 3 dmg × stack | end of round | **−1/round** | — | ✓ |
| Reap | Monster Train (DLC) | dmg, scales w/ Echoes | end of turn | **none** (non-fading) | — | ✓✓ |
| Poison | Backpack Battles | 1 dmg × stack | every 2 sec | none ⚠ (verify) | bypasses Block | ✓ payload / ⚠ decay |
| Burn | Across the Obelisk | 1 Fire × charge | start of turn | **−1/charge** | blockable + resistible | ✓✓ |
| Bleed | Across the Obelisk | 1 HP × charge | start of turn | **−1/charge** | **ignores Block** | ✓✓ |
| Spark | Across the Obelisk | lightning × charge, **to sides** | start of turn | **−1/charge** | blockable | ✓✓ |
| Dark | Across the Obelisk | builds → 25 → explode (self + sides) | on threshold | **build-up**, −1/turn baseline | blockable | ✓✓ |
| Bleed | Darkest Dungeon | flat/turn (weak; rides a real attack) | per turn | duration ⚠ | DoT (Bleed-resist gates application) | ✓✓ |
| Blight | Darkest Dungeon | flat/turn (strong; the hit is weak) | per turn | duration ⚠ | DoT (Blight-resist gates application) | ✓✓ |
| Horror | Darkest Dungeon | **Stress** over time (alt-resource DoT) | per turn | duration ⚠ | hits the Stress bar, not HP | ✓ |
| Bleed | Path of Exile (ref) | phys DoT; **~3× while the target moves** | continuous | duration | bypasses armor (phys) | ◆/✓ verify |

**Frostbite conflict — resolved.** Base Frostbite **does** decay −1/turn (four independent sources). The "does not decay" claim is the *Cuttlebeard* artifact overriding the base rule — which only confirms it. ✓✓
**DoT identity levers ◆:** (a) decay (none / self-consume / build-up); (b) bypass (true-damage vs HP-loss — bleed's signature is ignoring armor); (c) delivery (DD's "rider on a real hit" vs "payload, weak hit"); (d) resource (HP vs Stress/alt); (e) tick timing (start = pre-emptive, end = unit acts first).

### 4.2 Incoming-damage amplifiers — your **bruise** *and* **shock** ◆

> Bruise and shock as you described them are the *same family* — "afflicted unit takes more damage." They differ only on decay model, which damage types they amp, and synergy hook. None self-tick.

| Effect | Game | Amp | Decay | What it amps | Conf. |
|---|---|---|---|---|---|
| Vulnerable | Slay the Spire | +50% | duration −1/turn | attacks only | ✓✓ |
| Mark | Across the Obelisk | +1/charge | duration −1/turn | all damage | ✓ |
| Crack | Across the Obelisk | +1/charge | duration −1/turn | **blunt only** | ✓ |
| Demonize | Wildfrost | +dmg taken | **per-hit** | hits | ✓✓ |
| Bom | Wildfrost | +dmg taken | **permanent** | hits | ✓ |
| Shock | Path of Exile | +20–50% (scales w/ hit) | duration (2s P1 / 4s P2) | **all** sources | ✓✓ |
| Lock-On | StS (Defect) | +50% | duration −1/turn | **orbs only** | ✓ |
| Mark | Darkest Dungeon | + from mark-synergy attacks | duration ⚠ | mark-tagged attacks | ✓ |
| Spell Weakness | Monster Train | +1 copy of spell dmg/stack | ⚠ verify | spell damage | ✓ |

Bruise ≈ Vulnerable; shock ≈ Shock/Lock-On. **Decision: your Shock = +X incoming damage, −1 stack per hit taken (per-hit consume, Axis B).** So a burst of N hits each gets the amp but drains N stacks; few big hits keep Shock alive longer. Differentiate Shock from Bruise by **damage-type filter** (Bruise→physical/duration-decay; Shock→amps then consumed by hits) and synergy (Shock pairs with multi-hit attackers).

### 4.3 Outgoing-damage reducers (the inverse amp)

| Effect | Game | Effect | Decay | Filter | Conf. |
|---|---|---|---|---|---|
| Weak | Slay the Spire | −25% damage dealt | duration −1/turn | attacks only | ✓✓ |
| Frost | Wildfrost | −attack (temporary) | temporary ⚠ | attack | ✓ |
| Sap | Monster Train | −2 attack/stack | −1/turn | attack | ✓✓ |
| Insane | Across the Obelisk | −0.5% damage done/charge (+ mind-resist down) | −1/turn (end) | all | ✓ |
| Blind | Backpack Battles | −5% accuracy/stack | passive ⚠ | hit chance | ✓✓ |

---

## 5. Master tables — defense, sustain, offense buffs

### 5.1 Defensive / mitigation — the sub-archetypes ◆ (this is where "armor" vs "shield" actually diverge)

| Archetype | What it does | Examples | Conf. |
|---|---|---|---|
| **Expiring absorption pool** | a damage-soak that **resets each turn** | StS **Block** (resets to 0 at turn start) ✓✓; ATO **Block** (cleared end of round) ✓✓; Wildfrost Block ✓ |
| **Persistent absorption pool** | a soak that **doesn't reset** | StS Block **+ Barricade** ("no longer expires", stacks infinitely) ✓✓; Bazaar **Shield** (buffer above max HP; halves Burn; Poison ignores) ✓✓; HS/Hearthstone **Armor** (extra health pool) ✓ |
| **Delayed block** | converts to soak **next round** | ATO **Shield** → Block next round = stacks ✓ |
| **Eroding armor** | persistent reducer that **degrades when hit** | StS **Plated Armor** (gives Block end of turn; **−1 level per unblocked *attack***; immune to thorns-decrement) ✓✓ |
| **Armor generator** | passively makes soak each turn | StS **Metallicize** (+Block end of turn) ✓✓; MT **Armor** (flat reduction/hit) ✓ |
| **Instance negation** | negate **whole** hits regardless of size | HS **Divine Shield** (absorbs the first source) ✓✓; DD **Aegis** (negate next N direct attacks) ✓✓; MT **Damage Shield** (block N instances) ✓; StS2 **Buffer** (prevent next HP-loss) ✓; Wildfrost Block-per-stack ✓ |
| **Damage cap** | reduce **every** instance to a cap | StS **Intangible** (all damage + HP-loss → 1; duration −1/turn) ✓✓ |
| **Type resistance** | % reduction by damage type | ATO per-type resist; **Insulate** (+30% fire/cold/lightning), **Courage** (+30% holy/shadow/mind) ✓✓ |
| **Total immunity** | ignore damage **and** effects | ATO **Invulnerable** (immune to damage + harmful effects; doesn't stack; purgeable) ✓✓ |
| **Status resistance** | chance to **avoid being afflicted** | DD Bleed/Blight/Stun resist ✓✓; Wildfrost Resist (caps stacks) ✓ |

◆ **Absorption pool ≠ flat armor ≠ instance negation ≠ damage cap.** A 5-soak pool stops one 5-hit; flat 5-armor stops 5 off *every* hit; Divine Shield stops one hit of *any* size; Intangible caps *every* hit at 1. Pick deliberately — they reward completely different attack profiles (few-big vs many-small).

### 5.2 Sustain / healing

| Effect | Game | Mechanic | Decay/timing | Conf. |
|---|---|---|---|---|
| Heal | The Bazaar / many | instant restore (Bazaar Heal also cleanses ~poison/burn) | instant | ✓✓ |
| Regen (HoT) | Monster Train | +1 HP/stack, end of turn | **−1/turn** | ✓✓ |
| Regeneration | Backpack Battles | +1 HP/stack every 2s | passive ⚠ | ✓✓ |
| Regen/Vitality | Across the Obelisk | heal 1 HP/charge at turn start, −1 | self-consuming | ✓ |
| Restoration | Darkest Dungeon | HoT that mirrors Bleed/Blight; **triggers before DoTs** (can save from death) | per turn | ✓✓ |
| Lifesteal / Vampirism | Bazaar / Backpack Battles | heal on damage dealt (BB: on melee hit, cap 100%) | on-hit | ✓✓ |
| Rejuvenate | Monster Train | triggers an effect **when healed** (even at full HP) | on-heal | ✓ |
| Bless | Across the Obelisk | +1 heal received (and damage)/charge | −1/turn | ✓✓ |
| Anti-heal (Heartless) | Monster Train | afflicted unit **cannot heal** | binary | ✓ |

### 5.3 Offensive / utility buffs

| Effect | Game | Mechanic | Decay | Conf. |
|---|---|---|---|---|
| Strength | Slay the Spire | +dmg per stack | permanent (combat) | ✓ |
| Dexterity | Slay the Spire | +Block per stack | permanent (combat) | ✓ |
| Rage | Monster Train | +2 attack/stack | **−1/turn** | ✓✓ |
| Spice | Wildfrost | +attack/power | ⚠ | ✓ |
| Empower | Backpack Battles | +1 weapon dmg/stack | ⚠ | ✓✓ |
| Powerful | Across the Obelisk | +5% dmg & heal/charge (max 10) | −2/turn | ✓✓ |
| Sharp | Across the Obelisk | +1 slashing & piercing dmg/stack | ⚠ | ✓ |
| Fury | Across the Obelisk | +3% dmg/charge **but self-inflicts Bleed = Fury each turn** | −1/turn | ✓✓ |
| Bless | Across the Obelisk | +1 dmg & heal received/charge | −1/turn | ✓✓ |
| Crit | The Bazaar | doubles an item's effects (incl. burn/poison applied) | chance-based | ✓✓ |
| Spell Damage | Hearthstone | +spell damage | binary | ✓ |

---

## 6. Master tables — tempo/control, targeting, death, wards

### 6.1 Tempo & action disruption (freeze · stun · slow · silence · skip)

| Effect | Game | Mechanic | Conf. |
|---|---|---|---|
| Freeze | The Bazaar | stops an item's cooldown entirely; frozen items don't act | ✓✓ |
| Freeze | Hearthstone | minion can't attack next turn (can still defend); undone by Silence | ✓✓ |
| Slow / Haste | The Bazaar | slow/×2 cooldown progression | ✓✓ |
| Cold / Heat | Backpack Battles | items trigger 2% slower / faster per stack | ✓✓ |
| Chill / Daze | Across the Obelisk | −speed (turn-order); Daze −6 & removes Haste | ✓✓ |
| Stun | Backpack Battles / Darkest Dungeon | pause items / skip one turn (DD: Stun-resist + diminishing returns) | ✓✓ |
| Snow | Wildfrost | freezes a unit's attack counter until it attacks | ✓ |
| Ink / Silence | Wildfrost / Monster Train | cancels abilities for N turns / disables triggered abilities | ✓✓ |
| Silence | Hearthstone | **removes all card effects** (buffs, deathrattles, taunts — and debuffs like Freeze) | ✓✓ |
| Haze | Wildfrost | afflicted unit hits one of its **own allies** instead | ✓ |
| Transform | Hearthstone | permanently turn target into something harmless (Polymorph/Hex) | ✓ |
| Mind Control | Hearthstone | take ownership of an enemy unit | ✓ |

### 6.2 Targeting & position control

| Effect | Game | Mechanic | Conf. |
|---|---|---|---|
| Taunt | Hearthstone | enemies must target Taunt units first / can't hit non-Taunt | ✓✓ |
| Guard | Darkest Dungeon | redirect a **specific ally's** incoming attacks to self (+ their effects); ~3 turns | ✓✓ |
| Mark / Lock-On | DD / StS Defect | marked target takes more from synergy/orb attacks | ✓ |
| Stealth | Monster Train / Hearthstone | untargetable for N turns / until it attacks or takes damage | ✓✓ |
| Elusive | Hearthstone | can't be targeted by spells/hero powers | ✓ |
| Move | Darkest Dungeon | forcibly shuffle a unit's position (out of its effective row) | ✓ |
| Spark / Dark | Across the Obelisk | reach the afflicted unit's **sides** (adjacency-spread) | ✓✓ |

### 6.3 Reactive (thorns & counters)

| Effect | Game | Trigger | Payload | Decay | Conf. |
|---|---|---|---|---|---|
| Spikes | Monster Train | when this unit **is attacked** | dmg to attacker = stack | **none** | ✓✓ |
| Spikes | Backpack Battles | when hit by **melee** | 1/stack (cap 100% of hit) | ⚠ | ✓✓ |
| Riposte | Darkest Dungeon | when targeted by a **damaging attack** | a real counter-attack (can crit/apply effects); DoTs don't trigger it | duration/charges ⚠ | ✓✓ |
| Vampirism | Backpack Battles | when hitting w/ melee | heal 1/stack | ⚠ | ✓✓ |

### 6.4 Death, threshold & execute

| Effect | Game | Mechanic | Conf. |
|---|---|---|---|
| Reborn | Hearthstone | resummoned with 1 HP on first death | ✓✓ |
| Endless / Reform | Monster Train | dead unit returns (to draw pile / hand w/ Burnout 1) | ✓ |
| Burnout | Monster Train | countdown that **kills** the unit when it expires | ✓ |
| Death's Door | Darkest Dungeon | grace state at 0 HP before an actual death blow | ✓ |
| Doom | Slay the Spire 2 | accumulates, triggers at a threshold | ✓ |
| Poisonous / Venomous | Hearthstone | **destroy any unit it damages** (instakill on damage; Venomous = one use) | ✓✓ |
| Overkill | Hearthstone | triggers when damage exceeds target's HP | ✓ |

### 6.5 Wards & cleansing (directly relevant to your Disease)

| Effect | Game | Mechanic | Conf. |
|---|---|---|---|
| Buffer | StS2 / Across the Obelisk | prevent the next HP-loss / next Curse (counter; doesn't stack) | ✓✓ |
| Artifact | Slay the Spire | negate the next debuff applied | ✓ |
| Reflect | Backpack Battles | bounce the next debuff(s) onto the opponent; once/battle | ✓✓ |
| Cleanse / Purge / Dispel | most games | remove debuff(s); some debuffs are flagged "can't be purged unless specified" (ATO) | ✓✓ |
| Invulnerable | Across the Obelisk | immune to damage + harmful effects (purgeable; doesn't stack) | ✓✓ |
| Resist / Immune | Wildfrost | caps or blocks stacks of a specific status | ✓ |
| Orange Pellets | Slay the Spire | item that cleanses all debuffs | ✓ |

---

## 6.6 Triggers — the event taxonomy ◆

**Formal frame (MTG ✓):** a trigger is `[event] → [effect]`, and there are three ability classes — **triggered** (automatic, on an event; reads "when / whenever / at"), **activated** (pay a cost, player-chosen: `cost: effect`), and **static** (always-on, no event). **Ordering rule (Hearthstone ✓):** all triggers from one event resolve **before** the game re-checks for deaths — "damage pauses events," so e.g. an on-hit effect and the death it causes both finish before the unit leaves.

For an **async / turn auto-battler**, the spine is the **cooldown / periodic** trigger (units act on a timer or each turn); event hooks layer on top. Most unit effects should be *triggered*, not *activated* — players set up during planning, then the board resolves itself (The Bazaar, Mechabellum).

| Trigger | Fires when | Precedent | For your game |
|---|---|---|---|
| **Cooldown / periodic** | timer elapses / each turn / "every second" | The Bazaar; auto-battlers ✓✓ | the core auto-act; Haste/Slow/Chill bend it (§7C) |
| **On-deploy / on-play** | unit enters the board | HS **Battlecry** ✓ | summon/deploy effects |
| **On-death / on-destroyed** | unit dies | HS **Deathrattle**; Bazaar ✓✓ | last-gasp, death-spores; disease-purge-on-death lives here |
| **On-hit / on-attack** | this unit lands a hit | StS; ARPGs ✓ | plant a status on hit (Bleed/Burn) |
| **On-damaged (reactive)** | this unit takes a hit | StS thorns; "retaliation" ✓ | thorns, counters, **Shock −1/hit** (§4.2) |
| **On-kill** | this unit kills | many ✓ | snowball / refund |
| **Turn-start / turn-end** | unit's turn boundary | StS powers ✓ | **DoTs resolve at start** (§7C); regen at end |
| **Threshold / HP-cross** | HP crosses X% | Bazaar "below half"; execute ✓✓ | panic buttons, executes, enrage |
| **On-crit** | a crit lands | The Bazaar ✓ | crit-build payoffs |
| **On-enemy-action** | the opponent acts | HS **Secrets**; Bazaar ✓✓ | traps, interrupts, counter-tech |
| **On-status / stack-threshold** | a status applies / hits N stacks | ATO **Dark** threshold-burst ✓ | detonators; **disease T3 spread** (§7) |

---

## 7. Your custom viruses — **Virus** (bio) & **Worm** (code) ◆

The "disease" splits into **two parallel contagions** (§7E). Both are **Internal** (bypass Barrier & Plating), **stochastic** (per-tick roll, §3.5), and **build** over time — but they spread on different channels, are stopped by different stats, and hit different unit types. One disease-feel, two unrelated counterplay modes.

**Each side is a *family*, not one effect ◆.** "Virus" and "Worm" are **keyword pools** (consistent with types ≠ statuses, §3.6): different strains/worms do different things — spread, disable an implant, DoT, melt **ICE**, or **trip a target's hack-effects (a specific *named* one, or all at once)** (§7F) — sharing the per-side traits in the table. The spreading-contagion strain is the **flagship** of each. The Worm reaches a unit through its **Link** (the surface) by any of three vectors — **runner-directed, interaction, or ambient proximity** — all opposed by **ICE** (see §7F).

| | **Virus** (bio) | **Worm** (code) |
|---|---|---|
| Spreads by | **hex-adjacency** — passive, to physical neighbors (§7B); no action needed | **interaction** — *any* unit that **acts on** an infected unit (buff / heal / hack / aura / connected attack) rolls to catch it; transmission is **Link-gated** |
| Counterplay mode | **positional** — spread out, push to corners, quarantine by geometry | **behavioral + Link** — don't touch it, **drop Link / go dark**, cleanse first, or send a **low-Link unit** to engage it safely |
| Resisted by | the **Body** attribute (bio Internal-resist — biological resilience is Body itself, not a separate stat) | **ICE** (digital Internal-resist) + **anti-Worm specialists** (killer programs / hunters) |
| Catch modifier | proximity / # of infected neighbors | **Link** — net presence (higher = more exposed = higher catch roll) |
| Hits | flesh — organic + augmented | the connected — **any unit with Link > 0** (systems, augmented, networked drones) |
| Immune | pure-machine (no flesh) | **zero Link** (air-gapped — a build/positioning choice: organic-no-cyber, or an air-gapped drone) |

**Shared mechanics (tune per virus):**
- **Build-up stacking** (Axis A/B): stacks *grow*, don't decay.
- **Stochastic progression roll** each turn: **T1 tick** → **T2 build** (+1 stack) → **T3 spread**, higher-tier odds rising with stacks and pushed *down* by the relevant resist (**Body** for the Virus, **ICE** for the Worm). The roll is the contagion's pulse.
- **Weak per tick** (1×stacks, or **% max HP** for an anti-tank ramp, §3.4).
- **Cleanse →** a decaying resist buff (**−[value] stacks/turn**): **Vaccinated** vs Virus (bio), **Antimalware** vs Worm (code) — active treatment that fights the infection down, then wears off.
- **Self-cure**: a high enough resist makes net growth negative → the unit clears itself (frailty-seeker: festers on the weak, burns out on the tough).

**The Worm's signature — the commit-time trap ◆.** Because actions lock at planning and resolve later, the Worm can't be perfectly dodged: you queue an interaction (buff / heal / hack / aura — and **many units do this, not just dedicated support**, including positionally) on a unit that's clean *now*, and if it's infected by the time the action fires, the actor **catches the Worm off its own interaction**. Counterplay is informed betting, not reaction — exactly what an async engine is for. **Two outs keep it fair:** (1) the catch is an **ICE-modified roll**, not a guaranteed infect (so no one unit chain-spreads the board); and (2) transmission is **Link-gated** — an **air-gapped, low-Link unit** (a cyber-samurai with a non-networked blade) can **strike an infected enemy without catching it**. That makes low-Link melee the natural **anti-Worm specialist**: the quiet blade that wades into the infected where the connected can't.

**Design tensions ◆:**
1. **Runaway brake (still required).** A building + spreading contagion trends unkillable without ≥1 hard brake: stack cap, "cleansed = immune for K turns," spread gated on a threshold, or **death purges the carrier**. Pick ≥1 per virus.
2. **Two resists (decided), asymmetric.** The bio resist is the **Body** attribute itself (vs Virus); **ICE** is the digital resist (vs Worm) — **two distinct things**: a brute can be Body-tough but ICE-soft, a netrunner the reverse (build identity). The counter-ecosystems differ: **Virus** → high Body + cleanse (mostly innate); **Worm** → stackable ICE guards **plus active anti-Worm specialists** (killer programs, hunter units). The digital side is the more *tooled* one.
3. **Transmission scope = Link-gating (resolved).** Which interactions catch the Worm? **Connected** ones do (Link-gated); an air-gapped/low-Link physical strike doesn't. Remaining dials: Virus spread slow enough that repositioning keeps pace (§7B); Worm base catch-odds low enough that interacting stays worthwhile.
4. **Augmented = double-exposed.** Augmented units catch both — intended (flexible all-rounders paying a vulnerability tax), but watch they're not strictly worse than committing to one chassis.

---

## 7A. Your defense-penetration model (Internal / Contact / External) ◆

Layered defense, three concentric layers: **Shield (outer) → Armor (mid) → Integrity (inner)**. *Integrity* is the rename of HP — a **single pool that all damage reduces** (not split into per-type sub-pools). A damage *category* is defined by how many layers it skips — each tier penetrates one deeper. The three layers are each countered on one tier: Shield (External buffer), Armor (External/Contact reducer), the **Body** attribute and **ICE** (Internal reducers — see below).

| Category | Shield applies? | Armor applies? | Reaches | Closest shipped precedent |
|---|---|---|---|---|
| **External** | ✓ | ✓ | reduced by both | normal damage (Warframe IPS/elemental; ATO block→resist) ✓✓ |
| **Contact** | ✗ | ✓ | armor only | Warframe **Toxin** — bypasses shield, not armor ✓✓ |
| **Internal** | ✗ | ✗ | straight to Integrity (countered by the **Body** attribute / **ICE**) | Warframe **Slash/Bleed** (True damage, ignores armor) ✓✓; ATO Bleed (ignores Block) ✓✓ |

Warframe validates the split almost exactly: Toxin = Contact, Slash-Bleed = Internal, everything else = External. *(Caveat: Warframe patched Slash's shield interaction over the years — current Slash bleed is True-damage-to-health but also chips shields; treat the exact bypass as version-specific.)* Other layered-defense precedents worth studying ◆/general: **Borderlands** (Shock→shields, Corrosive→armor, Incendiary→flesh — each element targets a layer), **Mass Effect** (shields/armor/barrier/health × weapon-vs-power), **StarCraft** (damage types × armor types).

**Two-part hit rule** ◆ — a hit and the status it plants can sit in different tiers. A slashing weapon: **Contact** hit (armor blunts it) that leaves an **Internal** Bleed (armor can't touch the bleed). This is exactly Warframe's Slash-hit + True-damage proc, and it's *why DoTs feel dangerous*: once internal, only cleanse/cure/regen answers them. **Your Disease lives here (Internal).**

**Pierce = drop one tier** ◆ — rather than a separate armor-pierce stat, "piercing" shifts a hit's category down a rung: External→Contact (skip shield) or Contact→Internal (skip armor). One mechanic spans the whole ladder. Warframe shows the two flavors: Puncture (partial *ignore* armor) vs. Corrosive (*strip* armor over time).

**Resolution order — pick one** ◆ (two coherent ways to wire "external = both help"):
- (a) **Sequential gates:** Shield pool absorbs first; once depleted, Armor reduces; then HP. Armor only matters after the shield is gone.
- (b) **Reducer + pool:** Armor reduces *every* hit's number; Shield is a separate buffer that only catches External hits. Both contribute on the same External hit.

**Element → tier assignment (proposal)** ◆: **External** = ranged energy/magic (frost ray, lightning bolt, arcane) — a shield catches them. **Contact** = melee physical (slash/pierce/blunt) *and* **Fire/Burn** (a deterministic contact DoT) — past the shield, blunted/mitigated by armor. **Internal** = ailments (**Virus**, **Worm**, Corruption, **Bleed** — the stochastic ones roll) — bypass both, countered by the **Body** attribute (bio) / **ICE** (digital). A *weapon* can still apply a status its hit isn't (a slashing Contact hit plants an Internal Bleed) — that coupling lives on the weapon, not the type (§3.6).

**Physical sub-types × armor material — three tiers** ◆ — Armor has a **material** (Padding / Mail / Plate), and the three physical types interact differently with each. Grounded in Mount & Blade and Kingdom Come (both: blunt = anti-rigid, pierce = penetrator, slash = anti-light), this **revises the earlier hard/soft rule** — bludgeoning is *not* stopped by hard armor; it's the type that *defeats* rigid plate (percussion), while **padding** is the blunt-absorber.

Effectiveness of the *type* through each armor (high = armor fails / type strong):

| Type | Unarmored | Padding (gambeson) | Mail (chain) | Plate (rigid) |
|---|---|---|---|---|
| **Slashing** | high | medium | low | low |
| **Piercing** | high | high | high | low |
| **Bludgeoning** | medium | **low** | high | high |

Each material has exactly one hard counter; each type has a home:
- **Padding** ← Pierce (and it's the anti-Blunt material). **Mail** ← Pierce / Blunt (strong only vs Slash — the "needs layering" material). **Plate** ← Blunt only (its sole weakness).
- **Slashing** = unarmored-killer (falls off vs any metal). **Piercing** = universal penetrator, walled only by Plate. **Bludgeoning** = anti-metal concussion (great vs Mail/Plate, soaked by Padding).
- **Layering (KCD):** stacking materials covers gaps — gambeson under plate patches plate's blunt weakness. Armor becomes a loadout decision (carry mixed materials, or counter the enemy's).
- **Pierce = drop one tier** still holds and can be modeled as shifting the armor one step softer (Plate→behaves Mail, etc.), which reproduces the "Pierce strong vs all but Plate" row.

*Sourced shape (M&B + KCD, double-sourced); exact cell values are tunable.*

**The Internal-tier defense — two flavors ◆** — completes the set: Barrier counters External, Plating counters External/Contact, and the **Internal** tier is countered by a per-unit % that reduces everything attacking Integrity directly (past Barrier *and* Plating) — defined by **tier, not timing** (a *direct* True-damage burst that skips plating is reduced too, not just ticks). For **stochastic** effects it **modifies the roll** (lowers tier-odds / rolled value); for one-shot Internal hits it's a flat % reduction (§3.5).

Because the Internal tier has **two damage flavors** (§7E) — bio and digital — its resist splits **two ways**, but the two are different *kinds* of thing:
- **The Body attribute** — vs **bio** Internal (Virus, organic toxins). Biological resilience isn't a standalone stat; it *is* **Body** (which also carries Integrity/HP and melee damage). A tough body shrugs off virus, poison, plague, and organic toxins; Body is also the Virus growth-resist / self-cure (§7).
- **ICE** — vs **digital** Internal (Worm, intrusions, malware). The security-rating stat (Intrusion Countermeasures Electronics); also the Worm catch-resist and roll-modifier (§7), and the gate on how exposed your interactions/connectivity are.

They're independent on purpose — a brute can be **Body-tough, ICE-soft**; a netrunner the reverse — which is real build diversity. Folding bio-resist into the **Body** attribute already settles the old "doubled resist economy" worry: there's no separate Immunity stat to pay for — bio resistance rides Body (which you're buying anyway for HP and melee), while **ICE** is the one dedicated digital resist. Chrome (External/Contact) is still guarded by Barrier + Plating, untouched by either.
- **% not flat.** A percentage reduction is order-independent — identical whether applied per-effect or to the summed tick. (A *flat* per-tick reduction differs: flat-per-effect over-rewards stacking many small DoTs — so prefer %.)
- **Scope choice:** reduce only DoT *damage*, or also debuff *magnitude/duration* (stun length, slow amount)? Cleanest split: the Internal resists cover DoT/status damage; a separate **Tenacity** covers control duration. Or fold both in.
- **Fixed vs current-HP:** flat stat, or does it scale with current Integrity% (wounded = less vital = disease grows faster)? The latter is thematically apt (the sick get sicker) but risks a death-spiral — flag before tuning.

---

## 7B. Board geometry — flat-top hex, forward on the long side ◆

Three columns of flat-top hexes, extended along the **long (vertical) axis** — and **forward runs across the columns** (toward the enemy on the far long edge). So the **columns are depth ranks** (back → mid → front, ~3 deep, growing to 4–5 with army size) and the **long vertical edge is your frontage** (the battle line, many units abreast).

**Two boards meet at a vertical seam.** To interlock the flat-top hexes there, one board is shifted **±½ hex vertically** — direction **rolled at start, settable by item**. This offset decides the **cross-board front-line pairings**: each of your front hexes is edge-adjacent to **two** enemy front hexes, and which two depends on the offset. An item that sets up/down = choosing favorable front-line matchups. *(Diagram in chat — confirm orientation, §8 Q.)*

**Adjacency (orientation-independent geometry):**
- **Interior column = degree 6** — touches 2 same-column + 2 each side. With columns-as-ranks, the **middle rank** is the degree-6 hub, bridging your **front and back ranks**.
- **Outer columns = degree ≤4** — front and back ranks touch only the middle; **corner hexes** (line ends) have the fewest neighbors.
- **Generalizes to 4–5 columns:** more columns = more interior (high-degree) hexes = spread/AoE gets more internal reach; the **corners stay the safe valves**.

**Consequences** ◆:
- **The middle rank is the crossroads** — anything there (buff, AoE, spreading Disease) reaches front and back, and it's where incoming spread converges.
- **Quarantine = geometry** — to slow a spreading status, push infected units toward **low-degree hexes** (corners, line ends); keep it out of the high-degree interior.
- **Frontage vs depth** are distinct levers now: widen frontage (taller board) vs deepen ranks (more columns) — army growth chooses which.
- **Seam control** — the mesh item is a positional tempo tool: it sets who your front line engages before the first blow.

---

## 7C. Clock & initiative ◆

- **Turn-based** (decided); real-time emulated by fast-ticking turns.
- **Initiative.** Turn order = descending Initiative; ties → fixed rule (side → frontage → seed). Status hooks: **Lag/Chill** −Initiative, **Haste** +Initiative, **Crash/Stun** skips, "act again" re-inserts.
- **Two Initiative tracks, interwoven (decided).** Each unit has a **physical Initiative** and a **digital Initiative** — and **digital Initiative *is* Link** (§7D), not merely Link-driven. All physical and digital actions **interleave into one woven order** by their respective inits — *not* two separate phases. A chrome bruiser acts early in the world / late on the net; a netrunner the reverse.
- **Equipment = weight + a digital bundle (the unified load model) ◆.** Every piece of gear carries **physical weight** (→ lowers **physical Initiative**) *and* a **(Link, ICE, Hack-effect)** bundle (§7F): its Link **raises digital Initiative** but makes the unit louder/more exposed. So gearing up is one coherent trade — **slower body, faster/louder deck, better-defended, more hackable**; lean is the inverse. The whole **chrome-vs-flesh axis = how much equipment you run.**
- **Concurrency tax.** Doing many actions in a turn taxes *that realm's* Initiative (you can't do everything at once at full speed) — physical multitasking slows physical Initiative, digital multitasking slows Link.
- **DoT timing:** resolve a unit's DoTs at the **start of its turn** (pre-emptive — a lethal Virus/Worm can kill before it acts).
- **Guard:** cap per-turn Initiative swings so a fast army can't fully lock out a slow one.

---

## 7D. Secondary resource — Link (connectivity) ◆

You floated Power, Heat, and Connectivity. Verdict: **Link (= connectivity)** is the signature resource; **Heat** — *if* adopted — is a full **thermal layer** (below), not just overclock; Power is mana (cut).

| Candidate | What it is | Verdict |
|---|---|---|
| **Power** | energy budget all systems draw on | **cut** — mana reflavored |
| **Heat** | a thermal gauge — builds from overclock / energy weapons / heavy chrome; upside (overclock, heat-scaling) + exposure (thermal signature) + risk (overheat → throttle/shutdown/Integrity, BattleTech ✓) | optional **thermal layer** (below) — a real third resource if adopted |
| **Link** | a unit's **net presence** — a **continuous stat set by equipment / flavor** (not a boolean) | **the one** |

**Link = net presence ◆.** The digital realm's master stat — one value doing quadruple duty, all pointing the same way (*loud on the net = capable but exposed*):
- **Digital Initiative — Link *is* the digital-turn-order stat** (higher Link → acts sooner on the net, §7C).
- **Action throughput.** More Link = more digital actions per turn; concurrent digital actions tax it (the concurrency cap, §7C).
- **Worm exposure.** Higher Link = bigger target = higher Worm catch-roll, and the surface every digital attack reaches through (§7, §7F).
- **Hack gate.** You can hack / be hacked only with Link.

Link is **set by equipment** (each implant's Link value sums into it, §7F) plus unit flavor — so the chrome-vs-flesh axis *is* a Link axis.

**Continuous, with a meaningful floor ◆.** Link is a *stat* (set by gear/flavor), **not** online/offline — but **zero Link** is a real point on the dial: a **zero-Link unit is immune to all digital attack** (Worm, hacks, intrusions), at the cost of every digital benefit (no net Initiative, no hacks, no shared net buffs). "Going dark" = driving Link to zero — total digital safety, total digital isolation. Pure-chrome units and air-gapped drones live here by design; a connected unit can *choose* to drop Link as a defensive play, and **low-Link melee is the anti-Worm specialist** (§7).

**Spread channels (resolved, §7):** Virus → **adjacency** (position), Worm → **interaction** (action), **Link-gated**, with Link as the Worm's exposure dial. Two channels, one per virus.

**Heat — the thermal layer (if adopted) ◆.** A resource that only does "overclock and pray" doesn't justify a whole system; the version worth adopting makes Heat a per-unit gauge that **builds** from running hot — overclocking, firing **energy / EMP weapons** (§7I), heavy **cyberware** (chrome runs hot), aggressive hacking — and does **three** things as it rises (the *loud = capable but exposed* pattern, now thermal):
- **Upside — overclock & heat-scaling.** Run hot to boost actions (extra Initiative / power), and some abilities or units **scale with Heat** (the hotter, the stronger — a risk/reward ramp toward the ceiling).
- **Exposure — thermal signature.** A hot unit is **more detectable / targetable** (bigger signature → easier to hit, draws fire, can't hide). The physical parallel to Link's digital exposure: **Link = loud on the net, Heat = bright on thermal.**
- **Risk — overheat.** At the ceiling: **throttle** (−Initiative), **shutdown** (skip), or **Integrity damage** (BattleTech ✓). A hard cap on running hot.

**Counterplay:** **vent** (spend a turn to cool — tempo cost), **coolant** gear/cyberware (raises the ceiling / dissipation), or just **play cool**. **Ties:** **Burn** adds Heat; **Cold/Chill** dissipates it (a job for the cold side); **EMP / energy** weapons are Heat sources; **cyberware** runs hot passively — one more chrome-is-liability angle (more chrome = more Heat = closer to overheat *and* more detectable).

This gives the game a **third exposure axis** — **Link** (digital), **Heat** (thermal), **clustering / position** (physical) — each "capable but exposed," each with its own risk and counterplay. *That* earns a resource slot; bare overclock doesn't.

**Recommendation:** **Link** as the signature resource. **Heat** is worth adding *only as the full thermal layer above* (a third exposure axis); as bare overclock it doesn't earn a slot. **Power** is cut.

---

## 7E. Theme — cyberpunk / neo-Japan cyber-samurai ◆

**Working title candidate: *Chrome and Code*** — it names the core duality (the two combat layers below) and is clean of any IP. Cyberpunk with a **neo-Japan, melee-forward** aesthetic (cyber-samurai / neon-Tokyo), using **original / generic vocabulary only** — the names below are common-domain tech and sci-fi terms, *not* lifted from any specific franchise (no trademarked item, faction, or coined names; precedents like Cyberpunk 2077 are **cited as research, not copied**). The two penetration axes become the genre's two combat layers:

- **Chrome (hardware) = External / Contact.** Physical gear: **Barrier** (energy shield) → **Plating** (Composite / Reactive / Ablative — the armor-material matrix §7A) → **Integrity** (frame/chassis — armor-over-structure, BattleTech/Mechabellum ✓✓). Damage: **Kinetic** (ballistic = pierce), **Melee/blade** (slash/blunt), **Incendiary/Thermal** (= fire, a Contact DoT). The melee-forward flavor gives the blade types distinct identities that slot into the §7A matrix: **monomolecular** edge = the ultimate **pierce** (ignores Plating material — only the heaviest resists); **vibro-blade** = enhanced **slash** (treats Plating one tier softer); **mono-katana / blade** = the generalist slash.
- **Net (software) = Internal.** Bypasses chrome, hits the system directly: **viruses, intrusions, malware**. Guarded by **ICE** (the digital Internal-resist, §7A). **Stochastic** — intrusions roll (§3.5).

**Two viruses (§7), two channels:** the bio **Virus** (spreads by hex-adjacency, resisted by the **Body** attribute, hits flesh) and the digital **Worm** (spreads by interaction, resisted by **ICE**, hits systems). Position-counterplay vs action-counterplay.

**Types are decoupled from statuses (decided, §3.6).** Damage *type* governs only mitigation/penetration (which defense applies); *statuses* (Virus, Worm, Overload, Lag, Breach…) are a **separate keyword pool** any weapon/program can apply à la carte. A mono-katana deals Melee damage *and* may plant a Breach, but the type ≠ the status — which is what avoids the Warframe-style type bloat.

**Unit-type rock-paper-scissors ◆** — vulnerability keys off **two properties, not a fixed class**: the **Virus** infects **biology** (flesh), the **Worm** infects **Link** (net presence). So an air-gapped drone is Worm-immune; a networked one isn't.

| Unit | Virus (bio) | Worm (code) |
|---|---|---|
| **Organic, no cyber** | vulnerable | immune *if* zero Link |
| **Augmented** (flesh + chrome + Link) | vulnerable | vulnerable (the all-rounder's tax) |
| **Networked drone** (Link > 0, no flesh) | immune | vulnerable |
| **Air-gapped machine / blade** (zero Link, no flesh) | immune | **immune** (the anti-Worm specialist; beaten by Kinetic / EMP instead) |

**Reflavor map (mechanics unchanged; original placeholder names):**

| Mechanic | Cyberpunk name (generic) |
|---|---|
| Integrity (HP pool) | **Integrity / Frame** |
| Shield | **Barrier / Deflector** |
| Armor (+material) | **Plating** (Composite / Reactive / Ablative) |
| Melee weapon flavors | **Mono-katana** (slash) · **Vibro-blade** (slash+) · **Monomolecular** (pierce, armor-ignoring) |
| Internal defense (bio) | the **Body** attribute (no separate stat — bio resist *is* Body) |
| Internal defense (digital) | **ICE** (Intrusion Countermeasures Electronics) |
| Bio virus (adjacency) | **Virus / Strain** |
| Code virus (interaction) | **Worm / Logic-plague** |
| Poison (Internal stochastic DoT) | **Corruption / Nanite-swarm** |
| Fire (Contact DoT) | **Incendiary / Thermal** |
| Shock (incoming-dmg amp) | **Overload / Spike** |
| Bleed (Internal DoT) | **Breach / Leak** |
| Chill (−Initiative) | **Lag / Throttle** |
| Stun (skip turn) | **Crash / Lockout** |
| Virus cleanse (bio) | **Vaccinated** (or Immune) |
| Worm cleanse (code) | **Antimalware** |
| Cleanse | **Scrub / Purge** |
| Initiative | **Clock rate / Cycles** |
| Connectivity / net presence | **Link** (continuous, equipment-set) |
| Anti-Worm specialist | killer program · hunter · **low-Link blade** |
| Secondary resource | **Link** (+ optional **Heat**) |

*Names are placeholders and deliberately generic — swap freely; the value is the layer-mapping, not the words. Neo-Japan flavor (clans, dueling, honor) is open if you want a faction layer (§8).*

---

## 7F. Cyberware, netrunning & the digital economy ◆

**Cyberware / mods = the major purchasable** (the between-rounds shop). Every implant is a **three-part bundle**:

| Slot | What it does | Direction |
|---|---|---|
| **Link** | net-presence contribution (§7D) — digital Initiative, action capacity, **and the hackable surface** | improvement *and* exposure |
| **ICE** | digital defense on the implant / unit — opposes hacks & worms | improvement |
| **Hack-effect** | a capability the unit can use — *and a loaded liability*: when an enemy breaches this implant, its hack-effect fires **deleteriously on the owner** | **improvement and risk** |

So chrome is double-edged: more implants = more power (Link / ICE / hacks) **and** more hackable surfaces whose hack-effects can be turned against you. Lean = fewer liabilities; loaded = stronger but more exploitable. The deleterious hack-effect should be **telegraphed** so buying chrome is an *informed* gamble (§8).

**Inbuilt equipment = unit identity ◆.** Not all chrome is bought — some units ship with **integral, pre-applied** implants baked into their base profile (often **non-removable**). This is the cheapest way to give a unit a *feel* without a stat sheet: a **wired** unit comes pre-loaded with high-Link chrome (a born netrunner — fast/strong on the net but permanently exposed, can't strip its surfaces), while an **austere blade** ships near-**zero-Link** (the anti-Worm specialist by birth, §7). Built-in liabilities are part of the unit's cost; you buy *more* chrome on top.

**Equipment taxonomy ◆.** A unit's **equipment** (the major purchasable) = **weapons** (§7I) + **augments** (cyberware / bioware, below) + **consumables** (grenades). Every piece carries **weight** (→ physical Initiative, §7C); cyberware additionally carries the **(Link, ICE, Hack-effect)** digital bundle. Lean loadouts strike first; heavy ones hit harder but slower and more exposed.

**Augmentation — bioware vs cyberware ◆.** **Both branches enhance the physical body** — that axis is covered by the pair. The difference is the **digital surface**: cyberware drags a Link/ICE/Hack bundle along (net capability *and* exposure), bioware adds none. It's the chrome-vs-flesh axis at the *build* level:

| | **Cyberware** (chrome) | **Bioware** (bio-augment) |
|---|---|---|
| Profile | physical benefit **+ a (Link, ICE, Hack-effect) bundle** | physical benefit, **no Link** |
| Enhances | the **physical body** (stats, combat) — *plus* the digital realm (Link, hacks, beams, ICE) | the **physical body** only (stats, regen, resilience) |
| Digital exposure | **hackable**: worms, netrunner hacks, hack-effect liability | **none** — unhackable, no Worm exposure |
| EMP (§7I) | **vulnerable** — hardware can be fried | **immune** — no hardware |
| Flesh exposure | (host flesh still Virus-vulnerable) | **Virus-vulnerable** + a **Reject** liability (below) |
| Plays | the **wired / out-tech** build | the **air-gapped / abstain** build (§7H) |

Both buy physical enhancement; cyberware *additionally* buys digital capability — at the cost of a hackable, EMP-able surface. So **digital strength requires cyberware** (you can't be powerful on the net without exposure), while physical strength comes either way. Mixing both → **augmented = double-exposed** (flesh *and* Link). Pure-bio units are strong-bodied but **blind and mute on the net** — the cost of the flesh path.

**Reject — the bioware liability ◆** (parallel to hack-effects). Just as a breached *implant* fires its hack-effect on its owner, an infected *bio-augment* can **reject** — malfunction deleteriously when the host catches a Virus: an adrenal graft floods (self-DoT), a muscle graft seizes (−Initiative), a regen-organ turns septic (−Body, softening bio-resilience). So **all enhancement is double-edged** — cyber on the *hack* vector, bio on the *infect* vector. *(Design option: bioware could instead be simply "safer but digitally useless," with no Reject; Reject is the symmetry choice.)*

**Netrunner / hacker units** = the digital attackers. A hack **targets cyberware via its Link** (reachable only if Link > 0) and resolves **against its ICE** (ICE reduces/blocks; an attacker's **icebreaker breaks** it). Success → disable the implant, deploy a Worm, or **trigger the implant's hack-effect on its owner**.

**Worms trigger *named* hack-effects ◆.** Beyond netrunners, **worms** are a key way the cyberware-liability loop pays off: a worm can **trigger a specific *named* hack-effect** on the target (precision sabotage — trip *that* implant) **or multiple at once** (mass compromise — detonate the whole loadout). So **worm roster and cyberware roster are designed together**: each implant's hack-effect is simultaneously its owner's liability *and* a worm's payload, and the more chrome an enemy runs, the more a worm can do to them. (This is one axis of Worm **variety** — see the worm family, §7.)

**Three attack vectors — all gated by Link, opposed by ICE:**
1. **Runner-directed** — a netrunner deliberately hacks a target's cyberware.
2. **Interaction** — the commit-time trap (§7): acting on a compromised unit.
3. **Ambient proximity** — *no runner needed*: connected units in proximity leak across (worms jump, incidental exposure). Proximity + Link, not deliberate action.

→ **Zero Link = no surface = immune to all three.** Spreading out *and* running quiet (low Link) are the universal digital defenses — so positioning now matters for both the Virus (adjacency) and the Worm (proximity).

**Targeting & identification — the two realms ◆.** *Who* an effect may hit splits cleanly by realm:

- **Delivery shapes (axis 8, §7G):** **single** · **blast** (grenades — thrown AoE) · **line/beam** (**antenna** — a beam emanating from a **netrunner's antenna** along a hex-line; carries worms/hacks).
- **Physical = dumb & honest.** Physical attacks and grenades are **indiscriminate** — **friendly fire is on** (a blast catches your own units). No friend/foe filter and **nothing to spoof**; the only mitigation is **positioning**.
- **Digital = smart & selective, but corruptible.** Digital effects read each unit's **IFF** (Identify-Friend-or-Foe) signal, so beams/hacks can be **selective** — spare allies, strike foes. The friend/foe **targeting controls** that gate digital effects run on IFF.
- **Spoofing (a netrunner tool).** IFF is **code**, so a netrunner can **spoof** it — flip a unit's friend/foe reading: make your selective digital fire hit your **own**, slip an enemy past as "friendly," or blind your auto-targeting. The more your force trusts IFF, the more a spoof turns it inside out — the cyberware-liability theme applied to targeting.
- **Code-only.** IFF, spoofing, and beams are **digital-realm only** — the physical realm has no IFF layer, so a thrown grenade can't be "spoofed" into sparing or hitting anyone; it just hits the hexes it lands on. Physical = honest & positional; digital = selective & hackable.

The IFF-driven **targeting controls** (gating which units a digital effect may hit) are the friend/foe layer; see the **spoof toolkit** and **hack-effect roster** below.


**Spoof toolkit (what an IFF spoof does) ◆.** A spoof corrupts a unit's IFF — a netrunner hack or the **Spoofer** worm (§7G); resolves vs **ICE**, Link-gated, **code-only**:

| Spoof | Effect |
|---|---|
| **Flip-hostile** | unit reads allies as foes → it attacks its own / your selective digital fire hits it (friendly fire on demand) |
| **Masquerade** | an enemy reads as "friendly" → your auto-targeting / selective effects ignore it (a free approach) |
| **Scramble** | IFF returns noise → unit can't use selective targeting (can't fire, or fires blind) |
| **Ghost** | unit reads as absent → untargetable by IFF-gated digital effects (digital cloak; physical still sees it) |

**Hack-effect roster (the implant liability pool) ◆.** Each implant = a benefit **+** a **named** deleterious hack-effect that fires *on the owner* when the implant is breached (by a netrunner or a worm). The worms (§7G) trip these by name (**Logic-bomb**) or all at once (**Cascade**) — so the more chrome, the more there is to detonate. Numbers TBD.

| Implant (benefit) | Named hack-effect (on owner) |
|---|---|
| **Reflex booster** (+Initiative) | **Seizure** — skip / Crash (booster misfires) |
| **Smartgun** (accuracy, IFF-targeting) | **Misfire** — attack an ally or self |
| **Subdermal plating** (+defense) | **Shed** — plating offline (−Plating) |
| **Metabolic pump** (+regen) | **Overload** — Internal DoT (runs hot) |
| **Cyberdeck** (+Link / hacks) | **Lockout** — digital actions disabled / −Link (bricked) |
| **Sensor suite** (perception, range) | **Blind** — can't target / −accuracy |
| **Combat stim** (+damage / haste) | **Overdose** — self-DoT, then Crash |

**Link-effects (proposed menu) ◆** — the Link slot isn't just a number; each implant picks a flavor:

| Link-effect | Does | Cost |
|---|---|---|
| **Uplink** | flat **+Link** → faster digital Initiative, more digital actions, stronger hacks | more exposure (baked in) |
| **Relay / Mesh** | networks **allies** — shared buffs / routed support, wider "connected" range | widens the ambient-Worm surface across your formation |
| **Masking** | **−Link footprint** without losing other functions → lower Worm catch-odds, harder to hack | less digital throughput |
| **Spike** | a **burst of Link** on demand (one big hack / flurry), then drops | momentary exposure spike |
| **Leech** | **offensive** — drain a connected enemy's ICE/Link in proximity; soften for your runners | — |

A unit's digital identity = the sum of its implants' **link-effects + ICE + hack-effects** (liabilities included). *Open (§8): which link-effects ship, and the deleterious-hack-effect severity that makes chrome a real-but-fair gamble.*

---

## 7G. The status-effect system — composed by axes ◆

The payoff of §2 (axes), §3 (damage), §6.6 (triggers) and §7–§7F: **every status is one value (or a few) per axis.** Define the axes once and the roster is just points in that space — which also surfaces what's *missing* and which combos are illegal. Names below are generic/placeholder; numbers are TBD (the tuning pass, §8).

### The schema — 9 axes

1. **Effect** (the payload): DoT · burst · heal/HoT · stat-mod (Initiative / atk / def / ICE / Body / Link) · control (skip / lock) · defense-mod (grant or shred Barrier/Plating) · amplify/convert (vuln / resist) · spread/contagion · trigger-effect (detonate / trip a named hack-effect) · move/reposition.
2. **Trigger** (what makes it act): on-apply · per-turn/tick · on-action · on-hit / on-being-hit · on-attack / kill / death · on-threshold (e.g. below-half) · on-interaction (the Worm vector) · passive/static · conditional/secret.
3. **Timing** (when it resolves in order): start-of-turn (pre-emptive — can kill before the bearer acts) · end-of-turn · on-action (woven into the Initiative order, §7C) · pre-damage · post-damage · immediate.
4. **Decay** (how it ends): permanent · decrement-per-turn (counts down) · fixed-duration · one-shot (consumed on trigger) · cleanse-only · conditional (self-cure) · threshold-gated.
5. **Stacking** (how repeats combine): intensity (more magnitude) · duration (more time) · refresh (reset timer) · independent (separate instances) · count/charges · **capped** (the runaway brake) · binary (on/off).
6. **Magnitude** (numeric scale, §3.4): flat · %max (anti-tank, can kill) · %current (softener, never kills) · per-stack · N/A.
7. **Behavior** (§3.5): deterministic · **stochastic** (rolled each instance; the resist shifts the roll).
8. **Targeting / spread** (§2F, §7, §7F): self · single · **blast/AoE** (grenades) · **line/beam** (netrunner **antenna**) · **adjacency-spread** (Virus) · **interaction-spread** (Worm) · **proximity-ambient** (Worm, no runner) · directed (runner-chosen). *Physical delivery is **indiscriminate** (friendly fire on); digital delivery is **IFF-gated** — selective, but spoofable (§7F).*
9. **Resist / counterplay** (the defense model, §7A): tier (Internal bypasses Barrier+Plating / Contact mitigated by Plating / External hits Barrier) · the **Body** attribute (Virus) · **ICE** (Worm/hacks) · cleanse → Vaccinated/Antimalware · zero-Link (digital immunity) · stat-check self-cure.

### Composition — how a status is built

A status = **one value per axis** (a few axes allow a small set — e.g. two triggers). Most axes are **orthogonal** (magnitude ⊥ tier ⊥ behavior ⊥ trigger), which is what keeps the space large without type-bloat (§3.6). The combinations that *aren't* free — the **constraints**:

- **DoT / HoT effect** → per-turn trigger + start-of-turn timing (the recurring class).
- **Spread targeting** → a contagion effect + a gating resist (Body / ICE) + stochastic.
- **Stochastic behavior** → needs a resist that shifts the roll (otherwise it's just noise).
- **One-shot decay** → an event trigger + count/charge stacking (a proc).
- **Passive/static trigger** → permanent / cleanse-only / decrement decay (no event to consume it).
- **Control effect** → binary or refresh stacking (rarely intensity); short fixed/one-shot decay — it's tempo, not attrition.
- **Strip / shred effect** → an *enabler*: it pairs with a follow-up (ICE-strip opens the Worm; Plating-shred opens Contact damage).

### The roster (first pass ◆)

Columns: **Effect · Trigger → Timing · Decay / Stacking · Magnitude / Behavior · Resist / Spread.**

**A — Contagions** (Internal, stochastic, build over time; the centerpiece, §7):

| Status | Effect | Trigger → Timing | Decay / Stacking | Mag / Behavior | Resist / Spread |
|---|---|---|---|---|---|
| **Virus** (bio) | DoT + **attacks Body** (wasting disease) + self-spread | per-turn → start-of-turn | cleanse-only → *Vaccinated*; intensity, **capped** | %max ramp; **stochastic** | Internal; **Body**, cleanse; **adjacency** |
| **Worm** (code) | DoT + **melts ICE** + self-spread (+ trip named hack-effect) | per-turn / on-interaction → start-of-turn | cleanse-only → *Antimalware*; intensity, **capped** | %max ramp; **stochastic** | Internal; **ICE**, Link-gated, anti-Worm units; **interaction + proximity** |

**Worm strains (Family A, code side) ◆** — all share the Worm base (Internal · stochastic · ICE-resisted · Link-gated · code-only; reaches via runner / interaction / proximity, §7F); they vary by *job*:

| Worm | Job |
|---|---|
| **Spreader** (flagship) | DoT + self-spread (the contagion) |
| **Glitch** | pure Internal DoT — digital rot; no spread |
| **Lockware** | disable an implant / lock digital actions (control) |
| **Logic-bomb** | trigger **one named** hack-effect on the target (§7F) |
| **Cascade** | trigger **all** the target's hack-effects — mass compromise; punishes big loadouts |
| **Drainware** | −ICE / −Link over time — enabler that softens for follow-up |
| **Spoofer** | force an **IFF spoof** (friendly-fire / masquerade, §7F); can self-propagate |

*(Bio **Virus** strains below — the flesh-side mirror.)*

**Virus strains (Family A, bio side) ◆** — all share the Virus base (Internal · stochastic · resisted by the **Body** attribute · **adjacency spread** (positional, *not* Link) · infects flesh: organic + augmented, machines immune; cleanse → *Vaccinated*). They **mirror the worm strains in structure** but attack the **body**, not the chrome:

| Virus | Job | Worm parallel |
|---|---|---|
| **Plague** (flagship) | DoT + adjacency spread | Spreader |
| **Necrosis** | pure Internal DoT; no spread | Glitch |
| **Paralysis** | control — motor lockdown (skip / Crash) | Lockware |
| **Wasting** | −physical stats (attack / Initiative) over time | Drainware |
| **Delirium** | derangement — unit attacks randomly / allies (*biological* friendly-fire, **not** an IFF spoof) | Spoofer |
| **Blight** | −Body — susceptibility amp; softens bio-resilience, snowballs the bio family | Drainware |
| **Spore** | death-trigger — on death a spore-cloud infects neighbors (bio Data-spill, §7G G) | (death-side) |

**Family asymmetry ◆:** parallel in shape, divergent in target. Code worms exploit **cyberware** (Logic-bomb / Cascade have no flesh analog — flesh has no implants); bio viruses exploit the **body** (Spore / Delirium have no chrome analog). The two resist-shredders now line up cleanly across the split — Drainware (−ICE) mirrors **Blight** (−Body) — since both the bio resist (the **Body** attribute) and ICE can be eroded to soften the host. **Augmented units are double-exposed** (flesh *and* Link → both families). And counterplay is **orthogonal**: bio = **high Body + cleanse + spread out** (positional); code = **ICE + low/zero Link** — so going air-gapped dodges Worms but **not** Viruses (the Virus rides adjacency, not Link).

**B — Damage-over-time** (non-contagious):

| Status | Effect | Trigger → Timing | Decay / Stacking | Mag / Behavior | Resist / Spread |
|---|---|---|---|---|---|
| **Burn** | DoT | per-turn → start-of-turn | decrement; intensity | flat; deterministic | **Contact** (Plating mitigates); single/AoE |
| **Bleed** | DoT | per-turn → start-of-turn | fixed-duration; intensity | flat or %max; deterministic | Internal (bypasses Plating); single |
| **Poison** | **pure DoT** (ticks damage; attacks no stat) | per-turn → start-of-turn | cleanse-only or decrement; intensity, **capped** | flat or %; **stochastic** | Internal; **Body** (Internal resist); single (no spread) |
| **Corrode** | DoT + Plating-shred | per-turn → start-of-turn | decrement; intensity | flat; deterministic | Contact; single |

*Poison = the contagions' **non-spreading sibling** — same Internal-stochastic toxin (§3.5), minus the spread: Bleed (deterministic) → Poison (stochastic, stays put) → Virus/Worm (stochastic, spreads).*

**C — Control / tempo:**

| Status | Effect | Trigger → Timing | Decay / Stacking | Mag / Behavior | Resist / Spread |
|---|---|---|---|---|---|
| **Crash** (stun) | skip next action | on-apply → immediate | one-shot or 1-turn; binary | N/A; deterministic | control-resist / ICE (if digital); single |
| **Lag** (slow) | −Initiative | passive → on-action | decrement or fixed; intensity | flat; deterministic | — ; single |
| **Lock** (root) | can't move / reposition | on-apply → immediate | fixed; binary | N/A; deterministic | — ; single |

**D — Amplify / expose:**

| Status | Effect | Trigger → Timing | Decay / Stacking | Mag / Behavior | Resist / Spread |
|---|---|---|---|---|---|
| **Breach** (vuln) | +damage taken / −defense | passive → pre-damage | fixed or decrement; intensity | %; deterministic | — ; single |
| **Mark** | next hit amped / detonates stacks | on-hit → post-damage | one-shot; count | per-stack; deterministic | — ; directed |

**E — Strip / shred** (enabler debuffs):

| Status | Effect | Trigger → Timing | Decay / Stacking | Mag / Behavior | Resist / Spread |
|---|---|---|---|---|---|
| **ICE-strip** | −ICE (opens Worm) | on-apply → immediate | decrement or fixed; intensity | flat; deterministic | — ; single |
| **Immuno-suppress** | −Body (softens bio-resilience; opens Virus) | on-apply → immediate | decrement or fixed; intensity | flat; deterministic | — ; single |
| **Plating-shred** | −Plating tier/value | on-hit → pre-damage | permanent or decrement; intensity | flat; deterministic | — ; single |
| **Barrier-break** | strips External shield | on-hit → pre-damage | one-shot; — | flat; deterministic | — ; single |

**F — Buffs / sustain** (the positive mirror):

| Status | Effect | Trigger → Timing | Decay / Stacking | Mag / Behavior | Resist / Spread |
|---|---|---|---|---|---|
| **Adaptive System** (regen) | HoT | per-turn → start/end-of-turn | decrement or fixed; intensity | flat or %max; deterministic | self/ally; single/AoE |
| **Vaccinated** | virus-resist (−stacks/turn), self-cure | passive → start-of-turn | decrement-per-turn; intensity | flat; deterministic | self (cleanse result) |
| **Overclock** (haste) | +Initiative (Heat risk) | passive → on-action | fixed or decrement; intensity | flat; deterministic | self (risk: Heat) |
| **Hardening** | +defense layer / +resist | on-apply → pre-damage | fixed or decrement; intensity | flat; deterministic | self/ally |
| **Uplink / Spike** | +Link (digital init / throughput) | on-apply/passive → on-action | fixed or one-shot; intensity | flat; deterministic | self (raises exposure, §7F) |

**G — Death-triggered** (Deathrattle-class; fire when the unit is removed — precedent: Hearthstone Deathrattle, MTG dies-triggers, §6.6):

| Status | Effect | Trigger → Timing | Decay / Stacking | Mag / Behavior | Resist / Spread |
|---|---|---|---|---|---|
| **Detonate** | burst to neighbors on death | on-death → immediate (on removal) | one-shot; binary | flat or %max; deterministic | tier varies; **adjacency AoE** |
| **Legacy** | on death, trigger an effect (buff allies / debuff killer / spawn) | on-death → immediate | one-shot; binary | varies; deterministic | self → allies or killer |
| **Data-spill** | on death, release carried Virus/Worm to neighbors/connected | on-death → immediate | one-shot; binary | per-stack; spread | adjacency / Link-gated; **spread** |

*Death-timing knot ◆:* on-death resolves *after* the lethal blow, around the death-check (§6.6). It collides with the contagion brake (§8 #9): a dying carrier either **spills** its stacks (snowbally) or **death-purges** them (a clean runaway brake) — split per strain. Detonate also partially fills the AoE-burst gap; AoE *control* still needs a delivery (below).

**Delivery — grenades & beams ◆.** AoE/line are *deliveries* on axis 8, decoupled from payload (§3.6), each scaled by its **footprint**:
- **Grenade (explosion)** — scales by **radius** (rings on the hex grid, §7B): **footprint 1** = the target hex + its ring (**7 hexes**, default); **footprint 2** = + the next ring out (**19 hexes**).
- **Beam (antenna)** — emanates from a **netrunner's antenna** as a hex-**line**, scaled by **width**: **footprint 1** = a 1-wide line (default); **footprint 2** = a 3-wide band (the line + a hex on each side).

Either carries **any** status — a **Crash grenade** = the AoE stun the roster otherwise lacks, an ICE-strip beam = soften a line for follow-up. **Friendly fire is on (decided):** a physical blast catches your own units; positioning is the mitigation (digital beams can spare allies via IFF — §7F). The **clustering tension** stands — grenades + Virus adjacency + Worm proximity all punish bunching, while Relay/Mesh buffs + front-line density reward it.

*Three distinct terms now: **footprint** = hexes a delivery covers (this) · **magnitude** = hit size (flat / %max / %current, axis 6) · **spread** = contagion propagation (axis 8). Open: footprint cost / range / cap (§8 #12).*

This roster is the *first pass* — it fixes each status's **shape** (which axes it occupies); the **numbers** (magnitudes, durations, caps, roll odds) are the tuning pass (§8 #2, #6, #10), and the contagion/cyberware variants expand families A and E (§8 #3).

---

## 7H. Anti-Worm specialists — the digital defense ◆

The **answer half** of the digital realm. The offense in §7F (worms, hack-effects, spoofs, beams, netrunner hacks) needs dedicated counters, or going online is suicide, nobody builds Link, and the whole layer is wasted. These are the defenders — sitting on top of the baseline defenses (**ICE**, **low/zero Link**, **cleanse → Antimalware/Vaccinated**).

**Two answer-philosophies (the chrome-vs-flesh axis again):**
- **Abstain (flesh).** Run low/zero Link and refuse the net — immune to all of it, but blind and toothless digitally. Archetype: the **air-gapped blade**.
- **Out-tech (code).** Fight code with code — white-hat programs, hardened ICE, signal control. Stays capable on the net but pays slots/tempo for defense. Archetypes: **antivirus / bulwark / signals**.

**Specialist roster ◆** (roles — some units, some deployable programs or abilities):

| Specialist | Role | Counters |
|---|---|---|
| **Air-gapped blade** (flesh) | near-zero Link → immune to digital; strikes infected enemies **without catching** the Worm | the whole digital realm, by abstaining; safe carrier-killer |
| **Antivirus** (killer-program) | hunts and **eats Worm stacks** on enemies; can kill a worm's source | Worm spread / Glitch DoT (enemy-side suppression) |
| **Patcher** (medic) | **cleanses** Worms/Viruses from allies → *Antimalware / Vaccinated* | contagion; Lockware; cures *before* a hack-effect fires |
| **Bulwark** (ICE-projector) | very high ICE, **extended to allies** | hacks, worm-catch, spoofs, Drainware — raises the resist roll |
| **Signals officer** (counter-spoof) | **locks IFF** in range / reveals masqueraders & ghosts / reverses spoofs | the spoof menu (§7F) |
| **Jammer** (ECM) | projects a **dead-zone** — digital attacks weakened/blocked in an area | beams, ambient-proximity, netrunner range |
| **Honeypot** (decoy) | high-Link bait that **draws** worms/hacks, then quarantines / counter-hacks | netrunner targeting; worm spread (dead-ends it) |
| **Quarantine** (isolator) | **severs an infected ally's Link** — forces it dark to halt spread | Worm spread (containment) |

**Coverage check ◆:** every offense has an answer. **Bulwark** and **Patcher** are the load-bearing generalists (between them they blunt spread, DoT, hacks, and spoofs); the **air-gapped blade** is the flesh-side catch-all; **Signals** is the *only* dedicated spoof answer (a deliberate single point — a spoof-heavy enemy *forces* you to tech it); **Jammer** is the only beam/area answer. Watch one thin spot: nothing hard-counters **Cascade** (mass hack-effect detonation) except running **lean chrome** — by design, since the answer to "too much chrome" is "less chrome."

**Balance principle ◆:** every answer must cost something — a slot, a unit, tempo, or going blind on the net. If counterplay is free, digital offense dies and §7F is wasted. The arms race *is* the meta: worms rise → antivirus/jammers tech in → spoofs rise → signals officers → and round it goes.

---

## 7I. Weapons — guns & melee ◆

Physical weapons apply the §7A penetration model (which armor tier mitigates) and add **range** + **footprint** (§7G). Damage type governs *mitigation only* (§3.6) — the status payload is separate. Guns are the **ranged** option; melee is the **close** option and the natural home of the **low-Link / air-gapped** build (§7H).

**Guns (ranged) ◆:**

| Gun | Range | Footprint | Damage type | Signature |
|---|---|---|---|---|
| **Pistol** | short | single | Piercing | baseline sidearm |
| **SMG** | short–mid | volume (multi-hit) | Piercing | shreds light armor by volume; anti-chaff |
| **Rifle** | mid | single / burst | Piercing | generalist |
| **Sniper / Railgun** | long | single | **armor-ignoring** (drops a tier) | the penetrator — beats Plate |
| **Shotgun** | short | **blast/cone** (footprint 1) | Piercing + knockback | anti-group up close; **friendly-fire** |
| **Heavy / LMG** | mid–long | volume / suppress | **Bludgeoning** | beats Plate by mass; slow |
| **Energy / Plasma** | mid | single / line | **Thermal** (Contact) — applies **Burn** | the elemental gun |
| **Launcher** | long | **explosion** (footprint 1–2) | carries any payload | the grenade / AoE platform |

**Melee (close) ◆** — expands §7A's blades; slots into its Bludgeoning / Piercing / Slashing matrix:

| Melee | Damage type | Trait |
|---|---|---|
| **Mono-katana** | Slashing | generalist; balanced |
| **Monomolecular wire** | **Piercing, armor-ignoring** | drops a tier; pierces Plate |
| **Vibro-blade** | Slashing + **armor-soften** | softens Plating one tier |
| **War-maul / gauntlet** | **Bludgeoning** | anti-rigid (beats Plate); knockback / stagger |
| **Naginata / polearm** | Piercing, **+reach** | strikes from one rank back (§7B depth) |
| **Twin blades / claws** | Slashing, **volume** | many fast hits; anti-light |
| **Shock-fist** | Bludgeoning + **Crash on hit** | staggers / stuns |

**Melee = the air-gapped weapon ◆.** You don't need to be online to swing a blade, so melee is the natural fit for **low/zero-Link** builds — making it the weapon of the **abstain philosophy** (§7H): immune to the digital realm, and able to cut a worm-ridden enemy **without catching the Worm** (the cyber-samurai's edge, §7). Guns work at any Link; melee is where going dark pays off.

**Gun mods ◆.** Guns take **mods** (a mod slot); the flagship is the **Smartgun mod** — fits *any* gun and is worth taking:
- **IFF auto-targeting** → unlocks the **sophisticated targeting profiles** (§7J: lowest-Integrity, backline, threat-priority) instead of a dumb "nearest."
- **Fires on *digital* Initiative** (Link-timed) — the gun acts on the digital track too, so a **wired (high-Link) unit shoots fast/early**. A real reason to run Link.
- **Lock-on** — ignores some evasion / can't be juked off the chosen target (the "good effects").
- **Risk:** a smartgun is a **digital surface on your gun** — **spoofable** (a spoof → **Misfire**: it fires on an ally or self, or its targeting profile scrambles, §7F). Even your weapon becomes hackable once it's smart — chrome-is-liability, one more time.

Other mods slot here too — **EMP rounds** (the EMP payload, above), AP rounds, suppressor, extended cell. *(Mod roster + numbers: §8.)*

**EMP weapons — physical attacks on digital hardware ◆.** The physical realm's answer to the digital one. An EMP is a **physical** weapon (dumb, honest, unspoofable, hits its hexes) that **damages Link / antennae / cyberware directly** — and because it *fries hardware* rather than *hacking* it, it **bypasses ICE** (digital defense doesn't apply to a physical pulse). Effects: **−Link** (fry the antenna → less net presence, digital Initiative, surface), **disable cyberware** (implants offline — a strong EMP can even trip their **hack-effects** on the way down, §7F), and **kill beams** (no antenna, no transmission).
- **Forms:** EMP grenade (blast), EMP rounds (gun), EMP baton (melee) — any delivery can carry the payload (§7G).
- **Counters:** high-Link / heavy-cyberware / netrunner builds — the **bruiser's anti-netrunner tool**, no hacking skill required.
- **Mitigated by:** **hardening / EMP-shielding** (Faraday-hardened cyberware), running **bioware** (no hardware → **EMP-immune**), or simply **less chrome**.

This completes the **attack matrix** — physical force can now degrade *digital* capability, the mirror of a worm's DoT degrading *physical* Integrity:

| Attacker ↓ / Target → | **Physical** (Integrity, armor) | **Digital** (Link, cyberware) |
|---|---|---|
| **Physical** weapon | guns / melee — vs armor (§7A) | **EMP** — vs hardening; *ignores ICE* |
| **Digital** attack | worm / virus DoT (Internal, §7) | hacks / worms — vs ICE, via Link |

So heavy chrome is pressured from **two** directions — hackable *and* EMP-able — driving the chrome-is-liability theme home: the more wired you are, the more ways you break.

*(Numbers — range bands, damage values, footprint costs, volume hit-counts — are the tuning pass, §8.)*

---

## 7J. Units — classes, behavior & movement ◆

### Class = chassis; role = loadout

The principle: a unit's **class is its chassis** (innate — what it fundamentally *is*); its **role is its build** (weapons + augments + software + profiles — all swappable in the shop). Most "specialists" are **loadouts, not classes** — you become a netrunner by equipping a cyberdeck + Link + hack programs, a medic by running cleanse software, a sniper by carrying a sniper rifle. Don't make those classes; **devolve them to equipment/software** so any chassis can adopt them.

**Chassis classes (innate)** — the bio / cyber / synthetic axis:

| Chassis | Nature | Contagion exposure | Link floor | Notes |
|---|---|---|---|---|
| **Flesh** (organic) | biological body | **Virus**-vulnerable | low/zero (needs cyberware for Link) | the baseline; home of bioware + air-gapped builds |
| **Augmented** (cyborg) | flesh + chrome | **both** — double-exposed | mid–high | capable-but-exposed middle; most versatile |
| **Machine** (drone / construct) | pure synthetic | **Virus-immune**; Worm-vulnerable *only if* Link > 0 | varies | flesh-immune, all-digital; bioware n/a |

Chassis sets base stats, which contagions bite, the Link floor, and whether bioware/cyberware apply. **Drafting = pick a chassis (class); build the role in the shop.** The §7H "specialists" resolve as loadouts: antivirus / jammer / signals / quarantine / honeypot = **programs**; bulwark / air-gapped blade = **builds**.

### Movement profiles (scripted)

It's an auto-resolved battler, so units don't take manual orders each turn — they follow a **movement profile** you assign (you *program* the unit). A unit **moves only if not boxed in** (it needs a free adjacent hex; surrounded = stuck — a real vulnerability vs AoE/contagion, which punish the immobile cluster).

| Profile | Behavior |
|---|---|
| **Advance** | close to melee / optimal range — the frontline pusher |
| **Hold** | stay in formation — anchors, walls |
| **Kite / Retreat** | back off to keep range — fragile / ranged |
| **Flank** | swing to the enemy's side or backline (corners / seam, §7B) |
| **Swarm** | converge on the nearest / weakest enemy |
| **Disperse** | spread out — anti-AoE, anti-contagion (counters clustering, §7G) |

Movement + board geometry (§7B) + weapon range (§7I) = the tactical layer.

### Targeting profiles (scripted)

Each unit also follows a **targeting profile** — who it attacks:

| Profile | Picks |
|---|---|
| **Nearest** | closest enemy (the default / dumb) |
| **Lowest Integrity** | finish the wounded (executioner) |
| **Highest threat** | the biggest danger |
| **Backline / role** | reach past the front for the netrunner / medic / artillery |
| **Weakest armor / type-match** | exploit the penetration model (§7A) |

**Dumb weapons** fire on simple profiles (nearest / forced). The **Smartgun mod** (§7I) unlocks the **sophisticated** ones (lowest-Integrity, backline, threat-priority) and fires on **digital Initiative**. So smart targeting is a *digital capability* — and therefore **hackable**: a spoof scrambles the targeting profile (Misfire, §7F).

**Theme payoff ◆:** you **program your units** (movement + targeting profiles), and because that programming is *code*, the enemy can **hack it** — spoofs, Lockware, and worms don't just damage units, they **corrupt their behavior**. Scripting your squad is the cyberpunk fantasy; the enemy rewriting your script is the threat.

---

## 8. Open questions / to-verify

**Resolved (through v0.25):** **§10 Full rules drafted** — the complete play loop assembled into a runnable procedure (round build + interleaved activation order, move-then-act activations, the 8-step attack-resolution sequence, contagion & digital-combat procedures, death triggers, the economy, a turn-sequence quick-reference). First-pass **procedural calls** (marked ◆, open to revision): win = eliminate the enemy army; defense order **Barrier → Plating → Integrity**; hack **margin = hack-power − ICE**; Initiative tie-break = other-track-then-random; basic attacks auto-hit (rolls reserved for contagion). These resolve §8 #5 / #13 and the Barrier/Plating-order question · **units layer (§7J)** — **class = chassis** (Flesh / Augmented / Machine, the bio/cyber/synthetic axis), **role = loadout** (netrunner / medic / sniper / the §7H specialists all devolve to equipment + software); **scripted movement profiles** (Advance / Hold / Kite / Flank / Swarm / Disperse — units move if not boxed in) + **targeting profiles** (Nearest / Lowest-Integrity / Threat / Backline / Weakest-armor); you **program your units**, and the enemy can **hack the script** (spoof / Lockware corrupt behavior) · **Smartgun = a gun mod** (fits any gun) — IFF auto-targeting (unlocks smart targeting profiles), **fires on digital Initiative**, lock-on; spoofable → Misfire (§7I) · **Firewall renamed to ICE** (Intrusion Countermeasures Electronics — the one digital Internal-resist) · **Immunity folded into the Body attribute** (bio Internal-resist is Body itself, no standalone stat; Health folded into Body too — three primary attributes: Body, Dexterity, Intellect) · **cleanse names swapped** — Virus → **Vaccinated** (bio), Worm → **Antimalware** (code) · HoT "Patch" → **Adaptive System** · **Heat fleshed (§7D)** — *if adopted*, a full **thermal layer** (overclock + heat-scaling, thermal-signature exposure, overheat risk; vent / coolant counterplay), giving a **third exposure axis** (Link = digital, Heat = thermal, position = physical); bare overclock is cut · **augmentation = bioware vs cyberware (§7F)** — chrome-vs-flesh at the build level: both branches enhance the **physical body** (axis covered); cyberware adds a (Link, ICE, Hack-effect) digital surface (net capability + exposure, hackable + EMP-able); bioware adds **no Link** (unhackable, EMP-immune) but Virus-vulnerable + an optional **Reject** liability; augmented = double-exposed · **EMP weapons (§7I)** = *physical* attacks that fry **Link / antennae / cyberware**, **bypassing ICE** — the physical counter to digital builds; completes the attack matrix (physical→digital) · **equipment taxonomy**: weapons + augments + consumables, each with weight (§7C) · **bio-Virus strains drafted (§7G)** — Plague / Necrosis / Paralysis / Wasting / Delirium / Blight / Spore, mirroring the worms in shape but attacking the body (both contagion families now complete; counterplay orthogonal — Body+positional for bio, ICE+Link for code) · **weapons drafted (§7I)** — guns (pistol / SMG / rifle / sniper / shotgun / heavy / energy / launcher) + melee (katana / monomolecular / vibro / maul / naginata / twin-blades / shock-fist); **melee = the air-gapped weapon** (low-Link); guns take **mods** (Smartgun is the flagship — see v0.24) · **anti-Worm specialists drafted (§7H)** — the digital defense: air-gapped blade / antivirus / patcher / bulwark / signals officer / jammer / honeypot / quarantine; two answer-philosophies (abstain-flesh vs out-tech-code); every offense has a costed counter (the arms race = the meta) · **footprint size term = "footprint"** (settles "mag"/"spread"; now three distinct terms — footprint = area, magnitude = hit size, spread = contagion) · **netrunner beam emitter = "antenna"** · **delivery footprints**: explosion scales by **radius** (footprint 1 = hex + ring = 7, footprint 2 = + next ring = 19); beam scales by **width** (footprint 1 = 1-wide line, footprint 2 = 3-wide band); both default footprint 1 (§7G) · **"status controls" = IFF-driven targeting controls** (confirmed) · **Poison added** — the contagions' non-spreading Internal-stochastic sibling (§7G B) · **code-worm strains drafted** (Spreader / Glitch / Lockware / Logic-bomb / Cascade / Drainware / Spoofer, §7G) · **hack-effect pool drafted** (Seizure / Misfire / Shed / Overload / Lockout / Blind / Overdose, §7F) · **spoof menu drafted** (Flip-hostile / Masquerade / Scramble / Ghost, §7F) · **friendly fire is ON** (physical AoE indiscriminate; positioning mitigates — §8 #12) · **antenna beam** = a netrunner **line/beam** delivery (carries worms/hacks along a hex-line) · **IFF** (friend/foe) makes *digital* effects selective, but a netrunner can **spoof** it (turn your fire on your own / masquerade / blind targeting) — **physical = dumb & honest** (no IFF, unspoofable), **digital = smart & corruptible**; IFF/spoofing/beams are **code-realm only** (§7F) · **death-triggered (Deathrattle) family** — Detonate / Legacy / Data-spill, with the death↔contagion spill-vs-purge knot (§7G, §8 #9) · **full status-effect schema (§7G):** every status = one value per **9 axes** (effect / trigger / timing / decay / stacking / magnitude / behavior / targeting / resist), with composition constraints + a ~23-status first-pass roster across 7 families · working title **Chrome and Code** · **cyberware = the major purchasable**, each implant a **(Link, ICE, Hack-effect)** bundle — improvement *and* risk (a breached implant's hack-effect fires on its owner, §7F) · **some units ship with inbuilt/pre-applied equipment** (unit identity, often non-removable) · **netrunners** attack cyberware **via Link, vs ICE** (icebreaker breaks it) · digital attacks reach via **3 vectors** (runner / interaction / ambient proximity), all Link-gated · **worms trigger *named* hack-effects** — single or multiple (precision vs mass compromise) · **"Worm" & "Virus" are families** (varied effects), flagship = the spreading strain · **two Initiative tracks, interwoven**; **digital Initiative *is* Link** (§7C/7D) · **unified load model**: gear = weight (−physical Init) + (Link, ICE, Hack) bundle (+digital Init, +exposure); concurrency taxes each realm · **two resists** (asymmetric): bio-resist *is* the **Body** attribute (vs Virus), **ICE** vs Worm — and the Worm also has active anti-Worm specialists · bio **Virus** = adjacency spread / resisted by **Body** / infects flesh (and **attacks Body** — a wasting disease dragging Integrity) · code **Worm** = **interaction spread**, **Link-gated**, **ICE**-resisted (the worm **melts ICE**), infects **Link > 0**; commit-time trap + **low-Link melee = anti-Worm specialist** · **Link** = a **continuous, equipment-set net-presence stat**; **zero Link = immune to all digital attack** · prior locks: types ≠ statuses, stochastic rolls, Fire=Contact DoT, HP→Integrity, meshed-seam board, cyberpunk/cyber-samurai theme.

**Still open (design intent):**
1. **Link-effects roster (§7F):** which of Uplink / Relay / Masking / Spike / Leech ship, and what each costs in numbers.
2. **Hack-effect severity (§7F):** how punishing a breached implant's deleterious hack-effect is — the dial that makes stacking chrome a real-but-fair gamble. Needs clear telegraphing.
3. **Family rosters (§7G/§7F/§7I):** *both contagion families drafted* — code worms (Spreader / Glitch / Lockware / Logic-bomb / Cascade / Drainware / Spoofer) **and** bio viruses (Plague / Necrosis / Paralysis / Wasting / Delirium / Blight / Spore); *hack-effect pool* + *weapon rosters* (guns & melee, §7I) drafted. Remaining: **how a worm picks** which named hack-effect(s) to trip, and all numbers.
4. **Ambient-proximity rule (§7F):** exactly when proximity + Link leaks a Worm with no runner — range, Link threshold, per-turn roll.
5. **Netrunner attack resolution (§7F):** hack-power vs ICE math, and what a successful hack can do (disable / deploy Worm / trip hack-effect).
6. **Worm catch math (§7):** base per-interaction odds, how ICE lowers and Link raises them; confirm no same-action re-spread; do connected *attacks* transmit, or only connective actions?
7. **Load curves (§7C/§7D):** how steeply equipment/multitasking penalize physical Initiative and drain Link — sets lean-vs-heavy balance.
8. *(drafted §7H)* **Anti-Worm specialists** — roster drafted (air-gapped blade / antivirus / patcher / bulwark / signals officer / jammer / honeypot / quarantine), with a two-philosophy split (abstain vs out-tech) and a coverage check. Remaining: strength vs the offense (numbers), and which are **units vs deployable programs/abilities**.
9. **Runaway brakes (§7):** ≥1 hard brake **per virus** (stack cap / cleansed-immune-K-turns / threshold-gated spread / death-purges-carrier) — and decide the **death interaction (§7G):** does a dying carrier **spill** stacks to neighbors (Data-spill) or **purge** them (the brake)? Split per strain.
10. **Magnitude (§3.4):** which effects are % vs flat. ~~**Heat — full thermal layer (§7D) in or out?**~~ — **resolved: dropped for now** (cut from scope; §10.10 parked). **Weapon numbers** (§7I — range bands, damage, volume hit-counts, footprint costs) + **Barrier/Plating order** (§7A). **Faction layer (§7E):** neo-Japan clans/dueling/honor in scope or aesthetic?
11. **Inbuilt equipment (§7F):** removability rules (which integral implants can/can't be stripped) and how built-in chrome is priced into a unit's cost.
12. **Grenade & beam tuning (§7G/§7F):** footprint **shapes defined** — explosion scales by **radius** (footprint 1 = 7 hexes, footprint 2 = 19), beam by **width** (footprint 1 = line, footprint 2 = 3-wide); both default footprint 1. Remaining: **footprint cost / range / cap**. *Friendly fire = on (decided).*
13. **Spoof resolution (§7F):** *spoof menu drafted* (Flip-hostile / Masquerade / Scramble / Ghost); *physical-weapon question resolved* — the **Smartgun mod** (§7I) is the one IFF-guided (thus spoofable) gun upgrade; dumb weapons can't be spoofed. Remaining: how a spoof **resolves** (hack-power vs ICE, Link-gated).
14. *(resolved v0.15)* **"Status controls" = IFF-driven friend/foe targeting controls** (confirmed). Status-manipulation tools, if wanted, would be a separate system — not currently planned.
15. **Augment rosters & EMP tuning (§7F/§7I):** a **bioware roster** (specific bio-augments, paralleling the implant list); confirm whether bioware carries the **Reject** liability or is just "safer but digitally useless"; **EMP numbers** (−Link amount, whether it trips hack-effects) and the **hardening / EMP-shield** defensive item.
16. **Profiles tuning (§7J):** which **movement** (Advance / Hold / Kite / Flank / Swarm / Disperse) and **targeting** (Nearest / Lowest-Integrity / Threat / Backline / Weakest-armor) profiles ship; move **speed/range** per turn; exact **boxed-in** rule; how a unit's profile is assigned/swapped (and what hacking it does).
17. **Chassis stats & gun-mod roster (§7J/§7I):** base stat lines + Link floors for **Flesh / Augmented / Machine**; the **gun-mod roster** (Smartgun, EMP rounds, AP, suppressor, cell) and numbers.

**To corroborate before relying (⚠):**
- Backpack Battles **Poison** decay (persistent vs decaying); BB Block/Regen/Empower/Spikes persistence.
- Monster Train **Spell Weakness** decay; MT **Armor / Damage Shield** exact numbers.
- Darkest Dungeon **Bleed/Blight** stack & decay specifics (per-turn flat vs counting down).
- Wildfrost **Snow/Frost/Spice** decay/duration.
- StS **Frail** exact value (−25% Block gained, duration-decay — widely cited, not re-confirmed here).
- PoE **Bleed** "×3 while moving" multiplier (general ARPG knowledge — verify exact figure before quoting).

---

## 9. Sources

Reported mechanics drawn from the following; ✓✓ rows had 2+ of these agree with differing wording.

**The Bazaar:** official wiki (Effects/Burn/Poison) (thebazaar.wiki.gg); Mobalytics; Game8; TV Tropes; bazaar-builds.net.
**Slay the Spire (1 & 2):** Fandom wiki (Poison/Weak/Vulnerable/Block/Plated Armor); slaythespire.wiki.gg (Debuffs/Block/Plated Armor); StratGG powers; NeonLightsMedia Ironclad guide; SpireSpy/maybelater (Metallicize); NamuWiki; Spire Codex; Steam discussions.
**Wildfrost:** wildfrostwiki.com (Shroom); Fandom Status Effects; TV Tropes; Gameranx; Shark Games.
**Monster Train (1 & 2):** SuperCheats; Metabomb; ChapterCheats; Pro Game Guides; Neoseeker; Monster Train 2 Miraheze (Frostbite/Decay); TV Tropes (Reap); Steam discussions.
**Backpack Battles:** backpackbattles.wiki.gg (Game Mechanics/Stun); Fandom. *(Distinct from Backpack Brawl — backpackbrawl.wiki.gg — and Backpack Hero — backpackhero.wiki.gg.)*
**Across the Obelisk:** Fandom/ato.fandom (Keywords/Effects, Glossary, Damage Types); gameplay.tips; steamah perk & conditions guides; Steam discussions.
**Darkest Dungeon (1 & 2):** darkestdungeon.wiki.gg (Status effects/Riposte/Category); Fandom (Status/Guard); NamuWiki; DD2 Fextralife.
**Reference vocabularies:** Hearthstone wiki.gg + Fandom (Ability/Keywords/Bonus Keyword), gamerant, egw.news, metatierlist; Path of Exile Fandom (Shock) + maxroll PoE2 + Steam; **Warframe wiki.warframe.com + Fandom (Damage/Toxin/Shield), Pro Game Guides, thegamer, 1v9** (layered shield/armor/health + damage-type bypass — the closest precedent for §7A); Wowhead (Virulent Plague); All The Tropes; MTG Fandom + Draftsim (Infect/Toxic). *Borderlands / Mass Effect / StarCraft cited in §7A from general knowledge, not freshly verified.*

**Triggers (§6.6):** Magic: The Gathering Comprehensive Rules 112/603 via MTG Wiki (Fandom), Draftsim, MTG Salvation, magicthegatheringauthority — triggered/activated/static frame; Hearthstone wiki.gg + Fandom (Triggered effect, Battlecry, Deathrattle) + hearthstonetopdecks, outof.games — named triggers & resolve-before-deathcheck; The Bazaar (mobalytics, bazaardb, bazaar-builds, thebazaarzone patch notes) — cooldown/on-use/on-crit/below-half/per-second triggers.

**Type → status systems (§3.6):** Warframe wiki.warframe.com + Fandom (Damage, Status Effect, per-type procs) + thegamer, 1v9 — ~13 type→status pairs at scale; Cyberpunk 2077 Fandom (Status Effects, Contagion) + gamerant, segmentnext, eip.gg — 4 damage types → statuses + spreading-virus quickhack (**cited as precedent, not copied**); Path of Exile ailments (general ARPG knowledge).

**Auto-battler & mecha mechanics (§7D/§7E — cited for mechanics; theme is cyberpunk):** Mechabellum (TV Tropes, Steam discussions, Wikipedia, NamuWiki) — async plan-then-auto-resolve, flat-armor-vs-chaff, status-immunity coating, hacker units; BattleTech (battletech.fandom How-to/Heat/Combat, gamerterra, GameFAQs) — armor-over-internal-structure ✓✓ with Mechabellum, heat-as-resource, stability/knockdown, called shots.

 Mount & Blade Warband/Bannerlord (TaleWorlds forums, Steam guides, calradiawar) + Battle Brothers (armor/fatigue) — three-tier blunt/pierce/slash × material; Kingdom Come Deliverance 1 & 2 (wiki.gg, Fandom, RPGSite, method.gg) — layered armor & stab/slash/blunt; League of Legends Wiki (Health; % max/current/missing scalings); Risk of Rain 2 wiki (Acrid/Blight % HP poison); TV Tropes (Percent Damage Attack). *M&B + KCD independently agree on the blunt = anti-rigid / pierce = penetrator shape (different wording → ✓✓ for §7A's matrix); exact cell values not standardized across games.*

*Patch-sensitivity: The Bazaar and Backpack Battles are actively balanced; numeric values (not the mechanics' shape) may shift between patches. ATO/StS/MT/Wildfrost/DD mechanics are stable across their current versions.*

---

## 10. Full rules (first pass) ◆

*The complete play procedure, assembling §§2–7J into a runnable loop. **[Bracketed] = a number still TBD.** **◆ = a procedural call made here for the first time** — the rules force decisions the systems only implied (resolution order, tie-breaks, hit/hack math); all are open to revision. Several resolve §8 items, noted inline.*

### 10.1 Object of the battle
Two armies face off across the seam. **A battle is won by eliminating the enemy army** — every enemy unit reduced to 0 Integrity ◆; specific battles may instead set an objective (hold a hex, survive **[N]** rounds, destroy a target) — TBD. In the roguelike run, winning a battle advances you; losing your army ends the run.

### 10.2 Setup
Each player deploys their units on their board half — **columns = depth ranks**, the long edge = frontage (§7B). Roll the **seam offset** (±½ hex ◆) to set the cross-board front-line pairings. Each unit enters with its full profile:

> **chassis** (Flesh / Augmented / Machine) · the three primary attributes **Body / Dexterity / Intellect** (Body carries Integrity/HP, melee damage, **and** bio Internal-resist) · **Integrity** · **Barrier**, **Plating** (+ material) · **physical Initiative** · **Link** (= digital Initiative) · **ICE** (digital Internal-resist) · **weapon(s) + mods** · **augments** (cyberware / bioware) · **software** · a **movement profile** · a **targeting profile**. *(Heat — a thermal axis — was here; dropped for now.)*

### 10.3 Round structure
A battle runs in **rounds** until one army is gone. Each round:

1. **Build the activation order.** Every unit contributes up to two activations into one shared order:
   - a **physical activation**, ranked by its **physical Initiative**;
   - a **digital activation**, ranked by its **digital Initiative (= Link)** — only if Link > 0 and it has a digital action available.
   The two **interleave** into a single order, highest Initiative first, physical and digital mixed (§7C). **Tie-break ◆:** higher value on the *other* track, then random.
2. **Resolve activations** top to bottom (10.4).
3. **Cleanup:** end-of-round decay / duration ticks (10.6); check for elimination.

### 10.4 Activations
**At the start of any activation,** resolve the unit's **start-of-activation statuses** — DoTs and **contagion ticks** fire here (10.7) and *can kill the unit before it acts* (§7C).

**Physical activation** ◆ = **move, then act:**
- **Move** per the movement profile (10.5a), if able.
- **Act:** one **attack** per the targeting profile (10.5b–c), or use an ability.

**Digital activation** = one **digital action** — a hack, worm deployment, spoof, or program — resolved by 10.8.

*A unit thus acts up to twice per round — once in the world, once on the net — at its two Initiative positions.*

### 10.5 Movement & attacks
**(a) Movement.** Move up to **[move]** hexes toward the **movement profile's** goal — Advance (toward nearest enemy / optimal range), Hold (stay), Kite (away to keep range), Flank (toward enemy side / backline), Swarm (toward nearest / weakest), Disperse (away from allies) — pathing only through **free hexes**. **Boxed in** (no free adjacent hex) → no move (§7J). Occupied hexes block movement and pathing.

**(b) Target selection** (targeting profile). Choose the target matching the profile — Nearest / Lowest-Integrity / Highest-threat / Backline-role / Weakest-armor (§7J) — among **valid targets** in range/reach. **Dumb weapons** use simple profiles (Nearest / forced); the **Smartgun mod** unlocks the smart ones (§7I). **Digital effects choose via IFF** — and can be **spoofed** (10.8).

**(c) Attack resolution** — in this order:
1. **Range / reach.** Melee = adjacent (+reach for polearms); gun = within its **range band**; **beam** = along the hex-line; **explosion** = its footprint (§7G). No target in range → no attack.
2. **Footprint & friendly fire.** Mark every hex/unit in the footprint (single / blast radius / beam width). **Physical AoE is indiscriminate — it hits your own units too** (§7F). Digital effects spare allies via IFF unless spoofed.
3. **Hit ◆.** Attacks **auto-hit** a valid target; **evasion / cover [TBD]** can force a miss; **Smartgun lock-on** ignores evasion. (Basic hits are *not* rolled — *stochastic* behavior is reserved for contagion / Poison, 10.7.)
4. **Penetration vs defense ◆** *(resolves §8: Barrier/Plating order).* For each target, apply the attack's **penetration tier** (§7A):
   - **External** → **Barrier** absorbs first → then **Plating** mitigates → then **Integrity**.
   - **Contact** → bypasses Barrier → **Plating** mitigates (by material × the **Bludgeoning / Piercing / Slashing** matrix, §7A) → **Integrity**.
   - **Internal** → bypasses **both** → straight to **Integrity**.
   - **Pierce** keyword = drop the outermost remaining tier.
5. **Magnitude.** Compute damage — **flat** / **%max** / **%current** (§3.4). Stochastic effects roll here (§3.5).
6. **Apply.** Reduce Barrier / Plating / Integrity in tier order by the final value.
7. **On-hit statuses.** Apply any statuses the weapon/effect carries (Burn, Crash, Breach, a contagion, …) per 10.6.
8. **Death check.** Integrity ≤ 0 → destroyed → 10.9.

### 10.6 Status effects
A status is applied with its **stacks / duration** and behaves per its **9-axis** entry (§7G): it **stacks** by its rule (intensity / duration / refresh / charges / capped), resolves at its **timing** (start-of-activation DoTs, pre-/post-damage modifiers, on-death, …), and **decays** by its rule (decrement / fixed / one-shot / cleanse-only / conditional). Within an activation: **start** (DoTs / contagion tick) → **action** → **end** (decay / duration tick) ◆.

### 10.7 The two contagions
An infected unit carries **[stacks]** of a Virus or Worm strain (§7G). **At the start of its activation, the contagion rolls (stochastic, modified by the resist):**
- **Tick** — **[weak] Internal** damage to Integrity (the **Body** attribute lowers the Virus roll, **ICE** the Worm roll). The **Virus attacks Body** as a wasting disease — because Body *is* the Integrity/HP pool, chipping it drags Integrity down, and because bio-resist *is* Body it also softens the host for the next strain; the **Worm melts ICE** as its rot. (The bio **Poison** is a pure DoT — flat tick damage, no stat attack, resisted by Body.)
- **Build** — +1 stack [odds rise with stacks].
- **Spread** — roll to infect one eligible target [odds rise with stacks, lowered by the target's resist]:
  - **Virus** → a **flesh** unit (organic / augmented) in an **adjacent hex**.
  - **Worm** → a **Link > 0** unit the carrier **interacts with** (buff / heal / hack / connected action) **or** within **ambient proximity [range]** while connected.

**Commit-time trap (Worm) ◆.** Actions lock at planning, so queuing an interaction with a *clean* unit can still infect it by the time it resolves — and your actor catches it back (ICE-modified roll; **no same-action re-spread**) (§7).

**Resist & self-cure.** The **Body** attribute (Virus) / **ICE** (Worm) lowers tick damage and build/spread odds; at **[threshold]** the unit **self-cures**. **Cleanse** removes **[value]** stacks and applies **Vaccinated** (Virus) / **Antimalware** (Worm) — a decaying buff that strips **[value]** stacks/turn and grants temporary resist. **Runaway brake [required]:** ≥1 hard cap per contagion (§8 #9). **On death:** the strain either **spills** (Data-spill → neighbors) or **purges** its stacks.

### 10.8 Digital combat
**Reachability.** A digital attack can target a unit **only if Link > 0** (zero-Link = immune), reached by a **runner**, an **interaction**, or **ambient proximity** (§7F).

**Hack resolution ◆** *(resolves §8 #5/#13).* **Margin = hack-power − target ICE** (the attacker's **icebreaker breaks** the ICE); success if **> 0**, with effect magnitude scaling on the margin. A success can: **disable** an implant, **deploy a worm**, **trip the implant's hack-effect on its owner** (Seizure / Misfire / Shed / Overload / Lockout / Blind / Overdose, §7F), or **spoof** the unit's IFF — **Flip-hostile** (attacks its own), **Masquerade** (enemy reads friendly), **Scramble** (can't target), **Ghost** (untargetable by IFF effects).

**EMP** (physical). Damages **Link / antennae / cyberware** and **bypasses ICE entirely** (no hack roll — it's a physical pulse): −Link, disable cyberware (may trip hack-effects), kill antennae / beams. Mitigated by **hardening / bioware** (§7I).

**Beam (antenna).** A digital **line** attack (footprint by width) carrying worms / hacks along the hex-line; IFF-gated, spoofable.

### 10.9 Death & removal
Integrity ≤ 0 → the unit is **destroyed and removed**. Resolve its **death triggers** on removal (§7G): **Detonate** (blast neighbors), **Legacy** (its effect), **Data-spill / Spore** (release its contagion). A carrier's contagion **spills or purges** per strain (10.7).

### 10.10 Heat *(dropped for now, §7D)*
**Cut from scope** — parked, not deleted. The design, for the record: a unit's **Heat** rises from overclocking, energy / EMP weapons, and heavy cyberware. Effects: **overclock / heat-scaling** (upside), **thermal signature → +detectability / targetability** (exposure), and at **[ceiling]** → **throttle (−Initiative) / shutdown (skip) / Integrity damage** (risk). **Vent** (skip to cool) or **coolant** gear reduces it.

### 10.11 Between battles — the economy
In the roguelike shop, spend **[currency]** to draft / upgrade units and buy **weapons + mods, cyberware, bioware, software, and consumables** — and assign each unit a **movement** and **targeting profile**. Augments are the major purchasable; every piece adds **weight** (→ physical Initiative) and cyberware adds the **(Link, ICE, Hack-effect)** bundle (§7C / §7F).

### 10.12 Turn-sequence summary (quick reference)
1. **Round start** → build the interleaved physical + digital activation order (Initiative high → low).
2. **Each activation:** start-of-activation (DoTs / contagion tick — *may kill*) → **move** (movement profile) → **act** (attack via targeting profile, *or* a digital action) → resolve **penetration → defense → magnitude → Integrity → on-hit statuses → death**.
3. **Round end** → decay / duration ticks; check army elimination.
4. **Battle end** → between-battle shop; next battle.
