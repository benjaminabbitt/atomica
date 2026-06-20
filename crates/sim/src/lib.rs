//! `atomica-sim` — engine-agnostic, deterministic battle simulation for
//! **CHROME AND CODE**.
//!
//! This crate has no rendering or game-engine dependencies. The front-end reads
//! [`Battle`] state each frame and draws it; nothing here knows about a screen.
//!
//! ## Determinism
//! All randomness flows through a [`RandomSource`] (seeded). Given the same seed and the same
//! initial [`Battle`], [`Battle::step`] always produces the same result. That is
//! what makes the design's *async / replayable auto-resolution* possible.
//!
//! ## Scope (so far)
//! Models the locked *shapes* with placeholder values:
//! - the unit stat line, layered defense, penetration tiers;
//! - the [`armor`] matrix (damage type vs armor class);
//! - the [`status`] pool on the design's 9-axis schema (DoTs, Crash/Lag, Breach,
//!   Corrode), processed each tick;
//! - [`hack`]ing — the netrunning digital attack (`2d10 ≤ avg(Hacking, channel) − Firewall`
//!   vs `Firewall`; the channel is the weaker endpoint's Link, §7F/§13);
//! - an initiative-ordered tick loop with a minimal "attack nearest / step toward"
//!   resolution plus a digital pass.
//!
//! Not yet built: the two contagion families (a spreading special case of
//! statuses) and IFF/spoof. (Heat — a thermal layer — was dropped from scope.)

pub mod armor;
mod board;
mod chargen;
mod corruption;
mod event;
mod hack;
mod hex;
mod implant;
mod objective;
mod profile;
mod rng;
mod roll;
mod skills;
mod status;
mod terrain;

pub use armor::ArmorClass;
pub use board::{Board, SeamOffset};
pub use chargen::{
    Amount, BaseLine, Capability, Character, Condition, Contagion, Decorator, DamageEvent, Event,
    Expiration, Factor, FactorKind, Flag, Gate, GenId, Hook, HookEffect, Modifier, ModifierKind,
    Override, Priority, Reaction, Realized, Remove, Stat, Tag, Vector, Wear,
};
pub use corruption::Corruption;
pub use event::{BreachVector, CombatEvent, EventLog, FieldValue, Record};
pub use hack::{hack_rating, Hack, HackResult};
pub use hex::Hex;
pub use implant::{Contribution, Implant, Pan};
pub use profile::{MovementProfile, TargetingProfile};
pub use objective::{
    FoundAction, Goal, Hold, MarginLoss, Objective, ObjectiveKind, ObjectiveStatus, Objectives,
    Reach, Survive, TimeAttack, WinFight, PLAYER,
};
pub use rng::{RandomSource, ScriptedRng, SplitMix64};
pub use roll::{resolve_check, resolve_opposed, resolve_versus, Opposed, RollOutcome};
pub use skills::{Chassis, Skill, SkillTier, Skills};
pub use terrain::{Bounds, Terrain, Tile};
pub use status::{
    Behavior, Decay, Effect, Magnitude, Resist, Stacking, StatusSpec, Targeting, Timing, Trigger,
};

/// Which side a unit fights for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Team {
    A,
    B,
}

impl Team {
    pub fn enemy(self) -> Team {
        match self {
            Team::A => Team::B,
            Team::B => Team::A,
        }
    }
}

/// A stable unit identifier.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct UnitId(pub u32);

/// The 3-tier armor matrix axis carried by an attack.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DamageType {
    /// Anti-rigid: beats Plate, soaked by Padding.
    Bludgeoning,
    /// Universal penetrator: only Plate resists.
    Piercing,
    /// Anti-unarmored.
    Slashing,
}

/// Which defense layer an attack engages first.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PenTier {
    /// Hits the Barrier/Shield layer first.
    External,
    /// Plating mitigates.
    Contact,
    /// Bypasses Barrier + Plating — straight to Integrity.
    Internal,
}

/// The area an attack covers (§7G). **Physical AoE has friendly fire on** — it hits
/// *every* living unit in the footprint, allies included; only the attacker is spared.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Footprint {
    /// Just the chosen target hex.
    #[default]
    Single,
    /// A disc of `radius` centred on the target hex (a grenade / blast).
    Blast(i32),
    /// A `length`-hex line from the attacker along the bearing to the target (a beam
    /// / sweep), the attacker's own hex excluded.
    Beam(i32),
}

/// What a unit does **on death** (§10.9) — fired once when it's removed; a
/// `Detonate` can chain-kill, which fires further triggers. Feeds the contagion
/// system via `DataSpill`.
#[derive(Clone, Copy, Debug, Default)]
pub enum DeathTrigger {
    #[default]
    None,
    /// A parting **blast**: physical AoE to every living unit in `radius` (friendly
    /// fire — a corpse-bomb doesn't discriminate).
    Detonate { damage: f32, dtype: DamageType, pen: PenTier, radius: i32 },
    /// **Data-spill**: leak a status onto living **enemies** in `radius` (the dying
    /// system's payload infects whoever's near — the contagion seed, §10.9).
    DataSpill { spec: StatusSpec, stacks: u32, duration: u32, radius: i32 },
    /// **Legacy**: bequeath a status to living **allies** in `radius` (a martyr's gift).
    Legacy { spec: StatusSpec, stacks: u32, duration: u32, radius: i32 },
}

/// A **set of equipment tags** — boolean traits on a piece of equipment (a weapon's
/// `Attack`, an `Implant`, …) packed as a bitset of `const` flags, so equipment carries
/// any combination (`SMART | AWKWARD`, …) in one field instead of a bool per trait. The
/// shared tag set for the equipment base; test membership with [`EquipmentTags::has`].
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct EquipmentTags(u32);

impl EquipmentTags {
    /// No tags.
    pub const NONE: EquipmentTags = EquipmentTags(0);

    /// **IFF / smartgun** (§7F) — *rule:* [`spares_team`](EquipmentTags::spares_team):
    /// the line of fire / blast spares the attacker's team (filtered in `footprint_targets`).
    pub const SMART: EquipmentTags = EquipmentTags(1 << 0);
    /// **Awkward** (§7G) — *rule:* a long / unwieldy weapon is clumsy **up close**: a
    /// to-hit penalty that fades to none at proper range (see [`to_hit_penalty`](EquipmentTags::to_hit_penalty)).
    pub const AWKWARD: EquipmentTags = EquipmentTags(1 << 1);
    /// **Ranged** (§7G) — *rule:* a projectile weapon's to-hit penalty **grows with
    /// distance** (discrete from `AWKWARD`; a rifle is `RANGED | AWKWARD`, hard far *and*
    /// near with a sweet spot between).
    pub const RANGED: EquipmentTags = EquipmentTags(1 << 2);
    /// **Digital** (§6) — *rule:* [`breachable`](EquipmentTags::breachable): networked
    /// chrome a breach can trip (deck, smartware); absent ⇒ **inert physical** cyberware,
    /// immune to hack / worm / EMP.
    pub const DIGITAL: EquipmentTags = EquipmentTags(1 << 3);
    /// **Volatile** (`netrunning.md`) — powered / overclocked chrome that *runs hot*: the
    /// subset a runner's **Overheat** program can cook (combat stim, metabolic pump, reflex
    /// booster…). Passive armor and comms gear lack it, so heat only bites a heat-prone build.
    pub const VOLATILE: EquipmentTags = EquipmentTags(1 << 4);

    /// Does this set contain every flag in `tag`?
    pub const fn has(self, tag: EquipmentTags) -> bool {
        self.0 & tag.0 == tag.0
    }
    /// This set with `tag` added.
    pub const fn with(self, tag: EquipmentTags) -> EquipmentTags {
        EquipmentTags(self.0 | tag.0)
    }

    /// The **to-hit TN** these tags add at hex-distance `dist` (§7G) — the sum of each
    /// range rule the set carries: `AWKWARD` bites up close, `RANGED` grows with distance.
    /// One place for the rules, universal to any equipment (an item with neither pays 0).
    pub fn to_hit_penalty(self, dist: i32) -> i32 {
        let mut tn = 0;
        if self.has(Self::AWKWARD) {
            tn += match dist {
                d if d <= 0 => AWKWARD_POINT_BLANK, // same hex — practically unreachable
                1 => AWKWARD_ADJACENT,              // jammed in close
                _ => 0,                             // at proper range
            };
        }
        if self.has(Self::RANGED) {
            tn += match dist {
                d if d >= RANGED_LONG => RANGED_LONG_PENALTY,
                d if d >= RANGED_MEDIUM => RANGED_MEDIUM_PENALTY,
                _ => 0, // short range — clean
            };
        }
        tn
    }

    /// `SMART`'s rule (§7F): does this equipment **spare the attacker's team** in its
    /// line of fire / blast? (`footprint_targets` filters teammates out when true.)
    pub const fn spares_team(self) -> bool {
        self.has(Self::SMART)
    }

    /// `DIGITAL`'s rule (§6): can a **breach** (hack / worm / EMP) trip this equipment?
    /// `false` ⇒ inert physical chrome, immune to every vector.
    pub const fn breachable(self) -> bool {
        self.has(Self::DIGITAL)
    }
}

impl std::ops::BitOr for EquipmentTags {
    type Output = EquipmentTags;
    fn bitor(self, rhs: EquipmentTags) -> EquipmentTags {
        EquipmentTags(self.0 | rhs.0)
    }
}

impl std::fmt::Debug for EquipmentTags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut names = Vec::new();
        if self.has(Self::SMART) {
            names.push("SMART");
        }
        if self.has(Self::AWKWARD) {
            names.push("AWKWARD");
        }
        if self.has(Self::RANGED) {
            names.push("RANGED");
        }
        if self.has(Self::DIGITAL) {
            names.push("DIGITAL");
        }
        write!(f, "EquipmentTags({})", names.join(" | "))
    }
}

/// A single attack profile. (Weapons/loadouts will compose these later.)
#[derive(Clone, Copy, Debug)]
pub struct Attack {
    pub damage: f32,
    /// The attack's **Speed** (`docs/stats.md`) — a peer of `damage`: how fast the blow
    /// arrives. A high-velocity round (Speed ~3–4) is far harder to dodge than a swung blade
    /// (Speed ~1), so Speed is a flat **penalty to the defender's Evade**; it may also feed
    /// penetration. Melee/thrown are slow (Dodge stays potent); guns are fast.
    pub speed: i32,
    pub dtype: DamageType,
    pub pen: PenTier,
    /// The **skill** that governs the to-hit roll (`design-delta §394`) — a weapon's
    /// *role*: [`Skill::Melee`] for blades/fists, [`Skill::Gunnery`] for ranged. The
    /// attacker rolls `2d10 ≤ this skill + accuracy`, opposed by the target's Evade.
    pub skill: Skill,
    /// The weapon's inherent **accuracy** mod (the "equipment" term of the roll) — a
    /// smartlink / scope lifts it. `0` for a plain weapon.
    pub accuracy: i32,
    /// Maximum reach in hexes — the top of the **range band** (§10.5).
    pub range: i32,
    /// Minimum reach — the bottom of the band (`1` = usable in melee). A gun band is
    /// e.g. `2..=6` (no point-blank); a polearm's **reach** is `2..=2`.
    pub min_range: i32,
    /// EMP weapon: a *physical* pulse that also fries the target's cyberware,
    /// **bypassing Firewall** (§7I) — the physical counter to digital builds.
    pub emp: bool,
    /// The area struck (§7G) — `Single` by default; `Blast`/`Beam` hit allies too.
    pub footprint: Footprint,
    /// The weapon's **tags** (`docs/combat.md`) — `SMART` (IFF), `AWKWARD` (clumsy up
    /// close), … as a set of `const` flags. See [`EquipmentTags`].
    pub tags: EquipmentTags,
}

impl Attack {
    /// Is this weapon usable at hex-distance `dist`? (Within its range band.)
    pub fn usable_at(&self, dist: i32) -> bool {
        dist >= self.min_range && dist <= self.range
    }

    /// A plain **melee** profile (`damage`, Piercing/Internal, reach 1, single-target) —
    /// the default weapon and the base most isolated tests start from.
    pub fn melee(damage: f32) -> Self {
        Self {
            damage,
            speed: 1, // a swung blade is slow — Dodge is potent against it
            dtype: DamageType::Piercing,
            pen: PenTier::Internal,
            skill: Skill::Melee,
            accuracy: 0,
            range: 1,
            min_range: 1,
            emp: false,
            footprint: Footprint::Single,
            tags: EquipmentTags::NONE,
        }
    }

    /// Builder: add the **`SMART`** tag (IFF) — its blast / line of fire spares the
    /// attacker's team (`docs/combat.md`). The "smartgun" mod over any base profile.
    pub fn smartlinked(mut self) -> Self {
        self.tags = self.tags.with(EquipmentTags::SMART);
        self
    }
}

/// A weapon decorator: a `Tag::Weapon` gear grant of `attack` on the gen (`docs/layers.md`
/// L6). The loadout is the set of these; breach / unequip drops one.
fn weapon_grant(attack: Attack) -> Decorator {
    Decorator::gear(Tag::Weapon, vec![]).with_grant(Capability::Weapon(attack))
}

/// An installed implant: the authored [`Implant`] (its `Contribution`, granted hack,
/// and breach `hack_effects`) paired with the [`GenId`] of its decorator on the unit's
/// `character`. The **live condition** lives on the decorator (`docs/layers.md` L5);
/// the breach ladder reads / writes it by this id and fires the spec's liabilities.
#[derive(Clone, Debug)]
pub struct InstalledImplant {
    pub spec: Implant,
    pub gen: GenId,
    /// Live **durability** — starts at `spec.max_hp`; physical hits (the hit-location roll)
    /// chip it, driving the [`Condition`] (Degraded < 50%, Destroyed at 0). Repair refills it.
    pub hp: f32,
}

/// A combatant. The stat line mirrors the design's "Unit anatomy".
#[derive(Clone, Debug)]
pub struct Unit {
    pub id: UnitId,
    pub name: String,
    pub team: Team,
    pub pos: Hex,

    // Armor class is **composed** (`docs/layers.md` L6): the base sits in `BaseLine.armor`
    // and gear overrides it (`Override::Armor`); read it via `Unit::armor_class()`.
    /// Innate class — sets the skill floor, contagion exposure, etc. (§7J).
    pub chassis: Chassis,
    /// Per-character skill levels (chassis baseline + earned) — roll modifiers (§10).
    pub skills: Skills,

    /// The **move stat** (§10.4): how many hexes the unit may step per activation
    /// (move-then-act). `0` ⇒ stationary. (A board concern, not a composed stat.)
    pub speed: i32,

    // Weapons are **`Capability::Weapon` grants** on the `character` (`docs/layers.md`
    // L6): `weapons()` / `weapon_at` read the composed loadout, `with_weapon` / `arm`
    // install grants. No flat weapon field — a chrome arm grants one like any gear.
    /// Installed cyberware (`docs/cyberware.md`) — each an [`InstalledImplant`]: a
    /// decorator on the `character` (its `Contribution` composes; its condition rides
    /// the decorator) plus the authored spec the breach ladder fires from.
    pub implants: Vec<InstalledImplant>,
    /// The implant network mode (§5): meshed (synergy, Cascade-vulnerable) vs
    /// segmented (contained, no synergy). A loadout commitment.
    pub pan: Pan,
    /// The unit's [`Character`] ([`layers.md`](../../docs/layers.md) L5): it owns
    /// **everything composed** — the authored `BaseLine`, the gen (implants / statuses
    /// / buffs / behavior as decorators), and the **live pools** (Integrity / Barrier /
    /// Plating / alive). The stat accessors and pools below all read through it; there
    /// are no flat stat fields left.
    pub character: Character,
    /// What fires when this unit dies (§10.9). `None` by default.
    pub on_death: DeathTrigger,
    /// Has the death trigger already fired? (Set by the reaper so it fires once.)
    pub death_resolved: bool,
}

impl Unit {
    /// A bare combatant with placeholder stats (Integrity 30, a basic melee hit).
    /// Tune via the `with_*` builders or by field — the convenience constructor
    /// the run layer and content build rosters from.
    pub fn new(id: u32, name: impl Into<String>, team: Team, chassis: Chassis) -> Self {
        let mut character =
            Character::new(BaseLine { max_integrity: 30.0, initiative: 5.0, ..BaseLine::default() });
        character.install(weapon_grant(Attack::melee(10.0))); // default melee
        Self {
            id: UnitId(id),
            name: name.into(),
            team,
            pos: Hex::new(0, 0),
            chassis,
            skills: chassis.baseline_skills(),
            speed: 1,
            implants: Vec::new(),
            pan: Pan::Meshed,
            character,
            on_death: DeathTrigger::None,
            death_resolved: false,
        }
    }

    /// Builder: set the deploy position.
    pub fn at(mut self, pos: Hex) -> Self {
        self.pos = pos;
        self
    }

    /// Builder: set Integrity (its base max **and** the live pool).
    pub fn with_integrity(mut self, hp: f32) -> Self {
        self.character.base_mut().max_integrity = hp;
        self.character.integrity = hp;
        self
    }

    /// Builder: set the physical Initiative.
    pub fn with_initiative(mut self, initiative: f32) -> Self {
        self.character.base_mut().initiative = initiative;
        self
    }

    /// Builder: set the innate **armor class** (gear can still override it, §L6).
    pub fn with_armor(mut self, armor: ArmorClass) -> Self {
        self.character.base_mut().armor = armor;
        self
    }

    /// Builder: set base **Evasion** (the physical to-hit TN, §7G).
    /// Builder: set a **primary attribute** (`docs/stats.md`). Skills are tiers on these and
    /// the combat stats derive from them — e.g. Evasion = `(Dexterity + Evade-tier) × 2`.
    pub fn with_body(mut self, body: f32) -> Self {
        self.character.base_mut().body = body;
        self
    }
    pub fn with_dexterity(mut self, dexterity: f32) -> Self {
        self.character.base_mut().dexterity = dexterity;
        self
    }
    pub fn with_intellect(mut self, intellect: f32) -> Self {
        self.character.base_mut().intellect = intellect;
        self
    }
    pub fn with_will(mut self, will: f32) -> Self {
        self.character.base_mut().will = will;
        self
    }

    /// Builder: set a **skill tier** — the modifier on the governing attribute (untrained −3 …
    /// elite +2; competent 0). Effective rating = governing attribute + this.
    pub fn with_skill(mut self, skill: Skill, tier: i32) -> Self {
        self.skills.set(skill, tier);
        self
    }

    /// The unit's effective **armor class** for the mitigation matrix — base, or the
    /// highest-priority `Override::Armor` from gear.
    pub fn armor_class(&self) -> ArmorClass {
        self.realized().armor_class()
    }

    /// Builder: set the move stat (hexes per activation).
    pub fn with_speed(mut self, speed: i32) -> Self {
        self.speed = speed;
        self
    }

    /// Builder: **add** a weapon to the loadout (selected by range band, §10.5) — an
    /// extra grant alongside any existing ones.
    pub fn with_weapon(mut self, weapon: Attack) -> Self {
        self.add_weapon(weapon);
        self
    }

    /// Builder: set the **sole** weapon — clears the loadout (incl. the default melee)
    /// and grants `attack`. Call before `with_weapon` to add extras.
    pub fn with_attack(mut self, attack: Attack) -> Self {
        self.set_weapon(attack);
        self
    }

    /// Add a weapon grant to the loadout.
    pub fn add_weapon(&mut self, weapon: Attack) {
        self.character.install(weapon_grant(weapon));
    }

    /// Replace the whole loadout with a single weapon.
    pub fn set_weapon(&mut self, weapon: Attack) {
        self.character.remove_where(Tag::Weapon);
        self.character.install(weapon_grant(weapon));
    }

    /// Modify the **primary** weapon in place (read it, tweak it, set it back as the
    /// sole weapon) — the ergonomic "make my weapon an EMP / a blast" path for content
    /// and tests.
    pub fn rearm(&mut self, f: impl FnOnce(&mut Attack)) {
        let mut w = self.weapon_at_any().unwrap_or(Attack::melee(10.0));
        f(&mut w);
        self.set_weapon(w);
    }

    /// Strip the weapon loadout — an **unarmed** unit (a data node / objective prop) that never
    /// attacks. Every unit ships a default melee; a terminal shouldn't fight back.
    pub fn disarm(&mut self) {
        self.character.remove_where(Tag::Weapon);
    }

    /// Builder: set the on-death trigger (§10.9).
    pub fn with_on_death(mut self, trigger: DeathTrigger) -> Self {
        self.on_death = trigger;
        self
    }

    /// The unit's composed **weapon loadout** (every active `Capability::Weapon` grant).
    pub fn weapons(&self) -> Vec<Attack> {
        self.realized().weapons()
    }

    /// The highest-damage weapon ignoring range (a threat proxy) — `None` if unarmed.
    fn weapon_at_any(&self) -> Option<Attack> {
        self.weapons()
            .into_iter()
            .max_by(|a, b| a.damage.partial_cmp(&b.damage).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// The longest reach in the loadout — a skirmisher's **standoff range** (the distance
    /// a [`MovementProfile::Kite`] unit tries to hold). `1` if unarmed.
    fn max_weapon_range(&self) -> i32 {
        self.weapons().iter().map(|w| w.range).max().unwrap_or(1)
    }

    /// The best weapon usable at hex-distance `dist` — the highest-damage grant whose
    /// **range band** covers `dist`, or `None` if the target is out of every band (§10.5).
    pub fn weapon_at(&self, dist: i32) -> Option<Attack> {
        self.weapons()
            .into_iter()
            .filter(|w| w.usable_at(dist))
            .max_by(|a, b| a.damage.partial_cmp(&b.damage).unwrap_or(std::cmp::Ordering::Equal))
    }

    /// Builder: program the **targeting** profile (§7J) — a `GEAR`-priority override
    /// on the behavior layer, so a `CORRUPTION` spoof still beats it.
    pub fn with_targeting(mut self, p: TargetingProfile) -> Self {
        self.character.install(
            Decorator::gear(Tag::Gear, vec![]).with_override(Override::Targeting(p)),
        );
        self
    }

    /// Builder: program the **movement** profile (§7J).
    pub fn with_movement(mut self, p: MovementProfile) -> Self {
        self.character.install(
            Decorator::gear(Tag::Gear, vec![]).with_override(Override::Movement(p)),
        );
        self
    }

    /// Compose the unit's stat line **on demand**: the `character`'s authored base +
    /// its modifier decorators (§0). The single read path the loop goes through.
    fn realized(&self) -> Realized {
        self.character.realize()
    }

    /// Effective **Link** — base + composed modifiers (§7D).
    pub fn link(&self) -> i32 {
        self.realized().link()
    }
    /// **Antenna reach** for hacking — the hexes a hack carries across (`netrunning.md`).
    /// **Link governs range**: a loud, high-Link runner projects far; a dark one barely
    /// reaches. (Link's fourth job, alongside the reachability gate, the channel, and digital
    /// initiative.) `0` ⇒ no presence to reach with.
    fn hack_reach(&self) -> i32 {
        self.link().max(0)
    }
    /// Effective **Firewall** (the digital TN, §13).
    pub fn firewall(&self) -> i32 {
        self.realized().firewall()
    }
    /// Effective **Immunity** (the bio TN).
    pub fn immunity(&self) -> i32 {
        self.realized().immunity()
    }
    /// Effective **Evasion** — the active-defense target a roll-under attack is opposed by
    /// (`docs/stats.md`): **derived** as `Dexterity + Evade-tier` (a *secondary* save, NOT on
    /// the attack's ×2 scale — defense is the harder roll, so solid attacks mostly land and a
    /// dodge is the exception). Agility and the Evade skill set it, a plating's −Dex lowers it;
    /// zero ⇒ undefended (can't dodge).
    pub fn evasion(&self) -> i32 {
        self.effective_skill(Skill::Evade)
    }
    /// Effective **Initiative** before status slows (see [`Unit::effective_initiative`]).
    pub fn initiative(&self) -> f32 {
        self.realized().initiative()
    }
    /// Effective **max Integrity** (composed).
    pub fn max_integrity(&self) -> f32 {
        self.realized().max_integrity()
    }
    /// The live **Integrity** pool (current HP).
    pub fn integrity(&self) -> f32 {
        self.character.integrity
    }
    /// The active statuses as `(name, stacks)` — the labelled decorators on the gen.
    pub fn statuses(&self) -> Vec<(&'static str, u32)> {
        self.character.status_labels()
    }
    /// The flat **damage bonus** this unit adds to every weapon hit — the composed
    /// `Damage` stat (an implant combat-stim, a buff). Weapons carry their own base.
    pub fn damage_bonus(&self) -> f32 {
        self.realized().damage()
    }

    /// Install a stat/behavior **modifier** on the unit (a buff, debuff, or gear) — a
    /// decorator on its `character`; the effective accessors compose it immediately.
    /// Returns the [`GenId`] for later removal (a cleanse / dispel).
    pub fn apply_modifier(&mut self, dec: Decorator) -> GenId {
        self.character.install(dec)
    }

    /// The unit's effective targeting profile — the behavior layer composed (a spoof
    /// overrides the program).
    pub fn targeting(&self) -> TargetingProfile {
        self.realized().targeting()
    }

    /// The unit's effective movement profile.
    pub fn movement(&self) -> MovementProfile {
        self.realized().movement()
    }

    /// **Spoof** the unit's behavior (§7J) — install a `CORRUPTION`-priority override
    /// that outranks its program for `turns` ticks. Returns the [`GenId`] so a
    /// counter-spoof / cleanse can remove it. "The enemy hacks your script."
    pub fn spoof(&mut self, targeting: TargetingProfile, turns: u32) -> GenId {
        self.character.install(
            Decorator::timed(Tag::Spoof, turns, vec![])
                .with_priority(Priority::CORRUPTION)
                .with_override(Override::Targeting(targeting)),
        )
    }

    pub fn is_alive(&self) -> bool {
        self.character.alive && self.character.integrity > 0.0
    }

    /// This unit's level in `skill` — the bonus it brings to a contested roll (§13).
    pub fn skill(&self, skill: Skill) -> i32 {
        self.skills.level(skill)
    }

    /// **Primary attributes** (`docs/stats.md`) — base + composed modifiers (a plating's
    /// −Dexterity folds in here). Skills are tiers *on* these.
    pub fn body(&self) -> i32 {
        self.realized().body()
    }
    pub fn dexterity(&self) -> i32 {
        self.realized().dexterity()
    }
    pub fn intellect(&self) -> i32 {
        self.realized().intellect()
    }
    pub fn will(&self) -> i32 {
        self.realized().will()
    }

    /// This unit's **effective rating** at `skill` — `governing attribute + skill tier`
    /// (`docs/stats.md`). The reworked roll-under combat target is this × 2; a lowered
    /// attribute (plating −Dex) drags every skill it governs down with it.
    pub fn effective_skill(&self, skill: Skill) -> i32 {
        self.realized().attribute(skill.governs()) + self.skills.level(skill)
    }

    /// Make a roll-under contest with this unit's **effective** `skill` (+ `equipment`) vs a
    /// static `tn` (`docs/stats.md` — a passive threshold, not an opposed defender).
    pub fn contest<R: RandomSource>(
        &self,
        skill: Skill,
        equipment: i32,
        tn: i32,
        rng: &mut R,
    ) -> RollOutcome {
        resolve_versus(rng, self.effective_skill(skill) + equipment, tn)
    }

    /// Apply a status, honoring its stacking axis — installs (or merges into) a
    /// decorator on the `character` ([`Character::apply_status`]).
    pub fn add_status(&mut self, spec: StatusSpec, duration: u32, stacks: u32) {
        let cap = match spec.stacking {
            Stacking::Refresh => None,
            Stacking::Stack { max } => Some(max),
        };
        self.character.apply_status(spec.to_decorator(stacks, duration), cap);
    }

    /// Install `implant` as a **decorator on the `character`** (`docs/layers.md` L5):
    /// its `Contribution` composes into the stat accessors, its condition rides the
    /// decorator, and a deck's loadout is **granted** (read via [`Unit::hack`]). No
    /// flat-field fold; the capacity it lends (a pump's +HP, a plate's armor) comes
    /// online **filled** via [`resize_pools`](Character::resize_pools).
    pub fn install(&mut self, implant: Implant) {
        let before = self.character.maxima();
        let gen = self.character.install(implant.to_decorator());
        self.character.resize_pools(before);
        let hp = implant.max_hp;
        self.implants.push(InstalledImplant { spec: implant, gen, hp });
    }

    /// The netrunning loadout this unit can run — granted by an active deck implant
    /// (composed; `None` ⇒ no deck or it's Offline).
    pub fn hack(&self) -> Option<Hack> {
        self.realized().hack()
    }

    /// Grant a netrunning loadout via a built-in deck (a granting decorator) — content
    /// / test convenience; normally a deck implant grants it. Returns its [`GenId`].
    pub fn grant_hack(&mut self, hack: Hack) -> GenId {
        self.character
            .install(Decorator::gear(Tag::Implant, vec![]).with_grant(Capability::Hack(hack)))
    }

    /// The live [`Condition`] of the implant at `idx` (read from its decorator).
    pub fn implant_condition(&self, idx: usize) -> Condition {
        self.character.condition_of(self.implants[idx].gen).unwrap_or(Condition::Destroyed)
    }

    /// The unit's **salvageable chrome** (`cyberware.md` §6) — the specs of every
    /// installed implant that isn't **Destroyed** (Destroyed is terminal, beyond
    /// salvage). The bare operation: a loot / run layer collects this off a corpse on
    /// death; condition is irrelevant to *what* can be stripped, only Destroyed is gone.
    pub fn salvage(&self) -> Vec<Implant> {
        (0..self.implants.len())
            .filter(|&i| self.implant_condition(i) != Condition::Destroyed)
            .map(|i| self.implants[i].spec.clone())
            .collect()
    }

    /// Move the implant at `idx` to `cond` on its decorator, then re-clamp the pools to
    /// the new composed maxima. Destroyed is terminal. Returns the implant's
    /// `hack_effects` iff this knocks it from active to inactive (a breach — the caller
    /// fires them per the §6 ladder).
    fn transition(&mut self, idx: usize, cond: Condition) -> Vec<StatusSpec> {
        let old = self.implant_condition(idx);
        if old == Condition::Destroyed {
            return Vec::new(); // terminal
        }
        let before = self.character.maxima();
        self.character.set_condition(self.implants[idx].gen, cond);
        self.character.resize_pools(before);
        if old.is_active() && !cond.is_active() {
            self.implants[idx].spec.hack_effects.clone()
        } else {
            Vec::new()
        }
    }

    /// Knock the implant at `idx` straight to **Offline** — a breach's "disable"
    /// floor (§6). Returns its `hack_effects` for the severity ladder.
    pub fn disable_implant(&mut self, idx: usize) -> Vec<StatusSpec> {
        self.transition(idx, Condition::Offline)
    }

    /// Wear the implant at `idx` **one step** down the condition ladder
    /// (`Online → Degraded → Offline → Destroyed`): physical wear, *reduced*
    /// benefit, **no liability fired** (§3.1 — wear is not a breach).
    pub fn degrade_implant(&mut self, idx: usize) {
        let next = self.implant_condition(idx).degraded();
        self.transition(idx, next);
    }

    /// Bring the implant at `idx` back **Online** — the Ripperdoc un-bricking /
    /// repairing it (§3.2): condition restored **and durability refilled**. Destroyed gear
    /// is terminal (a no-op).
    pub fn repair_implant(&mut self, idx: usize) {
        if self.implant_condition(idx) == Condition::Destroyed {
            return; // terminal — nothing to mend
        }
        self.implants[idx].hp = self.implants[idx].spec.max_hp;
        self.transition(idx, Condition::Online);
    }

    /// Indices of all active implants (physical and digital alike).
    fn active_implant_indices(&self) -> Vec<usize> {
        (0..self.implants.len()).filter(|&i| self.implant_condition(i).is_active()).collect()
    }

    /// Active implants a **breach** can trip — only **digital** ones (`Implant::is_digital`).
    /// Inert physical cyberware (subdermal plating) is invisible to every breach vector
    /// (hack / worm / EMP); it only wears or is destroyed physically.
    fn digital_implant_indices(&self) -> Vec<usize> {
        self.active_implant_indices()
            .into_iter()
            .filter(|&i| self.implants[i].spec.is_digital())
            .collect()
    }

    /// Does the unit carry any **`VOLATILE`** (heat-prone) active chrome — the subset an
    /// Overheat program can cook (`netrunning.md`)? `false` for a build of passive armor / comms.
    fn has_volatile_chrome(&self) -> bool {
        self.active_implant_indices()
            .iter()
            .any(|&i| self.implants[i].spec.tags.has(EquipmentTags::VOLATILE))
    }
    fn first_digital_implant(&self) -> Option<usize> {
        self.digital_implant_indices().into_iter().next()
    }

    /// Throughput bonus from a **meshed** PAN (§5): networked implants boost each
    /// other (set-effects). First-cut — +1 per active implant beyond the first,
    /// capped. A **segmented** PAN forfeits it (the price of Cascade-immunity).
    fn mesh_synergy(&self) -> i32 {
        if self.pan == Pan::Meshed {
            (self.active_implant_indices().len() as i32 - 1).clamp(0, MESH_SYNERGY_CAP)
        } else {
            0
        }
    }

    /// Is the unit **stunned** (a Crash / Seizure decorator present)? — composed.
    fn is_stunned(&self) -> bool {
        self.realized().stunned()
    }

    /// Initiative after Lag-style slows — composed (a `Slow` status is a `More` factor
    /// on Initiative, so the realized accessor already folds it).
    fn effective_initiative(&self) -> f32 {
        self.initiative()
    }

    /// Digital **bandwidth** — Link floored to an integer (§7D). The hack channel
    /// is the weaker endpoint's band (`min`), which the attacker's rating averages
    /// with its Hacking. (Defense is Link-blind; zero Link is the separate hard
    /// reachability gate.)
    fn digital_band(&self) -> i32 {
        self.link().max(0)
    }

    /// Incoming-damage multiplier from Breach-style `Vuln` flags — composed.
    fn vuln_mult(&self) -> f32 {
        self.realized().vuln()
    }
}

/// Ticks a crit-gated knockout stun lasts. Applied in the digital phase (after
/// the action phase), so it needs ≥2 to survive decay and skip the next action.
/// Placeholder (TBD).
const KNOCKOUT_STUN: u32 = 2;

/// **Knockout-gate width** (`cyberware.md` §6 knob): the stun / incapacitate class is
/// the *decisive*-only tier of the breach ladder. Strict by default — `i32::MAX` means
/// **a crit (nat-18) and nothing else** clears it. Lower it to widen the gate to a
/// "decisive margin" tier (then `margin ≥ this` also knocks out). Placeholder (TBD).
const KNOCKOUT_MARGIN: i32 = i32::MAX;

/// **Overheat** payload (`netrunning.md`): a **solid** breach cooks the target with an Internal
/// DoT for `OVERHEAT_DURATION` ticks, one stack per [`hack::margin_stacks`] of the breach margin
/// — so heat is the reward for a *deep* crack, scaling with degree of success. A marginal hack
/// (the §6 disable floor) draws **no heat**. This is netrunning's damage. (TBD.)
const OVERHEAT_DURATION: u32 = 3;

/// Does `outcome` clear the knockout gate — a crit, or a margin at/above the decisive
/// `tier`? (§6: "any hack-effect that stuns is crit-gated; everything else scales with
/// margin" — `tier` is the knob between strict-crit and a decisive-margin gate.)
fn knockout_gated(outcome: &RollOutcome, tier: i32) -> bool {
    outcome.crit || outcome.margin >= tier
}

/// Fixed magnitude of the degrade-class liabilities an EMP fires — it has no
/// margin/crit, being a blunt physical pulse. Placeholder (TBD).
const EMP_MAGNITUDE: u32 = 2;

/// Fixed magnitude of the degrade-class liabilities a **Worm** breach fires — like an
/// EMP it carries no roll/margin (a worm trips regardless); the finisher's bite is its
/// reach (Cascade), not its per-slot force. Placeholder (TBD).
const WORM_DEGRADE: u32 = 2;

/// To-hit TN bumps an **awkward** weapon pays for being too close (§7G): `-2` jammed
/// adjacent, `-4` same-hex (occupancy keeps units apart, so ≈never). Placeholder (TBD).
const AWKWARD_ADJACENT: i32 = 2;
const AWKWARD_POINT_BLANK: i32 = 4;

/// **Ranged** to-hit bands (§7G): a projectile weapon's inherent difficulty grows with
/// distance — `0` short, `-2` from `RANGED_MEDIUM` hexes, `-4` from `RANGED_LONG` (small
/// boards rarely reach the far band). Placeholder (TBD).
const RANGED_MEDIUM: i32 = 3;
const RANGED_LONG: i32 = 5;
const RANGED_MEDIUM_PENALTY: i32 = 2;
const RANGED_LONG_PENALTY: i32 = 4;

/// Cap on the meshed-PAN synergy bonus to a hack rating (§5). Placeholder (TBD).
const MESH_SYNERGY_CAP: i32 = 3;

/// Outcome of a resolved battle.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Outcome {
    Ongoing,
    Winner(Team),
    /// Both sides wiped on the same tick, or the tick cap was reached.
    Draw,
}

/// A full battle: the units, an injected [`RandomSource`], an injected
/// [`Objective`] (the win-condition), and a tick counter.
///
/// Generic over the RNG (defaulting to [`SplitMix64`]) so tests can inject a
/// [`ScriptedRng`] via [`Battle::with_rng`] and force every roll.
pub struct Battle<R: RandomSource = SplitMix64> {
    pub units: Vec<Unit>,
    pub tick: u32,
    rng: R,
    objectives: Objectives,
    /// The two-board **seam** stagger (§7B). Rolled at start; settable by item.
    pub board: Board,
    /// The **terrain** — bounds + blockers/cover/hazards (`docs/combat.md`). Default is an
    /// open, unbounded plane, so a battle behaves as before until it opts into a map.
    pub terrain: Terrain,
    withdrawn: bool,
    /// Opt-in structured event trace (`docs/...`) — off by default; `with_log` enables it.
    log: EventLog,
}

impl Battle<SplitMix64> {
    /// Build a battle with the production RNG seeded by `seed`. The seam offset is
    /// **rolled from the seed** (a pure function of it — it does *not* consume the
    /// battle RNG stream, so roll determinism is independent of the fight).
    pub fn new(units: Vec<Unit>, seed: u64) -> Self {
        let offset = if seed & 1 == 0 { SeamOffset::Down } else { SeamOffset::Up };
        Self::with_rng(units, SplitMix64::new(seed)).with_seam(offset)
    }
}

impl<R: RandomSource> Battle<R> {
    /// Build a battle over any [`RandomSource`] — inject a `ScriptedRng` in tests.
    /// Defaults to the [`Eliminate`] objective and the `Down` seam.
    /// Builder: turn on the **structured event log** — capture a [`CombatEvent`] trace
    /// of the fight (for a CLI / structured logging / replay). Off by default.
    pub fn with_log(mut self) -> Self {
        self.log.enable();
        self
    }

    /// The captured event trace (empty unless [`Battle::with_log`] was set).
    pub fn events(&self) -> &[Record] {
        self.log.records()
    }

    /// Record a structured [`CombatEvent`] at the current tick (no-op while the log is off).
    fn emit(&mut self, event: CombatEvent) {
        self.log.push(self.tick, event);
    }

    pub fn with_rng(units: Vec<Unit>, rng: R) -> Self {
        let objectives = Objectives::new(vec![Goal::new(Box::new(WinFight), 1, 1)]);
        Self {
            units,
            tick: 0,
            rng,
            objectives,
            board: Board::default(),
            terrain: Terrain::default(),
            withdrawn: false,
            log: EventLog::default(),
        }
    }

    /// Builder: set the seam offset (§7B — the mesh item that picks the stagger).
    pub fn with_seam(mut self, offset: SeamOffset) -> Self {
        self.board = Board::new(offset);
        self
    }

    /// Builder: lay the battle on a [`Terrain`] map — bounds, blockers, cover, hazards.
    pub fn with_terrain(mut self, terrain: Terrain) -> Self {
        self.terrain = terrain;
        self
    }

    /// Withdraw from the battle: forfeit it (objectives resolve as fight-over),
    /// but all still-standing units are preserved (§9.4). The run layer applies
    /// the bail penalty and banks the roster.
    pub fn withdraw(&mut self) {
        self.withdrawn = true;
    }

    /// Has the player withdrawn (forfeited to save units)?
    pub fn is_withdrawn(&self) -> bool {
        self.withdrawn
    }

    /// Replace the scored objectives (default: just [`WinFight`], the node's
    /// standard fight). A battle can carry any number; each is scored independently.
    pub fn with_objectives(mut self, objectives: Objectives) -> Self {
        self.objectives = objectives;
        self
    }

    /// The status of each scored objective at the current state.
    pub fn objectives_report(&self) -> Vec<ObjectiveStatus> {
        self.objectives.report(&self.units, self.tick, self.fight_over())
    }

    /// Sum of rewards from achieved objectives (the run-layer winnings).
    pub fn winnings(&self) -> i32 {
        self.objectives.winnings(&self.units, self.tick, self.fight_over())
    }

    /// Sum of penalties from failed objectives.
    pub fn losses(&self) -> i32 {
        self.objectives.losses(&self.units, self.tick, self.fight_over())
    }

    /// Objectives still pending — unachieved, but **not** failed.
    pub fn unachieved(&self) -> Vec<usize> {
        self.objectives.unachieved(&self.units, self.tick, self.fight_over())
    }

    /// Has the battle ended — one army wiped, or the player withdrew?
    fn fight_over(&self) -> bool {
        self.withdrawn || !matches!(self.outcome(), Outcome::Ongoing)
    }

    /// Advance one tick:
    /// 1. **status phase** — DoTs and plating-shred fire (may kill);
    /// 2. **action phase** — each non-stunned unit acts in effective-initiative
    ///    order (attack nearest enemy in range, else step toward it);
    /// 3. **digital phase** — netrunners hack the nearest reachable enemy, in
    ///    Link (digital-initiative) order;
    /// 4. **decay phase** — statuses wear off.
    pub fn step(&mut self) -> Outcome {
        if let o @ (Outcome::Winner(_) | Outcome::Draw) = self.outcome() {
            return o;
        }
        self.tick += 1;

        self.status_phase();
        self.terrain_phase(); // hazard hexes burn whoever stands on them
        self.reap(); // DoTs / hazards can kill — fire their death triggers
        self.woven_phase();
        self.contagion_phase(); // corruption jumps to fresh victims (contested)
        self.decay_phase();
        self.objectives.tick(&self.units, self.tick); // advance objective state (capture/hold/extract)

        let outcome = self.outcome();
        if !matches!(outcome, Outcome::Ongoing) {
            self.emit(CombatEvent::Ended { outcome }); // fires once: a decided battle early-returns
        }
        outcome
    }

    /// Run to completion (or the tick cap) and return the result.
    pub fn resolve(&mut self, max_ticks: u32) -> Outcome {
        for _ in 0..max_ticks {
            match self.step() {
                Outcome::Ongoing => {}
                done => return done,
            }
        }
        Outcome::Draw
    }

    /// The standard fight result (wipe the enemy / be wiped) — drives termination
    /// and the node's baseline [`WinFight`] objective.
    pub fn outcome(&self) -> Outcome {
        let a = self.units.iter().any(|u| u.is_alive() && u.team == Team::A);
        let b = self.units.iter().any(|u| u.is_alive() && u.team == Team::B);
        match (a, b) {
            (true, false) => Outcome::Winner(Team::A),
            (false, true) => Outcome::Winner(Team::B),
            (false, false) => Outcome::Draw,
            (true, true) => Outcome::Ongoing,
        }
    }

    /// The **status tick** (§1): dispatch `TickStart` to every living unit's gen — each
    /// active status decorator's hooks fire (DoTs / shred apply to its pools, gated by
    /// the stochastic `behavior` roll, resolved against its own composed resist TN), and
    /// the decorator lifetimes decay. Passive statuses (Stun / Slow / Vuln) carry no
    /// hook — they compose into the accessors and are read in other phases.
    fn status_phase(&mut self) {
        for i in 0..self.units.len() {
            if !self.units[i].is_alive() {
                continue;
            }
            let before = self.units[i].integrity();
            let reactions =
                self.units[i].character.dispatch(Event::TickStart, self.tick, &mut self.rng);
            // Surface DoT / status-tick damage as a structured event (`cause` = the
            // decorator's label, e.g. the plague's "Virus") so kills are attributable.
            let dealt = before - self.units[i].integrity();
            if dealt > 0.0 {
                let cause = reactions
                    .iter()
                    .find_map(|r| match r {
                        Reaction::Damage { source, .. } => self.units[i].character.label_of(*source),
                        _ => None,
                    })
                    .unwrap_or("status");
                self.emit(CombatEvent::Damaged {
                    unit: self.units[i].id,
                    cause,
                    amount: dealt,
                    killed: !self.units[i].is_alive(),
                });
            }
        }
    }

    /// The **terrain phase** — every living unit standing on a [`Tile::Hazard`] hex takes
    /// its tick of environmental damage (run through the armor matrix like any hit). It
    /// surfaces as a `Damaged{cause:"hazard"}` event, and a lethal tick is reaped (death
    /// triggers fire) like a DoT. The `u32::MAX` source marks "the environment".
    fn terrain_phase(&mut self) {
        for i in 0..self.units.len() {
            if !self.units[i].is_alive() {
                continue;
            }
            let Some((dmg, dtype, pen)) = self.terrain.hazard(self.units[i].pos) else {
                continue;
            };
            let mult =
                armor::matrix(dtype, self.units[i].armor_class()) * self.units[i].vuln_mult();
            let before = self.units[i].integrity();
            self.units[i].character.apply_pool_damage(self.tick, u32::MAX, dmg * mult, pen, true);
            let dealt = before - self.units[i].integrity();
            if dealt > 0.0 {
                self.emit(CombatEvent::Damaged {
                    unit: self.units[i].id,
                    cause: "hazard",
                    amount: dealt,
                    killed: !self.units[i].is_alive(),
                });
            }
        }
    }

    /// The standalone **physical** pass (initiative order) — superseded in `step` by
    /// [`Battle::woven_phase`]; retained for isolated tests.
    #[cfg(test)]
    fn action_phase(&mut self) {
        // Deterministic order: effective initiative desc, id asc as the tiebreak.
        let mut order: Vec<usize> =
            (0..self.units.len()).filter(|&i| self.units[i].is_alive()).collect();
        order.sort_by(|&a, &b| {
            let (ua, ub) = (&self.units[a], &self.units[b]);
            ub.effective_initiative()
                .partial_cmp(&ua.effective_initiative())
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(ua.id.cmp(&ub.id))
        });

        for i in order {
            if !self.units[i].is_alive() || self.units[i].is_stunned() {
                continue;
            }
            self.physical_activation(i);
        }
    }

    /// One unit's **physical** activation (§7J/§10.4/§10.5): move by a **cost budget** of
    /// `speed`, then fire the best weapon whose **range band** covers a target.
    ///
    /// The move goal is normally the enemy picked by the targeting profile (a *closing*
    /// profile halts at standoff — once a weapon reaches — so a ranged build doesn't walk
    /// into melee). But a player unit **not already in a fight** is pulled toward an unmet
    /// **objective** hex (Reach / Hold), flowing to the point and shooting through. Each
    /// step spends [`Terrain::move_cost`]; the soft zone edge costs more, so a unit can't
    /// push far past it (the friction that corners a kiter without a wall).
    fn physical_activation(&mut self, i: usize) {
        let enemy = self.select_target(i);
        // Objective pull: the nearest-N designated seekers push the point *through* combat —
        // they flow to it and still fire after moving — while everyone else fights normally.
        // (A seeker doesn't hang back for a melee; taking the objective is its job.)
        let objective = self.seeks_objective(i);
        let closes = matches!(
            self.units[i].movement(),
            MovementProfile::Advance | MovementProfile::Flank | MovementProfile::Swarm
        );

        let mut budget = self.units[i].speed.max(0);
        loop {
            let next = if let Some(goal) = objective {
                if self.units[i].pos == goal {
                    break; // standing on the objective — hold it
                }
                self.close_step(i, goal, false)
            } else if let Some(target) = enemy {
                if closes && self.units[i].weapon_at(self.reach(i, target)).is_some() {
                    break; // standoff: a weapon already reaches — stop closing and fire
                }
                self.movement_step(i, target)
            } else {
                break; // nothing to chase
            };
            if next == self.units[i].pos {
                break; // at the goal, boxed in, or priced out of the only step
            }
            let cost = self.terrain.move_cost(next);
            if cost > budget {
                break; // soft-edge friction: can't afford to push further out this tick
            }
            budget -= cost;
            let from = self.units[i].pos;
            self.units[i].pos = next;
            self.emit(CombatEvent::Moved { unit: self.units[i].id, from, to: next });
        }

        // Act: fire on the best enemy now in range (positions have changed).
        if let Some(target) = self.select_target(i) {
            let dist = self.reach(i, target);
            if let Some(weapon) = self.units[i].weapon_at(dist) {
                self.resolve_attack_with(i, target, weapon);
            }
        }
    }

    /// Should player unit `i` chase the objective this activation, and toward which hex?
    /// `Some(hex)` when `i` is among the **nearest N** living player units to an unmet
    /// positional objective (`N` = its `seeker_pct` of the live squad, ≥1) — so only a share
    /// peels off and the rest keep fighting. When the phase has **several foci** (a Search
    /// with multiple unswept spots) the seekers are **assigned across them** — each focus
    /// drawn to its nearest free seeker — so they fan out in parallel rather than queue.
    fn seeks_objective(&self, i: usize) -> Option<Hex> {
        if self.units[i].team != Team::A {
            return None;
        }
        let (foci, pct) = self.objectives.foci(&self.units, self.tick, self.fight_over())?;
        let players: Vec<usize> = (0..self.units.len())
            .filter(|&j| self.units[j].is_alive() && self.units[j].team == Team::A)
            .collect();
        // N = that percent of the live squad (≥1) — scales down as units fall. Seekers are
        // the N players nearest *any* focus.
        let n = ((pct as f32 / 100.0 * players.len() as f32).round() as usize).max(1);
        let dist_to_set = |j: usize| {
            foci.iter().map(|f| self.units[j].pos.distance(*f)).min().unwrap_or(i32::MAX)
        };
        let mut ranked = players;
        ranked.sort_by_key(|&j| (dist_to_set(j), self.units[j].id.0));
        let seekers: Vec<usize> = ranked.into_iter().take(n).collect();
        if !seekers.contains(&i) {
            return None;
        }
        // Assign: walk the foci, each claiming its nearest still-free seeker (cycling if
        // there are more seekers than foci) — i ends up on the focus it's drawn to.
        let mut free = seekers;
        let mut f = 0usize;
        while !free.is_empty() {
            let focus = foci[f % foci.len()];
            let pick = free
                .iter()
                .enumerate()
                .min_by_key(|(_, &j)| (self.units[j].pos.distance(focus), self.units[j].id.0))
                .map(|(p, _)| p)?;
            let j = free.remove(pick);
            if j == i {
                return Some(focus);
            }
            f += 1;
        }
        None
    }

    /// The **engagement distance** between two units — the grid distance, except a
    /// front-line pair the seam stagger joins (§7B) reads as **1** (the `Up` offset
    /// closes its half-hex gap into melee). Used for weapon-range / reach checks.
    fn reach(&self, a: usize, b: usize) -> i32 {
        let (pa, pb) = (self.units[a].pos, self.units[b].pos);
        let d = pa.distance(pb);
        if d > 1 && self.board.engages(pa, pb) {
            1
        } else {
            d
        }
    }

    /// Pick a target among living enemies by unit `i`'s **targeting profile** (§7J),
    /// `id` as the deterministic tiebreak.
    fn select_target(&self, i: usize) -> Option<usize> {
        let me = &self.units[i];
        // Don't **weapon**-target the **data node** — an *unarmed* enemy sitting on one of our
        // objective focus hexes — we mean to *hack* it, not slag it (`netrunning.md`). Both
        // conditions matter: an *armed* enemy contesting a Hold/Capture point is still shot
        // normally, and a stray unarmed unit off-objective isn't spared.
        let foci = self
            .objectives
            .foci(&self.units, self.tick, self.fight_over())
            .map(|(h, _)| h)
            .unwrap_or_default();
        let foci = &foci;
        let enemies = || {
            self.units.iter().enumerate().filter(move |(j, u)| {
                *j != i
                    && u.is_alive()
                    && u.team == me.team.enemy()
                    && !(foci.contains(&u.pos) && u.weapon_at_any().is_none())
            })
        };
        let by_f32 = |key: fn(&Unit) -> f32, want_max: bool| {
            enemies()
                .min_by(|(ja, a), (jb, b)| {
                    let (ka, kb) = (key(a), key(b));
                    let ord = ka.partial_cmp(&kb).unwrap_or(std::cmp::Ordering::Equal);
                    let ord = if want_max { ord.reverse() } else { ord };
                    ord.then(ja.cmp(jb))
                })
                .map(|(j, _)| j)
        };
        match me.targeting() {
            TargetingProfile::Nearest => enemies()
                .min_by_key(|(_, u)| (me.pos.distance(u.pos), u.id))
                .map(|(j, _)| j),
            TargetingProfile::Backline => enemies()
                .max_by_key(|(_, u)| (me.pos.distance(u.pos), std::cmp::Reverse(u.id)))
                .map(|(j, _)| j),
            TargetingProfile::LowestIntegrity => by_f32(|u| u.integrity(), false),
            TargetingProfile::HighestThreat => {
                by_f32(|u| u.weapon_at_any().map_or(0.0, |w| w.damage), true)
            }
            TargetingProfile::WeakestArmor => {
                by_f32(|u| u.character.barrier + u.character.plating, false)
            }
        }
    }

    /// One step for unit `i` toward what its **movement profile** wants (§7J), routed only
    /// through **passable, free** hexes — on-board, not a blocker (`terrain`), and not held
    /// by another unit (§10.5a). *Closing* profiles path around walls (BFS on a bounded
    /// board); *fleeing* profiles back off toward the farthest free hex (cornered against
    /// the board edge). Hazards are stepped around when there's a choice. Returns the
    /// unit's own hex when it's already at the goal or **boxed in**.
    fn movement_step(&self, i: usize, target: usize) -> Hex {
        let here = self.units[i].pos;
        let tpos = self.units[target].pos;
        // Stay put, or step to a passable, unoccupied neighbour. `once(here)` is first so
        // it wins ties — a unit only moves when a neighbour is strictly better.
        let open = |h: &Hex| self.terrain.passable(*h) && !self.occupied_by_other(i, *h);
        let burn = |h: &Hex| self.terrain.hazard(*h).is_some() as i32; // 1 ⇒ avoid if able
        let candidates = || std::iter::once(here).chain(here.neighbors().into_iter().filter(open));
        match self.units[i].movement() {
            MovementProfile::Hold => here,
            MovementProfile::Advance => self.close_step(i, tpos, false),
            MovementProfile::Flank => self.close_step(i, tpos, true),
            MovementProfile::Swarm => match self.nearest_enemy(i) {
                Some(e) => self.close_step(i, self.units[e].pos, false),
                None => here,
            },
            // Skirmish: hold the weapon's standoff range — *close* when out of range
            // (no fleeing to a stalemate), *back off* when crowded (bounded → cornered),
            // *hold and fire* at the sweet spot. The proper kite, not "flee to infinity".
            MovementProfile::Kite => match self.nearest_enemy(i) {
                Some(e) => {
                    let ep = self.units[e].pos;
                    let opt = self.units[i].max_weapon_range();
                    match here.distance(ep) {
                        d if d > opt => self.close_step(i, ep, false),
                        d if d < opt => candidates()
                            .max_by_key(|h| (h.distance(ep), -burn(h), -h.q, -h.r))
                            .unwrap_or(here),
                        _ => here,
                    }
                }
                None => here,
            },
            MovementProfile::Disperse => match self.nearest_ally(i) {
                Some(a) => {
                    let ap = self.units[a].pos;
                    candidates().max_by_key(|h| (h.distance(ap), -burn(h), -h.q, -h.r)).unwrap_or(here)
                }
                None => here,
            },
        }
    }

    /// One **closing** step from unit `i` toward `goal`, routing around blockers / edges
    /// and shying from hazards. On a **bounded** board this follows a BFS flow field (real
    /// pathing — a wall is rounded, not stuck against); on the open default plane it's the
    /// greedy nearest neighbour (no obstacles to route around, so prior behavior is kept,
    /// bit-for-bit). `flank` breaks ties toward the target's frontage row (approach wide).
    fn close_step(&self, i: usize, goal: Hex, flank: bool) -> Hex {
        let here = self.units[i].pos;
        let open = |h: Hex| self.terrain.passable(h) && !self.occupied_by_other(i, h);
        let burn = |h: Hex| self.terrain.hazard(h).is_some() as i32; // 1 ⇒ avoid
        let cover = |h: Hex| -self.terrain.cover_tn(h); // negative ⇒ prefer more cover
        let lateral = |h: Hex| if flank { -(h.r - goal.r).abs() } else { 0 };
        // Tiebreak among equally-progressing steps: dodge hazards, then take cover, then
        // the flank bias, then a stable positional key. Progress (path/grid) always wins.
        let neighbours = || here.neighbors().into_iter().filter(|h| open(*h));
        if self.terrain.zone().is_some() {
            // BFS hop-distance from the goal over traversable hexes: the step is the
            // neighbour nearest the goal *by real path*, so walls and edges get rounded.
            let flow = self.flow_field(i, goal);
            let here_d = flow.get(&here).copied().unwrap_or(i32::MAX);
            neighbours()
                .filter_map(|h| flow.get(&h).map(|d| (h, *d)))
                .filter(|(_, d)| *d < here_d) // only a step that genuinely closes
                .min_by_key(|(h, d)| (*d, burn(*h), cover(*h), lateral(*h), h.q, h.r))
                .map_or(here, |(h, _)| h)
        } else {
            // Open plane: greedy single-hex toward the goal (`once(here)` wins ties).
            std::iter::once(here)
                .chain(neighbours())
                .min_by_key(|h| (h.distance(goal), burn(*h), cover(*h), lateral(*h), h.q, h.r))
                .unwrap_or(here)
        }
    }

    /// BFS hop-distances **from `goal` outward** over hexes unit `i` could traverse
    /// (passable terrain, not held by another unit) — a flow field for pathing around
    /// obstacles. The goal hex seeds distance 0 even when occupied (it's the thing being
    /// approached), and `i`'s own hex is always traversable. Bounded by the board extent,
    /// so it terminates; only called on a bounded board.
    fn flow_field(&self, i: usize, goal: Hex) -> std::collections::HashMap<Hex, i32> {
        use std::collections::{HashMap, VecDeque};
        let mut dist: HashMap<Hex, i32> = HashMap::new();
        let mut frontier = VecDeque::new();
        dist.insert(goal, 0);
        frontier.push_back(goal);
        while let Some(cur) = frontier.pop_front() {
            let d = dist[&cur];
            for n in cur.neighbors() {
                if dist.contains_key(&n) || !self.terrain.pathable(n) {
                    continue;
                }
                let traversable = (self.terrain.passable(n) && !self.occupied_by_other(i, n))
                    || n == self.units[i].pos;
                if traversable {
                    dist.insert(n, d + 1);
                    frontier.push_back(n);
                }
            }
        }
        dist
    }

    /// Is hex `h` occupied by a *living* unit other than `i`? (§10.5a — occupied hexes
    /// block movement; sequential resolution means a unit sees those that already
    /// moved this activation.)
    fn occupied_by_other(&self, i: usize, h: Hex) -> bool {
        self.units
            .iter()
            .enumerate()
            .any(|(j, u)| j != i && u.is_alive() && u.pos == h)
    }

    fn nearest_ally(&self, i: usize) -> Option<usize> {
        let me = &self.units[i];
        self.units
            .iter()
            .enumerate()
            .filter(|(j, u)| *j != i && u.is_alive() && u.team == me.team)
            .min_by_key(|(_, u)| (me.pos.distance(u.pos), u.id))
            .map(|(j, _)| j)
    }

    /// The **cleanup phase** (§1): wear every unit's gen by one decay step — durations
    /// lose a tick, stack-decay statuses lose a stack — and drop the expired.
    fn decay_phase(&mut self) {
        for u in &mut self.units {
            u.character.decay();
        }
    }

    fn nearest_enemy(&self, i: usize) -> Option<usize> {
        let me = &self.units[i];
        self.units
            .iter()
            .enumerate()
            .filter(|(j, u)| *j != i && u.is_alive() && u.team == me.team.enemy())
            .min_by_key(|(_, u)| (me.pos.distance(u.pos), u.id))
            .map(|(j, _)| j)
    }

    /// Resolve the attacker's **primary** weapon onto `target` (the simple path used by
    /// isolated tests).
    #[cfg(test)]
    fn resolve_attack(&mut self, attacker: usize, target: usize) {
        if let Some(w) = self.units[attacker].weapon_at_any() {
            self.resolve_attack_with(attacker, target, w);
        }
    }

    /// Resolve a chosen weapon `atk` from `attacker` onto `target`. Resolves the
    /// footprint to the struck units (friendly fire on for physical AoE), then runs the
    /// damage pipeline per target — the armor matrix (type vs class) × that unit's
    /// Breach vulnerability.
    fn resolve_attack_with(&mut self, attacker: usize, target: usize, atk: Attack) {
        let atk_id = self.units[attacker].id;
        // To-hit — the **opposed roll-under** core, pure GURPS (`docs/stats.md`): the attacker
        // rolls `2d10 ≤ weapon skill + accuracy − range − cover` (weapon skill = governing
        // attribute + tier, ~10-14), and the target rolls an active **Evade**, `2d10 ≤ Dex +
        // evade-tier`. Evade is just a Dex skill — but it *defaults to untrained* (−4), so a
        // non-dodger sits at `Dex − 4` (a secondary save) while an acrobat climbs. No doubling,
        // no base: the skill *is* the target. The blow lands only if the attacker connects
        // **and** the defender fails to dodge. Range = `ranged` (grows with distance) + `awkward`
        // (bites up close); cover is the hex's bonus — both shrink the to-hit target. An
        // **undefended** target (Evade ≤ 0) with a clear shot is auto-hit (no RNG; dice only
        // matter once the target can dodge or the shot is hard).
        let dist = self.reach(attacker, target);
        let penalty = atk.tags.to_hit_penalty(dist) + self.terrain.cover_tn(self.units[target].pos);
        let atk_target =
            self.units[attacker].effective_skill(atk.skill) + atk.accuracy - penalty;
        // A fast attack is far harder to dodge than a slow one — the weapon's Speed docks Evade.
        let evade = self.units[target].evasion() - atk.speed;
        let landed = if evade <= 0 {
            penalty <= 0 || resolve_check(&mut self.rng, atk_target).success
        } else {
            resolve_opposed(&mut self.rng, atk_target, evade).landed
        };
        if !landed {
            self.emit(CombatEvent::Missed { attacker: atk_id, target: self.units[target].id });
            return; // whiff — the whole attack (incl. its AoE) misses
        }
        // Weapon base + the attacker's composed damage bonus (an implant combat-stim).
        let base = atk.damage + self.units[attacker].damage_bonus();
        let src = atk_id.0;
        for t in self.footprint_targets(attacker, target, atk) {
            let mult =
                armor::matrix(atk.dtype, self.units[t].armor_class()) * self.units[t].vuln_mult();
            let dmg = base * mult;
            let (before, was_alive) = (self.units[t].integrity(), self.units[t].is_alive());
            // Pipeline: Barrier/Plating pools soak first, then the **hit-location** roll —
            // a blow that lands on cyberware is taken by *its* HP (chrome is extra hit-table,
            // effectively more HP — fewer meat hits), overflow from a wrecked piece rerolls;
            // only what reaches the **flesh** wounds Integrity.
            let past_armor = self.units[t].character.absorb_armor(dmg, atk.pen);
            let to_flesh = self.resolve_hit_location(t, past_armor);
            self.units[t].character.apply_integrity(self.tick, src, to_flesh, true);
            self.emit(CombatEvent::Attacked {
                attacker: atk_id,
                target: self.units[t].id,
                dtype: atk.dtype,
                amount: before - self.units[t].integrity(),
                killed: was_alive && !self.units[t].is_alive(),
            });
            if atk.emp && self.units[t].is_alive() {
                self.apply_emp(t);
            }
        }
    }

    /// Roll **hit location** for a blow of `dmg` that got past the armor pools, and return
    /// how much ultimately wounds the **flesh** (Integrity). The body owns a fixed slice of
    /// the roll domain (a `1..100`-style band) and each *active* implant **extends** it by
    /// its coverage (e.g. body `1..100` + a coverage-25 cyberleg ⇒ domain `1..125`, with
    /// `101..125` the leg) — so more chrome means **fewer meat hits**. A body slice sends the
    /// whole `dmg` to the flesh; an implant slice means *it* takes the hit into its HP, and
    /// any **overflow** past a wrecked piece **rerolls** over the smaller table. No RNG is
    /// drawn for a chrome-less target, so flesh-and-bone fights stay bit-for-bit as before.
    fn resolve_hit_location(&mut self, t: usize, dmg: f32) -> f32 {
        let mut remaining = dmg;
        while remaining > 0.0 {
            let live: Vec<usize> = (0..self.units[t].implants.len())
                .filter(|&k| self.units[t].implant_condition(k) != Condition::Destroyed)
                .collect();
            if live.is_empty() {
                return remaining; // no chrome left — the rest wounds the flesh
            }
            let body = self.units[t].chassis.coverage();
            let total =
                body + live.iter().map(|&k| self.units[t].implants[k].spec.coverage).sum::<i32>();
            let roll = (self.rng.next_u64() % total.max(1) as u64) as i32; // 0..total
            if roll < body {
                return remaining; // struck the meat — wound the flesh
            }
            let mut acc = body;
            let mut hit = live[live.len() - 1]; // roll ≥ body lands in some implant slice
            for &k in &live {
                acc += self.units[t].implants[k].spec.coverage;
                if roll < acc {
                    hit = k;
                    break;
                }
            }
            remaining = self.wreck_implant(t, hit, remaining); // chrome eats up to its HP; overflow loops
        }
        0.0 // fully absorbed by chrome — the flesh is spared
    }

    /// Implant `k` of unit `t` takes a `dmg` hit into its HP — absorbing up to its remaining
    /// durability, stepping its [`Condition`] as it falls (**Degraded** < 50%, **Destroyed**
    /// at 0: terminal, benefit gone, no salvage) — and returns the **overflow** (0 unless the
    /// piece was destroyed). Physical wreckage fires **no liability** (§3.1 — wear isn't a
    /// breach); surfaces a `Mangled` event.
    fn wreck_implant(&mut self, t: usize, k: usize, dmg: f32) -> f32 {
        let name = self.units[t].implants[k].spec.name;
        let max = self.units[t].implants[k].spec.max_hp;
        let hp = self.units[t].implants[k].hp;
        let absorbed = dmg.min(hp);
        let left = hp - absorbed;
        self.units[t].implants[k].hp = left;
        let destroyed = left <= 0.0;
        if destroyed {
            let _ = self.units[t].transition(k, Condition::Destroyed);
        } else if left < 0.5 * max && self.units[t].implant_condition(k) == Condition::Online {
            let _ = self.units[t].transition(k, Condition::Degraded);
        }
        self.emit(CombatEvent::Mangled { unit: self.units[t].id, implant: name, destroyed });
        dmg - absorbed // overflow to reroll
    }

    /// The living units an attack strikes (§7G). `Single` is just the target;
    /// `Blast` is the disc around the target hex; `Beam` is the line from the attacker
    /// along the bearing to the target. AoE includes **allies** (friendly fire) — only
    /// the attacker is spared — **unless the weapon is `smart`** (IFF), which also spares
    /// the attacker's whole team. Returned in ascending index order (deterministic).
    fn footprint_targets(&self, attacker: usize, target: usize, atk: Attack) -> Vec<usize> {
        use std::collections::HashSet;
        let hexes: HashSet<Hex> = match atk.footprint {
            Footprint::Single => return vec![target],
            Footprint::Blast(radius) => self.units[target].pos.within(radius).into_iter().collect(),
            Footprint::Beam(length) => {
                let from = self.units[attacker].pos;
                let dir = from.direction_to(self.units[target].pos);
                from.line(dir, length + 1).into_iter().skip(1).collect() // skip the attacker's own hex
            }
        };
        let team = self.units[attacker].team;
        (0..self.units.len())
            .filter(|&j| {
                j != attacker
                    && self.units[j].is_alive()
                    && hexes.contains(&self.units[j].pos)
                    // IFF: a smart weapon holds fire on the attacker's own team.
                    && !(atk.tags.spares_team() && self.units[j].team == team)
            })
            .collect()
    }

    /// An **EMP** pulse on `target` (§7I) — a *physical* breach that **bypasses
    /// Firewall** (no roll): fries **every** active implant (→ Offline) and fires
    /// its **degrade-class** liabilities at a fixed magnitude. The stun-class
    /// knockout is the hacker's finesse — EMP is blunt. Flesh / bioware (no chrome)
    /// are immune, and the more implants a target runs, the more an EMP ruins.
    fn apply_emp(&mut self, target: usize) {
        // Blunt: every active *digital* implant (inert physical armor is EMP-proof —
        // no circuitry to fry), degrade liabilities only (no knockout finesse).
        let slots = self.units[target].digital_implant_indices();
        self.breach_slots(target, slots, EMP_MAGNITUDE, false, BreachVector::Emp);
    }

    /// The **contagion phase** (`docs/corruption.md`) — every contagious corruption tries
    /// to **jump** to fresh victims along its [`Vector`]: a *contested* roll of the
    /// carrier's `virulence` vs the victim's resist stat (Immunity / Firewall). A win
    /// **copies the whole decorator** onto the victim; a unit already carrying that
    /// corruption is skipped (no re-infection / stacking), and jumps land *after* the
    /// scan, so a contagion spreads at most one hop per round (controlled exponential).
    /// Friend or foe: a plague doesn't read uniforms — keep your infected clear of allies.
    fn contagion_phase(&mut self) {
        use std::collections::HashSet;
        let n = self.units.len();
        let mut infected: HashSet<(usize, &'static str)> = HashSet::new();
        let mut jumps: Vec<(usize, usize, Decorator)> = Vec::new(); // (carrier, victim, strain)
        for i in 0..n {
            if !self.units[i].is_alive() {
                continue;
            }
            for dec in self.units[i].character.active_contagions() {
                let c = dec.contagion.expect("active_contagions filters Some");
                let label = dec.label;
                for j in 0..n {
                    // Target only units that can **host** it — i.e. carry *and re-spread*
                    // it (a valid surface) — and that the vector reaches.
                    if j == i
                        || !self.units[j].is_alive()
                        || !self.can_host(j, c)
                        || !self.in_vector(i, j, c.vector)
                    {
                        continue;
                    }
                    // Re-infection guard: already carries it, or already caught it this phase.
                    if let Some(l) = label {
                        if self.units[j].character.carries(l) || !infected.insert((j, l)) {
                            continue;
                        }
                    }
                    let tn = self.resist_of(j, c.resist);
                    if resolve_versus(&mut self.rng, c.virulence, tn).success {
                        jumps.push((i, j, dec.clone()));
                    }
                }
            }
        }
        for (i, j, dec) in jumps {
            let (from, to, family) =
                (self.units[i].id, self.units[j].id, dec.label.unwrap_or("corruption"));
            self.units[j].character.install(dec); // install re-stamps the GenId
            self.emit(CombatEvent::Spread { from, to, family });
        }
    }

    /// Can unit `j` **host** this contagion — carry it *and re-spread* it? "Target those
    /// who can spread it": a contagion only takes in a unit with the matching surface — a
    /// **bio** strain (Immunity-resisted) needs a biological body; a **digital** one
    /// (Firewall-resisted) needs a live net surface (`Link > 0`). A unit that can't host
    /// it is a dead end, so it's never infected.
    fn can_host(&self, j: usize, c: Contagion) -> bool {
        match c.resist {
            Stat::Firewall => self.units[j].link() > 0,
            Stat::Immunity => self.units[j].chassis.is_biological(),
            _ => true,
        }
    }

    /// Can a contagion on carrier `i` reach unit `j` along `vector`?
    fn in_vector(&self, i: usize, j: usize, vector: Vector) -> bool {
        match vector {
            Vector::Proximity(r) => self.units[i].pos.distance(self.units[j].pos) <= r,
            Vector::Net => self.units[j].link() > 0, // the net is everywhere
        }
    }

    /// Unit `j`'s value of a contagion-resist stat (only Immunity / Firewall defend a jump).
    fn resist_of(&self, j: usize, resist: Stat) -> i32 {
        match resist {
            Stat::Firewall => self.units[j].firewall(),
            _ => self.units[j].immunity(),
        }
    }

    /// The standalone **digital** pass (Link order) — superseded in `step` by
    /// [`Battle::woven_phase`]; retained for isolated tests.
    #[cfg(test)]
    fn digital_phase(&mut self) {
        let mut order: Vec<usize> = (0..self.units.len())
            .filter(|&i| {
                let u = &self.units[i];
                u.is_alive() && u.hack().is_some() && u.link() > 0
            })
            .collect();
        order.sort_by(|&a, &b| {
            let (ua, ub) = (&self.units[a], &self.units[b]);
            ub.link().cmp(&ua.link()).then(ua.id.cmp(&ub.id))
        });

        for i in order {
            if !self.units[i].is_alive() || self.units[i].is_stunned() {
                continue;
            }
            self.digital_activation(i);
        }
    }

    /// One unit's **digital** activation (§10.8): hack the nearest reachable enemy.
    /// (Caller has already gated alive / not-stunned and the Link-presence filter.)
    fn digital_activation(&mut self, i: usize) {
        if let Some(target) = self.nearest_hackable_enemy(i) {
            self.resolve_hack(i, target);
        }
    }

    /// The **woven** activation order (§7C/§10.3): every living unit contributes a
    /// **physical** activation (ranked by effective Initiative) and, if it can project
    /// onto the net (a hack + Link > 0), a **digital** one (ranked by Link) — all on
    /// one descending track, so a high-Link runner hacks before a sluggish bruiser
    /// swings. Ties: lower `id` first, then physical before digital. Each entry is
    /// `(unit, is_digital)`.
    fn woven_order(&self) -> Vec<(usize, bool)> {
        // (key desc, id, kind-tiebreak, unit, is_digital)
        let mut order: Vec<(f32, u32, u8, usize, bool)> = Vec::new();
        for i in 0..self.units.len() {
            let u = &self.units[i];
            if !u.is_alive() {
                continue;
            }
            order.push((u.effective_initiative(), u.id.0, 0, i, false));
            if u.hack().is_some() && u.link() > 0 {
                order.push((u.link() as f32, u.id.0, 1, i, true));
            }
        }
        order.sort_by(|a, b| {
            b.0.partial_cmp(&a.0)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(a.1.cmp(&b.1))
                .then(a.2.cmp(&b.2))
        });
        order.into_iter().map(|(_, _, _, i, d)| (i, d)).collect()
    }

    /// Run the woven order (§10.3). The order is snapshotted at phase start; the dead /
    /// stunned are skipped as it runs, and death triggers are reaped after each
    /// activation.
    fn woven_phase(&mut self) {
        for (i, digital) in self.woven_order() {
            if !self.units[i].is_alive() || self.units[i].is_stunned() {
                continue;
            }
            if digital {
                self.digital_activation(i);
            } else {
                self.physical_activation(i);
            }
            self.reap();
        }
    }

    /// Fire the **death trigger** (§10.9) of every newly-dead unit, once each. Loops
    /// because a `Detonate` can chain-kill, which fires further triggers; the
    /// lowest-index unfired death goes first (deterministic).
    fn reap(&mut self) {
        while let Some(i) = (0..self.units.len())
            .find(|&i| !self.units[i].is_alive() && !self.units[i].death_resolved)
        {
            self.units[i].death_resolved = true;
            self.emit(CombatEvent::Died { unit: self.units[i].id });
            self.fire_death_trigger(i);
        }
    }

    fn fire_death_trigger(&mut self, i: usize) {
        let center = self.units[i].pos;
        match self.units[i].on_death {
            DeathTrigger::None => {}
            DeathTrigger::Detonate { damage, dtype, pen, radius } => {
                use std::collections::HashSet;
                let hexes: HashSet<Hex> = center.within(radius).into_iter().collect();
                let src = self.units[i].id.0;
                for t in 0..self.units.len() {
                    if t != i && self.units[t].is_alive() && hexes.contains(&self.units[t].pos) {
                        let mult = armor::matrix(dtype, self.units[t].armor_class())
                            * self.units[t].vuln_mult();
                        self.units[t]
                            .character
                            .apply_pool_damage(self.tick, src, damage * mult, pen, true);
                    }
                }
            }
            DeathTrigger::DataSpill { spec, stacks, duration, radius } => {
                let team = self.units[i].team;
                for t in 0..self.units.len() {
                    let u = &self.units[t];
                    if u.is_alive() && u.team != team && center.distance(u.pos) <= radius {
                        self.units[t].add_status(spec, duration, stacks);
                    }
                }
            }
            DeathTrigger::Legacy { spec, stacks, duration, radius } => {
                let team = self.units[i].team;
                for t in 0..self.units.len() {
                    let u = &self.units[t];
                    if t != i && u.is_alive() && u.team == team && center.distance(u.pos) <= radius {
                        self.units[t].add_status(spec, duration, stacks);
                    }
                }
            }
        }
    }

    /// The best **net defense** available to `target`'s side (`netrunning.md` §2) — the value
    /// a hack is opposed by. It's the highest of: the target's passive **Firewall**; its own
    /// **Hacking**, if the target is itself a runner (it parries code with code); and the
    /// **Hacking of any allied netrunner covering it** — a living ally with a deck (Link > 0)
    /// whose antenna reach spans the target, so a runner can actively defend a **node it
    /// controls**. `≤ 0` ⇒ an undefended surface (no active defense, no defender roll).
    fn net_defense(&self, target: usize) -> i32 {
        let t = &self.units[target];
        let mut d = t.firewall();
        if t.hack().is_some() {
            d = d.max(t.effective_skill(Skill::Hacking));
        }
        for (i, ally) in self.units.iter().enumerate() {
            if i == target || ally.team != t.team || !ally.is_alive() {
                continue;
            }
            if ally.hack().is_some() && ally.link() > 0 && ally.pos.distance(t.pos) <= ally.hack_reach() {
                d = d.max(ally.effective_skill(Skill::Hacking));
            }
        }
        d
    }

    /// Resolve a netrunning hack from `attacker` onto `target` (§7F, §10.8): an **opposed**
    /// roll — the runner rolls `2d10 ≤ avg(effective Hacking, channel)` and the target's
    /// **net defense** ([`Battle::net_defense`]) rolls back; the breach lands only if the
    /// runner connects **and** the defense fails. The **channel** is the weaker endpoint's
    /// Link bandwidth (`min`), so a darker target is harder to crack. Zero Link stays the
    /// hard reachability gate. Lands the hack's payload (margin-scaled) on success.
    pub fn resolve_hack(&mut self, attacker: usize, target: usize) -> HackResult {
        let Some(hack) = self.units[attacker].hack() else {
            return HackResult::NoHack;
        };
        // Hard gate (§7D/§7F): a runner needs net presence; the target a surface.
        if self.units[attacker].link() <= 0 {
            return HackResult::Offline;
        }
        if self.units[target].link() <= 0 {
            return HackResult::NoSurface;
        }
        // The connection runs at the weaker endpoint's bandwidth (the channel); the rating
        // averages the attacker's **effective Hacking** (Intellect + tier, GURPS-scaled) with
        // it. The hack is an **opposed roll** (`docs/stats.md` §4, like combat): the runner
        // rolls to crack while the target's **Firewall** rolls an active defense — the breach
        // lands only if the runner connects **and** the Firewall fails to repel it. A darker
        // target (lower channel) is harder to crack; a stiffer Firewall defends more often.
        let channel = self.units[attacker].digital_band().min(self.units[target].digital_band());
        let rating = hack_rating(self.units[attacker].effective_skill(Skill::Hacking), channel)
            + self.units[attacker].mesh_synergy(); // meshed PAN throughput (§5)
        // **Active net defense** (`netrunning.md` §2): the breach is opposed by whichever
        // defense is **favorable** — the target's passive Firewall, its own Hacking if it is a
        // runner, or the Hacking of an **allied netrunner covering it** (so a runner defends a
        // node it controls). See [`Battle::net_defense`].
        let defense = self.net_defense(target);
        // An **undefended** surface (defense ≤ 0) has no active defense — the runner just rolls
        // to crack (like an undefended melee blow, §4). Otherwise the defense rolls back, and
        // the breach lands only if the runner connects **and** the defense fails. The runner's
        // roll carries the margin / crit that scale the payload either way.
        let outcome = if defense > 0 {
            let opposed = resolve_opposed(&mut self.rng, rating, defense);
            RollOutcome { success: opposed.landed, ..opposed.attack }
        } else {
            resolve_check(&mut self.rng, rating)
        };
        self.emit(CombatEvent::Hacked {
            attacker: self.units[attacker].id,
            target: self.units[target].id,
            success: outcome.success,
            crit: outcome.crit,
            margin: outcome.margin,
        });
        // Read heat-prone chrome **before** the breach (which disables the very implant) — forcing
        // hot chrome is what cooks it.
        let runs_hot = self.units[target].has_volatile_chrome();
        let stacks =
            if outcome.success { self.apply_breach(target, &outcome, hack) } else { 0 };
        // The **Overheat program** (`netrunning.md`): an *equipped* deck loadout (not innate to
        // hacking) that, on a **solid** breach, cooks the target's **`VOLATILE`** chrome — an
        // Internal DoT scaling with the margin. No program, a marginal hack (the §6 floor), or a
        // target with no heat-prone chrome ⇒ no heat.
        let burn = hack::margin_stacks(outcome.margin);
        if outcome.success && hack.overheats && burn > 0 && runs_hot {
            self.units[target].add_status(StatusSpec::overheat(), OVERHEAT_DURATION, burn);
        }
        HackResult::Rolled { outcome, stacks }
    }

    /// Apply a successful hack's consequences — the §6 **severity ladder**
    /// (`docs/cyberware.md`): breach a target implant (**disable** floor →
    /// margin-scaled **degrade** → crit **knockout**), firing its `hack_effects`
    /// per effect. If the target carries no chrome to trip, land the deck's own
    /// payload instead (a generic intrusion). Returns the magnitude landed.
    fn apply_breach(&mut self, target: usize, outcome: &RollOutcome, hack: Hack) -> u32 {
        let Some(first) = self.units[target].first_digital_implant() else {
            // No *digital* chrome to trip — run the deck's own payload.
            let stacks = hack.stacks_for(outcome);
            if stacks > 0 {
                self.units[target].add_status(hack.payload, hack.duration, stacks);
            }
            return stacks;
        };
        // Cascade (§5): a crit on a **meshed** PAN rides the net to *every*
        // implant; a segmented PAN contains it to the one slot.
        let slots = if outcome.crit && self.units[target].pan == Pan::Meshed {
            self.units[target].digital_implant_indices()
        } else {
            vec![first]
        };
        let degrade = hack::margin_stacks(outcome.margin);
        self.breach_slots(target, slots, degrade, knockout_gated(outcome, KNOCKOUT_MARGIN), BreachVector::Hack);
        degrade
    }

    /// Fire the §6 breach ladder across `slots` of `target`: **disable** each (the
    /// floor), then per returned liability fire the **degrade** class at magnitude
    /// `degrade` and — iff `decisive` — the gated **stun** class. The shared core of
    /// every breach vector (hack, EMP, Worm); only the slot set, `degrade`, and
    /// `decisive` differ.
    fn breach_slots(
        &mut self,
        target: usize,
        slots: Vec<usize>,
        degrade: u32,
        decisive: bool,
        vector: BreachVector,
    ) {
        let target_id = self.units[target].id;
        for idx in slots {
            let liabilities = self.units[target].disable_implant(idx);
            self.emit(CombatEvent::Breached { target: target_id, vector });
            for spec in liabilities {
                if matches!(spec.effect, Effect::Stun) {
                    if decisive {
                        self.units[target].add_status(spec, KNOCKOUT_STUN, 1);
                    }
                } else if degrade > 0 {
                    self.units[target].add_status(spec, degrade, degrade);
                }
            }
        }
    }

    /// A **Worm breach** (`cyberware.md` §6) — the logic-bomb / Cascade vector, no roll.
    /// Trips `target`'s first active implant; on a **meshed** PAN it **Cascades** to
    /// *every* implant, a **segmented** PAN contains it to the one. A worm trips
    /// *regardless of margin* (the finisher), so it disables + fires the degrade
    /// liabilities at a fixed base — but the **stun class stays gated** (a worm doesn't
    /// crit). Returns how many slots it tripped. Flesh / no chrome ⇒ nothing to trip.
    pub fn worm_breach(&mut self, target: usize) -> usize {
        let Some(first) = self.units[target].first_digital_implant() else {
            return 0;
        };
        let slots = if self.units[target].pan == Pan::Meshed {
            self.units[target].digital_implant_indices()
        } else {
            vec![first]
        };
        let n = slots.len();
        self.breach_slots(target, slots, WORM_DEGRADE, false, BreachVector::Worm);
        n
    }

    /// Nearest enemy with a digital surface (Link > 0) within the unit's antenna
    /// range — the hack's target selection.
    fn nearest_hackable_enemy(&self, i: usize) -> Option<usize> {
        let me = &self.units[i];
        let range = me.hack_reach(); // Link governs antenna reach (`netrunning.md`)
        // **Objective-aware** (`netrunning.md`): a runner prioritizes cracking a hackable enemy
        // sitting on an objective focus hex (a Datamine **node**) over poking the nearest grunt
        // — so the dive actually drives toward the prize. Falls back to nearest otherwise.
        let foci = self
            .objectives
            .foci(&self.units, self.tick, self.fight_over())
            .map(|(hexes, _)| hexes)
            .unwrap_or_default();
        self.units
            .iter()
            .enumerate()
            .filter(|(j, u)| {
                *j != i
                    && u.is_alive()
                    && u.team == me.team.enemy()
                    && u.link() > 0
                    && me.pos.distance(u.pos) <= range
            })
            .min_by_key(|(_, u)| (!foci.contains(&u.pos), me.pos.distance(u.pos), u.id))
            .map(|(j, _)| j)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(id: u32, team: Team, q: i32) -> Unit {
        // Unit::new's defaults already match (Mail armor, Augmented chassis, 10 melee,
        // Integrity 30 / Initiative 5); just place it.
        Unit::new(id, format!("U{id}"), team, Chassis::Augmented).at(Hex::new(q, 0))
    }

    /// A unit wired to hack: net presence + a Lockware deck (antenna `range`).
    /// Its hack strength comes from its own Link & Hacking — set those per test.
    fn runner(id: u32, team: Team, q: i32, range: i32) -> Unit {
        let mut u = unit(id, team, q);
        u.character.base_mut().link = 3.0;
        u.character.base_mut().intellect = 10.0; // GURPS-scale Intellect ⇒ effective Hacking = 10 + tier
        u.grant_hack(Hack::new(range, StatusSpec::lockware(), 1, 5));
        u
    }

    /// A unit with a hackable digital surface: Link > 0 and a Firewall security rating
    /// (now a flat penalty on a hacker's roll, `docs/stats.md` — not a TN).
    fn networked(id: u32, team: Team, q: i32, firewall: i32) -> Unit {
        let mut u = unit(id, team, q);
        u.character.base_mut().link = 2.0;
        u.character.base_mut().firewall = firewall as f32;
        u
    }

    fn duel() -> Battle {
        let mut a = unit(0, Team::A, 0);
        a.character.base_mut().initiative = 6.0;
        let b = unit(1, Team::B, 3);
        Battle::new(vec![a, b], 123)
    }

    #[test]
    fn layers_absorb_then_integrity() {
        let mut u = unit(0, Team::A, 0);
        u.character.barrier = 5.0;
        u.character.plating = 5.0;
        u.character.apply_pool_damage(0, 0, 12.0, PenTier::External, true);
        assert_eq!(u.character.integrity, 28.0); // 5 + 5 soaked, 2 through
        assert_eq!(u.character.barrier, 0.0);
        assert_eq!(u.character.plating, 0.0);
    }

    #[test]
    fn internal_bypasses_layers() {
        let mut u = unit(0, Team::A, 0);
        u.character.barrier = 99.0;
        u.character.plating = 99.0;
        u.character.apply_pool_damage(0, 0, 10.0, PenTier::Internal, true);
        assert_eq!(u.character.integrity, 20.0);
        assert_eq!(u.character.barrier, 99.0);
    }

    #[test]
    fn softener_never_kills() {
        let mut u = unit(0, Team::A, 0);
        u.character.apply_pool_damage(0, 0, 9999.0, PenTier::Internal, false);
        assert_eq!(u.character.integrity, 1.0);
        assert!(u.character.alive);
    }

    #[test]
    fn burn_dot_ticks_down_integrity() {
        let mut b = Battle::new(vec![unit(0, Team::A, 0)], 1);
        b.units[0].add_status(StatusSpec::burn(), 3, 2); // 2 stacks × 2 dmg, Contact
        let before = b.units[0].character.integrity;
        b.status_phase();
        // No plating ⇒ full 4 reaches Integrity.
        assert_eq!(b.units[0].character.integrity, before - 4.0);
    }

    #[test]
    fn full_immunity_blocks_poison() {
        let mut u = unit(0, Team::A, 0);
        u.character.base_mut().immunity = 30.0; // a −30 penalty: target (power − 30) is hopeless
        // Scripted mid-rolls (no natural crit, which would auto-fire regardless): every tick
        // the poison gate fails the immunity check, so it never bites.
        let mut b = Battle::with_rng(vec![u], ScriptedRng::from_d10([5, 5, 5, 5, 5, 5, 5, 5, 5, 5]));
        b.units[0].add_status(StatusSpec::poison(), 5, 1);
        let before = b.units[0].character.integrity;
        for _ in 0..5 {
            b.status_phase();
        }
        assert_eq!(b.units[0].character.integrity, before);
    }

    #[test]
    fn breach_amplifies_incoming_damage() {
        let attacker = unit(0, Team::A, 0);
        let mut target = unit(1, Team::B, 0); // same hex ⇒ in melee range
        target.add_status(StatusSpec::breach(), 3, 1); // ×1.5
        let mut b = Battle::new(vec![attacker, target], 1);
        // Piercing vs Mail = 1.0, so 10 base × 1.5 breach = 15.
        b.resolve_attack(0, 1);
        assert_eq!(b.units[1].character.integrity, 30.0 - 15.0);
    }

    #[test]
    fn crash_skips_the_action() {
        let mut b = duel();
        // Stun the faster unit; it should not move/attack this tick.
        b.units[0].add_status(StatusSpec::crash(), 1, 1);
        let pos_before = b.units[0].pos;
        b.action_phase();
        assert_eq!(b.units[0].pos, pos_before);
    }

    #[test]
    fn battle_terminates_with_a_winner() {
        let mut b = duel();
        assert!(matches!(b.resolve(1000), Outcome::Winner(_)));
    }

    #[test]
    fn resolution_is_deterministic() {
        let setup = || {
            let mut b = duel();
            b.units[1].add_status(StatusSpec::poison(), 99, 1); // exercise the RNG
            b
        };
        let mut x = setup();
        let mut y = setup();
        assert_eq!(x.resolve(1000), y.resolve(1000));
        assert_eq!(x.tick, y.tick);
        for (a, b) in x.units.iter().zip(&y.units) {
            assert_eq!(a.character.integrity, b.character.integrity);
            assert_eq!(a.pos, b.pos);
        }
    }

    #[test]
    fn injected_scripted_rng_forces_poison_to_fire() {
        let mut u = unit(0, Team::A, 0);
        u.character.base_mut().immunity = 5.0; // a −5 resist penalty
        u.add_status(StatusSpec::poison(), 5, 1);
        // versus: target = (power 10 + 1 stack) − Immunity 5 = 6; 2d10 = 6 makes it ⇒ fires.
        let mut b = Battle::with_rng(vec![u], ScriptedRng::from_d10([3, 3]));
        let before = b.units[0].character.integrity;
        b.status_phase();
        assert!(b.units[0].character.integrity < before);
    }

    #[test]
    fn injected_scripted_rng_forces_poison_to_whiff() {
        let mut u = unit(0, Team::A, 0);
        u.character.base_mut().immunity = 30.0; // resist TN out of reach
        u.add_status(StatusSpec::poison(), 5, 1);
        // 2d10 = 6, + power + stack = 10 < TN 30 ⇒ whiffs.
        let mut b = Battle::with_rng(vec![u], ScriptedRng::from_d10([3, 3]));
        let before = b.units[0].character.integrity;
        b.status_phase();
        assert_eq!(b.units[0].character.integrity, before);
    }

    #[test]
    fn unit_contest_uses_its_skill() {
        let mut u = unit(0, Team::A, 0);
        u.character.base_mut().body = 12.0;
        u.skills.set(Skill::Melee, 0); // competent ⇒ effective Melee 12
        // resolve_versus (resist as a modifier): target = effective 12 − resist 4 = 8; 2d10 = 7.
        let mut rng = ScriptedRng::from_d10([3, 4]);
        let o = u.contest(Skill::Melee, 0, 4, &mut rng);
        assert_eq!(o.margin, 1);
        assert!(o.success);
    }

    #[test]
    fn raising_a_skill_flips_a_contest() {
        let mut u = unit(0, Team::A, 0);
        u.character.base_mut().body = 12.0;
        u.skills.set(Skill::Melee, 0); // effective 12 ⇒ target 12 − resist 4 = 8
        let before = u.contest(Skill::Melee, 0, 4, &mut ScriptedRng::from_d10([5, 5])); // 10 > 8 ⇒ miss
        u.skills.raise(Skill::Melee, 4); // elite ⇒ effective 16 ⇒ target 12
        let after = u.contest(Skill::Melee, 0, 4, &mut ScriptedRng::from_d10([5, 5])); // 10 ≤ 12 ⇒ hit
        assert!(!before.success && after.success);
    }

    #[test]
    fn winfight_tracks_the_standard_result() {
        let won = vec![unit(0, Team::A, 0)]; // only player alive ⇒ enemy wiped
        assert_eq!(WinFight.status(&won, 1, true), ObjectiveStatus::Achieved);
        let mut lost = vec![unit(0, Team::A, 0), unit(1, Team::B, 1)];
        lost[0].character.alive = false; // player wiped
        assert_eq!(WinFight.status(&lost, 1, true), ObjectiveStatus::Failed);
    }

    #[test]
    fn winfight_fails_on_fight_end_without_victory() {
        // Both sides still standing: Pending while ongoing, Failed once the fight ends.
        let standoff = vec![unit(0, Team::A, 0), unit(1, Team::B, 1)];
        assert_eq!(WinFight.status(&standoff, 9, false), ObjectiveStatus::Pending);
        assert_eq!(WinFight.status(&standoff, 9, true), ObjectiveStatus::Failed);
    }

    #[test]
    fn survive_objective_met_at_the_deadline() {
        let obj = Survive { rounds: 3 };
        let alive = vec![unit(0, Team::A, 0)];
        assert_eq!(obj.status(&alive, 0, false), ObjectiveStatus::Pending);
        assert_eq!(obj.status(&alive, 3, false), ObjectiveStatus::Achieved);
        let mut dead = vec![unit(0, Team::A, 0)];
        dead[0].character.alive = false;
        assert_eq!(obj.status(&dead, 1, false), ObjectiveStatus::Failed);
    }

    #[test]
    fn hold_captures_by_round_or_by_clearing() {
        let hex = Hex::new(2, 0);
        let mut held = vec![unit(0, Team::A, 2), unit(1, Team::B, 5)]; // player on the hex
        held[0].pos = hex;
        // Controlled but ticked too early — doesn't latch yet.
        let mut early = Hold::new(hex, 3);
        early.tick(&held, 1);
        assert_eq!(early.status(&held, 1, false), ObjectiveStatus::Pending);
        // Ticked at the round while held → latches Achieved.
        let mut obj = Hold::new(hex, 3);
        obj.tick(&held, 3);
        assert_eq!(obj.status(&held, 3, false), ObjectiveStatus::Achieved);
        // Clearing the enemy while on the hex seals it immediately.
        let mut cleared = held.clone();
        cleared[1].character.alive = false;
        let mut cobj = Hold::new(hex, 9);
        cobj.tick(&cleared, 1);
        assert_eq!(cobj.status(&cleared, 1, false), ObjectiveStatus::Achieved);
        // Never held when the fight ends → Failed.
        let away = vec![unit(0, Team::A, 0), unit(1, Team::B, 5)];
        let mut aobj = Hold::new(hex, 3);
        aobj.tick(&away, 9);
        assert_eq!(aobj.status(&away, 9, true), ObjectiveStatus::Failed);
    }

    #[test]
    fn hold_latches_so_the_holder_can_wander_off() {
        // The fix for the capture scenario's buzzer-beaters: once held, achievement sticks
        // even after the unit leaves to mop up.
        let hex = Hex::new(2, 0);
        let mut held = vec![unit(0, Team::A, 2), unit(1, Team::B, 5)];
        held[0].pos = hex;
        let mut obj = Hold::new(hex, 1);
        obj.tick(&held, 1); // latched while on the hex
        let mut away = held.clone();
        away[0].pos = Hex::new(5, 0); // ...then walked off
        assert_eq!(obj.status(&away, 9, true), ObjectiveStatus::Achieved); // still ours
    }

    #[test]
    fn hold_is_not_held_while_contested() {
        let hex = Hex::new(2, 0);
        let mut both = vec![unit(0, Team::A, 2), unit(1, Team::B, 2)];
        both[0].pos = hex;
        both[1].pos = hex; // an enemy contests the hex
        let mut obj = Hold::new(hex, 1);
        obj.tick(&both, 5);
        assert_eq!(obj.status(&both, 5, false), ObjectiveStatus::Pending);
    }

    #[test]
    fn capture_hold_needs_cumulative_turns_of_control() {
        let hex = Hex::new(2, 0);
        let mut on = vec![unit(0, Team::A, 2), unit(1, Team::B, 5)];
        on[0].pos = hex;
        let mut obj = ObjectiveKind::CaptureHold(hex, 3).build();
        obj.tick(&on, 1);
        obj.tick(&on, 2);
        assert_eq!(obj.status(&on, 2, false), ObjectiveStatus::Pending); // 2 of 3 turns
        obj.tick(&on, 3);
        assert_eq!(obj.status(&on, 3, false), ObjectiveStatus::Achieved); // quota met, latched
    }

    #[test]
    fn the_flag_capture_is_sticky() {
        let hex = Hex::new(2, 0);
        let mut on = vec![unit(0, Team::A, 2), unit(1, Team::B, 5)];
        on[0].pos = hex;
        let mut obj = ObjectiveKind::Flag(hex, 2).build();
        obj.tick(&on, 1); // touched — captured for good
        // Driven off the flag: the grab still counts, and clearing the field seals it.
        let mut off = on.clone();
        off[0].pos = Hex::new(0, 0);
        off[1].character.alive = false; // enemy cleared
        obj.tick(&off, 2);
        assert_eq!(obj.status(&off, 2, false), ObjectiveStatus::Achieved);
    }

    #[test]
    fn search_sweeps_then_does_the_follow_up() {
        let spots = [Hex::new(2, 0), Hex::new(3, 0), Hex::new(4, 0)];
        // Correct is spot 1; finding it spins up a CaptureHold there.
        let mut obj = ObjectiveKind::search(&spots, 1, FoundAction::Capture(1)).build();
        assert_eq!(obj.focus(), Some(spots[0])); // sweep the first unsearched spot
        // A unit checks the wrong spot 0 → still searching, focus advances.
        let mut at0 = vec![unit(0, Team::A, 2), unit(1, Team::B, 7)];
        at0[0].pos = spots[0];
        obj.tick(&at0, 1);
        assert_eq!(obj.status(&at0, 1, false), ObjectiveStatus::Pending);
        assert_eq!(obj.focus(), Some(spots[1])); // wrong one searched → on to the next
        // Standing on the correct spot 1 reveals the prize; the follow-up now focuses there.
        let mut at1 = at0.clone();
        at1[0].pos = spots[1];
        obj.tick(&at1, 2);
        assert_eq!(obj.focus(), Some(spots[1]));
        // Hold it (enemy cleared while on it) → the follow-up latches Achieved.
        let mut held = at1.clone();
        held[1].character.alive = false;
        obj.tick(&held, 3);
        assert_eq!(obj.status(&held, 3, false), ObjectiveStatus::Achieved);
    }

    #[test]
    fn search_lists_every_unswept_spot_so_seekers_fan_out() {
        let spots = [Hex::new(2, 0), Hex::new(3, 0), Hex::new(4, 0)];
        let mut obj = ObjectiveKind::search(&spots, 2, FoundAction::Flag(2)).build();
        assert_eq!(obj.foci(), spots.to_vec()); // all three are targets at once
        // Sweep the wrong spot 0 → it drops out of the foci, the rest remain.
        let mut at0 = vec![unit(0, Team::A, 2), unit(1, Team::B, 7)];
        at0[0].pos = spots[0];
        obj.tick(&at0, 1);
        assert_eq!(obj.foci(), vec![spots[1], spots[2]]);
    }

    #[test]
    fn extract_is_a_two_phase_grab_and_run() {
        let item = Hex::new(6, 0);
        let exit = Hex::new(0, 0);
        let mut obj = ObjectiveKind::Extract { item, exit }.build();
        assert_eq!(obj.focus(), Some(item)); // phase 1: go to the item
        let mut at_item = vec![unit(0, Team::A, 6), unit(1, Team::B, 5)];
        at_item[0].pos = item;
        obj.tick(&at_item, 1);
        assert_eq!(obj.focus(), Some(exit)); // picked up → phase 2: run to the exit
        let mut at_exit = at_item.clone();
        at_exit[0].pos = exit;
        obj.tick(&at_exit, 2);
        assert_eq!(obj.status(&at_exit, 2, false), ObjectiveStatus::Achieved);
    }

    #[test]
    fn margin_loss_is_a_close_defeat() {
        let obj = MarginLoss { max_enemy_survivors: 2 };
        let both = vec![unit(0, Team::A, 0), unit(1, Team::B, 1)];
        assert_eq!(obj.status(&both, 5, false), ObjectiveStatus::Pending); // lose first
        let mut close = vec![unit(0, Team::A, 0), unit(1, Team::B, 1), unit(2, Team::B, 2)];
        close[0].character.alive = false; // player down, 2 enemies left ≤ 2
        assert_eq!(obj.status(&close, 9, true), ObjectiveStatus::Achieved);
        let mut blown =
            vec![unit(0, Team::A, 0), unit(1, Team::B, 1), unit(2, Team::B, 2), unit(3, Team::B, 3)];
        blown[0].character.alive = false; // 3 enemies left > 2 → lost too badly
        assert_eq!(obj.status(&blown, 9, true), ObjectiveStatus::Failed);
    }

    #[test]
    fn objectives_sum_winnings_and_surface_unachieved() {
        let mut us = vec![unit(0, Team::A, 0)]; // no enemy ⇒ fight won
        us[0].pos = Hex::new(5, 0); // standing on the target hex
        let objs = Objectives::new(vec![
            Goal::new(Box::new(WinFight), 10, 5),
            Goal::new(Box::new(Reach { hex: Hex::new(5, 0) }), 3, 0), // reached
            Goal::new(Box::new(Reach { hex: Hex::new(9, 9) }), 3, 0), // not reached
        ]);
        let b = Battle::with_rng(us, SplitMix64::new(1)).with_objectives(objs);
        assert_eq!(b.winnings(), 13); // WinFight 10 + reached 3
        assert_eq!(b.losses(), 0);
        assert_eq!(b.unachieved(), vec![2]); // far hex pending, not failed
    }

    #[test]
    fn losing_the_simple_objective_counts_as_a_loss() {
        let mut us = vec![unit(0, Team::A, 0), unit(1, Team::B, 1)];
        us[0].character.alive = false; // player wiped → WinFight Failed
        let objs = Objectives::new(vec![Goal::new(Box::new(WinFight), 10, 5)]);
        let b = Battle::with_rng(us, SplitMix64::new(1)).with_objectives(objs);
        assert_eq!(b.winnings(), 0);
        assert_eq!(b.losses(), 5);
        assert!(b.unachieved().is_empty());
    }

    #[test]
    fn satisfied_until_fail_condition() {
        // Pending and Achieved are "satisfied"; only Failed is not.
        assert!(ObjectiveStatus::Pending.is_satisfied());
        assert!(ObjectiveStatus::Achieved.is_satisfied());
        assert!(!ObjectiveStatus::Failed.is_satisfied());
    }

    #[test]
    fn withdraw_forfeits_but_saves_units() {
        // Both sides alive; withdrawing forfeits the fight yet preserves the roster.
        let mut b = Battle::with_rng(
            vec![unit(0, Team::A, 0), unit(1, Team::B, 3)],
            SplitMix64::new(1),
        )
        .with_objectives(Objectives::new(vec![Goal::new(Box::new(WinFight), 10, 5)]));
        assert_eq!(b.objectives_report(), vec![ObjectiveStatus::Pending]); // ongoing
        b.withdraw();
        assert!(b.is_withdrawn());
        assert_eq!(b.objectives_report(), vec![ObjectiveStatus::Failed]); // forfeited
        assert_eq!(b.losses(), 5);
        assert!(b.units[0].is_alive() && b.units[1].is_alive()); // everyone saved
    }

    #[test]
    fn zero_link_target_is_immune_to_hacks() {
        let atk = runner(0, Team::A, 0, 1);
        let mut tgt = networked(1, Team::B, 0, 8);
        tgt.character.base_mut().link = 0.0; // air-gapped — no surface to reach
                        // Empty RNG: a roll here would panic, proving the gate short-circuits.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        assert_eq!(b.resolve_hack(0, 1), HackResult::NoSurface);
        assert!(b.units[1].statuses().is_empty());
    }

    #[test]
    fn offline_attacker_cannot_hack() {
        let mut atk = runner(0, Team::A, 0, 1);
        atk.character.base_mut().link = 0.0; // dark — no presence to reach with
        let tgt = networked(1, Team::B, 0, 8);
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        assert_eq!(b.resolve_hack(0, 1), HackResult::Offline);
        assert!(b.units[1].statuses().is_empty());
    }

    #[test]
    fn the_channel_is_the_weaker_endpoints_link() {
        // Attacker Link 6, target Link 2 → channel 2 (the target bottlenecks it);
        // rating avg(Hacking 4, 2) = 3.
        let mut atk = runner(0, Team::A, 0, 1);
        atk.character.base_mut().link = 6.0;
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = networked(1, Team::B, 0, 0);
        tgt.character.base_mut().link = 2.0;
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([2, 3])); // 5
        let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
        // channel min(6,2)=2 → rating avg(eff Hacking 14, 2)=8; target 8 − fw 0, dice 5 ⇒ margin 3.
        assert_eq!(outcome.margin, 3);
    }

    #[test]
    fn a_thin_runner_link_bottlenecks_the_channel() {
        // Mirror: attacker Link 2, target Link 6 → channel 2; same rating 3.
        let mut atk = runner(0, Team::A, 0, 1);
        atk.character.base_mut().link = 2.0;
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = networked(1, Team::B, 0, 0);
        tgt.character.base_mut().link = 6.0;
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([2, 3])); // 5
        let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
        assert_eq!(outcome.margin, 3); // channel min(2,6)=2 → rating avg(14,2)=8, same target 8
    }

    #[test]
    fn a_darker_target_is_harder_to_hack() {
        // Same runner; only the target's Link (the channel) changes. Higher rating lifts the
        // roll-under target, so the same dice clear it by a wider margin.
        let margin_vs = |target_link: i32| {
            let mut atk = runner(0, Team::A, 0, 1);
            atk.character.base_mut().link = 6.0;
            atk.skills.set(Skill::Hacking, 6);
            let mut tgt = networked(1, Team::B, 0, 0);
            tgt.character.base_mut().link = target_link as f32;
            let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([2, 3])); // 5
            let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
            outcome.margin
        };
        // eff Hacking 16. Fat channel (Link 6): avg(16,6)=11 → target 11, margin 6. Dark (Link 1): avg(16,1)=8.
        assert!(margin_vs(6) > margin_vs(1));
        assert_eq!(margin_vs(6), 6); // target 11 − dice 5
        assert_eq!(margin_vs(1), 3); // target 8 − dice 5
    }

    #[test]
    fn firewall_rolls_an_active_defense() {
        // The hack is **opposed** (stats.md §4): the runner rolls to crack, the Firewall rolls
        // back, and the breach lands only if the runner connects *and* the Firewall fails.
        let setup = || {
            let mut atk = runner(0, Team::A, 0, 1);
            atk.character.base_mut().link = 4.0;
            atk.skills.set(Skill::Hacking, 6); // eff Hacking 16; channel min(4,4)=4 ⇒ rating 10
            let mut tgt = networked(1, Team::B, 0, 4); // Firewall 4
            tgt.character.base_mut().link = 4.0;
            (atk, tgt)
        };
        // Runner rolls 5 ≤ rating 10 (margin 5); Firewall 4 rolls 18 > 4 (fails) ⇒ breach lands.
        let (atk, tgt) = setup();
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([2, 3, 9, 9]));
        let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
        assert!(outcome.success && outcome.margin == 5);
        // Same runner roll, but the Firewall makes its defense (rolls 3 ≤ 4) ⇒ repelled.
        let (atk, tgt) = setup();
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([2, 3, 1, 2]));
        assert!(!b.resolve_hack(0, 1).landed());
    }

    #[test]
    fn margin_scales_the_landed_stacks() {
        let mut atk = runner(0, Team::A, 0, 1);
        atk.character.base_mut().link = 8.0;
        atk.skills.set(Skill::Hacking, 6); // eff Hacking 16; channel min(8,8)=8 → rating avg(16,8)=12
        let mut tgt = networked(1, Team::B, 0, 0);
        tgt.character.base_mut().link = 8.0;
        // target = rating 12 − Firewall 0 = 12; dice 5 ⇒ margin 7 → 1 + 7/3 = 3 stacks.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([2, 3]));
        assert!(b.resolve_hack(0, 1).landed());
        assert_eq!(b.units[1].statuses()[0].0, "Lockware");
        assert_eq!(b.units[1].statuses()[0].1, 3);
    }

    #[test]
    fn digital_phase_hacks_the_nearest_reachable_enemy() {
        let mut atk = runner(0, Team::A, 0, 4); // antenna range 4
        atk.character.base_mut().link = 4.0;
        atk.skills.set(Skill::Hacking, 4);
        let mut near = networked(1, Team::B, 2, 0); // distance 2 ≤ range 4
        near.character.base_mut().link = 4.0;
        let mut far = networked(2, Team::B, 9, 0); // out of range
        far.character.base_mut().link = 4.0;
        let mut b = Battle::with_rng(vec![atk, near, far], ScriptedRng::from_d10([3, 3]));
        b.digital_phase();
        assert!(!b.units[1].statuses().is_empty()); // near got hacked
        assert!(b.units[2].statuses().is_empty()); // far one untouched (out of range)
    }

    #[test]
    fn a_stunned_runner_skips_its_hack() {
        let mut atk = runner(0, Team::A, 0, 4);
        atk.add_status(StatusSpec::crash(), 1, 1); // Seizure freezes the net action too
        let tgt = networked(1, Team::B, 1, 2);
        // Empty RNG: a stunned runner must not roll.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        b.digital_phase();
        assert!(b.units[1].statuses().is_empty());
    }

    #[test]
    fn hacking_resolution_is_deterministic() {
        let setup = || {
            let mut atk = runner(0, Team::A, 0, 4);
            atk.character.base_mut().link = 4.0;
            atk.skills.set(Skill::Hacking, 4);
            let mut tgt = networked(1, Team::B, 2, 4);
            tgt.character.base_mut().link = 4.0;
            Battle::new(vec![atk, tgt], 99)
        };
        let mut x = setup();
        let mut y = setup();
        for _ in 0..10 {
            x.step();
            y.step();
        }
        for (a, b) in x.units.iter().zip(&y.units) {
            assert_eq!(a.character.integrity, b.character.integrity);
            assert_eq!(a.statuses().len(), b.statuses().len());
        }
    }

    #[test]
    fn installing_a_cyberdeck_grants_the_hack_and_folds_the_surface() {
        let mut u = unit(0, Team::A, 0); // base: no deck, Link 0, Firewall 0
        assert!(u.hack().is_none());
        u.install(Implant::cyberdeck());
        assert!(u.hack().is_some()); // benefit: the unit can now hack
        assert_eq!(u.link(), 5); // folded surface
        assert_eq!(u.firewall(), 2); // folded wall
    }

    #[test]
    fn a_crit_cascades_across_a_meshed_pan() {
        // A decisive hack on a meshed PAN rides to every implant, not just one.
        let mut atk = runner(0, Team::A, 0, 4);
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = unit(1, Team::B, 1);
        tgt.character.base_mut().link = 3.0;
        tgt.character.base_mut().firewall = 4.0;
        tgt.install(Implant::combat_stim());
        tgt.install(Implant::reflex_booster());
        assert_eq!(tgt.pan, Pan::Meshed); // the default
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([1, 2, 9, 9])); // crit (3); Firewall 4 fails its defense (18)
        b.resolve_hack(0, 1);
        assert!((0..b.units[1].implants.len()).all(|i| b.units[1].implant_condition(i) == Condition::Offline));
    }

    #[test]
    fn a_segmented_pan_contains_the_crit() {
        // Same decisive hack, but segmentation isolates the breach to one slot.
        let mut atk = runner(0, Team::A, 0, 4);
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = unit(1, Team::B, 1);
        tgt.character.base_mut().link = 3.0;
        tgt.character.base_mut().firewall = 4.0;
        tgt.pan = Pan::Segmented;
        tgt.install(Implant::combat_stim()); // idx 0 — the targeted (digital) slot
        tgt.install(Implant::reflex_booster()); // idx 1 — contained
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([1, 2, 9, 9])); // crit (3); Firewall 4 fails its defense (18)
        b.resolve_hack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline);
        assert_eq!(b.units[1].implant_condition(1), Condition::Online); // contained
    }

    #[test]
    fn a_worm_logic_bomb_trips_an_implant_without_a_roll() {
        // The Worm breach vector (§6): no hack roll — it disables the slot and fires
        // its degrade liability outright. On a segmented PAN it hits just the one.
        let mut tgt = unit(1, Team::B, 1);
        tgt.pan = Pan::Segmented;
        tgt.install(Implant::combat_stim()); // idx 0 — digital, Bleed (degrade) liability
        tgt.install(Implant::reflex_booster()); // idx 1 — contained
        let mut b = Battle::new(vec![unit(0, Team::A, 0), tgt], 1);
        assert_eq!(b.worm_breach(1), 1); // one slot tripped
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline); // disabled, no roll
        assert_eq!(b.units[1].implant_condition(1), Condition::Online); // segmented: contained
        assert_eq!(b.units[1].statuses()[0].0, "Bleed"); // degrade liability fired
    }

    #[test]
    fn a_worm_cascades_across_a_meshed_pan() {
        // On a meshed PAN the worm rides the net to every implant — the finisher.
        let mut tgt = unit(1, Team::B, 1);
        assert_eq!(tgt.pan, Pan::Meshed); // default
        tgt.install(Implant::combat_stim());
        tgt.install(Implant::reflex_booster());
        let mut b = Battle::new(vec![unit(0, Team::A, 0), tgt], 1);
        assert_eq!(b.worm_breach(1), 2); // both slots
        assert!((0..b.units[1].implants.len())
            .all(|i| b.units[1].implant_condition(i) == Condition::Offline));
        // A worm doesn't crit — the reflex booster's Seizure (stun) stays gated.
        assert!(!b.units[1].is_stunned());
    }

    #[test]
    fn a_plague_jumps_to_an_adjacent_victim() {
        // Contagion phase: a virulent plague on a low-Immunity neighbour wins the jump
        // and copies itself over — the victim is now both sick *and* contagious.
        let mut carrier = unit(0, Team::B, 0);
        carrier.apply_modifier(Corruption::plague(4.0, 3.0, 18, 5)); // virulent (18) vs Immunity 0
        let victim = unit(1, Team::B, 1); // adjacent, default Immunity 0
        let bystander = unit(2, Team::B, 5); // far away — out of proximity
        let mut b = Battle::new(vec![carrier, victim, bystander], 7);
        b.contagion_phase();
        assert!(b.units[1].character.carries("Virus")); // caught it
        assert_eq!(b.units[1].character.active_contagions().len(), 1); // now spreads too
        assert!(!b.units[2].character.carries("Virus")); // too far to reach
    }

    #[test]
    fn a_bio_plague_only_takes_those_who_can_host_it() {
        // "Target those who can spread it": a Virus needs a biological body. An adjacent
        // augmented neighbour catches it; an adjacent Machine (no body to carry / re-spread
        // it) is a dead end and never infected — even at Immunity 0.
        let mut carrier = unit(0, Team::B, 0);
        carrier.apply_modifier(Corruption::plague(4.0, 3.0, 18, 5)); // virulent (18)
        let bio = unit(1, Team::B, 1); // augmented — a valid host
        let drone = Unit::new(2, "Drone", Team::B, Chassis::Machine).at(Hex::new(0, 1)); // adjacent
        let mut b = Battle::new(vec![carrier, bio, drone], 7);
        b.contagion_phase();
        assert!(b.units[1].character.carries("Virus")); // bio host: infected
        assert!(!b.units[2].character.carries("Virus")); // machine: can't host a bio plague
    }

    #[test]
    fn a_virus_fever_chips_the_infected_each_tick() {
        // The plague's combat bite: a fever DoT (Internal — bypasses armor) damages the
        // host on the status tick, independent of any attacks.
        let mut tgt = unit(1, Team::B, 6); // far from the attacker — only the fever can hurt it
        tgt.apply_modifier(Corruption::virus(0.0, 5.0, 5)); // 5/tick, no Immunity rot
        let before = tgt.integrity();
        let mut b = Battle::new(vec![unit(0, Team::A, 0), tgt], 1);
        b.status_phase(); // the tick that fires DoTs
        assert_eq!(b.units[1].integrity(), before - 5.0);
    }

    #[test]
    fn a_dot_tick_logs_a_structured_damaged_event() {
        // DoT damage is now attributable in the trace: the fever logs a Damaged event
        // naming its cause ("Virus"), so plague kills aren't anonymous any more.
        let mut tgt = unit(1, Team::B, 6).with_integrity(4.0); // fragile — the fever finishes it
        tgt.apply_modifier(Corruption::virus(0.0, 5.0, 5));
        let mut b = Battle::new(vec![unit(0, Team::A, 0), tgt], 1).with_log();
        b.status_phase();
        let ev = b.events().iter().find(|r| r.event.kind() == "damaged").expect("a damaged event");
        assert!(matches!(ev.event, CombatEvent::Damaged { cause: "Virus", killed: true, .. }));
    }

    #[test]
    fn deploy_keeps_a_plague_but_clears_combat_statuses() {
        // A carrier's plague is authored loadout — it survives the between-combats cleanse
        // (`clear_statuses`); an acquired combat status (Burn) does not.
        let mut u = unit(0, Team::B, 0);
        u.apply_modifier(Corruption::plague(4.0, 3.0, 10, 5)); // loadout corruption
        u.add_status(StatusSpec::burn(), 4, 2); // acquired in combat
        u.character.clear_statuses();
        assert_eq!(u.character.active_contagions().len(), 1); // plague kept
        assert!(u.statuses().iter().all(|(n, _)| *n != "Burn")); // burn cleared
    }

    #[test]
    fn immunity_resists_the_jump() {
        // Same proximity, but a hardened immune system (high Immunity TN) beats the
        // contest — a weak plague can't take hold.
        let mut carrier = unit(0, Team::B, 0);
        carrier.apply_modifier(Corruption::plague(4.0, 3.0, 2, 5)); // virulence 2 ⇒ target 2, barely ever spreads
        let mut victim = unit(1, Team::B, 1);
        victim.character.base_mut().immunity = 30.0; // TN 30 — unbeatable here
        let mut b = Battle::new(vec![carrier, victim], 7);
        b.contagion_phase();
        assert!(!b.units[1].character.carries("Virus")); // resisted
    }

    #[test]
    fn a_worm_swarm_rides_the_net_not_the_flesh() {
        // The digital vector ignores distance but needs a surface: a wired unit catches
        // it across the map; a fleshy one (Link 0) is untouchable however close.
        let mut carrier = unit(0, Team::A, 0);
        carrier.apply_modifier(Corruption::worm_swarm(3.0, 18, 5)); // virulent (18) vs Firewall 0
        let wired = runner(1, Team::B, 6, 4); // Link 3, far away
        let fleshy = unit(2, Team::B, 1); // adjacent but Link 0
        let mut b = Battle::new(vec![carrier, wired, fleshy], 7);
        b.contagion_phase();
        assert!(b.units[1].character.carries("Worm")); // rode the net across the board
        assert!(!b.units[2].character.carries("Worm")); // no surface — immune to the worm
    }

    #[test]
    fn the_event_log_records_a_structured_trace() {
        // Opt-in capture: a lethal hit produces a structured Attacked{killed} + a Died,
        // and the kind()/Display surfaces are stable for a logger to key on.
        let atk = unit(0, Team::A, 0);
        let tgt = unit(1, Team::B, 1).with_integrity(1.0); // fragile — the hit kills
        let mut b = Battle::new(vec![atk, tgt], 1).with_log();
        b.resolve_attack(0, 1);
        b.reap();
        let kinds: Vec<&str> = b.events().iter().map(|r| r.event.kind()).collect();
        assert!(kinds.contains(&"attacked"));
        assert!(kinds.contains(&"died"));
        let hit = b.events().iter().find(|r| r.event.kind() == "attacked").unwrap();
        assert!(matches!(hit.event, CombatEvent::Attacked { killed: true, .. }));
        assert!(!hit.to_string().is_empty()); // renders a human line
    }

    #[test]
    fn the_log_is_off_by_default() {
        let mut b = Battle::new(vec![unit(0, Team::A, 0), unit(1, Team::B, 1)], 1);
        b.resolve_attack(0, 1);
        assert!(b.events().is_empty()); // no capture unless with_log()
    }

    #[test]
    fn a_meshed_pan_boosts_hacking_throughput() {
        // Same roll: a meshed netrunner with extra chrome out-hacks a segmented one.
        let total_for = |pan: Pan| {
            let mut atk = unit(0, Team::A, 0);
            atk.skills.set(Skill::Hacking, 4);
            atk.pan = pan;
            atk.install(Implant::cyberdeck()); // grants the hack + Link 5
            atk.install(Implant::reflex_booster()); // 2 active → meshed synergy +1
            let mut tgt = networked(1, Team::B, 0, 4);
            tgt.character.base_mut().link = 5.0;
            let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([6, 6, 1, 1]));
            let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
            outcome.margin // mesh synergy lifts the rating → the roll-under target → the margin
        };
        assert!(total_for(Pan::Meshed) > total_for(Pan::Segmented));
    }

    #[test]
    fn emp_fries_digital_chrome_but_not_inert_plating() {
        // EMP bypasses Firewall to brick **digital** chrome — but inert physical armor
        // (subdermal plating, no circuitry) is EMP-proof and keeps its benefit.
        let mut tgt = unit(1, Team::B, 0);
        tgt.character.base_mut().firewall = 99.0; // EMP ignores the wall entirely
        tgt.install(Implant::subdermal_plating()); // idx 0 — inert physical: EMP-proof
        tgt.install(Implant::cyberdeck()); // idx 1 — digital: fried
        assert!(tgt.hack().is_some());
        let mut atk = unit(0, Team::A, 0);
        atk.rearm(|w| w.emp = true);
        // The EMP itself rolls nothing; the kinetic hit's hit-location draw of 0 lands on
        // flesh (no chrome chipped), so the conditions below are purely the EMP's doing.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::new([0]));
        b.resolve_attack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Online); // plating survives
        assert_eq!(b.units[1].implant_condition(1), Condition::Offline); // deck bricked
        assert!(b.units[1].hack().is_none()); // deck gone
        assert_eq!(b.units[1].character.plating, 6.0); // plating benefit intact
    }

    #[test]
    fn physical_cyberware_is_immune_to_breach() {
        // Subdermal plating presents no digital surface — neither a hack nor a worm can
        // trip it (only physical wear degrades it). EMP-immunity is covered above.
        assert!(!Implant::subdermal_plating().is_digital());
        assert!(Implant::cyberdeck().is_digital());
        assert!(Implant::reflex_booster().is_digital()); // smartware: networked, hackable

        let mut tgt = unit(1, Team::B, 1);
        tgt.character.base_mut().link = 4.0; // the unit *has* a surface to hack at...
        tgt.install(Implant::subdermal_plating()); // ...but the plating itself is inert
        let mut b = Battle::new(vec![unit(0, Team::A, 0), tgt], 1);
        assert_eq!(b.worm_breach(1), 0); // no digital chrome to trip
        assert_eq!(b.units[1].implant_condition(0), Condition::Online); // untouched
    }

    #[test]
    fn emp_disables_but_does_not_knock_out() {
        // EMP is blunt — it fires degrade-class liabilities but not the stun.
        let mut tgt = unit(1, Team::B, 0);
        tgt.install(Implant::reflex_booster()); // Seizure (stun) liability
        let mut atk = unit(0, Team::A, 0);
        atk.rearm(|w| w.emp = true);
        // Hit-location draw of 0 lands on flesh; the Offline below is the EMP, not the hit.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::new([0]));
        b.resolve_attack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline); // disabled
        assert!(!b.units[1].is_stunned());
    }

    #[test]
    fn emp_is_harmless_to_unchromed_targets() {
        let tgt = unit(1, Team::B, 0); // flesh — no implants
        let mut atk = unit(0, Team::A, 0);
        atk.rearm(|w| w.emp = true);
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        b.resolve_attack(0, 1);
        assert!(b.units[1].implants.is_empty());
        // no chrome to fry → EMP adds no statuses (only the kinetic hit landed)
        assert!(b.units[1].statuses().is_empty());
    }

    #[test]
    fn a_hit_on_chrome_is_taken_by_its_hp_not_the_flesh() {
        // Hit location: chrome extends the roll table (body 90 + deck 10 = 100); a roll in
        // the deck's band [90,100) is **taken by the deck's HP**, sparing the flesh — until
        // it's worn to Destroyed, after which the overflow reaches Integrity.
        let mut tgt = unit(1, Team::B, 0); // Augmented chassis, body coverage 90; Integrity 30
        tgt.install(Implant::cyberdeck()); // coverage 10, HP 18 → domain 1..100
        assert!(tgt.hack().is_some());
        let atk = unit(0, Team::A, 0); // default melee 10, Internal → ~10 a hit
        // Evasion 0 auto-hits (no to-hit draw); each attack spends location rolls. A roll of
        // 90 lands in the deck's slice [90,100).
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::new([90, 90]));
        b.resolve_attack(0, 1);
        assert_eq!(b.units[1].integrity(), 30.0); // the deck's HP took all 10 — flesh untouched
        assert_eq!(b.units[1].implant_condition(0), Condition::Degraded); // deck 18 → 8 HP (<50%)
        b.resolve_attack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Destroyed); // deck 8 → 0 HP
        assert!(b.units[1].hack().is_none()); // the deck is wrecked
        assert_eq!(b.units[1].integrity(), 28.0); // 8 absorbed, 2 overflowed to the flesh
    }

    #[test]
    fn overflow_from_a_wrecked_implant_rerolls_location() {
        // A big blow destroys the small deck and the **overflow rerolls** over the smaller
        // table — landing on the reflex booster, not straight onto the flesh.
        let mut tgt = unit(1, Team::B, 0); // body 90
        tgt.install(Implant::cyberdeck()); // idx 0: coverage 10, HP 18 → band [90,100)
        tgt.install(Implant::reflex_booster()); // idx 1: coverage 25, HP 24 → band [100,125)
        let mut atk = unit(0, Team::A, 0);
        atk.rearm(|w| w.damage = 35.0); // Internal → 35 to the location table
        // Roll 90 → deck (absorbs 18, destroyed, 17 overflow); reroll 90 → reflex (table now
        // 90+25=115, so [90,115)) absorbs 17 → 7 HP left.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::new([90, 90]));
        b.resolve_attack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Destroyed); // deck wrecked
        assert_eq!(b.units[1].implant_condition(1), Condition::Degraded); // reflex caught the overflow
        assert_eq!(b.units[1].integrity(), 30.0); // chrome ate all 35 — the flesh is spared
    }

    #[test]
    fn a_flesh_target_draws_no_location_roll() {
        // A unit with no chrome never rolls location — so a scripted RNG isn't even touched
        // (the kinetic hit just lands), keeping flesh-and-bone fights bit-for-bit as before.
        let atk = unit(0, Team::A, 0);
        let tgt = unit(1, Team::B, 0); // no implants
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default()); // empty: must not draw
        b.resolve_attack(0, 1);
        assert!(b.units[1].integrity() < 30.0); // it was hit...
        assert!(b.units[1].implants.is_empty()); // ...with nothing to mangle
    }

    #[test]
    fn a_degraded_implant_delivers_half_its_benefit() {
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::subdermal_plating()); // +6 Plating, Online
        assert_eq!(u.character.plating, 6.0);
        u.degrade_implant(0); // Online → Degraded
        assert_eq!(u.implant_condition(0), Condition::Degraded);
        assert_eq!(u.character.plating, 3.0); // half benefit
        u.degrade_implant(0); // Degraded → Offline
        assert_eq!(u.implant_condition(0), Condition::Offline);
        assert_eq!(u.character.plating, 0.0); // none
    }

    #[test]
    fn the_ripperdoc_repairs_a_degraded_implant_to_online() {
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::subdermal_plating());
        u.degrade_implant(0); // Degraded, plating 3
        u.repair_implant(0);
        assert_eq!(u.implant_condition(0), Condition::Online);
        assert_eq!(u.character.plating, 6.0); // restored full
    }

    #[test]
    fn wear_destroys_and_destroyed_is_terminal() {
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::subdermal_plating());
        u.degrade_implant(0); // Degraded
        u.degrade_implant(0); // Offline
        u.degrade_implant(0); // Destroyed
        assert_eq!(u.implant_condition(0), Condition::Destroyed);
        u.repair_implant(0); // terminal — a no-op
        assert_eq!(u.implant_condition(0), Condition::Destroyed);
        assert_eq!(u.character.plating, 0.0); // stays gone
    }

    #[test]
    fn a_degraded_deck_still_hacks_on_a_thinner_surface() {
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::cyberdeck()); // Link 5, grants hack
        assert_eq!(u.link(), 5);
        u.degrade_implant(0); // Degraded — still active
        assert!(u.hack().is_some()); // a degraded deck still hacks
        assert_eq!(u.link(), 3); // round(0.5 × 5) = 3 — thinner surface
        u.degrade_implant(0); // Offline
        assert!(u.hack().is_none()); // now bricked
        assert_eq!(u.link(), 0); // and exact — no rounding drift
    }

    #[test]
    fn firewall_suite_folds_the_wall() {
        let mut u = unit(0, Team::A, 0); // base Firewall 0
        u.install(Implant::firewall_suite());
        assert_eq!(u.firewall(), 4);
        assert_eq!(u.link(), 1); // a little surface comes with it
    }

    #[test]
    fn combat_stim_folds_damage_and_lifts_integrity_with_the_pump() {
        let mut u = unit(0, Team::A, 0);
        let base_hp = u.max_integrity();
        u.install(Implant::combat_stim());
        assert_eq!(u.damage_bonus(), 4.0); // +4 composed damage bonus (weapon base unchanged)
        assert_eq!(u.weapon_at(1).unwrap().damage, 10.0);
        u.install(Implant::metabolic_pump());
        assert_eq!(u.max_integrity(), base_hp + 8.0);
        assert_eq!(u.character.integrity, base_hp + 8.0); // gained the HP too
    }

    #[test]
    fn breaching_combat_stim_fires_the_dot_on_a_solid_hit_not_the_stun() {
        // Overdose is multi-effect: the self-DoT is degrade (margin), the Crash is
        // crit-gated — so a solid, non-crit hit lands only the DoT.
        let mut atk = runner(0, Team::A, 0, 4);
        atk.character.base_mut().link = 8.0;
        atk.skills.set(Skill::Hacking, 6); // eff Hacking 16; channel 8 → rating 12
        let mut tgt = unit(1, Team::B, 1);
        tgt.character.base_mut().link = 8.0;
        tgt.character.base_mut().firewall = 0.0; // target 12; dice 9 ⇒ margin 3, no crit
        tgt.install(Implant::combat_stim());
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([4, 5]));
        b.resolve_hack(0, 1);
        let has_dot = b.units[1].statuses().iter().any(|(n, _)| *n == "Bleed");
        let has_stun = b.units[1].is_stunned();
        assert!(has_dot && !has_stun);
    }

    #[test]
    fn salvage_yields_every_implant_but_the_destroyed_ones() {
        // §6: a corpse's chrome is loot — except Destroyed gear, which is terminal.
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::cyberdeck()); // idx 0 — stays Online
        u.install(Implant::subdermal_plating()); // idx 1 — we'll wreck it
        // Wear idx 1 all the way down to Destroyed (Online → Degraded → Offline → Destroyed).
        for _ in 0..3 {
            u.degrade_implant(1);
        }
        assert_eq!(u.implant_condition(1), Condition::Destroyed);
        let salvage = u.salvage();
        assert_eq!(salvage.len(), 1); // only the deck survives as loot
        assert_eq!(salvage[0].name, Implant::cyberdeck().name);
    }

    #[test]
    fn subdermal_plating_folds_into_defense() {
        let mut u = unit(0, Team::A, 0);
        let base = u.character.plating;
        u.install(Implant::subdermal_plating());
        assert_eq!(u.character.plating, base + 6.0);
    }

    #[test]
    fn an_offline_implant_contributes_nothing() {
        let mut u = unit(0, Team::A, 0);
        let mut deck = Implant::cyberdeck();
        deck.condition = Condition::Offline; // installed dead
        u.install(deck);
        assert!(u.hack().is_none());
        assert_eq!(u.link(), 0);
    }

    #[test]
    fn disabling_drops_the_benefit_then_repair_restores_it() {
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::cyberdeck());
        let effects = u.disable_implant(0); // the §6 disable floor (a breach)
        assert!(u.hack().is_none()); // bricked — lost the deck
        assert_eq!(u.link(), 0); // surface folded back out
        assert!(!effects.is_empty()); // liabilities returned for the ladder (later phase)
        assert_eq!(u.implant_condition(0), Condition::Offline);
        u.repair_implant(0); // Ripperdoc
        assert!(u.hack().is_some());
        assert_eq!(u.link(), 5);
        assert_eq!(u.implant_condition(0), Condition::Online);
    }

    #[test]
    fn a_marginal_hack_just_disables_the_implant() {
        // Margin 0 success → the §6 floor: a pure disable. No heat (it needs a solid margin) and
        // no implant liability (Seizure needs margin / a crit) — nothing fires but the disable.
        let mut atk = runner(0, Team::A, 0, 4);
        atk.character.base_mut().link = 4.0;
        atk.skills.set(Skill::Hacking, 4); // eff Hacking 14; channel 4 → rating 9
        let mut tgt = unit(1, Team::B, 1);
        tgt.character.base_mut().link = 4.0;
        tgt.install(Implant::reflex_booster()); // Seizure liability (stun)
        tgt.character.base_mut().firewall = 0.0; // target = rating 9; dice 9 ⇒ margin 0
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([4, 5]));
        assert!(b.resolve_hack(0, 1).landed());
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline); // disabled
        assert!(b.units[1].statuses().is_empty()); // nothing fired (margin 0: no heat, no liability)
    }

    #[test]
    fn overheat_is_a_program_that_cooks_only_volatile_chrome() {
        // Heat is an *equipped* deck program, gated on the target's **VOLATILE** chrome — not
        // innate to hacking. A solid breach lands Overheat only when both hold.
        let heat_landed = |program: bool, volatile: bool| {
            let mut atk = unit(0, Team::A, 0);
            atk.character.base_mut().link = 8.0;
            atk.character.base_mut().intellect = 10.0;
            atk.skills.set(Skill::Hacking, 6); // eff Hacking 16; channel 8 ⇒ rating 12
            let mut hack = Hack::new(6, StatusSpec::lockware(), 1, 6);
            if program {
                hack = hack.with_overheat();
            }
            atk.grant_hack(hack);
            let mut tgt = unit(1, Team::B, 1);
            tgt.character.base_mut().link = 8.0;
            if volatile {
                tgt.install(Implant::combat_stim()); // runs hot (VOLATILE); adds no Firewall
            }
            tgt.character.base_mut().firewall = 0.0; // undefended ⇒ a clean attacker-only roll
            // rating 12, dice 4 ⇒ margin 8 (a solid breach, not a crit).
            let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([1, 3]));
            b.resolve_hack(0, 1);
            b.units[1].statuses().iter().any(|(n, _)| *n == "Overheat")
        };
        assert!(heat_landed(true, true)); // program + volatile chrome ⇒ it cooks
        assert!(!heat_landed(false, true)); // no program ⇒ no heat, even on volatile chrome
        assert!(!heat_landed(true, false)); // program but cool chrome ⇒ nothing to cook
    }

    #[test]
    fn a_solid_hack_disables_and_fires_the_degrade_liability() {
        // Strong margin → disable + the degrade-class hack-effect, margin-scaled.
        let mut atk = runner(0, Team::A, 0, 4);
        atk.character.base_mut().link = 8.0;
        atk.skills.set(Skill::Hacking, 6); // eff Hacking 16; channel 8 → rating 12
        let mut tgt = unit(1, Team::B, 1);
        tgt.character.base_mut().link = 8.0;
        tgt.install(Implant::combat_stim()); // digital; Bleed (degrade, non-stun) fires
        tgt.character.base_mut().firewall = 0.0; // target = rating 12; dice 6 → margin 6
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([3, 3]));
        b.resolve_hack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline);
        assert_eq!(b.units[1].statuses()[0].0, "Bleed"); // degrade liability fired
        assert_eq!(b.units[1].statuses()[0].1, 2); // margin 6 / 3
    }

    #[test]
    fn a_crit_delivers_the_knockout_stun() {
        // The stun class is crit-gated — only a decisive hack lands it.
        let mut atk = runner(0, Team::A, 0, 4);
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = unit(1, Team::B, 1);
        tgt.character.base_mut().link = 3.0;
        tgt.character.base_mut().firewall = 12.0;
        tgt.install(Implant::reflex_booster()); // Seizure (stun)
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([1, 2, 9, 9])); // crit (3); Firewall 12 fails its defense (18)
        b.resolve_hack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline);
        assert!(b.units[1].is_stunned());
    }

    #[test]
    fn the_knockout_gate_is_strict_by_default_but_widenable() {
        // §6 knob: a crit always clears the gate; a big margin alone does not (strict
        // default), but lowering the tier widens it to a "decisive margin".
        let crit = RollOutcome { dice: 18, total: 30, margin: 5, success: true, crit: true, fumble: false };
        let solid = RollOutcome { dice: 12, total: 24, margin: 12, success: true, crit: false, fumble: false };
        assert!(knockout_gated(&crit, KNOCKOUT_MARGIN)); // a crit knocks out regardless
        assert!(!knockout_gated(&solid, KNOCKOUT_MARGIN)); // strict: margin alone doesn't
        assert!(knockout_gated(&solid, 10)); // widened tier: a decisive margin does
    }

    #[test]
    fn breaching_a_deck_silences_the_targets_own_hacking() {
        // A netrunner whose deck is breached can't hack back (benefit unfolds).
        let mut atk = runner(0, Team::A, 0, 4);
        atk.skills.set(Skill::Hacking, 6);
        let mut tgt = unit(1, Team::B, 1);
        tgt.skills.set(Skill::Hacking, 4);
        tgt.install(Implant::cyberdeck()); // grants the hack + Link 5
        assert!(tgt.hack().is_some());
        // Opposed: runner rolls 6 ≤ rating 9 (margin 3); the deck's own Firewall 2 fails (18) ⇒ lands.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([3, 3, 9, 9]));
        b.resolve_hack(0, 1);
        assert!(b.units[1].hack().is_none()); // deck bricked → no hacking back
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline);
    }

    // -- stat read-through: modifiers compose into the effective line --------

    #[test]
    fn a_modifier_composes_into_the_effective_stat() {
        let mut u = unit(0, Team::A, 0);
        u.character.base_mut().firewall = 9.0;
        assert_eq!(u.firewall(), 9); // base
        u.apply_modifier(Decorator::gear(Tag::Gear, vec![Factor::add(Stat::Firewall, 4.0)]));
        assert_eq!(u.firewall(), 13); // base + flat add
        // an Increased factor scales the *base* — proof the base is inside the fold.
        u.apply_modifier(Decorator::gear(Tag::Buff, vec![Factor::increased(Stat::Firewall, 0.5)]));
        assert_eq!(u.firewall(), 20); // round((9 + 4) × 1.5) = round(19.5)
    }

    #[test]
    fn a_firewall_debuff_makes_a_hack_land_in_the_loop() {
        let mut atk = runner(0, Team::A, 0, 4);
        atk.character.base_mut().link = 4.0;
        atk.skills.set(Skill::Hacking, 4); // rating avg(4, 4) = 4
        let mut tgt = networked(1, Team::B, 1, 6); // base Firewall 6 (a −6 penalty)
        tgt.character.base_mut().link = 4.0;
        // Debuff Firewall by 4 → effective 2; the loop reads firewall() through the gen.
        tgt.apply_modifier(Decorator::gear(Tag::Debuff, vec![Factor::add(Stat::Firewall, -4.0)]));
        // Opposed: runner rolls 5 ≤ rating 9; the Firewall rolls 4 — vs base 6 that defends (4 ≤ 6),
        // but debuffed to 2 it fails (4 > 2), so the debuff is exactly what lets the breach land.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([2, 3, 2, 2]));
        assert!(b.resolve_hack(0, 1).landed());
    }

    #[test]
    fn an_installed_deck_lets_a_unit_hack_in_the_digital_phase() {
        let mut atk = unit(0, Team::A, 0);
        atk.character.base_mut().intellect = 10.0; // GURPS Intellect ⇒ eff Hacking 14
        atk.skills.set(Skill::Hacking, 4);
        atk.install(Implant::cyberdeck()); // grants hack + Link 5
        let tgt = networked(1, Team::B, 1, 0); // soft target, in deck range 6
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d10([3, 3]));
        b.digital_phase();
        assert!(!b.units[1].statuses().is_empty()); // hacked via the installed deck
    }

    // -- L3: behavior profiles drive the action phase (§7J) -------------------

    #[test]
    fn targeting_lowest_integrity_finishes_the_wounded() {
        let a = unit(0, Team::A, 0).with_targeting(TargetingProfile::LowestIntegrity);
        let healthy = unit(1, Team::B, 2); // nearer, full HP
        let mut wounded = unit(2, Team::B, 4); // farther, low HP
        wounded.character.integrity = 5.0;
        let b = Battle::new(vec![a, healthy, wounded], 1);
        assert_eq!(b.select_target(0), Some(2)); // the wounded, despite the distance
    }

    #[test]
    fn targeting_backline_reaches_past_the_front() {
        let a = unit(0, Team::A, 0).with_targeting(TargetingProfile::Backline);
        let front = unit(1, Team::B, 2);
        let back = unit(2, Team::B, 6);
        let b = Battle::new(vec![a, front, back], 1);
        assert_eq!(b.select_target(0), Some(2)); // the farthest enemy
    }

    #[test]
    fn a_spoof_overrides_the_program_in_battle() {
        let a = unit(0, Team::A, 0).with_targeting(TargetingProfile::Nearest);
        let near = unit(1, Team::B, 2);
        let far = unit(2, Team::B, 6);
        let mut b = Battle::new(vec![a, near, far], 1);
        assert_eq!(b.select_target(0), Some(1)); // program: Nearest
        b.units[0].spoof(TargetingProfile::Backline, 3); // the enemy hacks the script
        assert_eq!(b.units[0].targeting(), TargetingProfile::Backline);
        assert_eq!(b.select_target(0), Some(2)); // now strikes the backline
    }

    #[test]
    fn movement_hold_stays_and_kite_retreats() {
        // Hold: never close, even out of range.
        let holder = unit(0, Team::A, 0).with_movement(MovementProfile::Hold);
        let enemy = unit(1, Team::B, 5);
        let b = Battle::new(vec![holder, enemy], 1);
        assert_eq!(b.movement_step(0, 1), Hex::new(0, 0));

        // Kite (skirmish): a ranged unit crowded inside its standoff range backs off to
        // reopen it. (A melee-range kiter would instead hold — nothing to reopen.)
        let mut gun = Attack::melee(5.0);
        gun.range = 3; // standoff range 3
        let kiter = unit(0, Team::A, 3).with_movement(MovementProfile::Kite).with_attack(gun);
        let foe = unit(1, Team::B, 4); // adjacent (distance 1 < standoff)
        let b2 = Battle::new(vec![kiter, foe], 1);
        let step = b2.movement_step(0, 1);
        assert!(step.distance(Hex::new(4, 0)) > 1); // backed off to reopen the gap
    }

    #[test]
    fn a_skirmisher_closes_when_out_of_range() {
        // The other half of skirmish: too far to shoot ⇒ advance into range, don't idle.
        let mut gun = Attack::melee(5.0);
        gun.range = 3;
        let kiter = unit(0, Team::A, 0).with_movement(MovementProfile::Kite).with_attack(gun);
        let foe = unit(1, Team::B, 6); // distance 6 > standoff 3
        let b = Battle::new(vec![kiter, foe], 1);
        assert!(b.movement_step(0, 1).distance(Hex::new(6, 0)) < 6); // closed the gap
    }

    #[test]
    fn default_behavior_is_attack_nearest_advance() {
        // The pre-L3 baseline still holds with no program set.
        let a = unit(0, Team::A, 0);
        let near = unit(1, Team::B, 2);
        let far = unit(2, Team::B, 6);
        let b = Battle::new(vec![a, near, far], 1);
        assert_eq!(b.units[0].targeting(), TargetingProfile::Nearest);
        assert_eq!(b.units[0].movement(), MovementProfile::Advance);
        assert_eq!(b.select_target(0), Some(1)); // nearest
        assert_eq!(b.movement_step(0, 1), Hex::new(0, 0).step_toward(Hex::new(2, 0)));
    }

    // -- Phase 2: the move stat, move-then-act, occupancy (§10.4/§10.5a) --------

    #[test]
    fn move_then_act_closes_and_strikes_same_activation() {
        // Speed 5: a unit 3 hexes out closes *and* attacks in one activation.
        let atk = unit(0, Team::A, 0).with_speed(5).with_initiative(10.0);
        let mut dummy = unit(1, Team::B, 3).with_movement(MovementProfile::Hold);
        dummy.character.integrity = 100.0;
        let mut b = Battle::new(vec![atk, dummy], 1);
        b.action_phase();
        assert!(b.units[0].pos.distance(Hex::new(3, 0)) <= 1); // closed to melee
        assert!(b.units[1].character.integrity < 100.0); // and hit, same turn
    }

    #[test]
    fn speed_zero_never_moves() {
        let atk = unit(0, Team::A, 0).with_speed(0);
        let foe = unit(1, Team::B, 4);
        let mut b = Battle::new(vec![atk, foe], 1);
        b.action_phase();
        assert_eq!(b.units[0].pos, Hex::new(0, 0)); // rooted
    }

    #[test]
    fn an_occupied_lane_boxes_the_unit_in() {
        // The only distance-reducing hex (1,0) is taken by an ally → no improving
        // free neighbour, so the mover stays put (boxed in).
        let mover = unit(0, Team::A, 0).with_movement(MovementProfile::Advance);
        let blocker = unit(1, Team::A, 1);
        let target = unit(2, Team::B, 2);
        let b = Battle::new(vec![mover, blocker, target], 1);
        assert!(b.occupied_by_other(0, Hex::new(1, 0)));
        assert_eq!(b.movement_step(0, 2), Hex::new(0, 0)); // boxed
        // remove the blocker (id 1) and it advances into the freed lane.
        let b2 = Battle::new(vec![unit(0, Team::A, 0), unit(2, Team::B, 2)], 1);
        assert_eq!(b2.movement_step(0, 1), Hex::new(1, 0));
    }

    // -- Stat/skill rework: attributes + tier skills ----------------------------------

    #[test]
    fn effective_skill_is_attribute_plus_tier() {
        // Skills are tiers on the governing attribute: effective = attribute + tier.
        let mut u = unit(0, Team::A, 0);
        u.character.base_mut().dexterity = 10.0;
        u.skills.set(Skill::Gunnery, SkillTier::Expert.modifier()); // +2
        assert_eq!(u.effective_skill(Skill::Gunnery), 12); // Dex 10 + expert 2
        u.skills.set(Skill::Gunnery, SkillTier::Untrained.modifier()); // −4
        assert_eq!(u.effective_skill(Skill::Gunnery), 6); // Dex 10 − 4
    }

    #[test]
    fn plating_reduces_dexterity_and_drags_dex_skills_with_it() {
        // The armor tradeoff: heavy plating's −Dexterity lowers the attribute, which drags
        // every Dex-governed skill (and, later, Evasion) down in one stroke.
        let mut u = unit(0, Team::A, 0);
        u.character.base_mut().dexterity = 6.0;
        u.skills.set(Skill::Gunnery, SkillTier::Competent.modifier()); // 0
        assert_eq!(u.effective_skill(Skill::Gunnery), 6);
        u.character.install(Decorator::gear(Tag::Implant, vec![Factor::add(Stat::Dexterity, -2.0)]));
        assert_eq!(u.dexterity(), 4); // 6 − 2 plating
        assert_eq!(u.effective_skill(Skill::Gunnery), 4); // dragged down with Dex
    }

    // -- Terrain: bounds, pathing, cover, hazards -------------------------------------

    #[test]
    fn zone_friction_pins_a_kiter_at_the_edge() {
        let kiter = || {
            let mut gun = Attack::melee(5.0);
            gun.range = 3; // a skirmisher wanting standoff 3, crowded at distance 1
            unit(0, Team::A, 0).with_movement(MovementProfile::Kite).with_attack(gun) // corner (0,0)
        };
        let foe = || unit(1, Team::B, 1).with_movement(MovementProfile::Hold); // adjacent
        // It *wants* to flee outward — on the open plane it does (cost-1 step, uncatchable).
        let open = Battle::new(vec![kiter(), foe()], 1);
        assert!(open.movement_step(0, 1).distance(Hex::new(1, 0)) > 1);
        // With a zone the flee hex lies past the soft edge, costing more move (>1) than a
        // speed-1 unit has — so a full activation can't afford it and the kiter stays put,
        // pinned where a pursuer runs it down. (The friction fix for the kite stalemate.)
        let mut zoned = Battle::new(vec![kiter(), foe()], 1).with_terrain(Terrain::arena(6, 4));
        assert!(zoned.terrain.move_cost(zoned.movement_step(0, 1)) > 1); // flee step is dear
        zoned.physical_activation(0);
        assert!(zoned.terrain.in_zone(zoned.units[0].pos)); // couldn't leave the zone
    }

    #[test]
    fn a_player_unit_flows_to_a_reach_objective() {
        // Objective-seeking: with an unmet Reach goal and no enemy in range, a player unit
        // moves toward the point (not just the enemy) so the objective resolves in auto-play.
        let mover = unit(0, Team::A, 1).at(Hex::new(0, 0)).with_speed(2);
        let foe = unit(1, Team::B, 7).at(Hex::new(7, 0)).with_movement(MovementProfile::Hold);
        let goal = Hex::new(4, 0);
        let objs = Objectives::new(vec![Goal::new(ObjectiveKind::Reach(goal).build(), 0, 0)]);
        let mut b = Battle::new(vec![mover, foe], 1)
            .with_terrain(Terrain::arena(8, 4))
            .with_objectives(objs);
        let before = b.units[0].pos.distance(goal);
        b.physical_activation(0);
        assert!(b.units[0].pos.distance(goal) < before); // closed on the objective hex
    }

    #[test]
    fn advance_paths_around_a_wall() {
        // A blocker on the direct lane: grid-distance can't improve (every closer hex is
        // the wall or off-board), so a greedy mover would stick. The BFS flow field routes
        // it around through the open flank instead.
        let mover = unit(0, Team::A, 1).at(Hex::new(0, 1)).with_movement(MovementProfile::Advance);
        let target = unit(1, Team::B, 2).at(Hex::new(2, 1));
        let terrain = Terrain::arena(6, 4).set(Hex::new(1, 1), Tile::Blocked);
        let b = Battle::new(vec![mover, target], 1).with_terrain(terrain);
        let step = b.movement_step(0, 1);
        assert_ne!(step, Hex::new(1, 1)); // never walks into the wall
        assert_ne!(step, Hex::new(0, 1)); // and doesn't stay boxed — it goes around
        assert_eq!(step, Hex::new(0, 2)); // the flanking hex on the shortest real path
    }

    #[test]
    fn cover_lowers_the_to_hit_target() {
        // Opposed roll-under: attacker rolls 13 vs to-hit target (Body 6 + Melee 2) × 2 = 16
        // (open) — a hit; cover 4 drops the target to 12, so the same 13 now whiffs. The
        // defender rolls 12 vs Evasion (Dex 4) × 2 = 8, failing to dodge both times, which
        // isolates the cover effect.
        let make = |cover: bool| {
            let attacker = unit(0, Team::A, 0).with_body(6.0).with_skill(Skill::Melee, 2);
            let target = unit(1, Team::B, 1).with_dexterity(4.0);
            let mut terrain = Terrain::default();
            if cover {
                terrain = terrain.set(Hex::new(1, 0), Tile::Cover(4));
            }
            Battle::with_rng(vec![attacker, target], ScriptedRng::from_d10([12, 13]))
                .with_terrain(terrain)
                .with_log()
        };
        let mut open = make(false);
        let w = open.units[0].weapon_at_any().unwrap();
        open.resolve_attack_with(0, 1, w);
        assert_eq!(open.events().last().unwrap().event.kind(), "attacked"); // no cover — lands

        let mut covered = make(true);
        let w2 = covered.units[0].weapon_at_any().unwrap();
        covered.resolve_attack_with(0, 1, w2);
        assert_eq!(covered.events().last().unwrap().event.kind(), "missed"); // cover saved it
    }

    #[test]
    fn a_hazard_hex_burns_its_occupant() {
        let mut victim = unit(0, Team::A, 2); // stands on the hazard at (2,0)
        victim.character.integrity = 50.0;
        let bystander = unit(1, Team::B, 5); // on open ground — untouched
        let terrain = Terrain::arena(8, 4).set(
            Hex::new(2, 0),
            Tile::Hazard { damage: 9.0, dtype: DamageType::Piercing, pen: PenTier::Internal },
        );
        let mut b = Battle::new(vec![victim, bystander], 1).with_terrain(terrain).with_log();
        b.terrain_phase();
        assert!(b.units[0].character.integrity < 50.0); // the field burned it
        assert_eq!(b.units[1].character.integrity, 30.0); // bystander on open ground is fine
        assert_eq!(b.events().last().unwrap().event.kind(), "damaged"); // surfaced as an event
    }

    #[test]
    fn living_units_never_share_a_hex() {
        // Two allies racing the same enemy must queue, not stack.
        let a = unit(0, Team::A, 0).with_speed(3);
        let ally = unit(2, Team::A, 1).with_speed(3);
        let foe = unit(1, Team::B, 6).with_speed(3);
        let mut b = Battle::new(vec![a, ally, foe], 7);
        for _ in 0..12 {
            b.step();
            let live: Vec<Hex> =
                b.units.iter().filter(|u| u.is_alive()).map(|u| u.pos).collect();
            for x in 0..live.len() {
                for y in (x + 1)..live.len() {
                    assert_ne!(live[x], live[y], "two units overlapped");
                }
            }
        }
    }

    // -- Phase 3: AoE footprints + friendly fire (§7G) -------------------------

    #[test]
    fn blast_friendly_fires_everyone_in_the_radius() {
        let mut atk = unit(0, Team::A, 0);
        atk.rearm(|w| w.footprint = Footprint::Blast(1)); // disc around the target hex
        let enemy = unit(1, Team::B, 3); // target at (3,0)
        let enemy_mate = unit(2, Team::B, 4); // (4,0), adjacent to target
        let our_own = unit(3, Team::A, 2); // (2,0), adjacent to target — our ally
        let mut b = Battle::new(vec![atk, enemy, enemy_mate, our_own], 1);
        let hp: Vec<f32> = b.units.iter().map(|u| u.character.integrity).collect();
        b.resolve_attack(0, 1);
        assert!(b.units[1].character.integrity < hp[1]); // the target
        assert!(b.units[2].character.integrity < hp[2]); // its neighbour
        assert!(b.units[3].character.integrity < hp[3]); // OUR unit — friendly fire is on
        assert_eq!(b.units[0].character.integrity, hp[0]); // the attacker is spared
    }

    #[test]
    fn beam_strikes_every_unit_along_the_line() {
        let mut atk = unit(0, Team::A, 0);
        atk.rearm(|w| w.footprint = Footprint::Beam(4)); // line of 4 from the attacker
        let on1 = unit(1, Team::B, 1); // (1,0) — the target, on the beam
        let on2 = unit(2, Team::A, 2); // (2,0) — ally on the beam (friendly fire)
        let on3 = unit(3, Team::B, 3); // (3,0) — on the beam
        let mut off = unit(4, Team::B, 1);
        off.pos = Hex::new(1, 1); // off the +q axis — spared
        let mut b = Battle::new(vec![atk, on1, on2, on3, off], 1);
        let hp: Vec<f32> = b.units.iter().map(|u| u.character.integrity).collect();
        b.resolve_attack(0, 1);
        assert!(b.units[1].character.integrity < hp[1]);
        assert!(b.units[2].character.integrity < hp[2]);
        assert!(b.units[3].character.integrity < hp[3]);
        assert_eq!(b.units[4].character.integrity, hp[4]); // off the line
    }

    #[test]
    fn a_smart_beam_holds_fire_on_allies_in_the_line() {
        // Same beam through an ally — but smart-linked (IFF). The enemies on the line
        // are hit; the friendly in the path is identified and spared.
        let mut atk = unit(0, Team::A, 0);
        atk.rearm(|w| {
            w.footprint = Footprint::Beam(4);
            w.tags = w.tags.with(EquipmentTags::SMART); // smartgun
        });
        let on1 = unit(1, Team::B, 1); // (1,0) — enemy on the beam
        let ally = unit(2, Team::A, 2); // (2,0) — ally in the path
        let on3 = unit(3, Team::B, 3); // (3,0) — enemy on the beam
        let mut b = Battle::new(vec![atk, on1, ally, on3], 1);
        let hp: Vec<f32> = b.units.iter().map(|u| u.character.integrity).collect();
        b.resolve_attack(0, 1);
        assert!(b.units[1].character.integrity < hp[1]); // enemy struck
        assert_eq!(b.units[2].character.integrity, hp[2]); // ally spared by IFF
        assert!(b.units[3].character.integrity < hp[3]); // enemy past the ally still struck
    }

    #[test]
    fn single_footprint_spares_bystanders() {
        let atk = unit(0, Team::A, 0); // default Single
        let target = unit(1, Team::B, 1);
        let bystander = unit(2, Team::B, 2);
        let mut b = Battle::new(vec![atk, target, bystander], 1);
        let hp: Vec<f32> = b.units.iter().map(|u| u.character.integrity).collect();
        b.resolve_attack(0, 1);
        assert!(b.units[1].character.integrity < hp[1]);
        assert_eq!(b.units[2].character.integrity, hp[2]); // untouched
    }

    // -- Phase 4: woven initiative (§7C/§10.3) ---------------------------------

    #[test]
    fn woven_order_interleaves_physical_and_digital_by_speed() {
        // Runner: Link 6 (digital key 6) but slow (Initiative 1). Bruiser: no deck,
        // Initiative 5. The single track interleaves: the runner *hacks* first (6),
        // the bruiser *swings* (5), then the runner *moves* (1).
        let mut runner = unit(0, Team::A, 0);
        runner.character.base_mut().initiative = 1.0;
        runner.character.base_mut().link = 6.0;
        runner.grant_hack(Hack::new(4, StatusSpec::lockware(), 1, 4));
        let mut bruiser = unit(1, Team::B, 1);
        bruiser.character.base_mut().initiative = 5.0;
        let b = Battle::new(vec![runner, bruiser], 1);
        assert_eq!(b.woven_order(), vec![(0, true), (1, false), (0, false)]);
    }

    #[test]
    fn a_non_hacker_has_only_a_physical_activation() {
        let a = unit(0, Team::A, 0); // link 0, no deck
        let foe = unit(1, Team::B, 1);
        let b = Battle::new(vec![a, foe], 1);
        let order = b.woven_order();
        assert_eq!(order.len(), 2); // two physicals, no digital entries
        assert!(order.iter().all(|&(_, digital)| !digital));
    }

    // -- Phase 5: weapons & range bands (§10.5) --------------------------------

    fn gun(damage: f32, min_range: i32, range: i32) -> Attack {
        Attack {
            damage,
            speed: 3, // a fast round
            dtype: DamageType::Piercing,
            pen: PenTier::Internal,
            skill: Skill::Gunnery,
            accuracy: 0,
            range,
            min_range,
            emp: false,
            footprint: Footprint::Single,
            tags: EquipmentTags::NONE,
        }
    }

    #[test]
    fn an_undefended_melee_blow_auto_hits_without_a_roll() {
        // Evasion 0 + melee (no range penalty) ⇒ TN ≤ 0 ⇒ auto-hit, no RNG burned.
        let atk = unit(0, Team::A, 0); // default melee
        let tgt = unit(1, Team::B, 1); // Evasion 0
        let mut b = Battle::new(vec![atk, tgt], 1).with_log();
        b.resolve_attack(0, 1);
        assert!(b.events().iter().any(|r| r.event.kind() == "attacked"));
        assert!(!b.events().iter().any(|r| r.event.kind() == "missed"));
    }

    #[test]
    fn evasion_lets_a_target_dodge() {
        // A nimble target (Evasion derived from Dexterity) gets an opposed Evade roll: even a
        // solid attacker is dodged some of the time. Over many seeds, some land and some whiff.
        let mut hits = 0;
        let mut misses = 0;
        for seed in 0..40 {
            let mut atk = unit(0, Team::A, 0).with_skill(Skill::Melee, 2);
            atk.character.base_mut().body = 6.0; // effective Melee 8 ⇒ to-hit target 16 (usually connects)
            let mut tgt = unit(1, Team::B, 1).with_skill(Skill::Evade, 2);
            tgt.character.base_mut().dexterity = 6.0; // Evasion 6 + 2 = 8 ⇒ dodges ~a quarter
            let mut b = Battle::new(vec![atk, tgt], seed).with_log();
            b.resolve_attack(0, 1);
            match b.events()[0].event.kind() {
                "attacked" => hits += 1,
                "missed" => misses += 1,
                k => panic!("unexpected {k}"),
            }
        }
        assert!(hits > 0 && misses > 0, "both outcomes occur: {hits} hits / {misses} misses");
    }

    #[test]
    fn equipment_tags_are_a_set_of_const_flags() {
        let plain = EquipmentTags::NONE;
        assert!(!plain.has(EquipmentTags::SMART) && !plain.has(EquipmentTags::AWKWARD));
        // Equipment can carry several tags at once.
        let both = EquipmentTags::SMART | EquipmentTags::AWKWARD;
        assert!(both.has(EquipmentTags::SMART) && both.has(EquipmentTags::AWKWARD));
        // `with` adds one without disturbing the rest.
        let added = EquipmentTags::SMART.with(EquipmentTags::AWKWARD);
        assert_eq!(added, both);
        assert!(!EquipmentTags::SMART.has(EquipmentTags::AWKWARD)); // distinct flags
    }

    #[test]
    fn the_smart_and_digital_tags_carry_their_rules() {
        // Each tag's behaviour lives on the tag set, next to the flag.
        assert!(EquipmentTags::SMART.spares_team()); // IFF: spares the team
        assert!(!EquipmentTags::NONE.spares_team());
        assert!(EquipmentTags::DIGITAL.breachable()); // a breach can trip it
        assert!(!EquipmentTags::NONE.breachable()); // inert physical: immune
        // The implant accessor defers to the tag rule.
        assert!(Implant::cyberdeck().is_digital());
        assert!(!Implant::subdermal_plating().is_digital());
    }

    #[test]
    fn the_awkward_tag_adds_a_close_range_to_hit_rule() {
        // The AWKWARD tag carries the rule: clumsy jammed in close, fading to none at
        // proper range. The untagged set pays nothing.
        let t = EquipmentTags::AWKWARD;
        assert_eq!(t.to_hit_penalty(0), AWKWARD_POINT_BLANK); // same hex (≈never): -4
        assert_eq!(t.to_hit_penalty(1), AWKWARD_ADJACENT); // adjacent: clumsy (-2)
        assert_eq!(t.to_hit_penalty(2), 0); // at proper range: clean
        assert_eq!(t.to_hit_penalty(8), 0);
        assert_eq!(EquipmentTags::NONE.to_hit_penalty(1), 0); // no tag, no penalty
        assert_eq!(Attack::melee(10.0).tags.to_hit_penalty(1), 0); // melee: never awkward
    }

    #[test]
    fn the_ranged_tag_adds_a_distance_to_hit_rule() {
        // The RANGED tag carries the rule: an inherent penalty growing with distance,
        // discrete from awkward.
        let t = EquipmentTags::RANGED;
        assert_eq!(t.to_hit_penalty(1), 0); // short range: clean
        assert_eq!(t.to_hit_penalty(2), 0);
        assert_eq!(t.to_hit_penalty(3), RANGED_MEDIUM_PENALTY); // medium: -2
        assert_eq!(t.to_hit_penalty(6), RANGED_LONG_PENALTY); // long: -4
        assert_eq!(EquipmentTags::NONE.to_hit_penalty(6), 0); // untagged: no penalty
    }

    #[test]
    fn a_rifle_is_penalised_close_and_far_with_a_sweet_spot() {
        // RANGED | AWKWARD compose: the two rules sum to clumsy adjacent (awkward),
        // harder far (ranged), a clean band between — the U-curve, from one summed rule.
        let rifle = EquipmentTags::AWKWARD | EquipmentTags::RANGED;
        assert_eq!(rifle.to_hit_penalty(1), AWKWARD_ADJACENT); // jammed in close: +2 (awkward)
        assert_eq!(rifle.to_hit_penalty(2), 0); // the sweet spot: clean
        assert_eq!(rifle.to_hit_penalty(4), RANGED_MEDIUM_PENALTY); // reaching out: +2 (ranged)
        assert_eq!(rifle.to_hit_penalty(6), RANGED_LONG_PENALTY); // long shot: +4 (ranged)
    }

    #[test]
    fn weapon_selection_picks_the_band_that_covers_the_distance() {
        // Rifle 2..=6 (dmg 12) + knife 1..=1 (dmg 8).
        let mut u = unit(0, Team::A, 0);
        u.set_weapon(gun(12.0, 2, 6)); // primary rifle
        u.add_weapon(gun(8.0, 1, 1)); // knife sidearm
        assert_eq!(u.weapon_at(1).map(|w| w.damage), Some(8.0)); // melee → knife
        assert_eq!(u.weapon_at(4).map(|w| w.damage), Some(12.0)); // mid → rifle
        assert!(u.weapon_at(7).is_none()); // out of every band
    }

    #[test]
    fn a_weapon_granted_by_a_decorator_joins_the_loadout() {
        // Weapons are Capability::Weapon grants (L6): a chrome arm granting one adds to
        // the loadout, and removing it (unequip / breach) drops it — like a deck/hack.
        let mut u = unit(0, Team::A, 0); // starts with the default melee (dmg 10)
        assert_eq!(u.weapons().len(), 1);
        let arm = u.apply_modifier(weapon_grant(gun(20.0, 2, 5))); // a granted rifle
        assert_eq!(u.weapons().len(), 2);
        assert_eq!(u.weapon_at(4).map(|w| w.damage), Some(20.0)); // the rifle reaches mid
        assert_eq!(u.weapon_at(1).map(|w| w.damage), Some(10.0)); // melee in close
        u.character.remove(arm); // unequip
        assert_eq!(u.weapons().len(), 1);
        assert!(u.weapon_at(4).is_none()); // only melee left
    }

    #[test]
    fn gear_upgrades_armor_class_compositionally() {
        // Armor class is an Override (L6): a plate vest wins over the innate class,
        // and dropping it reverts — the mitigation matrix reads the composed class.
        let mut u = unit(0, Team::A, 0); // innate Mail
        assert_eq!(u.armor_class(), ArmorClass::Mail);
        let vest = u.apply_modifier(
            Decorator::gear(Tag::Gear, vec![]).with_override(Override::Armor(ArmorClass::Plate)),
        );
        assert_eq!(u.armor_class(), ArmorClass::Plate);
        u.character.remove(vest);
        assert_eq!(u.armor_class(), ArmorClass::Mail); // back to innate
    }

    #[test]
    fn a_polearm_cannot_strike_point_blank() {
        let pike = gun(10.0, 2, 2); // reach 2 only
        assert!(!pike.usable_at(1)); // adjacent is too close
        assert!(pike.usable_at(2));
        assert!(!pike.usable_at(3));
    }

    #[test]
    fn a_ranged_closer_halts_at_standoff_then_fires() {
        // Rifle 2..=6, speed 5, target 6 away. Advance closes only until in band
        // (distance 6), then fires from there instead of walking into melee.
        let mut atk = unit(0, Team::A, 0).with_speed(5).with_initiative(10.0);
        atk.set_weapon(gun(12.0, 2, 6)); // a handy rifle — no awkward / range penalty
        let mut dummy = unit(1, Team::B, 6).with_movement(MovementProfile::Hold);
        dummy.character.integrity = 100.0;
        let mut b = Battle::new(vec![atk, dummy], 1);
        b.action_phase();
        assert_eq!(b.units[0].pos, Hex::new(0, 0)); // already in band — never moved
        assert!(b.units[1].character.integrity < 100.0); // fired from range
    }

    #[test]
    fn the_knife_finishes_what_the_rifle_started_in_melee() {
        // No standoff weapon at distance 1, but the knife covers it.
        let atk = unit(0, Team::A, 0).with_weapon(gun(7.0, 1, 1)); // primary 1..=1 + knife 1..=1
        let target = unit(1, Team::B, 1);
        let mut b = Battle::new(vec![atk, target], 1);
        let before = b.units[1].character.integrity;
        b.action_phase();
        assert!(b.units[1].character.integrity < before);
    }

    // -- Phase 6: death triggers (§10.9) ---------------------------------------

    #[test]
    fn detonate_blasts_neighbours_on_death() {
        // A bomb dies and explodes, hurting both an enemy and an ally nearby.
        let mut bomb = unit(0, Team::B, 0);
        bomb.character.integrity = 1.0;
        bomb.on_death =
            DeathTrigger::Detonate { damage: 20.0, dtype: DamageType::Piercing, pen: PenTier::Internal, radius: 1 };
        let mut killer = unit(1, Team::A, 0);
        killer.pos = Hex::new(1, 0); // adjacent → caught in the blast
        let bystander = unit(2, Team::B, 1); // ally of the bomb, also adjacent
        let mut b = Battle::new(vec![bomb, killer, bystander], 1);
        // kill the bomb directly via a status DoT path: just zero it and reap.
        b.units[0].character.apply_pool_damage(0, 0, 5.0, PenTier::Internal, true);
        assert!(!b.units[0].is_alive());
        let (k, s) = (b.units[1].character.integrity, b.units[2].character.integrity);
        b.reap();
        assert!(b.units[1].character.integrity < k); // the killer caught the blast
        assert!(b.units[2].character.integrity < s); // and the bomb's own ally (friendly fire)
        assert!(b.units[0].death_resolved); // fired exactly once
    }

    #[test]
    fn detonate_can_chain_through_a_second_bomb() {
        let bomb = |id, q| {
            let mut u = unit(id, Team::B, q);
            u.character.integrity = 1.0;
            u.on_death = DeathTrigger::Detonate {
                damage: 50.0,
                dtype: DamageType::Piercing,
                pen: PenTier::Internal,
                radius: 1,
            };
            u
        };
        let mut b = Battle::new(vec![bomb(0, 0), bomb(1, 1)], 1);
        b.units[0].character.apply_pool_damage(0, 0, 5.0, PenTier::Internal, true); // pop the first
        b.reap();
        // the first blast killed the second, whose blast fired in turn.
        assert!(!b.units[1].is_alive());
        assert!(b.units[1].death_resolved);
    }

    // -- Phase 7: the board seam (§7B) -----------------------------------------

    #[test]
    fn the_seam_lets_a_staggered_front_clash_in_melee() {
        // A at (0,0), B at (1,1): raw grid distance 2, but the Up seam pairs them, so
        // the melee attacker strikes without moving (speed 0 isolates the seam).
        let atk = unit(0, Team::A, 0).with_speed(0).with_initiative(10.0);
        let mut foe = unit(1, Team::B, 1);
        foe.pos = Hex::new(1, 1);
        foe.character.integrity = 100.0;
        let mut b = Battle::new(vec![atk, foe], 1).with_seam(SeamOffset::Up);
        assert_eq!(b.units[0].pos.distance(b.units[1].pos), 2); // not grid-adjacent
        b.action_phase();
        assert!(b.units[1].character.integrity < 100.0); // the seam engaged them anyway
    }

    #[test]
    fn the_other_stagger_is_out_of_melee_without_the_seam() {
        // Same geometry, Down seam: the pair isn't joined, so a speed-0 melee unit
        // can't reach.
        let atk = unit(0, Team::A, 0).with_speed(0).with_initiative(10.0);
        let mut foe = unit(1, Team::B, 1);
        foe.pos = Hex::new(1, 1);
        foe.character.integrity = 100.0;
        let mut b = Battle::new(vec![atk, foe], 1).with_seam(SeamOffset::Down);
        b.action_phase();
        assert_eq!(b.units[1].character.integrity, 100.0); // distance 2, no seam pairing → untouched
    }

    #[test]
    fn data_spill_infects_nearby_enemies_on_death() {
        let mut host = unit(0, Team::B, 0);
        host.character.integrity = 1.0;
        host.on_death =
            DeathTrigger::DataSpill { spec: StatusSpec::lockware(), stacks: 2, duration: 3, radius: 1 };
        let mut enemy = unit(1, Team::A, 0);
        enemy.pos = Hex::new(1, 0); // adjacent enemy
        let ally = unit(2, Team::B, 1); // adjacent ally — NOT infected by a spill
        let mut b = Battle::new(vec![host, enemy, ally], 1);
        b.units[0].character.apply_pool_damage(0, 0, 5.0, PenTier::Internal, true);
        b.reap();
        assert_eq!(b.units[1].statuses().len(), 1); // the enemy got the leaked payload
        assert!(b.units[2].statuses().is_empty()); // the ally did not
    }
}
