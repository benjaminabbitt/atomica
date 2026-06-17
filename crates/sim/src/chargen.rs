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

use crate::{Hex, MovementProfile, TargetingProfile};

/// A decorator's stable handle. A [`Modifier`]'s `source` links back to the
/// decorator that spawned it, so removing/expiring the decorator drops exactly its
/// modifiers.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct GenId(pub u32);

/// A modifier's own stable handle (referenceable within a realized set).
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct ModId(pub u32);

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

/// What a [`Modifier`] carries — a numeric [`Factor`] or a behavior [`Override`].
#[derive(Clone, Copy, Debug)]
pub enum ModifierKind {
    Factor(Factor),
    Override(Override),
}

/// The **standard interface** every modifying component shares (§1): identity so the
/// realized set can find / remove it. `source` links it to the [`Decorator`] that
/// spawned it.
#[derive(Clone, Copy, Debug)]
pub struct Modifier {
    pub id: ModId,
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
    /// Lasts `n` more ticks (a buff / debuff); decremented by
    /// [`Character::tick_expirations`].
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

/// A **character generator** (`chargen`) — a stateful decorator. Its **passive face**
/// is the modifiers it contributes (`factors` + `overrides`); its **active face** is
/// its lifecycle (`expiration`) plus the modifiers it **removes** from earlier
/// decorators (`removes`). Static gear is a `Permanent` decorator with no removes.
///
/// Built without an `id`; [`Character::install`] stamps one on insertion.
#[derive(Clone, Debug)]
pub struct Decorator {
    pub tag: Tag,
    pub expiration: Expiration,
    pub factors: Vec<Factor>,
    pub overrides: Vec<Override>,
    /// Modifiers this decorator strips from those applied **before** it (cleanse,
    /// counter-spoof, Ripperdoc).
    pub removes: Vec<Remove>,
    /// Set on install — its own [`GenId`], stamped onto every modifier it spawns.
    pub id: GenId,
}

impl Decorator {
    /// A permanent piece of gear contributing `factors`.
    pub fn gear(tag: Tag, factors: Vec<Factor>) -> Self {
        Self {
            tag,
            expiration: Expiration::Permanent,
            factors,
            overrides: Vec::new(),
            removes: Vec::new(),
            id: GenId(0),
        }
    }

    /// A timed buff/debuff contributing `factors` for `turns` ticks.
    pub fn timed(tag: Tag, turns: u32, factors: Vec<Factor>) -> Self {
        Self {
            tag,
            expiration: Expiration::Duration(turns),
            factors,
            overrides: Vec::new(),
            removes: Vec::new(),
            id: GenId(0),
        }
    }

    /// Builder: add a behavior override (a smartgun, a spoof).
    pub fn with_override(mut self, o: Override) -> Self {
        self.overrides.push(o);
        self
    }

    /// Builder: this decorator strips matching modifiers from earlier ones.
    pub fn with_remove(mut self, r: Remove) -> Self {
        self.removes.push(r);
        self
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
        self.gen.push(dec);
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

    /// Tick all `Duration` expirations down one; drop any that reach zero (§1).
    pub fn tick_expirations(&mut self) {
        for d in &mut self.gen {
            if let Expiration::Duration(n) = &mut d.expiration {
                *n = n.saturating_sub(1);
            }
        }
        self.gen.retain(|d| !matches!(d.expiration, Expiration::Duration(0)));
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
        let mut mods: Vec<Modifier> = Vec::new();
        let mut next_mod = 0u32;
        for dec in &self.gen {
            // active face: strip earlier modifiers this decorator counters.
            for r in &dec.removes {
                mods.retain(|m| !r.matches(m));
            }
            // passive face: contribute this decorator's modifiers.
            for &f in &dec.factors {
                mods.push(Modifier {
                    id: ModId(next_mod),
                    source: dec.id,
                    tag: dec.tag,
                    kind: ModifierKind::Factor(f),
                });
                next_mod += 1;
            }
            for &o in &dec.overrides {
                mods.push(Modifier {
                    id: ModId(next_mod),
                    source: dec.id,
                    tag: dec.tag,
                    kind: ModifierKind::Override(o),
                });
                next_mod += 1;
            }
        }
        Realized { base: self.base, mods }
    }

    // -- the pools (live state, §3c) --

    /// Apply `amount` damage to Integrity: clamp at 0, latch death on the crossing,
    /// and record the [`DamageEvent`] (attribution `source`). Returns the event.
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
        let mut c = Character::new(base());
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
    fn removing_a_decorator_drops_its_modifiers() {
        let mut c = Character::new(base());
        let deck = c.install(Decorator::gear(Tag::Implant, vec![Factor::add(Stat::Link, 5.0)]));
        assert_eq!(c.realize().link(), 9);
        c.remove(deck); // the deck goes Offline
        assert_eq!(c.realize().link(), 4); // back to base
    }

    #[test]
    fn cleanse_decorator_strips_earlier_tagged_modifiers() {
        let mut c = Character::new(base());
        // a Virus debuff: -3 firewall.
        c.install(Decorator::timed(Tag::Virus, 3, vec![Factor::add(Stat::Firewall, -3.0)]));
        assert_eq!(c.realize().firewall(), 6);
        // Antivirus: a decorator that removes Virus-tagged modifiers applied before it.
        c.install(Decorator::gear(Tag::Gear, vec![]).with_remove(Remove::Tag(Tag::Virus)));
        assert_eq!(c.realize().firewall(), 9); // cleansed
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
        c.tick_expirations(); // 2 -> 1
        assert_eq!(c.realize().damage(), 16.0);
        c.tick_expirations(); // 1 -> 0, dropped
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
