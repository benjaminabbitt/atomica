//! Character architecture — generators (`chargen`), modifiers, composition
//! (`docs/layers.md`, **L1**).
//!
//! A [`Character`] **wraps the gen** (an ordered set of [`Decorator`]s — the
//! persistent modifier state) **and owns the live pools** (current Integrity /
//! Barrier / Plating). It exposes [`Character::realize`] — the composed view,
//! produced on demand by running the decorators over the `base` into a flat,
//! referenceable [`Modifier`] set whose **accessors fold the factors** (§2 bucket
//! model) to answer each query. Reads split:
//!
//! - composed stats / behavior → `character.realize().link()` (through the fold);
//! - live pools → straight off the `Character` (`character.integrity`).
//!
//! Mutation (install / expire / breach / a DoT) acts on **the gen**; the next
//! `realize()` reflects it (composed fresh each call — the dirty-flag cache of §4 is
//! a pure optimization, omitted while the fold is cheap). HP loss is **not** a
//! modifier: it's a clamped, threshold-latched pool, and damage is a
//! [`DamageEvent`] against it (§3c), carrying attribution as telemetry — never a
//! second source of truth.
//!
//! This is the architecture the cyberware fold (`docs/cyberware.md`) and the status
//! pool refactor *onto* (L2 / L2b); it coexists with the current `Unit` fold until
//! those land.

use crate::{resolve_contest, Contest, Hex, MovementProfile, PenTier, RandomSource, TargetingProfile};

/// A decorator's stable handle. A [`Modifier`]'s `source` links back to the
/// decorator that spawned it, so removing/expiring the decorator drops exactly its
/// modifiers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GenId(pub u32);

/// Where a decorator sits in the **priority-ordered gen** (§2). The gen is kept
/// sorted ascending by `(priority, install-order)`, so composition is
/// priority-ordered regardless of *install* order: the **highest-priority
/// `Override` wins** (corruption outranks gear), while numeric factors fold
/// order-independently. Equal priorities keep install order.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Priority(pub i32);

impl Priority {
    /// Worn / installed gear and implants — the baseline.
    pub const GEAR: Priority = Priority(0);
    /// Transient buffs / debuffs — over gear.
    pub const BUFF: Priority = Priority(10);
    /// Hostile overrides (spoof / Lockware) — outrank everything, so the enemy's
    /// hack of your script wins however your loadout happens to be ordered.
    pub const CORRUPTION: Priority = Priority(100);
}

impl Default for Priority {
    fn default() -> Self {
        Priority::GEAR
    }
}

/// The numeric stats a [`Factor`] can compose. Behavior (targeting / movement) is
/// not numeric — it composes via [`Override`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Stat {
    Link,
    Firewall,
    Immunity,
    Initiative,
    MaxIntegrity,
    Plating,
    Barrier,
    Damage,
}

/// How a [`Factor`] combines with its peers — the **bucket** model (§2). `Add` and
/// `Increased` are additive-safe (runaway-proof); `More` genuinely multiplies and is
/// the only runaway risk (off by default, rare + capped if ever used).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FactorKind {
    /// Flat add, summed: `Σadd`. The default — most gear.
    Add,
    /// Additive percent, **summed** then applied once: `1 + Σincreased`.
    Increased,
    /// Multiplicative percent, producted: `Π(1 + moreᵢ)`. The only runaway.
    More,
}

/// A numeric modifier on one [`Stat`] (§2). `value` is a flat amount for `Add`, a
/// fraction (`0.2` = +20%) for `Increased` / `More`.
#[derive(Clone, Copy, Debug)]
pub struct Factor {
    pub stat: Stat,
    pub kind: FactorKind,
    pub value: f32,
}

impl Factor {
    pub fn add(stat: Stat, value: f32) -> Self {
        Self { stat, kind: FactorKind::Add, value }
    }
    pub fn increased(stat: Stat, value: f32) -> Self {
        Self { stat, kind: FactorKind::Increased, value }
    }
    pub fn more(stat: Stat, value: f32) -> Self {
        Self { stat, kind: FactorKind::More, value }
    }
}

/// A non-numeric modifier on **behavior** — the **last `Override` wins** (a spoof
/// *replaces* targeting; not a number). The substrate for "the enemy hacks your
/// script".
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Override {
    Targeting(TargetingProfile),
    Movement(MovementProfile),
}

/// A **capability** a decorator grants — not a number on the stat line but a whole
/// action the character can now take (§1 "capability"). A cyberdeck grants a
/// [`Hack`]; breach the deck (or take it Offline) and the capability drops with it.
/// The **highest-priority** active grant wins.
#[derive(Clone, Copy, Debug)]
pub enum Capability {
    /// The netrunning loadout (`docs/netrunning.md`) — granted by a deck.
    Hack(crate::Hack),
}

/// A non-numeric passive **flag** a status imposes — read by other phases, not a
/// number on the stat line. `Stun` gates the owner's action (Crash / Seizure);
/// `Vuln(f)` multiplies **incoming** damage (Breach). The passive face of the status
/// effects that aren't DoTs; `Slow` instead folds in as a `More` factor on Initiative
/// (Lag), so it isn't here.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Flag {
    Stun,
    Vuln(f32),
}

/// A modifier's category, for matching on removal (a cleanse strips `Virus`-tagged).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tag {
    Gear,
    Implant,
    Buff,
    Debuff,
    Virus,
    Worm,
    Spoof,
}

/// What a [`Modifier`] carries — a numeric [`Factor`], a behavior [`Override`], or a
/// passive [`Flag`].
#[derive(Clone, Copy, Debug)]
pub enum ModifierKind {
    Factor(Factor),
    Override(Override),
    Flag(Flag),
}

/// The **standard interface** every modifying component shares (§1). Its `source` is
/// the durable referent — the [`Decorator`] that spawned it — so the realized set is
/// referenceable / removable **by decorator** (`source`) or category (`tag`). The
/// modifier itself is a transient projection, regenerated each `realize`, so it
/// carries no id of its own.
#[derive(Clone, Copy, Debug)]
pub struct Modifier {
    pub source: GenId,
    pub tag: Tag,
    pub kind: ModifierKind,
}

/// A decorator's lifecycle (§1). On expiry the decorator is dropped and its
/// modifiers go with it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Expiration {
    /// Gear / augments — never expires on its own.
    Permanent,
    /// Lasts `n` more ticks (a buff / debuff); decremented by [`Character::decay`].
    Duration(u32),
}

/// A selector for **removing** other components' modifiers — the counterplay
/// substrate (§1). A `Vaccinated` decorator carries `Remove::Tag(Virus)`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Remove {
    /// Strip every modifier with this tag.
    Tag(Tag),
    /// Strip every modifier spawned by this decorator.
    Source(GenId),
}

impl Remove {
    fn matches(self, m: &Modifier) -> bool {
        match self {
            Remove::Tag(t) => m.tag == t,
            Remove::Source(g) => m.source == g,
        }
    }
}

/// A battle event a decorator's **active face** reacts to (taxonomy §6.6 trigger
/// set). Dispatched by [`Character::dispatch`].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Event {
    /// Start of the owner's activation — DoTs / contagion ticks fire here.
    TickStart,
    TickEnd,
    /// The owner was hit (on-hit riders).
    OnHit,
    /// The owner died (death triggers).
    OnDeath,
}

/// How a reaction's magnitude is computed from the owner — the status `magnitude`
/// axis. `PctCurrent` is the **softener** (never kills); resolved against the owner's
/// pools at dispatch, per stack.
#[derive(Clone, Copy, Debug)]
pub enum Amount {
    Flat(f32),
    PctMax(f32),
    PctCurrent(f32),
}

/// What a decorator does on an [`Event`] — its reaction *template*, scaled by the
/// decorator's `stacks` at dispatch.
#[derive(Clone, Copy, Debug)]
pub enum HookEffect {
    /// Damage the owner's pools (a DoT) through a penetration tier.
    Damage { amount: Amount, pen: PenTier, can_kill: bool },
    /// Shred the owner's Plating pool (Corrode).
    ShredPlating { amount: Amount },
}

/// An event reaction carried by a decorator: when `event` fires, apply `effect`.
#[derive(Clone, Copy, Debug)]
pub struct Hook {
    pub event: Event,
    pub effect: HookEffect,
}

/// A concrete reaction produced by [`Character::dispatch`] and applied to the pools —
/// returned for telemetry / tests (the §3c stream's active-face sibling). `source`
/// links it to the decorator that fired it.
#[derive(Clone, Copy, Debug)]
pub enum Reaction {
    Damage { amount: f32, pen: PenTier, can_kill: bool, source: GenId },
    ShredPlating { amount: f32, source: GenId },
}

/// Which of the owner's resist stats a **stochastic** gate rolls against (the status
/// `resist` axis).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Resist {
    #[default]
    None,
    Firewall,
    Immunity,
}

/// A **stochastic gate** (the status `behavior` axis): each tick the decorator rolls
/// `3d6 + power + stacks` vs the owner's [`Resist`] TN, and fires its hooks **only on
/// success**. Absent ⇒ deterministic (always fires).
#[derive(Clone, Copy, Debug)]
pub struct Gate {
    pub power: i32,
    pub resist: Resist,
}

/// How a decorator wears off — the decorator-lifetime form of the status `decay`
/// axis (named `Wear` to stay distinct from [`crate::Decay`]).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wear {
    /// Never (gear / implants).
    None,
    /// Lose one tick of `Duration` per decay step.
    ByDuration,
    /// Lose one `stack` per decay step; drop at zero.
    ByStacks,
}

/// A decorator's **condition ladder** (`docs/cyberware.md` §6, layers.md L5): the
/// 4-state benefit gate `Online → Degraded → Offline → Destroyed`. It scales every
/// numeric factor the decorator adds (and gates its overrides / grant / ward / hooks).
/// Gear, buffs and statuses sit at `Online`; only chrome wears down it. **Destroyed**
/// is terminal — a repair can't bring it back.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Condition {
    #[default]
    Online,
    Degraded,
    Offline,
    Destroyed,
}

impl Condition {
    /// Does it deliver (some of) its benefit now? (Online or Degraded.)
    pub fn is_active(self) -> bool {
        matches!(self, Condition::Online | Condition::Degraded)
    }

    /// Fraction delivered now: Online full, **Degraded half**, Offline / Destroyed none.
    pub fn benefit_factor(self) -> f32 {
        match self {
            Condition::Online => 1.0,
            Condition::Degraded => 0.5,
            Condition::Offline | Condition::Destroyed => 0.0,
        }
    }

    /// One step down the wear ladder (Destroyed is terminal).
    pub fn degraded(self) -> Condition {
        match self {
            Condition::Online => Condition::Degraded,
            Condition::Degraded => Condition::Offline,
            Condition::Offline | Condition::Destroyed => Condition::Destroyed,
        }
    }
}

/// A **character generator** (`chargen`) — a stateful decorator. Its **passive face**
/// is the modifiers it contributes (`factors` · `overrides` · `flags`); its **active
/// face** is its lifecycle (`expiration` / `decay` / `stacks`), the modifiers it
/// **removes** (a standing ward), and the **event `on`-hooks** it reacts through.
/// Static gear is a `Permanent` decorator with no reactions; a status is a decaying
/// one that hooks `TickStart`.
///
/// Built without an `id`; [`Character::install`] stamps one on insertion.
#[derive(Clone, Debug)]
pub struct Decorator {
    pub tag: Tag,
    /// Slot in the priority-ordered gen — higher wins overrides (default
    /// [`Priority::GEAR`]).
    pub priority: Priority,
    /// The [`Condition`] ladder gating its benefit (Online / Degraded / Offline /
    /// Destroyed). Scales every numeric factor and gates overrides / grant / ward /
    /// hooks; at `Offline`+ the decorator contributes **nothing** without losing its
    /// identity (`id`/`source`) — so a breach can degrade then repair it. Gear / buffs
    /// / statuses stay `Online`.
    pub condition: Condition,
    /// Mutable self-state: how many stacks the decorator holds — scales its event
    /// reactions and (under [`Wear::ByStacks`]) is its lifetime.
    pub stacks: u32,
    pub expiration: Expiration,
    /// How it wears off (status `decay` axis).
    pub decay: Wear,
    pub factors: Vec<Factor>,
    pub overrides: Vec<Override>,
    /// Passive flags it imposes (Stun / Vuln).
    pub flags: Vec<Flag>,
    /// A capability this decorator grants (a deck's [`Hack`]). Gated by `scale > 0`.
    pub grants: Option<Capability>,
    /// Event reactions — the active face (a DoT hooks `TickStart`).
    pub on: Vec<Hook>,
    /// A stochastic roll gating the hooks each dispatch (Poison); `None` ⇒ deterministic.
    pub gate: Option<Gate>,
    /// Modifiers this decorator strips from the set — a **standing ward**, applied
    /// order-independently (cleanse, counter-spoof, Ripperdoc).
    pub removes: Vec<Remove>,
    /// Set on install — its own [`GenId`], stamped onto every modifier it spawns.
    pub id: GenId,
}

impl Decorator {
    /// A permanent piece of gear contributing `factors`.
    pub fn gear(tag: Tag, factors: Vec<Factor>) -> Self {
        Self {
            tag,
            priority: Priority::GEAR,
            condition: Condition::Online,
            stacks: 1,
            expiration: Expiration::Permanent,
            decay: Wear::None,
            factors,
            overrides: Vec::new(),
            flags: Vec::new(),
            grants: None,
            on: Vec::new(),
            gate: None,
            removes: Vec::new(),
            id: GenId(0),
        }
    }

    /// A timed buff/debuff contributing `factors` for `turns` ticks (decays by
    /// duration).
    pub fn timed(tag: Tag, turns: u32, factors: Vec<Factor>) -> Self {
        Self {
            priority: Priority::BUFF,
            expiration: Expiration::Duration(turns),
            decay: Wear::ByDuration,
            ..Self::gear(tag, factors)
        }
    }

    /// A bare **status** decorator — `stacks` copies, decaying by `decay`, with no
    /// passive factors of its own (add them with the builders / hooks). The
    /// status-pool port (`docs/layers.md` L2b); see [`StatusSpec::to_decorator`].
    pub fn status(tag: Tag, stacks: u32, expiration: Expiration, decay: Wear) -> Self {
        Self { stacks, expiration, decay, ..Self::gear(tag, Vec::new()) }
    }

    /// Builder: add a behavior override (a smartgun, a spoof).
    pub fn with_override(mut self, o: Override) -> Self {
        self.overrides.push(o);
        self
    }

    /// Builder: impose a passive [`Flag`] (Stun / Vuln).
    pub fn with_flag(mut self, f: Flag) -> Self {
        self.flags.push(f);
        self
    }

    /// Builder: react to an [`Event`] with a [`HookEffect`] (a DoT on `TickStart`).
    pub fn with_hook(mut self, event: Event, effect: HookEffect) -> Self {
        self.on.push(Hook { event, effect });
        self
    }

    /// Builder: make the hooks **stochastic** — gated by a `3d6 + power` roll vs the
    /// owner's [`Resist`] each dispatch (Poison).
    pub fn with_gate(mut self, power: i32, resist: Resist) -> Self {
        self.gate = Some(Gate { power, resist });
        self
    }

    /// Builder: this decorator is a standing ward — it strips matching modifiers
    /// from the set, order-independently.
    pub fn with_remove(mut self, r: Remove) -> Self {
        self.removes.push(r);
        self
    }

    /// Builder: set composition priority (higher wins overrides; default
    /// [`Priority::GEAR`]). A spoof uses [`Priority::CORRUPTION`].
    pub fn with_priority(mut self, p: Priority) -> Self {
        self.priority = p;
        self
    }

    /// Builder: grant a [`Capability`] (a deck's [`Hack`]).
    pub fn with_grant(mut self, cap: Capability) -> Self {
        self.grants = Some(cap);
        self
    }

    /// Builder: set the initial [`Condition`] (default `Online`).
    pub fn with_condition(mut self, condition: Condition) -> Self {
        self.condition = condition;
        self
    }

    /// Does the decorator deliver (some of) its benefit right now? (Online / Degraded.)
    pub fn is_active(&self) -> bool {
        self.condition.is_active()
    }

    /// The benefit fraction it currently delivers (the condition's factor).
    pub fn benefit(&self) -> f32 {
        self.condition.benefit_factor()
    }

    /// Has it worn off — duration ran out, or stack-decay emptied it?
    pub fn is_expired(&self) -> bool {
        matches!(self.expiration, Expiration::Duration(0))
            || (self.decay == Wear::ByStacks && self.stacks == 0)
    }
}

/// The chassis innate stat line — the **base** every generator decorates. Generators
/// only ever *add factors* to it; they never carry the query surface themselves.
#[derive(Clone, Copy, Debug, Default)]
pub struct BaseLine {
    pub link: f32,
    pub firewall: f32,
    pub immunity: f32,
    pub initiative: f32,
    pub max_integrity: f32,
    pub plating: f32,
    pub barrier: f32,
    pub damage: f32,
    pub targeting: TargetingProfile,
    pub movement: MovementProfile,
}

impl BaseLine {
    fn of(&self, stat: Stat) -> f32 {
        match stat {
            Stat::Link => self.link,
            Stat::Firewall => self.firewall,
            Stat::Immunity => self.immunity,
            Stat::Initiative => self.initiative,
            Stat::MaxIntegrity => self.max_integrity,
            Stat::Plating => self.plating,
            Stat::Barrier => self.barrier,
            Stat::Damage => self.damage,
        }
    }
}

/// The composed view (§4): the flat, referenceable modifier set produced by running
/// the decorators over the base, plus the accessors that fold it. Produced on demand
/// by [`Character::realize`] and cached behind the `Character`'s dirty flag.
#[derive(Clone, Debug)]
pub struct Realized {
    base: BaseLine,
    mods: Vec<Modifier>,
    capability: Option<Capability>,
}

impl Realized {
    /// Fold every [`Factor`] on `stat` through the bucket model (§2):
    /// `(base + Σadd) × (1 + Σincreased) × Π(1 + moreᵢ)`.
    pub fn stat(&self, stat: Stat) -> f32 {
        let mut add = 0.0;
        let mut increased = 0.0;
        let mut more = 1.0;
        for m in &self.mods {
            if let ModifierKind::Factor(f) = m.kind {
                if f.stat == stat {
                    match f.kind {
                        FactorKind::Add => add += f.value,
                        FactorKind::Increased => increased += f.value,
                        FactorKind::More => more *= 1.0 + f.value,
                    }
                }
            }
        }
        (self.base.of(stat) + add) * (1.0 + increased) * more
    }

    /// The **last** override on `pick`'s axis wins, else the base (§2).
    fn last_override(&self) -> impl Iterator<Item = Override> + '_ {
        self.mods.iter().rev().filter_map(|m| match m.kind {
            ModifierKind::Override(o) => Some(o),
            _ => None,
        })
    }

    pub fn link(&self) -> i32 {
        self.stat(Stat::Link).round() as i32
    }
    pub fn firewall(&self) -> i32 {
        self.stat(Stat::Firewall).round() as i32
    }
    pub fn immunity(&self) -> i32 {
        self.stat(Stat::Immunity).round() as i32
    }
    pub fn initiative(&self) -> f32 {
        self.stat(Stat::Initiative)
    }
    pub fn max_integrity(&self) -> f32 {
        self.stat(Stat::MaxIntegrity)
    }
    pub fn plating(&self) -> f32 {
        self.stat(Stat::Plating)
    }
    pub fn barrier(&self) -> f32 {
        self.stat(Stat::Barrier)
    }
    pub fn damage(&self) -> f32 {
        self.stat(Stat::Damage)
    }

    pub fn targeting(&self) -> TargetingProfile {
        self.last_override()
            .find_map(|o| match o {
                Override::Targeting(t) => Some(t),
                _ => None,
            })
            .unwrap_or(self.base.targeting)
    }
    pub fn movement(&self) -> MovementProfile {
        self.last_override()
            .find_map(|o| match o {
                Override::Movement(m) => Some(m),
                _ => None,
            })
            .unwrap_or(self.base.movement)
    }

    /// The netrunning loadout this character can run, if any active decorator grants
    /// one (the highest-priority deck wins). `None` ⇒ no deck ⇒ can't hack.
    pub fn hack(&self) -> Option<crate::Hack> {
        self.capability.map(|Capability::Hack(h)| h)
    }

    /// Is the character **stunned** (a Crash / Seizure present)? Read by the action
    /// phase to skip its activation.
    pub fn stunned(&self) -> bool {
        self.mods.iter().any(|m| matches!(m.kind, ModifierKind::Flag(Flag::Stun)))
    }

    /// The **incoming-damage multiplier** from Breach-style `Vuln` flags — the product
    /// of every active vulnerability (`1.0` if none). Read where damage is applied.
    pub fn vuln(&self) -> f32 {
        self.mods
            .iter()
            .filter_map(|m| match m.kind {
                ModifierKind::Flag(Flag::Vuln(f)) => Some(f),
                _ => None,
            })
            .product()
    }

    /// The flat modifier set, for inspection / referencing (look-up by `source` or
    /// `tag`).
    pub fn modifiers(&self) -> &[Modifier] {
        &self.mods
    }
}

/// A single damage application against the Integrity pool (§3c) — emitted as
/// telemetry, **not** a modifier. Carries attribution (`source` = the dealing unit)
/// for kill credit / lifesteal / Data-spill; reactions read the stream within the
/// tick. The sim already replays from the seed, so this is never a second source of
/// truth.
#[derive(Clone, Copy, Debug)]
pub struct DamageEvent {
    pub tick: u32,
    /// The unit that dealt it (attribution).
    pub source: u32,
    pub amount: f32,
    /// Did this event cross Integrity to 0 (the killing blow)?
    pub lethal: bool,
}

/// A combatant under the layer architecture (§4): wraps the **gen** (ordered
/// decorators + base) and owns the **live pools**. Composed stats come from
/// [`Character::realize`]; pools are read/mutated directly.
#[derive(Clone, Debug)]
pub struct Character {
    base: BaseLine,
    gen: Vec<Decorator>,
    next_gen: u32,

    // --- live pools (path-dependent state, §3 — not composed) ---
    pub pos: Hex,
    pub integrity: f32,
    pub barrier: f32,
    pub plating: f32,
    pub alive: bool,

    // --- telemetry ---
    log: Vec<DamageEvent>,
}

impl Character {
    /// A fresh character on `base`, pools filled to the base maxima.
    pub fn new(base: BaseLine) -> Self {
        Self {
            base,
            gen: Vec::new(),
            next_gen: 0,
            pos: Hex::new(0, 0),
            integrity: base.max_integrity,
            barrier: base.barrier,
            plating: base.plating,
            alive: true,
            log: Vec::new(),
        }
    }

    // -- the gen: install / remove (each mutation dirties the realized cache) --

    /// Install a decorator, stamping it with a fresh [`GenId`]. Returns the id so the
    /// caller (or another component) can later reference / remove it.
    pub fn install(&mut self, mut dec: Decorator) -> GenId {
        let id = GenId(self.next_gen);
        self.next_gen += 1;
        dec.id = id;
        // The gen is a priority-ordered vec: insert by `(priority, id)`. Since `id`
        // increases monotonically, equal priorities keep install order, and the vec
        // stays sorted ascending — `realize` then composes low → high priority.
        let key = (dec.priority, id.0);
        let idx = self.gen.partition_point(|d| (d.priority, d.id.0) < key);
        self.gen.insert(idx, dec);
        id
    }

    /// Remove a decorator by its id (a deck going Offline, a buff dispelled). Its
    /// modifiers drop with it on the next `realize`.
    pub fn remove(&mut self, id: GenId) {
        self.gen.retain(|d| d.id != id);
    }

    /// Remove every decorator with `tag` (a mass cleanse by category).
    pub fn remove_where(&mut self, tag: Tag) {
        self.gen.retain(|d| d.tag != tag);
    }

    /// Set a decorator's [`Condition`] in place — the breach / degrade / repair path
    /// (`docs/cyberware.md` §6). Keeps the decorator's identity (`id`/`source`) so it
    /// can recover (unless Destroyed, which is terminal).
    pub fn set_condition(&mut self, id: GenId, condition: Condition) {
        if let Some(d) = self.gen.iter_mut().find(|d| d.id == id) {
            d.condition = condition;
        }
    }

    /// The current [`Condition`] of the decorator `id` (if present).
    pub fn condition_of(&self, id: GenId) -> Option<Condition> {
        self.gen.iter().find(|d| d.id == id).map(|d| d.condition)
    }

    /// Set the [`Condition`] of **every** decorator with `tag` — the gen-level op
    /// behind EMP (`condition_where(Implant, Offline)` fries all chrome) and a mass
    /// repair (`docs/cyberware.md` §5). Returns the affected ids.
    pub fn condition_where(&mut self, tag: Tag, condition: Condition) -> Vec<GenId> {
        let mut hit = Vec::new();
        for d in self.gen.iter_mut().filter(|d| d.tag == tag) {
            d.condition = condition;
            hit.push(d.id);
        }
        hit
    }

    /// The ids of all **active** decorators with `tag` (Cascade / mesh-synergy use this).
    pub fn active_ids(&self, tag: Tag) -> Vec<GenId> {
        self.gen.iter().filter(|d| d.tag == tag && d.is_active()).map(|d| d.id).collect()
    }

    /// One **decay step** (the cleanup phase, §1): wear every decorator by its
    /// `decay` axis — duration loses a tick, stack-decay loses a stack — then drop
    /// any that have worn off.
    pub fn decay(&mut self) {
        for d in &mut self.gen {
            match d.decay {
                Wear::None => {}
                Wear::ByDuration => {
                    if let Expiration::Duration(n) = &mut d.expiration {
                        *n = n.saturating_sub(1);
                    }
                }
                Wear::ByStacks => d.stacks = d.stacks.saturating_sub(1),
            }
        }
        self.gen.retain(|d| !d.is_expired());
    }

    // -- the active face: events (§1, L2b) --

    /// Dispatch a battle [`Event`] to every **active** decorator's `on`-hooks: each
    /// matching hook fires a [`Reaction`] (scaled by the decorator's `stacks`), which
    /// is applied to the pools and returned for telemetry. A two-pass split (gather,
    /// then apply) keeps the borrow clean and the order deterministic (gen order).
    pub fn dispatch<R: RandomSource>(
        &mut self,
        event: Event,
        tick: u32,
        rng: &mut R,
    ) -> Vec<Reaction> {
        // Resolve `Amount`s and resist TNs against a snapshot of the pre-dispatch
        // composed view (so every reaction this tick reads the same numbers).
        let view = self.realize();
        let (max, fw, imm) = (view.max_integrity(), view.firewall(), view.immunity());
        let current = self.integrity;
        let resolve = |a: Amount| match a {
            Amount::Flat(v) => v,
            Amount::PctMax(p) => p * max,
            Amount::PctCurrent(p) => p * current,
        };
        let mut reactions = Vec::new();
        for d in &self.gen {
            if !d.is_active() {
                continue;
            }
            // Stochastic gate (the status `behavior` axis): 3d6 + power + stacks vs the
            // owner's resist TN; on failure the decorator's hooks don't fire this tick.
            if let Some(gate) = d.gate {
                let tn = match gate.resist {
                    Resist::None => 0,
                    Resist::Firewall => fw,
                    Resist::Immunity => imm,
                };
                let skill = gate.power + d.stacks as i32;
                if !resolve_contest(rng, Contest::new(skill, 0, tn)).success {
                    continue;
                }
            }
            let stacks = d.stacks.max(1) as f32;
            for h in d.on.iter().filter(|h| h.event == event) {
                reactions.push(match h.effect {
                    HookEffect::Damage { amount, pen, can_kill } => Reaction::Damage {
                        amount: resolve(amount) * stacks,
                        pen,
                        can_kill,
                        source: d.id,
                    },
                    HookEffect::ShredPlating { amount } => Reaction::ShredPlating {
                        amount: resolve(amount) * stacks,
                        source: d.id,
                    },
                });
            }
        }
        for r in &reactions {
            match *r {
                Reaction::Damage { amount, pen, can_kill, source } => {
                    self.apply_pool_damage(tick, source.0, amount, pen, can_kill);
                    if !self.alive {
                        break; // dead mid-pass; the rest doesn't land
                    }
                }
                Reaction::ShredPlating { amount, .. } => {
                    self.plating = (self.plating - amount).max(0.0);
                }
            }
        }
        reactions
    }

    /// The gen, for inspection (look-up by id / tag).
    pub fn generators(&self) -> &[Decorator] {
        &self.gen
    }

    // -- realize: the composed view (composed fresh each call, §4) --

    /// The composed view — runs the decorators over the base into the referenceable
    /// modifier set. Reads go through it: `character.realize().link()`. Composed
    /// fresh each call; a dirty-flag cache (§4) is a pure optimization to add only if
    /// the fold ever shows up hot.
    pub fn realize(&self) -> Realized {
        self.realize_with_base(self.base)
    }

    /// Realize over a **supplied** base instead of the stored one (§0 "created on
    /// demand"): lets an owner whose authored base lives elsewhere (a `Unit`'s flat
    /// stat line, mid-migration) compose its modifiers without copying them in.
    pub fn realize_with_base(&self, base: BaseLine) -> Realized {
        // Passive face: contribute every **active** decorator's modifiers, in priority
        // order (low → high, so the last seen — highest priority — wins for `Override`
        // and the granted `Capability`). Numeric factors are scaled by the decorator's
        // benefit fraction (Degraded = half); an inactive decorator (scale 0) is gated
        // off entirely.
        let mut mods: Vec<Modifier> = Vec::new();
        let mut capability: Option<Capability> = None;
        for dec in &self.gen {
            if !dec.is_active() {
                continue;
            }
            for &f in &dec.factors {
                let scaled = Factor { value: f.value * dec.benefit(), ..f };
                mods.push(Modifier {
                    source: dec.id,
                    tag: dec.tag,
                    kind: ModifierKind::Factor(scaled),
                });
            }
            for &o in &dec.overrides {
                mods.push(Modifier {
                    source: dec.id,
                    tag: dec.tag,
                    kind: ModifierKind::Override(o),
                });
            }
            for &fl in &dec.flags {
                mods.push(Modifier {
                    source: dec.id,
                    tag: dec.tag,
                    kind: ModifierKind::Flag(fl),
                });
            }
            if let Some(cap) = dec.grants {
                capability = Some(cap);
            }
        }
        // Active face: standing wards. Each active decorator's `removes` strips
        // matching modifiers from the whole set — order-independent (a ward cleanses
        // whether the infection arrived before or after it), but never its own.
        for dec in &self.gen {
            if !dec.is_active() {
                continue;
            }
            for r in &dec.removes {
                mods.retain(|m| m.source == dec.id || !r.matches(m));
            }
        }
        Realized { base, mods, capability }
    }

    // -- the pools (live state, §3c) --

    /// Apply `amount` damage straight to Integrity (Internal): clamp at 0, latch death
    /// on the crossing, and record the [`DamageEvent`] (attribution `source`). Returns
    /// the event.
    pub fn apply_damage(&mut self, tick: u32, source: u32, amount: f32) -> DamageEvent {
        let before = self.integrity;
        self.integrity = (self.integrity - amount).max(0.0);
        let lethal = before > 0.0 && self.integrity <= 0.0;
        if lethal {
            self.alive = false;
        }
        let ev = DamageEvent { tick, source, amount, lethal };
        self.log.push(ev);
        ev
    }

    /// Apply `amount` damage routed through the **layers** by penetration tier
    /// (External → Barrier → Plating, Contact → Plating, Internal → straight through),
    /// then Integrity — the DoT path the active face feeds. `can_kill == false` is the
    /// **softener** floor: it can drag Integrity to `1`, never to a kill.
    pub fn apply_pool_damage(
        &mut self,
        tick: u32,
        source: u32,
        amount: f32,
        pen: PenTier,
        can_kill: bool,
    ) {
        let mut remaining = amount;
        if matches!(pen, PenTier::External) {
            remaining = absorb(&mut self.barrier, remaining);
        }
        if matches!(pen, PenTier::External | PenTier::Contact) {
            remaining = absorb(&mut self.plating, remaining);
        }
        if remaining <= 0.0 {
            return;
        }
        let before = self.integrity;
        let mut after = before - remaining;
        let mut lethal = false;
        if after <= 0.0 {
            if can_kill {
                after = 0.0;
                lethal = before > 0.0;
                self.alive = false;
            } else {
                after = 1.0_f32.min(before); // softener never kills
            }
        }
        self.integrity = after;
        self.log.push(DamageEvent { tick, source, amount: remaining, lethal });
    }

    /// Fill every pool to its current composed maximum — the **deploy / spawn** step
    /// (loadout complete, the character enters the fight at full). Distinct from a
    /// mid-battle max change, which never refills (§3a): you build the gen, *then*
    /// fill, *then* fight.
    pub fn fill(&mut self) {
        let r = self.realize();
        self.integrity = r.max_integrity();
        self.plating = r.plating();
        self.barrier = r.barrier();
    }

    /// Heal Integrity, clamped to the **current** composed max (over-heal is wasted).
    pub fn heal(&mut self, amount: f32) {
        let max = self.realize().max_integrity();
        self.integrity = (self.integrity + amount).min(max);
    }

    /// Re-clamp current Integrity to the composed max after a max change (§3a): a
    /// max **drop** chunks current; a max **rise** does **not** refill (pool
    /// semantics — repair restores capacity, not health).
    pub fn clamp_integrity(&mut self) {
        let max = self.realize().max_integrity();
        if self.integrity > max {
            self.integrity = max;
        }
    }

    /// The damage/heal telemetry stream (attribution / replay UI) — an output, never
    /// state.
    pub fn log(&self) -> &[DamageEvent] {
        &self.log
    }
}

/// Subtract from a layer pool, returning the overflow that passes through it.
fn absorb(layer: &mut f32, amount: f32) -> f32 {
    let soaked = layer.min(amount);
    *layer -= soaked;
    amount - soaked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> BaseLine {
        BaseLine {
            link: 4.0,
            firewall: 9.0,
            initiative: 5.0,
            max_integrity: 30.0,
            plating: 2.0,
            damage: 10.0,
            ..Default::default()
        }
    }

    #[test]
    fn empty_character_reads_base() {
        let c = Character::new(base());
        let r = c.realize();
        assert_eq!(r.link(), 4);
        assert_eq!(r.firewall(), 9);
        assert_eq!(r.max_integrity(), 30.0);
        assert_eq!(r.damage(), 10.0);
    }

    #[test]
    fn add_factors_fold_additively() {
        let mut c = Character::new(base());
        c.install(Decorator::gear(
            Tag::Implant,
            vec![Factor::add(Stat::Link, 5.0), Factor::add(Stat::Firewall, 2.0)],
        ));
        let r = c.realize();
        assert_eq!(r.link(), 9); // 4 + 5
        assert_eq!(r.firewall(), 11); // 9 + 2
    }

    #[test]
    fn two_decorators_sum_order_independently() {
        let mut a = Character::new(base());
        a.install(Decorator::gear(Tag::Implant, vec![Factor::add(Stat::Plating, 6.0)]));
        a.install(Decorator::gear(Tag::Gear, vec![Factor::add(Stat::Plating, 3.0)]));

        let mut b = Character::new(base());
        b.install(Decorator::gear(Tag::Gear, vec![Factor::add(Stat::Plating, 3.0)]));
        b.install(Decorator::gear(Tag::Implant, vec![Factor::add(Stat::Plating, 6.0)]));

        assert_eq!(a.realize().plating(), b.realize().plating());
        assert_eq!(a.realize().plating(), 11.0); // 2 + 6 + 3
    }

    #[test]
    fn increased_sums_then_applies_once() {
        let mut c = Character::new(base());
        // +20% and +30% increased damage → +50%, applied once (additive-safe).
        c.install(Decorator::gear(
            Tag::Buff,
            vec![
                Factor::increased(Stat::Damage, 0.2),
                Factor::increased(Stat::Damage, 0.3),
            ],
        ));
        assert_eq!(c.realize().damage(), 15.0); // 10 * 1.5
    }

    #[test]
    fn more_multiplies_separately() {
        let mut c = Character::new(base());
        // two +50% More → ×1.5 ×1.5 = ×2.25 (the runaway bucket).
        c.install(Decorator::gear(
            Tag::Buff,
            vec![Factor::more(Stat::Damage, 0.5), Factor::more(Stat::Damage, 0.5)],
        ));
        assert!((c.realize().damage() - 22.5).abs() < 1e-5);
    }

    #[test]
    fn add_increased_more_compose_in_order() {
        let mut c = Character::new(base());
        c.install(Decorator::gear(
            Tag::Gear,
            vec![
                Factor::add(Stat::Damage, 10.0),      // 10 + 10 = 20
                Factor::increased(Stat::Damage, 0.5), // ×1.5 = 30
                Factor::more(Stat::Damage, 0.2),      // ×1.2 = 36
            ],
        ));
        assert!((c.realize().damage() - 36.0).abs() < 1e-5);
    }

    #[test]
    fn override_last_wins() {
        let mut c = Character::new(base());
        assert_eq!(c.realize().targeting(), TargetingProfile::Nearest); // base
        c.install(
            Decorator::gear(Tag::Gear, vec![])
                .with_override(Override::Targeting(TargetingProfile::Backline)),
        );
        c.install(
            Decorator::timed(Tag::Spoof, 2, vec![])
                .with_override(Override::Targeting(TargetingProfile::WeakestArmor)),
        );
        // the later spoof wins.
        assert_eq!(c.realize().targeting(), TargetingProfile::WeakestArmor);
    }

    #[test]
    fn override_priority_beats_install_order() {
        let mut c = Character::new(base());
        // install the hostile spoof FIRST...
        c.install(
            Decorator::timed(Tag::Spoof, 2, vec![])
                .with_priority(Priority::CORRUPTION)
                .with_override(Override::Targeting(TargetingProfile::WeakestArmor)),
        );
        // ...then the smartgun gear AFTER (lower priority, later install).
        c.install(
            Decorator::gear(Tag::Gear, vec![])
                .with_override(Override::Targeting(TargetingProfile::Backline)),
        );
        // priority, not install order, decides: the corruption still wins.
        assert_eq!(c.realize().targeting(), TargetingProfile::WeakestArmor);
    }

    #[test]
    fn removing_a_decorator_drops_its_modifiers() {
        let mut c = Character::new(base());
        let deck = c.install(Decorator::gear(Tag::Implant, vec![Factor::add(Stat::Link, 5.0)]));
        assert_eq!(c.realize().link(), 9);
        c.remove(deck); // the deck goes Offline
        assert_eq!(c.realize().link(), 4); // back to base
    }

    #[test]
    fn standing_ward_cleanses_regardless_of_order() {
        // ward installed FIRST, infection AFTER — the standing ward still cleanses.
        let mut c = Character::new(base());
        c.install(Decorator::gear(Tag::Gear, vec![]).with_remove(Remove::Tag(Tag::Virus)));
        c.install(Decorator::timed(Tag::Virus, 3, vec![Factor::add(Stat::Firewall, -3.0)]));
        assert_eq!(c.realize().firewall(), 9); // cleansed despite arriving later

        // and the other order: infection first, ward after.
        let mut d = Character::new(base());
        d.install(Decorator::timed(Tag::Virus, 3, vec![Factor::add(Stat::Firewall, -3.0)]));
        assert_eq!(d.realize().firewall(), 6);
        d.install(Decorator::gear(Tag::Gear, vec![]).with_remove(Remove::Tag(Tag::Virus)));
        assert_eq!(d.realize().firewall(), 9);
    }

    #[test]
    fn remove_where_strips_by_tag() {
        let mut c = Character::new(base());
        c.install(Decorator::timed(Tag::Buff, 2, vec![Factor::add(Stat::Damage, 5.0)]));
        c.install(Decorator::timed(Tag::Buff, 2, vec![Factor::add(Stat::Damage, 5.0)]));
        c.install(Decorator::gear(Tag::Implant, vec![Factor::add(Stat::Damage, 2.0)]));
        assert_eq!(c.realize().damage(), 22.0);
        c.remove_where(Tag::Buff); // dispel all buffs
        assert_eq!(c.realize().damage(), 12.0); // only the implant remains
    }

    #[test]
    fn expiration_ticks_and_drops() {
        let mut c = Character::new(base());
        c.install(Decorator::timed(Tag::Buff, 2, vec![Factor::add(Stat::Damage, 6.0)]));
        assert_eq!(c.realize().damage(), 16.0);
        c.decay(); // 2 -> 1
        assert_eq!(c.realize().damage(), 16.0);
        c.decay(); // 1 -> 0, dropped
        assert_eq!(c.realize().damage(), 10.0);
    }

    #[test]
    fn modifiers_link_back_to_their_source() {
        let mut c = Character::new(base());
        let deck = c.install(Decorator::gear(Tag::Implant, vec![Factor::add(Stat::Link, 5.0)]));
        let r = c.realize();
        assert!(r.modifiers().iter().all(|m| m.source == deck));
        assert_eq!(r.modifiers().len(), 1);
    }

    // -- condition / scale (cyberware ladder, L1.1) --

    #[test]
    fn condition_scales_then_gates_factors_keeping_identity() {
        let mut c = Character::new(base()); // max_integrity 30
        let imp = c.install(Decorator::gear(
            Tag::Implant,
            vec![Factor::add(Stat::MaxIntegrity, 20.0)],
        ));
        assert_eq!(c.realize().max_integrity(), 50.0); // Online: full +20
        c.set_condition(imp, Condition::Degraded);
        assert_eq!(c.realize().max_integrity(), 40.0); // Degraded: +10
        c.set_condition(imp, Condition::Offline);
        assert_eq!(c.realize().max_integrity(), 30.0); // Offline: nothing
        c.set_condition(imp, Condition::Online);
        assert_eq!(c.realize().max_integrity(), 50.0); // repaired — same decorator
    }

    // -- capability grant (L1.1) --

    fn a_deck(range: i32) -> Capability {
        Capability::Hack(crate::Hack::new(range, crate::StatusSpec::lockware(), 1, range as u32))
    }

    #[test]
    fn deck_grants_hack_gated_by_condition() {
        let mut c = Character::new(base());
        assert!(c.realize().hack().is_none()); // no deck → can't hack
        let deck = c.install(
            Decorator::gear(Tag::Implant, vec![Factor::add(Stat::Link, 5.0)]).with_grant(a_deck(6)),
        );
        assert!(c.realize().hack().is_some()); // Online → can hack
        c.set_condition(deck, Condition::Offline); // breach
        assert!(c.realize().hack().is_none()); // hack drops with the deck
        c.set_condition(deck, Condition::Degraded); // Degraded still grants
        assert!(c.realize().hack().is_some());
        c.remove(deck);
        assert!(c.realize().hack().is_none()); // removed entirely
    }

    #[test]
    fn highest_priority_deck_grant_wins() {
        let mut c = Character::new(base());
        // a strong deck at BUFF priority, a weak one at GEAR.
        c.install(
            Decorator::gear(Tag::Implant, vec![])
                .with_grant(a_deck(9))
                .with_priority(Priority::BUFF),
        );
        c.install(Decorator::gear(Tag::Implant, vec![]).with_grant(a_deck(2)));
        assert_eq!(c.realize().hack().unwrap().range, 9); // priority decides
    }

    // -- stochastic gate (L5 stage B) --

    #[test]
    fn a_stochastic_gate_fires_only_on_a_passing_roll() {
        let mut c = Character::new(BaseLine { firewall: 15.0, max_integrity: 30.0, ..Default::default() });
        c.install(
            Decorator::status(Tag::Debuff, 1, Expiration::Duration(3), Wear::ByDuration)
                .with_hook(
                    Event::TickStart,
                    HookEffect::Damage {
                        amount: Amount::Flat(5.0),
                        pen: crate::PenTier::Internal,
                        can_kill: true,
                    },
                )
                .with_gate(0, Resist::Firewall),
        );
        // fumble (3d6 = 3) vs Firewall 15 → gated out, no reaction.
        let r = c.dispatch(Event::TickStart, 1, &mut crate::ScriptedRng::from_d6([1, 1, 1]));
        assert!(r.is_empty());
        assert_eq!(c.integrity, 30.0);
        // crit (3d6 = 18) → fires.
        let r = c.dispatch(Event::TickStart, 2, &mut crate::ScriptedRng::from_d6([6, 6, 6]));
        assert_eq!(r.len(), 1);
        assert_eq!(c.integrity, 25.0);
    }

    // -- pools (§3c) --

    #[test]
    fn damage_clamps_at_zero_and_latches_death() {
        let mut c = Character::new(base()); // 30 integrity
        let ev = c.apply_damage(1, 7, 12.0);
        assert_eq!(c.integrity, 18.0);
        assert!(!ev.lethal && c.alive);
        let ev = c.apply_damage(2, 7, 25.0);
        assert_eq!(c.integrity, 0.0);
        assert!(ev.lethal && !c.alive);
        // a later heal cannot un-fire death; the pool is latched.
        assert_eq!(c.log().len(), 2);
        assert_eq!(c.log()[1].source, 7);
    }

    #[test]
    fn heal_clamps_to_current_max() {
        let mut c = Character::new(base());
        c.apply_damage(1, 0, 10.0); // 30 -> 20
        c.heal(100.0); // overheal wasted
        assert_eq!(c.integrity, 30.0);
    }

    #[test]
    fn max_drop_chunks_current_but_max_rise_does_not_refill() {
        let mut c = Character::new(base());
        // +20 max from an implant (50 total); current still 30.
        let imp = c.install(Decorator::gear(
            Tag::Implant,
            vec![Factor::add(Stat::MaxIntegrity, 20.0)],
        ));
        assert_eq!(c.realize().max_integrity(), 50.0);
        assert_eq!(c.integrity, 30.0); // not auto-filled by the buffer

        c.heal(100.0);
        assert_eq!(c.integrity, 50.0); // now topped to the new max

        // breach the implant: max drops to 30, current chunks to 30.
        c.remove(imp);
        c.clamp_integrity();
        assert_eq!(c.realize().max_integrity(), 30.0);
        assert_eq!(c.integrity, 30.0);

        // repair: max back to 50, but current does NOT refill (capacity, not health).
        c.install(Decorator::gear(Tag::Implant, vec![Factor::add(Stat::MaxIntegrity, 20.0)]));
        c.clamp_integrity();
        assert_eq!(c.realize().max_integrity(), 50.0);
        assert_eq!(c.integrity, 30.0);
    }
}
