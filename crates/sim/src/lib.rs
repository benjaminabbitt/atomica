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
//! - [`hack`]ing — the netrunning digital attack (`3d6 + avg(Hacking, channel)`
//!   vs `Firewall`; the channel is the weaker endpoint's Link, §7F/§13);
//! - an initiative-ordered tick loop with a minimal "attack nearest / step toward"
//!   resolution plus a digital pass.
//!
//! Not yet built: the two contagion families (a spreading special case of
//! statuses) and IFF/spoof. (Heat — a thermal layer — was dropped from scope.)

pub mod armor;
mod board;
mod chargen;
mod hack;
mod hex;
mod implant;
mod objective;
mod profile;
mod rng;
mod roll;
mod skills;
mod status;

pub use armor::ArmorClass;
pub use board::{Board, SeamOffset};
pub use chargen::{
    Amount, BaseLine, Capability, Character, Condition, Decorator, DamageEvent, Event, Expiration,
    Factor, FactorKind, Flag, Gate, GenId, Hook, HookEffect, Modifier, ModifierKind, Override,
    Priority, Reaction, Realized, Remove, Stat, Tag, Wear,
};
pub use hack::{hack_rating, Hack, HackResult};
pub use hex::Hex;
pub use implant::{Contribution, Implant, Pan};
pub use profile::{MovementProfile, TargetingProfile};
pub use objective::{
    Goal, Hold, MarginLoss, Objective, ObjectiveKind, ObjectiveStatus, Objectives, Reach, Survive,
    TimeAttack, WinFight, PLAYER,
};
pub use rng::{RandomSource, ScriptedRng, SplitMix64};
pub use roll::{resolve_contest, Contest, RollOutcome};
pub use skills::{Chassis, Skill, Skills};
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

/// A single attack profile. (Weapons/loadouts will compose these later.)
#[derive(Clone, Copy, Debug)]
pub struct Attack {
    pub damage: f32,
    pub dtype: DamageType,
    pub pen: PenTier,
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
            dtype: DamageType::Piercing,
            pen: PenTier::Internal,
            range: 1,
            min_range: 1,
            emp: false,
            footprint: Footprint::Single,
        }
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
}

/// A combatant. The stat line mirrors the design's "Unit anatomy".
#[derive(Clone, Debug)]
pub struct Unit {
    pub id: UnitId,
    pub name: String,
    pub team: Team,
    pub pos: Hex,

    /// Armor class for the [`armor`] matrix.
    pub armor_class: ArmorClass,
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
            armor_class: ArmorClass::default(),
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
    /// Effective **Firewall** (the digital TN, §13).
    pub fn firewall(&self) -> i32 {
        self.realized().firewall()
    }
    /// Effective **Immunity** (the bio TN).
    pub fn immunity(&self) -> i32 {
        self.realized().immunity()
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

    /// Make a contested roll with this unit's `skill` (+ `equipment`) vs `tn`.
    pub fn contest<R: RandomSource>(
        &self,
        skill: Skill,
        equipment: i32,
        tn: i32,
        rng: &mut R,
    ) -> RollOutcome {
        resolve_contest(rng, Contest::new(self.skill(skill), equipment, tn))
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
        self.implants.push(InstalledImplant { spec: implant, gen });
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
    /// repairing it (§3.2). Destroyed gear is terminal (a no-op).
    pub fn repair_implant(&mut self, idx: usize) {
        self.transition(idx, Condition::Online);
    }

    /// The first active (breachable) implant — the hack's target slot (a
    /// targeting rule is a later refinement).
    fn first_active_implant(&self) -> Option<usize> {
        (0..self.implants.len()).find(|&i| self.implant_condition(i).is_active())
    }

    /// Indices of all active (breachable) implants — Cascade and EMP hit them all.
    fn active_implant_indices(&self) -> Vec<usize> {
        (0..self.implants.len()).filter(|&i| self.implant_condition(i).is_active()).collect()
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

/// Fixed magnitude of the degrade-class liabilities an EMP fires — it has no
/// margin/crit, being a blunt physical pulse. Placeholder (TBD).
const EMP_MAGNITUDE: u32 = 2;

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
    withdrawn: bool,
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
    pub fn with_rng(units: Vec<Unit>, rng: R) -> Self {
        let objectives = Objectives::new(vec![Goal::new(Box::new(WinFight), 1, 1)]);
        Self {
            units,
            tick: 0,
            rng,
            objectives,
            board: Board::default(),
            withdrawn: false,
        }
    }

    /// Builder: set the seam offset (§7B — the mesh item that picks the stagger).
    pub fn with_seam(mut self, offset: SeamOffset) -> Self {
        self.board = Board::new(offset);
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
        self.reap(); // DoTs can kill — fire their death triggers
        self.woven_phase();
        self.decay_phase();

        self.outcome()
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
        for unit in self.units.iter_mut() {
            if !unit.is_alive() {
                continue;
            }
            unit.character.dispatch(Event::TickStart, self.tick, &mut self.rng);
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

    /// One unit's **physical** activation (§7J/§10.4/§10.5): pick a target by its
    /// targeting profile, move up to `speed` hexes by its movement profile (through
    /// free hexes), then fire the best weapon whose **range band** covers the target.
    /// A *closing* profile (Advance/Flank/Swarm) halts once any weapon can reach —
    /// standoff — so a ranged build doesn't walk into melee.
    fn physical_activation(&mut self, i: usize) {
        let Some(target) = self.select_target(i) else {
            return;
        };
        let closes = matches!(
            self.units[i].movement(),
            MovementProfile::Advance | MovementProfile::Flank | MovementProfile::Swarm
        );
        for _ in 0..self.units[i].speed.max(0) {
            let dist = self.reach(i, target);
            if closes && self.units[i].weapon_at(dist).is_some() {
                break; // standoff: a weapon already reaches — stop closing and fire
            }
            let next = self.movement_step(i, target);
            if next == self.units[i].pos {
                break; // at the profile's goal, or boxed in
            }
            self.units[i].pos = next;
        }
        let dist = self.reach(i, target);
        if let Some(weapon) = self.units[i].weapon_at(dist) {
            self.resolve_attack_with(i, target, weapon);
        }
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
        let enemies = || {
            self.units
                .iter()
                .enumerate()
                .filter(move |(j, u)| *j != i && u.is_alive() && u.team == me.team.enemy())
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

    /// One step for unit `i` toward what its **movement profile** wants (§7J), routed
    /// only through **free** hexes (§10.5a occupancy). Returns the best free neighbour,
    /// or the unit's own hex when it's already at the goal or **boxed in** (every
    /// improving neighbour occupied). Greedy single-hex pathing — full A* is later.
    fn movement_step(&self, i: usize, target: usize) -> Hex {
        let here = self.units[i].pos;
        let tpos = self.units[target].pos;
        // Candidate hexes: stay put, or step to a free neighbour. `once(here)` is
        // first so it wins ties — a unit only moves when a neighbour is strictly
        // better by the profile's scoring.
        let free = |h: &Hex| !self.occupied_by_other(i, *h);
        let candidates =
            || std::iter::once(here).chain(here.neighbors().into_iter().filter(free));
        match self.units[i].movement() {
            MovementProfile::Hold => here,
            MovementProfile::Advance => {
                candidates().min_by_key(|h| h.distance(tpos)).unwrap_or(here)
            }
            MovementProfile::Flank => candidates()
                .min_by_key(|h| (h.distance(tpos), -(h.r - tpos.r).abs(), h.q, h.r))
                .unwrap_or(here),
            MovementProfile::Swarm => match self.nearest_enemy(i) {
                Some(e) => {
                    let ep = self.units[e].pos;
                    candidates().min_by_key(|h| h.distance(ep)).unwrap_or(here)
                }
                None => here,
            },
            MovementProfile::Kite => match self.nearest_enemy(i) {
                Some(e) => {
                    let ep = self.units[e].pos;
                    candidates().max_by_key(|h| (h.distance(ep), -h.q, -h.r)).unwrap_or(here)
                }
                None => here,
            },
            MovementProfile::Disperse => match self.nearest_ally(i) {
                Some(a) => {
                    let ap = self.units[a].pos;
                    candidates().max_by_key(|h| (h.distance(ap), -h.q, -h.r)).unwrap_or(here)
                }
                None => here,
            },
        }
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
        // Weapon base + the attacker's composed damage bonus (an implant combat-stim).
        let base = atk.damage + self.units[attacker].damage_bonus();
        let src = self.units[attacker].id.0;
        for t in self.footprint_targets(attacker, target, atk) {
            let mult =
                armor::matrix(atk.dtype, self.units[t].armor_class) * self.units[t].vuln_mult();
            let dmg = base * mult;
            self.units[t].character.apply_pool_damage(self.tick, src, dmg, atk.pen, true);
            if atk.emp && self.units[t].is_alive() {
                self.apply_emp(t);
            }
        }
    }

    /// The living units an attack strikes (§7G). `Single` is just the target;
    /// `Blast` is the disc around the target hex; `Beam` is the line from the attacker
    /// along the bearing to the target. AoE includes **allies** (friendly fire) — only
    /// the attacker is spared. Returned in ascending index order (deterministic).
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
        (0..self.units.len())
            .filter(|&j| {
                j != attacker && self.units[j].is_alive() && hexes.contains(&self.units[j].pos)
            })
            .collect()
    }

    /// An **EMP** pulse on `target` (§7I) — a *physical* breach that **bypasses
    /// Firewall** (no roll): fries **every** active implant (→ Offline) and fires
    /// its **degrade-class** liabilities at a fixed magnitude. The stun-class
    /// knockout is the hacker's finesse — EMP is blunt. Flesh / bioware (no chrome)
    /// are immune, and the more implants a target runs, the more an EMP ruins.
    fn apply_emp(&mut self, target: usize) {
        for idx in self.units[target].active_implant_indices() {
            for spec in self.units[target].disable_implant(idx) {
                if !matches!(spec.effect, Effect::Stun) {
                    self.units[target].add_status(spec, EMP_MAGNITUDE, EMP_MAGNITUDE);
                }
            }
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
                        let mult = armor::matrix(dtype, self.units[t].armor_class)
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

    /// Resolve a netrunning hack from `attacker` onto `target` (§7F, §10.8): roll
    /// `3d6 + avg(Hacking, channel)` vs the target's `Firewall` (the TN, §13),
    /// where the **channel** is the weaker endpoint's Link bandwidth (`min`). The
    /// target's Link feeds the channel, not the wall, so a darker target is harder
    /// to hack; defense is the Firewall alone. Zero Link stays the hard
    /// reachability gate. Lands the hack's payload (margin-scaled) on success.
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
        // The connection runs at the weaker endpoint's bandwidth (the channel);
        // the rating averages the attacker's Hacking with it. Firewall is the TN.
        let channel = self.units[attacker].digital_band().min(self.units[target].digital_band());
        let rating = hack_rating(self.units[attacker].skill(Skill::Hacking), channel)
            + self.units[attacker].mesh_synergy(); // meshed PAN throughput (§5)
        let tn = self.units[target].firewall();
        let outcome = resolve_contest(&mut self.rng, Contest::new(rating, 0, tn));
        let stacks =
            if outcome.success { self.apply_breach(target, &outcome, hack) } else { 0 };
        HackResult::Rolled { outcome, stacks }
    }

    /// Apply a successful hack's consequences — the §6 **severity ladder**
    /// (`docs/cyberware.md`): breach a target implant (**disable** floor →
    /// margin-scaled **degrade** → crit **knockout**), firing its `hack_effects`
    /// per effect. If the target carries no chrome to trip, land the deck's own
    /// payload instead (a generic intrusion). Returns the magnitude landed.
    fn apply_breach(&mut self, target: usize, outcome: &RollOutcome, hack: Hack) -> u32 {
        let Some(first) = self.units[target].first_active_implant() else {
            // No chrome to trip — run the deck's own payload.
            let stacks = hack.stacks_for(outcome);
            if stacks > 0 {
                self.units[target].add_status(hack.payload, hack.duration, stacks);
            }
            return stacks;
        };
        // Cascade (§5): a crit on a **meshed** PAN rides the net to *every*
        // implant; a segmented PAN contains it to the one slot.
        let slots = if outcome.crit && self.units[target].pan == Pan::Meshed {
            self.units[target].active_implant_indices()
        } else {
            vec![first]
        };
        let degrade = hack::margin_stacks(outcome.margin);
        for idx in slots {
            // Floor: disable the implant; then the ladder per liability.
            for spec in self.units[target].disable_implant(idx) {
                if matches!(spec.effect, Effect::Stun) {
                    // Knockout class — crit-gated (a decisive hack only).
                    if outcome.crit {
                        self.units[target].add_status(spec, KNOCKOUT_STUN, 1);
                    }
                } else if degrade > 0 {
                    // Degrade class — magnified by the margin.
                    self.units[target].add_status(spec, degrade, degrade);
                }
            }
        }
        degrade
    }

    /// Nearest enemy with a digital surface (Link > 0) within the unit's antenna
    /// range — the hack's target selection.
    fn nearest_hackable_enemy(&self, i: usize) -> Option<usize> {
        let me = &self.units[i];
        let range = me.hack().map_or(0, |h| h.range);
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
            .min_by_key(|(_, u)| (me.pos.distance(u.pos), u.id))
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
        u.grant_hack(Hack::new(range, StatusSpec::lockware(), 1, 5));
        u
    }

    /// A unit with a hackable digital surface: Link > 0 and a Firewall TN.
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
        u.character.base_mut().immunity = 30.0; // resist TN beyond any 3d6 + power roll
        let mut b = Battle::new(vec![u], 7);
        b.units[0].add_status(StatusSpec::poison(), 5, 1);
        let before = b.units[0].character.integrity;
        for _ in 0..20 {
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
        u.character.base_mut().immunity = 5.0; // low resist TN
        u.add_status(StatusSpec::poison(), 5, 1);
        // 3d6 = 6, + poison power 3 + 1 stack = 10 ≥ TN 5 ⇒ fires.
        let mut b = Battle::with_rng(vec![u], ScriptedRng::from_d6([2, 2, 2]));
        let before = b.units[0].character.integrity;
        b.status_phase();
        assert!(b.units[0].character.integrity < before);
    }

    #[test]
    fn injected_scripted_rng_forces_poison_to_whiff() {
        let mut u = unit(0, Team::A, 0);
        u.character.base_mut().immunity = 30.0; // resist TN out of reach
        u.add_status(StatusSpec::poison(), 5, 1);
        // 3d6 = 6, + power + stack = 10 < TN 30 ⇒ whiffs.
        let mut b = Battle::with_rng(vec![u], ScriptedRng::from_d6([2, 2, 2]));
        let before = b.units[0].character.integrity;
        b.status_phase();
        assert_eq!(b.units[0].character.integrity, before);
    }

    #[test]
    fn unit_contest_uses_its_skill() {
        let u = unit(0, Team::A, 0); // Augmented baseline Melee = 1
        let mut rng = ScriptedRng::from_d6([4, 4, 3]); // 3d6 = 11
        let o = u.contest(Skill::Melee, 0, 12, &mut rng); // 11 + 1 = 12 vs TN 12 ⇒ success
        assert_eq!(o.total, 12);
        assert!(o.success);
    }

    #[test]
    fn raising_a_skill_flips_a_contest() {
        let mut u = unit(0, Team::A, 0); // Melee 1
        let before = u.contest(Skill::Melee, 0, 12, &mut ScriptedRng::from_d6([3, 3, 3])); // 9+1=10 < 12
        u.skills.raise(Skill::Melee, 3); // → 4
        let after = u.contest(Skill::Melee, 0, 12, &mut ScriptedRng::from_d6([3, 3, 3])); // 9+4=13 ≥ 12
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
        let obj = Hold { hex, by_round: 3 };
        assert_eq!(obj.status(&held, 1, false), ObjectiveStatus::Pending); // holds, but too early
        assert_eq!(obj.status(&held, 3, false), ObjectiveStatus::Achieved); // held to the round
        let mut cleared = held.clone();
        cleared[1].character.alive = false; // enemy gone → captured immediately
        assert_eq!(obj.status(&cleared, 1, false), ObjectiveStatus::Achieved);
        let away = vec![unit(0, Team::A, 0), unit(1, Team::B, 5)]; // player not on the hex
        assert_eq!(obj.status(&away, 9, true), ObjectiveStatus::Failed); // fight over, never held
    }

    #[test]
    fn hold_is_not_held_while_contested() {
        let hex = Hex::new(2, 0);
        let mut both = vec![unit(0, Team::A, 2), unit(1, Team::B, 2)];
        both[0].pos = hex;
        both[1].pos = hex; // an enemy contests the hex
        assert_eq!(Hold { hex, by_round: 1 }.status(&both, 5, false), ObjectiveStatus::Pending);
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
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4])); // 12
        let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
        assert_eq!(outcome.total, 12 + 3);
    }

    #[test]
    fn a_thin_runner_link_bottlenecks_the_channel() {
        // Mirror: attacker Link 2, target Link 6 → channel 2; same rating 3.
        let mut atk = runner(0, Team::A, 0, 1);
        atk.character.base_mut().link = 2.0;
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = networked(1, Team::B, 0, 0);
        tgt.character.base_mut().link = 6.0;
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4])); // 12
        let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
        assert_eq!(outcome.total, 12 + 3); // channel min(2, 6) = 2
    }

    #[test]
    fn a_darker_target_is_harder_to_hack() {
        // Same runner; only the target's Link (the channel) changes.
        let total_vs = |target_link: i32| {
            let mut atk = runner(0, Team::A, 0, 1);
            atk.character.base_mut().link = 6.0;
            atk.skills.set(Skill::Hacking, 6);
            let mut tgt = networked(1, Team::B, 0, 0);
            tgt.character.base_mut().link = target_link as f32;
            let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4])); // 12
            let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
            outcome.total
        };
        // Fat channel (Link 6): avg(6, 6) = 6. Dark (Link 1): avg(6, 1) = 3.
        assert!(total_vs(6) > total_vs(1));
        assert_eq!(total_vs(6), 12 + 6);
        assert_eq!(total_vs(1), 12 + 3);
    }

    #[test]
    fn firewall_is_the_full_target_number() {
        // Defense is the Firewall wall in full — target Link feeds the channel,
        // not the TN (if Link capped the TN here it would be 4, not 12).
        let mut atk = runner(0, Team::A, 0, 1);
        atk.character.base_mut().link = 4.0;
        atk.skills.set(Skill::Hacking, 6);
        let mut tgt = networked(1, Team::B, 0, 12);
        tgt.character.base_mut().link = 4.0;
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4])); // 12
        let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
        // channel min(4, 4) = 4; rating avg(6, 4) = 5; total 17 vs full Firewall 12.
        assert_eq!(outcome.total, 17);
        assert_eq!(outcome.margin, 17 - 12);
        assert!(outcome.success);
    }

    #[test]
    fn margin_scales_the_landed_stacks() {
        let mut atk = runner(0, Team::A, 0, 1);
        atk.character.base_mut().link = 4.0;
        atk.skills.set(Skill::Hacking, 6); // channel min(4, 4) = 4 → rating avg(6, 4) = 5
        let mut tgt = networked(1, Team::B, 0, 2);
        tgt.character.base_mut().link = 4.0;
        // 3d6 = 9, + rating 5 = 14 vs Firewall 2 → margin 12 → 1 + 12/3 = 5 stacks.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 3]));
        assert!(b.resolve_hack(0, 1).landed());
        assert_eq!(b.units[1].statuses()[0].0, "Lockware");
        assert_eq!(b.units[1].statuses()[0].1, 5);
    }

    #[test]
    fn digital_phase_hacks_the_nearest_reachable_enemy() {
        let mut atk = runner(0, Team::A, 0, 4); // antenna range 4
        atk.character.base_mut().link = 4.0;
        atk.skills.set(Skill::Hacking, 4);
        let mut near = networked(1, Team::B, 2, 2); // distance 2 ≤ range 4
        near.character.base_mut().link = 4.0;
        let mut far = networked(2, Team::B, 9, 2); // out of range
        far.character.base_mut().link = 4.0;
        let mut b = Battle::with_rng(vec![atk, near, far], ScriptedRng::from_d6([4, 4, 4]));
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
        tgt.install(Implant::subdermal_plating());
        tgt.install(Implant::reflex_booster());
        assert_eq!(tgt.pan, Pan::Meshed); // the default
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([6, 6, 6])); // crit
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
        tgt.install(Implant::subdermal_plating()); // idx 0 — the targeted slot
        tgt.install(Implant::reflex_booster()); // idx 1 — contained
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([6, 6, 6])); // crit
        b.resolve_hack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline);
        assert_eq!(b.units[1].implant_condition(1), Condition::Online); // contained
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
            let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4]));
            let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
            outcome.total
        };
        assert!(total_for(Pan::Meshed) > total_for(Pan::Segmented));
    }

    #[test]
    fn emp_fries_all_chrome_bypassing_firewall() {
        let mut tgt = unit(1, Team::B, 0);
        tgt.character.base_mut().firewall = 99.0; // EMP ignores the wall entirely
        tgt.install(Implant::subdermal_plating()); // +6 Plating, Shed liability
        tgt.install(Implant::cyberdeck()); // Link 5, grants hack, Lockout liability
        assert!(tgt.hack().is_some());
        let mut atk = unit(0, Team::A, 0);
        atk.rearm(|w| w.emp = true);
        // Empty RNG: an EMP rolls nothing (a physical pulse, not a contest).
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        b.resolve_attack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline);
        assert_eq!(b.units[1].implant_condition(1), Condition::Offline);
        assert!(b.units[1].hack().is_none()); // deck bricked
        assert_eq!(b.units[1].character.plating, 0.0); // plating benefit fried
    }

    #[test]
    fn emp_disables_but_does_not_knock_out() {
        // EMP is blunt — it fires degrade-class liabilities but not the stun.
        let mut tgt = unit(1, Team::B, 0);
        tgt.install(Implant::reflex_booster()); // Seizure (stun) liability
        let mut atk = unit(0, Team::A, 0);
        atk.rearm(|w| w.emp = true);
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
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
        atk.skills.set(Skill::Hacking, 6);
        let mut tgt = unit(1, Team::B, 1);
        tgt.character.base_mut().link = 4.0;
        tgt.character.base_mut().firewall = 4.0; // big margin, no crit
        tgt.install(Implant::combat_stim());
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 3]));
        b.resolve_hack(0, 1);
        let has_dot = b.units[1].statuses().iter().any(|(n, _)| *n == "Bleed");
        let has_stun = b.units[1].is_stunned();
        assert!(has_dot && !has_stun);
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
        // Margin 0 success → the §6 floor: disable only, no liability fired.
        let mut atk = runner(0, Team::A, 0, 4); // Link 3
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = unit(1, Team::B, 1);
        tgt.character.base_mut().link = 2.0;
        tgt.install(Implant::reflex_booster()); // Seizure liability (stun)
        tgt.character.base_mut().firewall = 12.0; // channel min(3,2)=2 → rating avg(4,2)=3; dice 9 → 12 = TN, margin 0
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 3]));
        assert!(b.resolve_hack(0, 1).landed());
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline); // disabled
        assert!(b.units[1].statuses().is_empty()); // nothing fired (margin 0, no crit)
    }

    #[test]
    fn a_solid_hack_disables_and_fires_the_degrade_liability() {
        // Strong margin → disable + the degrade-class hack-effect, margin-scaled.
        let mut atk = runner(0, Team::A, 0, 4); // Link 3
        atk.skills.set(Skill::Hacking, 6);
        let mut tgt = unit(1, Team::B, 1);
        tgt.character.base_mut().link = 4.0;
        tgt.install(Implant::subdermal_plating()); // Shed = Corrode (not stun)
        tgt.character.base_mut().firewall = 4.0; // channel min(3,4)=3 → rating avg(6,3)=4; dice 9 → 13 vs 4, margin 9
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 3]));
        b.resolve_hack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline);
        assert_eq!(b.units[1].statuses()[0].0, "Corrode"); // Shed fired
        assert_eq!(b.units[1].statuses()[0].1, 3); // margin 9 / 3
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
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([6, 6, 6])); // nat 18
        b.resolve_hack(0, 1);
        assert_eq!(b.units[1].implant_condition(0), Condition::Offline);
        assert!(b.units[1].is_stunned());
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
        tgt.character.base_mut().firewall = 4.0;
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 3]));
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
        let mut tgt = networked(1, Team::B, 1, 14); // base Firewall 14
        tgt.character.base_mut().link = 4.0;
        // Debuff Firewall by 4 → effective 10; the loop reads firewall() through the gen.
        tgt.apply_modifier(Decorator::gear(Tag::Debuff, vec![Factor::add(Stat::Firewall, -4.0)]));
        // 3d6 = 8, + rating 4 = 12: misses base 14, but beats the debuffed 10.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 2]));
        assert!(b.resolve_hack(0, 1).landed());
    }

    #[test]
    fn an_installed_deck_lets_a_unit_hack_in_the_digital_phase() {
        let mut atk = unit(0, Team::A, 0);
        atk.skills.set(Skill::Hacking, 4);
        atk.install(Implant::cyberdeck()); // grants hack + Link 5
        let tgt = networked(1, Team::B, 1, 9); // soft target, in deck range 6
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4]));
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

        // Kite: step to keep distance from the nearest enemy.
        let kiter = unit(0, Team::A, 3).with_movement(MovementProfile::Kite);
        let foe = unit(1, Team::B, 4); // adjacent (distance 1)
        let b2 = Battle::new(vec![kiter, foe], 1);
        let step = b2.movement_step(0, 1);
        assert!(step.distance(Hex::new(4, 0)) > 1); // backed off
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
            dtype: DamageType::Piercing,
            pen: PenTier::Internal,
            range,
            min_range,
            emp: false,
            footprint: Footprint::Single,
        }
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
        atk.set_weapon(gun(12.0, 2, 6));
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
