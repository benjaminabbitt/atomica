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
//! - [`hack`]ing — the netrunning digital attack (`3d6 + Hacking + deck` vs
//!   Firewall, Link-gated, §7F/§13);
//! - an initiative-ordered tick loop with a minimal "attack nearest / step toward"
//!   resolution plus a digital pass.
//!
//! Not yet built: the two contagion families (a spreading special case of
//! statuses), IFF/spoof, and Heat.

pub mod armor;
mod hack;
mod hex;
mod objective;
mod rng;
mod roll;
mod skills;
mod status;

pub use armor::ArmorClass;
pub use hack::{Hack, HackResult};
pub use hex::Hex;
pub use objective::{
    Goal, MarginLoss, Objective, ObjectiveStatus, Objectives, Reach, Survive, TimeAttack, WinFight,
    PLAYER,
};
pub use rng::{RandomSource, ScriptedRng, SplitMix64};
pub use roll::{resolve_contest, Contest, RollOutcome};
pub use skills::{Chassis, Skill, Skills};
pub use status::{
    Behavior, Decay, Effect, Magnitude, Resist, Stacking, Status, StatusSpec, Targeting, Timing,
    Trigger,
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

/// The external defense layers that sit in front of Integrity.
#[derive(Clone, Copy, Debug, Default)]
pub struct Defense {
    /// Barrier/Shield — the outermost (External) layer.
    pub barrier: f32,
    /// Plating/Armor — the Contact layer.
    pub plating: f32,
}

/// A single attack profile. (Weapons/loadouts will compose these later.)
#[derive(Clone, Copy, Debug)]
pub struct Attack {
    pub damage: f32,
    pub dtype: DamageType,
    pub pen: PenTier,
    /// Reach in hexes (1 = melee/adjacent).
    pub range: i32,
}

/// A combatant. The stat line mirrors the design's "Unit anatomy".
#[derive(Clone, Debug)]
pub struct Unit {
    pub id: UnitId,
    pub name: String,
    pub team: Team,
    pub pos: Hex,

    /// The single HP pool — all damage ultimately reduces it.
    pub integrity: f32,
    pub max_integrity: f32,
    pub defense: Defense,
    /// Armor class for the [`armor`] matrix.
    pub armor_class: ArmorClass,
    /// Innate class — sets the skill floor, contagion exposure, etc. (§7J).
    pub chassis: Chassis,
    /// Per-character skill levels (chassis baseline + earned) — roll modifiers (§10).
    pub skills: Skills,

    /// Physical Initiative — turn order in the world (higher acts first).
    pub initiative: f32,
    /// Digital Initiative / net presence. `0.0` ⇒ immune to all digital attack.
    pub link: f32,
    /// Resist vs Worm + hacks — the Target Number a digital stochastic roll must
    /// beat (`3d6 + power` vs this), §13.
    pub firewall: i32,
    /// Resist vs Virus — the Target Number a bio stochastic roll must beat, §13.
    pub immunity: i32,

    pub attack: Attack,
    /// Optional netrunning loadout — the digital action this unit takes on its
    /// turn (§7F). `None` ⇒ no deck (a pure physical fighter).
    pub hack: Option<Hack>,
    /// Active à-la-carte statuses.
    pub statuses: Vec<Status>,
    pub alive: bool,
}

impl Unit {
    pub fn is_alive(&self) -> bool {
        self.alive && self.integrity > 0.0
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

    /// Apply a status, honoring its stacking axis (merge with any same-named one).
    pub fn add_status(&mut self, spec: StatusSpec, duration: u32, stacks: u32) {
        if let Some(existing) = self.statuses.iter_mut().find(|s| s.spec.name == spec.name) {
            match spec.stacking {
                Stacking::Refresh => existing.duration = existing.duration.max(duration),
                Stacking::Stack { max } => {
                    existing.stacks = (existing.stacks + stacks).min(max);
                    existing.duration = existing.duration.max(duration);
                }
            }
        } else {
            self.statuses.push(Status { spec, stacks, duration });
        }
    }

    fn is_stunned(&self) -> bool {
        self.statuses.iter().any(|s| matches!(s.spec.effect, Effect::Stun))
    }

    /// Initiative after Lag-style slows.
    fn effective_initiative(&self) -> f32 {
        let mut init = self.initiative;
        for s in &self.statuses {
            if let Effect::Slow(f) = s.spec.effect {
                init *= f;
            }
        }
        init
    }

    /// Incoming-damage multiplier from Breach-style vulnerabilities.
    fn vuln_mult(&self) -> f32 {
        let mut m = 1.0;
        for s in &self.statuses {
            if let Effect::Vuln(f) = s.spec.effect {
                m *= f;
            }
        }
        m
    }
}

/// The Target Number a stochastic status rolls against (its `resist` axis, §13).
fn resist_tn(unit: &Unit, resist: Resist) -> i32 {
    match resist {
        Resist::None => 0,
        Resist::Immunity => unit.immunity,
        Resist::Firewall => unit.firewall,
    }
}

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
    withdrawn: bool,
}

impl Battle<SplitMix64> {
    /// Build a battle with the production RNG seeded by `seed`.
    pub fn new(units: Vec<Unit>, seed: u64) -> Self {
        Self::with_rng(units, SplitMix64::new(seed))
    }
}

impl<R: RandomSource> Battle<R> {
    /// Build a battle over any [`RandomSource`] — inject a `ScriptedRng` in tests.
    /// Defaults to the [`Eliminate`] objective.
    pub fn with_rng(units: Vec<Unit>, rng: R) -> Self {
        let objectives = Objectives::new(vec![Goal::new(Box::new(WinFight), 1, 1)]);
        Self { units, tick: 0, rng, objectives, withdrawn: false }
    }

    /// Withdraw from the Flight: forfeit it (objectives resolve as fight-over),
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
    /// standard fight). A Flight can carry any number; each is scored independently.
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

    /// Has the Flight ended — one army wiped, or the player withdrew?
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
        self.action_phase();
        self.digital_phase();
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

    fn status_phase(&mut self) {
        for i in 0..self.units.len() {
            if !self.units[i].is_alive() {
                continue;
            }
            // Take the list out so we can mutate the unit while iterating it.
            let statuses = std::mem::take(&mut self.units[i].statuses);
            for st in &statuses {
                let fires = match st.spec.behavior {
                    Behavior::Deterministic => true,
                    Behavior::Stochastic { power } => {
                        // 3d6 + power + stacks vs the target's resist TN (§13).
                        let tn = resist_tn(&self.units[i], st.spec.resist);
                        let skill = power + st.stacks as i32;
                        resolve_contest(&mut self.rng, Contest::new(skill, 0, tn)).success
                    }
                };
                if !fires {
                    continue;
                }
                match st.spec.effect {
                    Effect::Dot { magnitude, pen } => {
                        let amt = magnitude.amount(&self.units[i]) * st.stacks as f32;
                        apply_damage(&mut self.units[i], amt, pen, magnitude.can_kill());
                    }
                    Effect::PlatingShred(mag) => {
                        let amt = mag.amount(&self.units[i]) * st.stacks as f32;
                        let p = &mut self.units[i].defense.plating;
                        *p = (*p - amt).max(0.0);
                    }
                    // Stun / Slow / Vuln are passive modifiers, read in other phases.
                    Effect::Stun | Effect::Slow(_) | Effect::Vuln(_) => {}
                }
                if !self.units[i].is_alive() {
                    break;
                }
            }
            self.units[i].statuses = statuses;
        }
    }

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
            let Some(target) = self.nearest_enemy(i) else {
                continue;
            };
            let (pos, atk) = {
                let u = &self.units[i];
                (u.pos, u.attack)
            };
            let tpos = self.units[target].pos;
            if pos.distance(tpos) <= atk.range {
                self.resolve_attack(i, target);
            } else {
                self.units[i].pos = pos.step_toward(tpos);
            }
        }
    }

    fn decay_phase(&mut self) {
        for u in &mut self.units {
            for st in &mut u.statuses {
                match st.spec.decay {
                    Decay::Duration => st.duration = st.duration.saturating_sub(1),
                    Decay::Stacks => st.stacks = st.stacks.saturating_sub(1),
                }
            }
            u.statuses.retain(|st| match st.spec.decay {
                Decay::Duration => st.duration > 0,
                Decay::Stacks => st.stacks > 0,
            });
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

    fn resolve_attack(&mut self, attacker: usize, target: usize) {
        let atk = self.units[attacker].attack;
        // Armor matrix (type vs class) × Breach vulnerability.
        let mult = armor::matrix(atk.dtype, self.units[target].armor_class)
            * self.units[target].vuln_mult();
        let dmg = atk.damage * mult;
        apply_damage(&mut self.units[target], dmg, atk.pen, true);
    }

    /// The digital activation pass (§10.3/§10.8): every unit with a hack and net
    /// presence (Link > 0) acts in **digital-initiative = Link** order, hacking
    /// the nearest reachable enemy. A unit frozen by a Crash/Seizure stun is off
    /// the net too. *(First pass: a discrete phase after the physical one; the
    /// design's fully interleaved physical+digital order is a later step.)*
    fn digital_phase(&mut self) {
        let mut order: Vec<usize> = (0..self.units.len())
            .filter(|&i| {
                let u = &self.units[i];
                u.is_alive() && u.hack.is_some() && u.link > 0.0
            })
            .collect();
        order.sort_by(|&a, &b| {
            let (ua, ub) = (&self.units[a], &self.units[b]);
            ub.link
                .partial_cmp(&ua.link)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(ua.id.cmp(&ub.id))
        });

        for i in order {
            if !self.units[i].is_alive() || self.units[i].is_stunned() {
                continue;
            }
            if let Some(target) = self.nearest_hackable_enemy(i) {
                self.resolve_hack(i, target);
            }
        }
    }

    /// Resolve a netrunning hack from `attacker` onto `target` (§7F, §10.8): roll
    /// `3d6 + Hacking + deck` vs the target's Firewall (the TN, §13), gated by
    /// Link on both ends, landing the hack's payload (margin-scaled) on success.
    pub fn resolve_hack(&mut self, attacker: usize, target: usize) -> HackResult {
        let Some(hack) = self.units[attacker].hack else {
            return HackResult::NoHack;
        };
        // Link gate (§7D/§7F): a runner needs net presence; the target a surface.
        if self.units[attacker].link <= 0.0 {
            return HackResult::Offline;
        }
        if self.units[target].link <= 0.0 {
            return HackResult::NoSurface;
        }
        let skill = self.units[attacker].skill(Skill::Hacking);
        let tn = self.units[target].firewall;
        let outcome = resolve_contest(&mut self.rng, Contest::new(skill, hack.power, tn));
        let stacks = hack.stacks_for(&outcome);
        if stacks > 0 {
            self.units[target].add_status(hack.payload, hack.duration, stacks);
        }
        HackResult::Rolled { outcome, stacks }
    }

    /// Nearest enemy with a digital surface (Link > 0) within the unit's antenna
    /// range — the hack's target selection.
    fn nearest_hackable_enemy(&self, i: usize) -> Option<usize> {
        let me = &self.units[i];
        let range = me.hack.map_or(0, |h| h.range);
        self.units
            .iter()
            .enumerate()
            .filter(|(j, u)| {
                *j != i
                    && u.is_alive()
                    && u.team == me.team.enemy()
                    && u.link > 0.0
                    && me.pos.distance(u.pos) <= range
            })
            .min_by_key(|(_, u)| (me.pos.distance(u.pos), u.id))
            .map(|(j, _)| j)
    }
}

/// Route `amount` through the defense layers selected by `pen`, spilling any
/// remainder inward. `can_kill == false` (the PctCurrent "softener") floors
/// Integrity at 1.0 instead of dropping the unit.
fn apply_damage(unit: &mut Unit, amount: f32, pen: PenTier, can_kill: bool) {
    let mut remaining = amount;
    if matches!(pen, PenTier::External) {
        remaining = absorb(&mut unit.defense.barrier, remaining);
    }
    if matches!(pen, PenTier::External | PenTier::Contact) {
        remaining = absorb(&mut unit.defense.plating, remaining);
    }
    unit.integrity -= remaining;
    if unit.integrity <= 0.0 {
        if can_kill {
            unit.integrity = 0.0;
            unit.alive = false;
        } else {
            unit.integrity = 1.0;
        }
    }
}

/// Subtract from a layer, returning the overflow that passes through it.
fn absorb(layer: &mut f32, amount: f32) -> f32 {
    let soaked = layer.min(amount);
    *layer -= soaked;
    amount - soaked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(id: u32, team: Team, q: i32) -> Unit {
        Unit {
            id: UnitId(id),
            name: format!("U{id}"),
            team,
            pos: Hex::new(q, 0),
            integrity: 30.0,
            max_integrity: 30.0,
            defense: Defense::default(),
            armor_class: ArmorClass::Mail,
            chassis: Chassis::Augmented,
            skills: Chassis::Augmented.baseline_skills(),
            initiative: 5.0,
            link: 0.0,
            firewall: 0,
            immunity: 0,
            attack: Attack {
                damage: 10.0,
                dtype: DamageType::Piercing,
                pen: PenTier::Internal,
                range: 1,
            },
            hack: None,
            statuses: Vec::new(),
            alive: true,
        }
    }

    /// A unit wired to hack: net presence + a Lockware deck (`power`, `range`).
    fn runner(id: u32, team: Team, q: i32, power: i32, range: i32) -> Unit {
        let mut u = unit(id, team, q);
        u.link = 3.0;
        u.hack = Some(Hack::new(power, range, StatusSpec::lockware(), 1, 5));
        u
    }

    /// A unit with a hackable digital surface: Link > 0 and a Firewall TN.
    fn networked(id: u32, team: Team, q: i32, firewall: i32) -> Unit {
        let mut u = unit(id, team, q);
        u.link = 2.0;
        u.firewall = firewall;
        u
    }

    fn duel() -> Battle {
        let mut a = unit(0, Team::A, 0);
        a.initiative = 6.0;
        let b = unit(1, Team::B, 3);
        Battle::new(vec![a, b], 123)
    }

    #[test]
    fn layers_absorb_then_integrity() {
        let mut u = unit(0, Team::A, 0);
        u.defense = Defense { barrier: 5.0, plating: 5.0 };
        apply_damage(&mut u, 12.0, PenTier::External, true);
        assert_eq!(u.integrity, 28.0); // 5 + 5 soaked, 2 through
        assert_eq!(u.defense.barrier, 0.0);
        assert_eq!(u.defense.plating, 0.0);
    }

    #[test]
    fn internal_bypasses_layers() {
        let mut u = unit(0, Team::A, 0);
        u.defense = Defense { barrier: 99.0, plating: 99.0 };
        apply_damage(&mut u, 10.0, PenTier::Internal, true);
        assert_eq!(u.integrity, 20.0);
        assert_eq!(u.defense.barrier, 99.0);
    }

    #[test]
    fn softener_never_kills() {
        let mut u = unit(0, Team::A, 0);
        apply_damage(&mut u, 9999.0, PenTier::Internal, false);
        assert_eq!(u.integrity, 1.0);
        assert!(u.alive);
    }

    #[test]
    fn burn_dot_ticks_down_integrity() {
        let mut b = Battle::new(vec![unit(0, Team::A, 0)], 1);
        b.units[0].add_status(StatusSpec::burn(), 3, 2); // 2 stacks × 2 dmg, Contact
        let before = b.units[0].integrity;
        b.status_phase();
        // No plating ⇒ full 4 reaches Integrity.
        assert_eq!(b.units[0].integrity, before - 4.0);
    }

    #[test]
    fn full_immunity_blocks_poison() {
        let mut u = unit(0, Team::A, 0);
        u.immunity = 30; // resist TN beyond any 3d6 + power roll
        let mut b = Battle::new(vec![u], 7);
        b.units[0].add_status(StatusSpec::poison(), 5, 1);
        let before = b.units[0].integrity;
        for _ in 0..20 {
            b.status_phase();
        }
        assert_eq!(b.units[0].integrity, before);
    }

    #[test]
    fn breach_amplifies_incoming_damage() {
        let attacker = unit(0, Team::A, 0);
        let mut target = unit(1, Team::B, 0); // same hex ⇒ in melee range
        target.add_status(StatusSpec::breach(), 3, 1); // ×1.5
        let mut b = Battle::new(vec![attacker, target], 1);
        // Piercing vs Mail = 1.0, so 10 base × 1.5 breach = 15.
        b.resolve_attack(0, 1);
        assert_eq!(b.units[1].integrity, 30.0 - 15.0);
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
            assert_eq!(a.integrity, b.integrity);
            assert_eq!(a.pos, b.pos);
        }
    }

    #[test]
    fn injected_scripted_rng_forces_poison_to_fire() {
        let mut u = unit(0, Team::A, 0);
        u.immunity = 5; // low resist TN
        u.add_status(StatusSpec::poison(), 5, 1);
        // 3d6 = 6, + poison power 3 + 1 stack = 10 ≥ TN 5 ⇒ fires.
        let mut b = Battle::with_rng(vec![u], ScriptedRng::from_d6([2, 2, 2]));
        let before = b.units[0].integrity;
        b.status_phase();
        assert!(b.units[0].integrity < before);
    }

    #[test]
    fn injected_scripted_rng_forces_poison_to_whiff() {
        let mut u = unit(0, Team::A, 0);
        u.immunity = 30; // resist TN out of reach
        u.add_status(StatusSpec::poison(), 5, 1);
        // 3d6 = 6, + power + stack = 10 < TN 30 ⇒ whiffs.
        let mut b = Battle::with_rng(vec![u], ScriptedRng::from_d6([2, 2, 2]));
        let before = b.units[0].integrity;
        b.status_phase();
        assert_eq!(b.units[0].integrity, before);
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
        lost[0].alive = false; // player wiped
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
        dead[0].alive = false;
        assert_eq!(obj.status(&dead, 1, false), ObjectiveStatus::Failed);
    }

    #[test]
    fn margin_loss_is_a_close_defeat() {
        let obj = MarginLoss { max_enemy_survivors: 2 };
        let both = vec![unit(0, Team::A, 0), unit(1, Team::B, 1)];
        assert_eq!(obj.status(&both, 5, false), ObjectiveStatus::Pending); // lose first
        let mut close = vec![unit(0, Team::A, 0), unit(1, Team::B, 1), unit(2, Team::B, 2)];
        close[0].alive = false; // player down, 2 enemies left ≤ 2
        assert_eq!(obj.status(&close, 9, true), ObjectiveStatus::Achieved);
        let mut blown =
            vec![unit(0, Team::A, 0), unit(1, Team::B, 1), unit(2, Team::B, 2), unit(3, Team::B, 3)];
        blown[0].alive = false; // 3 enemies left > 2 → lost too badly
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
        us[0].alive = false; // player wiped → WinFight Failed
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
        let atk = runner(0, Team::A, 0, 5, 1);
        let mut tgt = networked(1, Team::B, 0, 0);
        tgt.link = 0.0; // air-gapped — no surface to reach
                        // Empty RNG: a roll here would panic, proving the gate short-circuits.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        assert_eq!(b.resolve_hack(0, 1), HackResult::NoSurface);
        assert!(b.units[1].statuses.is_empty());
    }

    #[test]
    fn offline_attacker_cannot_hack() {
        let mut atk = runner(0, Team::A, 0, 5, 1);
        atk.link = 0.0; // dark — no presence to reach with
        let tgt = networked(1, Team::B, 0, 8);
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        assert_eq!(b.resolve_hack(0, 1), HackResult::Offline);
        assert!(b.units[1].statuses.is_empty());
    }

    #[test]
    fn hack_lands_against_a_beatable_firewall() {
        let atk = runner(0, Team::A, 0, 3, 1); // Augmented Hacking baseline = 1
        let tgt = networked(1, Team::B, 0, 12);
        // 3d6 = 12, + skill 1 + deck 3 = 16 vs Firewall 12 → margin 4, lands.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4]));
        let r = b.resolve_hack(0, 1);
        assert!(r.landed());
        assert_eq!(b.units[1].statuses.len(), 1);
        assert_eq!(b.units[1].statuses[0].spec.name, "Lockware");
        assert_eq!(b.units[1].statuses[0].stacks, 2); // base 1 + margin 4/3
    }

    #[test]
    fn hack_whiffs_against_a_hard_firewall() {
        let atk = runner(0, Team::A, 0, 3, 1);
        let tgt = networked(1, Team::B, 0, 20);
        // 16 < Firewall 20 → whiff; nothing lands.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4]));
        assert!(!b.resolve_hack(0, 1).landed());
        assert!(b.units[1].statuses.is_empty());
    }

    #[test]
    fn digital_phase_hacks_the_nearest_reachable_enemy() {
        let atk = runner(0, Team::A, 0, 3, 4);
        let near = networked(1, Team::B, 2, 10); // distance 2 ≤ antenna range 4
        let far = networked(2, Team::B, 9, 10); // out of range
        let mut b = Battle::with_rng(vec![atk, near, far], ScriptedRng::from_d6([4, 4, 4]));
        b.digital_phase();
        assert!(!b.units[1].statuses.is_empty()); // near got hacked
        assert!(b.units[2].statuses.is_empty()); // far one untouched (out of range)
    }

    #[test]
    fn a_stunned_runner_skips_its_hack() {
        let mut atk = runner(0, Team::A, 0, 3, 4);
        atk.add_status(StatusSpec::crash(), 1, 1); // Seizure freezes the net action too
        let tgt = networked(1, Team::B, 1, 10);
        // Empty RNG: a stunned runner must not roll.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        b.digital_phase();
        assert!(b.units[1].statuses.is_empty());
    }

    #[test]
    fn hacking_resolution_is_deterministic() {
        let setup = || {
            Battle::new(vec![runner(0, Team::A, 0, 3, 4), networked(1, Team::B, 2, 11)], 99)
        };
        let mut x = setup();
        let mut y = setup();
        for _ in 0..10 {
            x.step();
            y.step();
        }
        for (a, b) in x.units.iter().zip(&y.units) {
            assert_eq!(a.integrity, b.integrity);
            assert_eq!(a.statuses.len(), b.statuses.len());
        }
    }
}
