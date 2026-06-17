# CHROME AND CODE — Design Delta v0.26

*Post-v0.25 synthesis. Extends [`../status-effects-taxonomy.md`](../status-effects-taxonomy.md) (the survey-grounded design doc) and [`../CHROME-AND-CODE.md`](../CHROME-AND-CODE.md) (the at-a-glance spec). Captures the design decisions made after v0.25. **Numbers throughout are TBD** — this fixes shapes, not values. ◆ = synthesis/decision, not a sourced game fact.*

> **Naming convention (decided):** **street names** — the gritty handle register, not honorifics. **IP stays air-gapped:** mechanics-as-flavor under original/generic vocabulary; trademarked names are cited-not-copied and flagged inline (e.g. avoid *Trauma Team*, *Netrunner*-as-a-class label).

---

## 1. The three-axis identity model ◆

A unit's identity is **three orthogonal axes**, not one class:

| Axis | The question | Set when | Governs | Hard/soft |
|---|---|---|---|---|
| **Chassis** | what are you *made of* | innate (draft) | contagion / EMP / morale exposure, base stats | **hard** (innate) |
| **Faction** | who do you *run with* | origin / recruit | economy, per-faction **Rep**, build *biases*, aesthetic | **soft** (economy) |
| **Role** | what are you *equipped for* | shop (re-speccable) | combat function | **free** (loadout) |

- **Chassis** stays as locked in §7J: Flesh / Augmented / Machine.
- **Role** stays as locked: role = loadout; netrunner / medic / solo / specialists all devolve to equipment + software (§7J).
- **Faction is new (§2):** it tilts **cost/access**, never capability. A Corp *can* field an air-gapped blade — it just costs different currency/Rep than a Clan fielding the same. **Build-shaping, not role-gating.**

---

## 2. The faction layer ◆

**Unifying principle:** *each faction is the economy for one pillar* — some **make** combat gear, some **sell services**. The faction map mirrors the systems map on the money side — your Rep portfolio = which subsystems your build can afford to lean on.

**Economy & Rep are emergent — by unit summation, not a player-held balance ◆.** There is no abstract player-level Rep number or treasury accumulated directly; **your standing and income *are* the running sum of your units' contributions.** Each unit carries an economic + per-faction-Rep profile alongside its combat stats, and the totals are **recomputed live** as the roster changes — recruit a unit and its contribution joins the sum; bench, lose, or **KO** one and its contribution drops out (the mechanism behind §9.4). The player shapes economy and Rep *only through the roster*: who you field, who they're affiliated with, and who survives. So every unit is at once a **fighter, an income source, and a Rep-vector** — dropping one costs all three. (Notable stance: Rep is **current-roster, not banked** — you can't stockpile standing and then fire the units that earned it; see §11 for the flow-vs-stock dial.)

**Factoring:** factions are **engine-blind.** A unit carries a `faction` tag that only `atomica-run` (economy/Rep) and `atomica-content` (aesthetic) read; the battle `sim` never knows factions exist. Zero coupling.

### 2.1 The Corps — a rival market (makers & service corps)

They **compete**: a Rep portfolio is a balancing act, and cozying up to one sector is debt with its rival. Corps split into **makers** (vendor combat gear — a loadout pillar) and **service corps** (shape the run economy, no gear of their own). Names are placeholders.

| Sector | Type | Example | Sells / economy | Shapes build toward |
|---|---|---|---|---|
| **Cyberware** | maker | *Halcyon Cybernetics* | premium chrome, Link gear, hack programs | heavy-cyber, netrunner hardware |
| **Medical** | maker | *Helix Biomed* | Doctors, bioware, Vaccinated — *and* engineered Virus | bio-augment, sustain, contagion |
| **Military-Industrial** | maker | *Bastion Defense* | guns, armor, EMP, war-drones / mechs | firepower, armor, EMP |
| **Vehicles** | maker | *Velocity Motors* | transports, mobility, ramming rigs | vehicle armies, mobility |
| **Financial** | service | *Meridian Capital* | currency, income, interest | buy-power, mercs |
| **Insurance** | service | *Sentinel Assurance* | **no gear of its own** — **pre-combat** coverage: a **money payout** when an insured unit dies (it still earns next phase via the payout — but insurance **never prevents the death**); **resells** makers' gear at a **discount** | death-payout, economic defense |
| **Media** | service | *Sygnal* | Rep, intel, info-war (vendors spoof / signals *software*) | signals, spoof offense, Rep manipulation |
| **Real Estate** | service | *Bedrock Holdings* | **not the battle board** — safe-houses, roster / stash capacity, between-battle recovery, run-map holdings | logistics & staying power |

The split keeps each honest to its real business: **Insurance** doesn't *make* anything — it sells **coverage** (a policy that revives or compensates a lost unit) and **resells** makers' gear cheap; **Financial** sells **capital**; **Real Estate** sells **staying power** (base, roster, recovery — *not* in-battle positioning); **Media** sells **information** (Rep, intel, and the info-war software behind IFF / spoof). The makers fill your loadout; the service corps shape your run.

### 2.2 Cops — the order faction

The lawful counter-pole to the street. Economy/build: **control & authority** (forced-target / Taunt, Crash / Lock lockdown), **surveillance** (reveal stealth, counter-spoof — Signals), **area lockdown**. Optional subdivisions: **Metro PD** (street control), **Corp-Sec** (private — overlaps Mil-Industrial), **Feds** (anti-net surveillance).

Introduces a **Notoriety** axis ◆ (distinct from thermal Heat): criminal dealings raise it; high Notoriety makes cop factions field enemies against you in the run.

### 2.3 Families / Clans — loyalty & blood ◆

Loyalty economy: melee, bioware, vehicles, **morale / Rally**, air-gapped. Loyalty-deep but **exclusive** (go in with one, you're suspect to its rivals). Starter set:

| Clan | Identity | Economy / build |
|---|---|---|
| **Steel Lotus** | neo-Japan cyber-samurai; the blade, honor, the duel | air-gapped melee, bioware, high Resolve |
| **The Haul** | road-nomad convoy clan; family, the open road | vehicles, reinforcements, mobility |
| **Nine Wires** | gutter syndicate; contraband & black-market chrome | stims (Overclock / Overdose), cheap illicit gear, fixers |
| **The Grafted** | flesh-cult body-sculptors; reject the chrome | bioware extremes, Virus / regen, Reject-resistant |

### 2.4 Independents — the unaffiliated baseline ◆

Freelancers, no perks and no penalties: **Runner crews** (the net-economy independents — cheap decks / software), **mercs** (flexible guns-for-hire), and **Fixers** (the Brokers who trade *everyone else's* Rep). Flexible and cheap.

### 2.5 The Rep market — the shop becomes politics ◆

Rep is **per-faction**, and the factions are a web of rivalries (corps compete · cops vs. street via Notoriety · clans demand exclusivity). Consequences:

- Rep portfolio sets **access, price, and exclusive stock** at each vendor.
- **Who you've angered supplies your enemies** — anger Bastion Defense → fight their war-drones; rack up Notoriety → Cops raid you. **Your economic choices pick your boss fights** (the roguelike-run engine).
- **Fixer / Media** are the levers that convert Rep across the web.

### 2.6 Equipment affiliation & the generic tier ◆

Equipment carries its **own affiliation, separate from the unit's faction.** A Steel Lotus blade can field a generic pistol *and* a Halcyon cyberdeck; affiliation gates **acquisition** (Rep / legality), **never use**.

| Affiliation | Gate | Character |
|---|---|---|
| **Generic** (unaffiliated) | none — open-market, always stocked | reliable baseline; the basics of *every* pillar; no signature gimmick |
| **Corp** (sector) | sector Rep | specialist / premium depth in one pillar; branded quirk |
| **Clan** | loyalty Rep | themed (blades / bioware / vehicles); often inbuilt identity gear |
| **Law Enforcement** | restricted / licit | control & surveillance gear; illicit acquisition → **Notoriety** |

Principles:
- **Generic covers the basics across all pillars; factions deepen one.** No capability is locked behind a faction — factions add depth/edge, not gates (upholds "build-shaping, not role-gating," §1).
- **Generic = the economy floor:** always available at zero Rep, so a fresh draft is playable; it's also the **1.0× balance baseline** faction gear is tuned against.
- **Upgrade path:** start generic → invest Rep → swap up to branded specialist gear in your chosen pillar(s).
- Orthogonal **illicit flag** (grey / black market): some generic *and* stolen faction gear is illicit → raises **Notoriety** (§2.2); licit generic is politically inert.

*The faction rosters above (corp sectors, clans, LE subdivisions) are **open scaffolds** — they'll expand. The generic tier is the constant beneath them.*

---

## 3. The sustain triad + cross-pool healing ◆

### 3.1 Equipment-condition state (enabler)

"Field repair" only matters if damage disables *capabilities*, not just Integrity. So every piece of gear (implant, weapon, armor layer) carries a condition:

`Online → Degraded → Offline → Destroyed`

EMP and hack-effects (Shed, Lockout, Blind, Seizure) knock slots **Offline**; Plating-shred / physical wear **Degrades**. That's the "field damage" hacks *and* kinetics both cause — and what gives repair a job.

### 3.2 The three menders — one per realm

Mirrors the §7I attack matrix:

| Realm | Restores | Role | Cures | Faction home |
|---|---|---|---|---|
| **Bio** | Integrity (flesh) | **Doctor** | Virus → Vaccinated; Reject | Clan (flesh path) |
| **Chrome** | Barrier / Plating; un-bricks Offline gear | **Ripperdoc** | EMP / Shed / Lockout damage | back-alley **Independent** ↔ premium **Corp** clinic |
| **Code** | Firewall, Link; clears corruption | **White-hat** | Worm → Antimalware; spoofs | Runner |

(All three names are generic genre / real-world vocabulary — IP-clean.)

### 3.3 Cross-pool — scope scales with magnitude ◆

Keep the baseline **siloed**, let the **big abilities cross**:

- **Small / cheap / on-tick → single-pool.** Keeps counterplay legible ("their Doctor can't fix the chrome you fried — so EMP the line").
- **Big / rare / expensive → cross-pool.** Signature abilities and high-end consumables span two or three pools — the marquee "stabilize *everything*" play. The triad is the readable default, *not* a rigid three-slot tax.

**Four pools** a mender can touch (the biggest reach the fourth):

1. **Integrity** (bio) · 2. **Barrier / Plating + equipment-online** (chrome) · 3. **Firewall / Link + de-corrupt** (code) · 4. **Resolve** (morale, §4)

Marquee cross-pool ult names (original — avoid the *Trauma Team* trademark): **Hard Reset**, **Cold Boot**, **Crash Cart**, **Code Blue**, **Dust-off**.

**Engine note:** scope is just a `pools: [...]` field on a heal effect — single- vs cross-pool is how many pools it lists. Still the §3/§7G status schema; engine-cheap, content-tuned.

---

## 4. Morale — the Resolve layer ◆

A **second pool**, **Resolve** (the mind's Integrity), built from the existing status schema:

- **Stress** = a DoT on Resolve (the **Mind / Psychic** damage type §3.1 reserved; Darkest Dungeon's Stress bar is the precedent).
- **Resolve 0 → Break:** *rout* (forced Kite / Disperse, can't attack) or *berserk* (forced Advance, hits nearest incl. allies).
- Triggers: nearby ally death, flanked, leader lost, heavy hit → Stress; kills, Rally, winning → restore.

### 4.1 The behavior-corruption symmetry ◆

A unit's **script can be corrupted from all three realms** — the payoff of adding morale:

| Vector | Corrupts behavior via | Immune chassis |
|---|---|---|
| **Digital** | spoof / Lockware / Worm (§7J) | zero-Link / Flesh |
| **Bio** | **Delirium** virus strain (§7G) | Machine (no flesh) |
| **Psych** | **morale Break** (rout / berserk) | Machine (no mind) |

→ **Machines don't panic and don't go delirious** but are the most digitally exposed; **Flesh** is mind/bio-fragile but can go air-gapped. Sharpens the chassis rock-paper-scissors.

**Leaders** project **+Resolve / Rally** to their formation (morale's Bulwark — the *Anthem* archetype); losing the leader → morale cascade.

---

## 5. Vehicles ◆

Two genuinely new **engine** mechanics (everything else this delta was content):

- **Multi-hex occupancy** — a vehicle spans 2–3 hexes. Board / adjacency / footprint (Phase 1) must handle a unit covering multiple hexes: bigger contagion surface, easier blast target, blocks more lanes.
- **Containment (crew / transport)** — units ride *inside*; the vehicle is their outer armor layer. On destruction, crew **spill** into adjacent hexes (a death-trigger), possibly damaged. Mounted weapons are crewed by occupants.
- **Extraction (= the survival lever, §9.4)** — a transport **loads a downed unit and drives off the board**, which **removes both the vehicle and the rescued from the battle.** The tactical cost: you pull the wheels (and their guns) out of the fight to save your people. **Nomad == vehicle** — the extraction route *is* the vehicle, not a separate ability.

**Vehicle chassis** (heavy Machine variant): high Integrity + Plating, big weight → low physical Initiative, **Virus-immune** (no flesh) but **EMP / Worm-exposed** if networked, **bright signature** (draws fire, AoE magnet — ties to Heat). Mobility: high move, can **ram** (kinetic + knockback). The Clan **Haul** vendors them; the vehicle *is* the nomad fantasy.

---

## 6. PAN — Personal Area Network ◆ (new)

The **intra-unit / local** tier of the digital realm — the mesh linking a unit's own implants. Three tiers now:

**PAN** (a unit's own implants) → **Squad Mesh** (Relay / Mesh link-effect, §7F) → **the wider net** (ambient / runner reach).

What the PAN buys the design:
- **Explains Cascade.** A breach **rides the PAN** from one implant to all — which is exactly why Cascade trips *every* hack-effect (§7F/§7G). The PAN is the internal road the worm drives.
- **New defensive lever — PAN segmentation.** **Meshed PAN** (default): implants network → cross-implant **synergy / set-effects**, full throughput, but a breach can **Cascade**. **Segmented / air-gapped PAN**: implants isolated → breach **contained** (no Cascade), but **no cross-implant synergy** and reduced throughput. The internal mirror of the squad-level "cluster vs. disperse."
- **Updates the Cascade counter (§7H):** Cascade now has a *second* soft counter — **compartmentalize your PAN** — alongside "run lean chrome." Still no hard counter, by design.
- **Mesh = networked PANs:** the Relay / Mesh link-effect stitches allied PANs together (shared buffs) — and widens the **ambient-Worm** surface across the formation.

---

## 7. AR — Augmented Reality ◆ (new)

The **perception / information** layer connected units see — the substrate beneath targeting, IFF, and Marks.

- **Smart targeting, IFF, and Marks read the AR overlay.** A unit blind to AR (low / zero Link, or **Blinded / Scrambled**) falls back to **dumb nearest-targeting**. This is *why* low-Link melee swings at the nearest enemy — it can't see the picture.
- **Spoofing = editing the AR overlay.** Flip-hostile / Masquerade / Ghost are edits to what units *see*: paint an enemy's tag "friendly," or ghost a unit out of AR entirely (§7F). AR gives spoofing a concrete substrate.
- **Stealth / Ghost = off the AR grid;** **Mark = an AR tag painted on a target** (lit up → smart effects prioritize it); **Recon / sensors** expand your AR picture; **Jammer** collapses AR in an area → forces dumb targeting + kills IFF.
- **Heat ties in:** a hot unit is **bright in AR** — the thermal-signature exposure (§7D) is literally an AR-detection effect.
- **Info dominance** = the Media / Cops / Signals meta — who controls the AR picture wins the targeting war.

### 7.1 Link unified ◆

Link = a unit's projection into **two shared layers**:

| Layer | Link governs |
|---|---|
| **AR** | what it *sees* and how it's *seen* (perception / targeting / IFF) |
| **PAN → net** | what it can *hack* and what can *reach* it (connectivity / attack surface) |

**Zero-Link = off both:** unhackable (PAN air-gapped) *and* AR-blind (dumb nearest-targeting, no smart profiles, no IFF, can't see Ghosts). "Going dark" now costs **perception**, not just offense — one dial, two layers. Low-Link melee is **blind-but-safe**.

---

## 8. The archetype roster ◆

> **Full authored roster + locked faction/clan/law names:** [`rosters.md`](rosters.md). The below is the schema + a sample.

An **Archetype** is a content preset (the §7F "inbuilt equipment = identity" lever): a chassis + non-removable signature gear + one signature ability + default profiles. Everything else is shop-bought & re-speccable.

```
Archetype { callsign · chassis · faction · link_floor · inbuilt[] · signature · default_move · default_target }
```

| Handle | Faction | Chassis | The fantasy |
|---|---|---|---|
| **Last-Mile** | Clan (Steel Lotus) | Augmented | air-gapped sword-courier — the quiet blade, the worm-carrier killer |
| **Glasshouse** | Clan (Grafted) | Flesh | all-bio EMP-proof juggernaut — nothing to hack or fry |
| **Stitch** | Clan | Flesh | **Doctor** — Integrity, Virus cure, Reject; the flesh medic |
| **Anthem** | Clan | Augmented | the leader — projects Resolve / Rally; the morale anchor |
| **Rig** | Clan (The Haul) | Vehicle | crewed transport — multi-hex, ram, spills crew on death |
| **Null** | Runner (Indep.) | Augmented | console-cowboy — high-Link, Spike, glass-jaw netrunner |
| **Patch** | Runner (Indep.) | Augmented | **White-hat** — Worm cleanse, Firewall / Link restore |
| **Ironclad** | Corp (Mil-Ind.) | Augmented | the anchor — subdermal Plate + Firewall projector |
| **Hollowpoint** | Corp (Mil-Ind.) | Machine | smartgun drone — Virus-immune; spoof its IFF and it guns your line |
| **Broker** | Independent | Flesh | the **Fixer** — run-layer; Rep → discounts, slots, intel |

Coverage: all 3 chassis, the full Link dial, both contagions, both initiative leans, every signature kind, and every faction category. *(Runner & Corp want a few more rows — a worm-slinger, a jammer, a heavy-drone — to feel as full as the Clans.)*

---

## 9. Jobs — faction contracts (PvE) ◆

The **Rep + gear faucet.** Alongside standard battles (which pay currency / survival), the run map offers **Jobs**: PvE contracts a faction posts, with a **non-standard objective** and often **oddball requirements**, paying **Rep + gear** instead of the usual win-rewards. Jobs are *how you build per-faction Rep and acquire branded gear* — the **Fixer** (§2.4) is the broker.

**Not PvP, objective-driven.** The opposition is scripted; the goal isn't "wipe them," it's the contract — and several objectives are **board-space** (hold / reach / escort a hex or node), not elimination.

### 9.1 Objective types (replace "eliminate the enemy")

| Objective | Win when… |
|---|---|
| **Survive** | you last **N** rounds |
| **Hold** | you control a target **hex / node** for N rounds *(board-space)* |
| **Extract / Heist** | a unit reaches a target space and exits *(board-space)* |
| **Escort / Protect** | a VIP / cargo / vehicle survives to its goal |
| **Assassinate** | a specific enemy dies (ignore the rest) |
| **Take the dive** | you **lose** — but by **no more than X** (a controlled, convincing loss) |
| **Time attack** | you win within **N** rounds |

A Flight carries a **list** of these — the node's standard **WinFight** (awarded for winning the fight) plus any bonus goals — and can **meet any number** of them independently; rewards are **commensurate** with what's achieved. The standard fight still drives termination.

### 9.2 Requirements (the oddball entry / run conditions)

- **Endorsement** — field **≥X gear from a named vendor** (a sponsorship / proving-ground deal → builds *that* vendor's Rep).
- **Handicap** — melee-only · no netrunning · no chrome · a named unit must field.
- **Preserve** — a named unit (or all allies) must survive.
- **Budget cap** — a weight / Link / cost ceiling.

### 9.3 Rewards & cost

- Pays **Rep** (with the offerer — primary) **+ gear** (often branded, otherwise gated) + occasional unlocks.
- **Forgoes** the standard currency win-reward — a Job is taken *instead of* a normal fight. **Take-the-dive** goes further: you give up the win itself, trading the match for the payout.
- The offerer's **rivals may sour** on you (the Rep web, §2.5).

### 9.4 Death, extraction & the downtime economy ◆

**Downed, then extracted — or dead.** A unit at 0 Integrity is **downed** (a Death's-Door grace state, §6.4), not instantly gone. **Extraction is the *only* thing that prevents death:**

| Outcome | Condition | Result |
|---|---|---|
| **Recovered** | **extracted** (pulled off the field in time) | survives → returns next battle |
| **Dead** | *not* extracted | **permanently lost** — collect its death-offsets (below); only extraction would have saved it |

**Extraction comes from two sources ◆:** a **vehicle** (the Rig, §5) — it loads the downed unit and **drives off the field**, which **removes both the vehicle *and* the extracted character(s) from the battle** (the cost: you pull the wheels out of the fight to save your people); or an **Extraction membership** — a pre-paid medevac outfit that lifts your downed out within a response window, no vehicle committed. *Commit the wheels, or subscribe.*

**Reward gradient ◆ — kills first, extraction close behind (decided).** The reward economy (XP §10, economy, scoring) is **kill-forward**: a **kill** pays the most (rewarding aggression and decisive play); an **extraction** pays **slightly less** — rescuing a downed unit is an *actively good, rewarded* play, not mere damage control, but you'd marginally rather land the kill. Keeps rescues worthwhile without letting the game turtle on them.

**The permadeath-rate dial is deferred ◆** — the downed→dead window and extraction difficulty are *feel* numbers that can only be tuned against the built game; see §12 #16 (⏳ resolve in playtest once code exists).

The **run still ends only on a battle loss** (army wiped), but you can now **bleed units permanently across a run while winning** — real stakes. *(Revises the earlier "always resurrect next battle" rule.)*

**On death — two offsets, neither prevents the death ◆:**
- **Insurance is *money*** (pre-paid, financial) — a **payout** when an insured unit dies, standing in for its **next-phase economic contribution** (a dead insured unit still "earns" via the payout). A bet on who falls; it **never saves the unit.**
- **Salvage / Medical** is *materials* — harvest the dead unit's detachable gear / chrome (Ripperdoc) + biomatter / **Medical benefit** (Doctor); more loss pays more.

A dead unit can be **both insured and salvaged** (money + materials), but it's still **dead** — extraction is the lone survival lever.

**Two-tier stakes ◆:**
- **Generic units are fungible** — salvage + insurance recover most of their value; losing one is a resource hit, then re-buy. Don't over-invest saving them.
- **Characters carry *unsalvageable* essence** — their **signature, inbuilt identity (§7F), and earned skill levels (§10)** can't be harvested *or insured back*. Money, gear, and chips return; **the character — and its mastery — is gone for good.** **Extraction is the only way to keep a character** — which is what makes named units worth pulling out at any cost.

**Downtime economy** (the live unit-summation, §2): a downed unit contributes **nothing** to the segment it fell in unless **insured** (the payout subs in); a Recovered unit rejoins the sum next battle, a Dead one never does. So —
- **Clean wins pay more** — fewer casualties → more contributors next segment.
- **Pyrrhic victories are taxed** — and now can cost you *units*, not just income.
- **Take-the-dive (§9.1) compounds** — throwing a match downs your units; without extraction you bleed roster (insurance softens the *money*, not the loss). Price it in.

*(Extraction mechanics, salvage tables, and the unsalvageable-essence model: §11.)*

### 9.5 Factoring

- **Engine:** generalize the verdict into an injected **`Objective`** (eliminate / survive-N / hold-hex / extract / protect / margin-loss / time) the orchestrator checks each tick — one more IoC seam (Phase 4). **Margin-loss** compares the army-strength differential against X.
- **Run:** `atomica-run` owns the **Job** (offerer, requirements, Rep + gear, run-map node); loadout **requirements validate at deploy time**, keeping the sim objective-only (Phase 10).
- **Casualties:** the sim handles **downing + extraction** (who fell, who got pulled out); `atomica-run` resolves **Recovered** (extracted) vs **Dead**, then applies the death-offsets — **Insurance** payout + **salvage** — drops downtime contributions per §9.4, and carries survivors forward. The sim owns the *event*; the run owns death's *consequences*.
- **Resolves** the taxonomy's §10.1 "alternate objective" TBD.

---

## 10. Skills & character progression ◆

The **RPG layer** — what a unit *knows*, separate from what it *is* (chassis) or *carries* (gear). A fourth identity dimension, and the engine behind the character/fungible split.

**Skills modify rolls ◆.** A skill shifts the **stochastic rolls** (§3.5) in its domain — hacking bends the hack-power-vs-Firewall margin, medical the cure / heal roll, a blade the crit / contagion-catch roll. Skills sit beside the resist stats (Immunity / Firewall) as the per-character roll-modifiers; deterministic effects (a flat Burn) ignore them, rolled ones don't.

**Two sources, asymmetric ◆:**

| | **Character skill** (earned) | **Skill chip** (chipware) |
|---|---|---|
| Bound to | the **character** — innate / earned | the **chip** — equipment |
| Transferable | **no** | **yes** (strip & re-install) |
| Level ceiling | **high** (mastery) | **low** (basics only) |
| Grows | **yes** — use earns **XP** → raise the skill | no — fixed at the chip's level |
| On death | **unsalvageable** — dies with the character | **salvageable** — recover the chip |

- **Character skills grow with use:** a unit that *uses* a skill earns **XP**, and XP **raises the skill** → bigger roll modifiers. A leveled character is **irreplaceable progression** you can't buy back.
- **Skill chips democratize the basics:** slot a chip to give *any* unit low-level competence (a hacking chip on a bruiser for basic netrunning) — transferable and salvageable, but **capped low.** Mastery is earned-only.
- **Chassis baseline ◆:** every **chassis** ships a **low** innate level in its native skills (Flesh → flesh/melee, Machine → gunnery, etc.); XP-growth and chips build *from* that floor. So a fresh recruit is competent-but-unremarkable, and the gap to a veteran is earned.

**The engine of the two-tier stakes (§9.4) ◆.** A character's **earned skill levels are the core of its unsalvageable essence** — lose a leveled character and that mastery is *gone* (money and chips return; the skill doesn't). So skills are *why* characters are worth extracting at any cost, and chips are the fungible counterpart. The investment compounds: the more a character grows, the more it's worth pulling out.

**Factoring:** skills live on the unit in `sim` — one more input to the seeded roll check, beside resists. The sim emits **skill-use events**; `atomica-run` persists **XP / level-ups** on the unit's record between battles. Chips are equipment (content + shop), salvageable on death. (Phase 3 roll-modifier hook; Phase 10 progression.)

---

## 11. Where it lands (factoring)

| Addition | Layer | New engine? | Touches phase |
|---|---|---|---|
| Three-axis identity / factions / Rep / Notoriety | `atomica-run` + `content` | none (engine faction-blind) | Phase 10 |
| Equipment-condition (online/offline) | `sim` | small new state | Phase 2 + 6 |
| Sustain triad + cross-pool (scope field) | mostly `content` | one engine "repair" op | Phase 3 + 9 |
| Morale: Resolve pool + Stress / Rally + Break | `sim` + `content` | new pool + behavior override | new sub-phase after Phase 5 |
| Vehicles (multi-hex + crew) | `sim` | **yes — the big one** | extends Phase 1 + new phase |
| **PAN** (segmentation, Mesh, Cascade road) | `sim` | implant-graph state + a spread channel | Phase 7 + 8 |
| **AR** (perception layer; targeting/IFF/Mark read it) | `sim` | a visibility/IFF lens over targeting | Phase 5 + 8 |
| **Jobs / Objectives** (alt win conditions + contracts) | `sim` (Objective seam) + `atomica-run` (contracts) | injected `Objective` | Phase 4 + 10 |
| **Skills** (roll-modifiers) + XP / chips | `sim` (roll hook) + `atomica-run` (XP / levels) | roll-modifier input + progression | Phase 3 + 10 |

None of it breaks the crate split or phase ordering.

---

## 12. Open questions added this session

*Tagged by how each resolves — **🎯 Design** (decide on paper now) · **◑ Mixed** (settle the structure now, tune the numbers later) · **⏳ Playtest** (feel numbers — need the built game). Tally: **7 design-now · 8 mixed · 2 playtest-only.** **→ Working-pass resolutions in §13 supersede #1–3, 5–8, 11–15, 17 (structure decided; ⏳ numbers remain); #9, #10 authored → [`rosters.md`](rosters.md). Still fully open: just #4, #16 (⏳ playtest).***

1. **[◑ Mixed]** **Per-faction Rep math** — how standing maps to price / access tiers; how the Fixer / Media convert Rep across the web.
2. **[◑ Mixed]** **Notoriety** — how criminal dealings raise it and how it seeds cop-faction enemies in the run.
3. **[🎯 Design]** **Corp rivalry graph** — which sectors are enemies; does courting one *cost* Rep with its rival, or are they independent?
4. **[⏳ Playtest]** **Cross-pool ability costs** — the magnitude→scope curve; what a 2- vs 3-pool heal should cost.
5. **[◑ Mixed]** **Morale tuning** — Resolve pool sizes, Stress magnitudes, Break thresholds, rout-vs-berserk split, Rally radius; Machine morale-immunity confirmed.
6. **[🎯 Design]** **Vehicle rules** — occupancy footprint shapes, crew capacity, disembark / spill-damage on death, ram resolution.
7. **[◑ Mixed]** **PAN segmentation** — the synergy bonuses you forfeit to compartmentalize; throughput penalty; is it a software toggle or a build commitment?
8. **[◑ Mixed]** **AR detection** — visible-in-AR rules, sensor range, what Jammer / Blind / Scramble do to the AR picture numerically; Heat→AR signature curve.
9. **[✅ Authored → rosters.md]** **Faction rosters** — Runner / Corp archetypes filled; 5 clans + 3 cop tiers locked; 19-archetype first-pass roster.
10. **[✅ Authored → rosters.md]** **Naming pass** — corp / clan / law names locked; street-name register confirmed.
11. **[◑ Mixed]** **Generic vs. faction gear** — how much edge branded gear buys over the generic 1.0× baseline; the licit/illicit split and its Notoriety cost; whether *any* gear (vs. only specialist depth) is ever truly faction-exclusive.
12. **[🎯 Design]** **Jobs** *(frequency ⏳)* — run-map availability / frequency; how margin-loss "X" is measured (army-strength differential? surviving units?); does *failing* a Job cost Rep or just forfeit the reward; can you abandon mid-Job; how endorsement requirements interact with the generic tier.
13. **[🎯 Design]** **Casualty economy** — each unit's downtime contribution model (flat? by tier/cost?); do **mid-battle revives** (a mender standing a downed unit back up before battle's end) count as "survived" for downtime; multi-segment downtime — do Recovered units sit out one segment or several.
14. **[🎯 Design]** **Economy as flow vs. stock** — since economy & Rep are a live unit-summation (§2): is spendable currency a per-segment **flow** (set by current roster, use-it-or-lose-it) or does it **bank** into an accumulated stock? Does Rep **drop** when a contributing unit leaves the roster (pure live sum), or **ratchet** (units build a standing that persists)? The dial sets how punishing roster churn is.
15. **[◑ Mixed]** **Casualty-offset dials (§9.4)** — Insurance pre-combat: premium cost, per-unit vs. blanket, and the purchase window; Medical-benefit conversion: salvage vs. claim model, what "more loss → more benefit" curves to, and whether it harvests the lost unit's *gear*; can a unit carry **both** offsets, and do they stack?
16. **[⏳ Playtest]** **Extraction & salvage dials (§9.4)** — *sources resolved* (a **vehicle** exiting the board — removing it *and* the rescued — or an extraction membership); *reward gradient resolved* (kills > extraction, slightly). **⏳ Deferred to playtest (needs code):** the **permadeath-rate dial** — the **downed→dead window** (how long a downed unit survives awaiting pickup — the Death's Door clock), extractions-per-battle, membership response time, and whether extraction costs tempo/risk. These are *feel* numbers, untunable on paper. Plus the **salvage tables** (what gear / biomatter a death returns).
17. **[◑ Mixed]** **Skills & progression (§10)** — the skill list and which rolls each modifies; the **XP curve** and whether levels persist across *runs* (meta-progression) or reset each run; the **skill-chip level cap** and slot cost; when a character's own skill and a chip cover the same domain, do they **stack or take the max**?

---

## 13. Decisions — working pass (v0.26) ◆

*Resolutions from the design-working session; these **supersede** the matching §12 items. Specifics marked ◆ are my fill on the user's call — override freely. Numbers still ⏳ where noted.*

**Roll resolution — the core mechanic: `3d6 + skill + equipment` vs. TN, no stats.** ◆
- **When you roll:** only **contested / stochastic** actions — hacks, spoofs, contagion (tick / build / spread), skill-gated abilities. **Basic attacks auto-hit** (the deterministic damage pipeline, §10.5 of the taxonomy); the dice live in the digital / contagion / skill layer, keeping the auto-battler fast.
- **The roll:** `3d6 + skill + equipment` vs. a **Target Number**. **Margin = total − TN**; **margin > 0 succeeds**, and the **margin's size = degree of success** (scales the effect — a bigger hack margin trips a stronger effect; a bigger contagion margin pushes a higher tier / harder-to-resist). This **subsumes** the old `margin = hack-power − Firewall` and the contagion **T1/T2/T3** roll into one rule.
- **Skills attack, stats defend.** The *only* additive bonus is **skill** (character or chip — take-the-max, chips capped, §10) **+ equipment** (deck hack-rating, weapon mod, …). **No chassis stat is added to a roll.** Instead the defender's stat line *is* the TN: **Firewall** (hacks / Worm / spoof), **Immunity** (Virus), a control-resist — thresholds you roll *against*, never bonuses to your own roll.
- **Why 3d6 — the bell curve.** Mean 10.5, tight spread → **skill dominates, luck is a small nudge**, big upsets rare (~0.5% each extreme). Fits the seeded-RNG, replayable, low-swing feel over d20 randomness.
- **Defaults ◆ (tunable):** succeed on **≥ TN**; **nat 3 = fumble, nat 18 = crit** (rare, splashy). **Calibration ⏳** — skill ranges, equipment bonuses, and TN bands set so a matched contest (skill + equip ≈ TN − 10.5) sits near 50%. *(Also closes the taxonomy's "resolution math" TBD, §8 #5.)*

**Economy & Rep (#14) — currency banks, Rep is live.** Currency accumulates into a spendable wallet (save for big buys); **Rep is a live unit-summation that drops when a contributing unit leaves** the roster. You can stockpile money, never standing — fire the earners and your Rep falls. Reconciles "spending must work" with "Rep isn't banked" (§2).

**Corp politics (#3) — coupled rivals, *dynamic per run*.** Courting a corp **costs Rep with its rival**; the market forces alignment and your standing picks your enemies. **The rivalry graph is rolled fresh at run start** — *no fixed pairs* — so every run is a new political map to read and adapt to; it's **stable within a run** (you can still plan), rolled by the run layer (`atomica-run`) and seedable for replays. The one constant is the category-level **Cops ↔ street** opposition (clans / independents), via Notoriety (#2). *(Thematic pairings like chrome↔flesh or road↔tower can surface as flavored/weighted outcomes, but none is permanent.)*

**Morale Break (#5) — by unit trait.** Each unit's Break is a trait ◆: **Steadfast → rout** (flee: forced Kite/Disperse, can't attack) vs **Feral → berserk** (attack nearest, incl. allies). Machines immune (no mind, §4). Disciplined archetypes (Steel Lotus) resist Break and lean rout; feral ones berserk.

**Skills (#17) — hybrid progression, survival-earned; take-the-max.** Skills/XP **mostly reset each run**, with a small **meta-unlock earned by survival ◆:** a character that **lives through a full run** banks a permanent fraction (a starting skill level, or graduates to a recruitable **veteran** template). So meta-progression is *earned by keeping veterans alive across a whole run* — the extraction fantasy at run scale. **Character skill vs chip in the same domain → take the max** (chips are a floor, not additive; redundant once innate exceeds the chip cap).

**Jobs (#12) — resolved (frequency ⏳).** Margin-loss "X" = **the enemy's remaining army-strength at your defeat** (lose, but leave them ≤ X — a costly, convincing loss). **Failing forfeits the reward** + a **minor Rep ding** (unreliability). **Abandoning** = a fail. **Endorsement requires the named vendor's gear** — generic doesn't satisfy it (that's the sponsorship).

**Casualty economy (#13) — contribution by tier; up-at-end contributes.** Downtime contribution **scales with unit tier/cost** (veterans worth more — losing them stings more). A unit contributes **only if it ended the battle *up* (not downed)** — a clean **mid-battle revive that holds counts**; any downed unit (extracted/recovered *or* dead) contributes nothing that segment **unless insured** (§9.4). Recovered units sit out **one** segment, full next battle.

**Vehicles (#6) — resolved.** Occupancy **2 hexes** (heavy = 3). **Ram** = a move-attack dealing **Bludgeoning/Contact** scaled by mass + **knockback**; friendly-fire on. **Crew spill** on destruction → adjacent free hexes with minor damage; **no free hex → the occupant is downed**. Capacity is a per-vehicle stat (content).

**PAN segmentation (#7) — a build commitment.** Meshed (synergy, Cascade-vulnerable) vs segmented (no cross-implant synergy, Cascade-proof) is **chosen at loadout, not a free in-battle toggle** — a build identity, not a panic button.

**Gear exclusivity (#11) — access-gated, with signature exclusives.** Affiliation gates **access/price, not capability** — nearly all gear is buyable (high Rep, or grey-market at a Notoriety cost). Exception: each faction keeps a few **signature / inbuilt** items that *are* exclusive (a corp's flagship implant, a clan's signature blade).

**Rep mapping (#1, structure) — tiers.** **Hostile → Neutral → Trusted → Inner-circle**, gating access (locked → available → discounted → exclusive stock), price scaling down as Rep rises. Thresholds ⏳.

**Notoriety (#2, structure) — escalating interdiction.** Illicit gear, anti-cop actions, and certain Jobs raise Notoriety; tiers trigger escalating **cop raids** (forced Job-like encounters) in the run. Rates ⏳.

**AR (#8, structure) — visibility rules.** **Visible-in-AR = targetable by smart/IFF effects**; low/zero-Link or **Blinded/Scrambled = off-AR** → dumb nearest-targeting. **Jammer** collapses AR in-area; **sensors** expand it; **Heat** raises a unit's AR signature. Ranges/curves ⏳.

**Casualty offsets (#15, structure) — per-unit insurance, salvage-as-claim, stackable.** Insurance is **per-unit, pre-combat** (pick who to insure — the bet). Salvage is a **tier-scaled claim** (payout per lost unit by value), not gear-by-gear harvesting. A dead unit can be **both** insured and salvaged (money + materials stack). Premiums/curves ⏳.

**Still genuinely open after this pass:** #4 cross-pool costs (⏳), #16 permadeath-rate (⏳). #9 rosters + #10 naming **authored → [`rosters.md`](rosters.md)**.

---

## 14. Shops & vendors ◆

The between-battle storefronts. **Each faction is a vendor** (makers sell gear, service corps sell services), plus universal stops. Currency buys; **Rep gates access/price** (§13 #1); illicit stock raises **Notoriety** (§13 #2). All `atomica-run` + `atomica-content`.

**Market structure ◆ — two vendors per category, two channels.** Each shop category opens a run with a **choice of two vendors** (more unlock over the run via Rep / exploration / events) — never a monopoly. Vendors come in two channels:
- **General vendors** carry a **mishmash** across categories (generic-tier stock, §2.6): convenient, always-available, **Rep-neutral** — no relationship, no politics. The open-market floor.
- **Direct-from-manufacturer** is a faction's own storefront — branded, specialist, signature stock, better depth — but **buying direct builds Rep with that manufacturer** (patronage), and, via the **dynamic rivalry** (§13 #3), **costs Rep with its current run-rival**. *Where you shop is a Rep play.*

So Rep has two inputs: **fielding affiliated units** (the live unit-summation, §13 #14) and **patronage** (buying direct). *(Open ⏳: whether patronage-Rep is a separate persistent track or feeds the same live sum — and how many vendors/category unlock over a run, gated by what.)*

**Realm symmetry ◆:** **Arms** (physical weapons + armor) ∥ **Programs** (digital software) — one offense-*and*-defense shop per realm, mirroring the two-realm spine.

**Gear vendors:**

| Shop | Sells | Vendor |
|---|---|---|
| **Cyber** | implants (Link / Firewall / Hack-effect) **+ skill chips** (§10) | Cyberware corp / Ripperdoc |
| **Bioware** | bio-augments (no Link, EMP-immune, Reject) | Medical corp / Doctor |
| **Arms** | **guns + melee/blades + armor/defense** — the whole physical realm | Mil-Industrial / clans |
| **Programs** | **all software — offense (hacks / worms / spoof) + defense (anti-Worm / firewall / Link-effects)** — the digital realm | Runner |
| **Consumables** | grenades, EMP, stims, coolant | Mil-Industrial / street |
| **Vehicles** | rigs / transports (extraction-capable, §5) | Vehicles corp / The Haul |

**Service vendors:**

| Shop | Sells | Vendor |
|---|---|---|
| **Insurance** | per-unit pre-combat policies (§9.4) | Insurance corp |
| **Extraction** | medevac membership (§9.4) | Nomad / independent |
| **Clinic** | between-battle heal · chrome-repair · Worm-cleanse · salvage | Doctor / Ripperdoc / White-hat |
| **Fixer** | grey-market & illicit gear, intel, Rep brokering | Independent (the Broker) |
| **Bank** | loans, income, interest | Financial corp |
| **Base** | roster & stash capacity, recovery, safe-house | Real Estate corp |

**Roster & meta:**
- **Recruitment** — draft / hire chassis · archetypes · veterans (§13 #17).
- **Jobs board** — the Rep + gear faucet (§9).
- **Loadout station** *(free config, not a purchase)* — assign movement / targeting profiles, set **PAN segmentation** (§13 #7), install skill chips, manage insurance.

---

## 15. Run structure — the navigation tree ◆

The roguelike run is a **branching navigation tree** (Slay-the-Spire-style map) of nodes from start to a boss, **rolled per run** (seeded). At each branch you **choose your route**, trading off reward, risk, Rep, and recovery. (**"Flight"** = a battle / combat sortie — the in-world term.)

**Node types:**

| Node | What |
|---|---|
| **Flight** (battle) | the auto-resolved combats; **elite / boss flights** are tougher |
| **Economy / shop** | a §14 market (2 vendors/category — buy, sell) |
| **Politicking** | offers a **selection of Jobs** (§9) to accept — contracts negotiated here, fought as objective Flights |
| **Event** | a choice / dilemma (narrative, gambles, faction overtures) |
| **Fixer** | intel (scout ahead), Rep brokering, black-market |
| **Clinic / Rest** | heal · chrome-repair · Worm-cleanse (menders as a service) |
| **Raid** | a forced **cop** encounter spawned by Notoriety (§13 #2) |

**Cadence ◆:** between **flights**, the route runs through **2–3 economy / shopping segments** (shop · Job · event · rest) — the downtime where the casualty economy (§9.4), the *Recovered-units-sit-out-one-segment* rule (§13 #13), and Rep flows play out across **multiple stops**, not one.

**Routing is the strategy:** you see the tree ahead and plan — dive for Jobs (Rep + gear, risk), stock up at shops, rest to recover downed units, or rush the boss. Branches force trade-offs (you can't hit every node), and **Notoriety + the dynamic rivalry (§13 #3) reshape which nodes and enemies appear** — so no two runs route the same.

**Factoring:** `atomica-run` owns the tree — seeded generation, node resolution, routing; the `sim` only runs **Flight** nodes. The tree is run-state. *(⏳ tuning: tree depth, flights-per-run, node mix, boss structure.)*

---

## 16. Rep → aligned shops ◆

Rep cashes out as **shop access**, not battle-board presence. *(Supersedes the earlier board-representative idea — there are **no combat-board rep-spots**.)*

- **Crossing a faction's Rep threshold spawns its aligned shop.** When standing hits a tier (§13 #1), that faction's **dedicated vendor** appears — filling an open slot in the 2-vendors-per-category market, or as a shop node on the run tree (§14 / §15). Deeper, branded, signature stock: the reward for standing.
- So §14's *"more vendors unlock over a run"* is **Rep-driven** — build standing, its shop spawns.
- The **dynamic rivalry** (§13 #3) still bites: a rival's shop may close (or price away) as the aligned one opens.

**Jobs come from politicking nodes, not the board.** A challenge **is a Job** (§9), offered at a **politicking node** (§15) — a run-tree stop presenting a **selection of Jobs** to accept; the accepted Job then runs as an objective **Flight**. No board reps, no board challenges.

**Factoring:** all `atomica-run` — Rep thresholds gate shop spawns; politicking nodes generate Job offers. The `sim` is untouched.

*Open ⏳: which thresholds spawn which shops; politicking Job-offer counts / refresh; whether a rival shop closes or just prices away.*
