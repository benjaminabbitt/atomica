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
//! statuses), IFF/spoof, and Heat.

pub mod armor;
mod hack;
mod hex;
mod implant;
mod objective;
mod rng;
mod roll;
mod skills;
mod status;

pub use armor::ArmorClass;
pub use hack::{hack_rating, Hack, HackResult};
pub use hex::Hex;
pub use implant::{Condition, Contribution, Implant, Pan};
pub use objective::{
    Goal, Hold, MarginLoss, Objective, ObjectiveKind, ObjectiveStatus, Objectives, Reach, Survive,
    TimeAttack, WinFight, PLAYER,
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
    /// EMP weapon: a *physical* pulse that also fries the target's cyberware,
    /// **bypassing Firewall** (§7I) — the physical counter to digital builds.
    pub emp: bool,
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
    /// Digital Initiative / net presence (Link, §7D). Integer **bandwidth tiers**;
    /// `0` ⇒ immune to all digital attack. Feeds the hack channel + digital init.
    pub link: i32,
    /// Resist vs Worm + hacks — the Target Number a digital stochastic roll must
    /// beat (`3d6 + power` vs this), §13.
    pub firewall: i32,
    /// Resist vs Virus — the Target Number a bio stochastic roll must beat, §13.
    pub immunity: i32,

    pub attack: Attack,
    /// Optional netrunning loadout — the digital action this unit takes on its
    /// turn (§7F). `None` ⇒ no deck. Usually **granted by a cyberdeck implant**
    /// (folded in by [`Unit::install`]), not hand-set.
    pub hack: Option<Hack>,
    /// Installed cyberware (`docs/cyberware.md`). Each implant folds its
    /// [`Contribution`] into the stat line above while active; its liabilities
    /// fire on breach. The stats above are the derived (base + Σ active) line.
    pub implants: Vec<Implant>,
    /// The implant network mode (§5): meshed (synergy, Cascade-vulnerable) vs
    /// segmented (contained, no synergy). A loadout commitment.
    pub pan: Pan,
    /// Active à-la-carte statuses.
    pub statuses: Vec<Status>,
    pub alive: bool,
}

impl Unit {
    /// A bare combatant with placeholder stats (Integrity 30, a basic melee hit).
    /// Tune via the `with_*` builders or by field — the convenience constructor
    /// the run layer and content build rosters from.
    pub fn new(id: u32, name: impl Into<String>, team: Team, chassis: Chassis) -> Self {
        Self {
            id: UnitId(id),
            name: name.into(),
            team,
            pos: Hex::new(0, 0),
            integrity: 30.0,
            max_integrity: 30.0,
            defense: Defense::default(),
            armor_class: ArmorClass::default(),
            chassis,
            skills: chassis.baseline_skills(),
            initiative: 5.0,
            link: 0,
            firewall: 0,
            immunity: 0,
            attack: Attack {
                damage: 10.0,
                dtype: DamageType::Piercing,
                pen: PenTier::Internal,
                range: 1,
                emp: false,
            },
            hack: None,
            implants: Vec::new(),
            pan: Pan::Meshed,
            statuses: Vec::new(),
            alive: true,
        }
    }

    /// Builder: set the deploy position.
    pub fn at(mut self, pos: Hex) -> Self {
        self.pos = pos;
        self
    }

    /// Builder: set Integrity (and its max).
    pub fn with_integrity(mut self, hp: f32) -> Self {
        self.integrity = hp;
        self.max_integrity = hp;
        self
    }

    /// Builder: set the physical Initiative.
    pub fn with_initiative(mut self, initiative: f32) -> Self {
        self.initiative = initiative;
        self
    }

    /// Builder: set the attack profile.
    pub fn with_attack(mut self, attack: Attack) -> Self {
        self.attack = attack;
        self
    }

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

    /// Install `implant`, folding the benefit it delivers (scaled by its
    /// condition, §6) into the derived stat line and granting any deck loadout
    /// while active.
    pub fn install(&mut self, implant: Implant) {
        self.refold(&implant, 0.0, implant.condition.benefit_factor());
        if implant.condition.is_active() {
            if let Some(h) = implant.grant_hack {
                self.hack = Some(h);
            }
        }
        self.implants.push(implant);
    }

    /// Apply the *change* in an implant's delivered contribution between two
    /// condition factors. Integer stats use `round(new) − round(old)` (exact and
    /// reversible — no rounded-delta drift across half-steps); continuous stats
    /// scale linearly.
    fn refold(&mut self, implant: &Implant, old: f32, new: f32) {
        let c = implant.contribution;
        let at = |f: f32, x: i32| (f * x as f32).round() as i32;
        self.link += at(new, c.link) - at(old, c.link);
        self.firewall += at(new, c.firewall) - at(old, c.firewall);
        let d = new - old;
        self.defense.plating += d * c.plating;
        self.initiative += d * c.initiative;
        self.attack.damage += d * c.damage;
        self.max_integrity += d * c.max_integrity;
        if d > 0.0 {
            self.integrity += d * c.max_integrity; // gain the extra HP
        } else if c.max_integrity != 0.0 {
            self.integrity = self.integrity.min(self.max_integrity); // clamp on loss
        }
    }

    /// Move the implant at `idx` to `cond`, folding the change in delivered
    /// benefit and toggling its granted hack on the active boundary. Destroyed is
    /// terminal. Returns the implant's `hack_effects` iff this knocks it from
    /// active to inactive (a breach — the caller fires them per the §6 ladder).
    fn transition(&mut self, idx: usize, cond: Condition) -> Vec<StatusSpec> {
        let old = self.implants[idx].condition;
        if old == Condition::Destroyed {
            return Vec::new(); // terminal
        }
        let im = self.implants[idx].clone();
        self.refold(&im, old.benefit_factor(), cond.benefit_factor());
        if let Some(h) = im.grant_hack {
            self.hack = cond.is_active().then_some(h);
        }
        self.implants[idx].condition = cond;
        if old.is_active() && !cond.is_active() {
            im.hack_effects
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
        let next = self.implants[idx].condition.degraded();
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
        self.implants.iter().position(|im| im.condition.is_active())
    }

    /// Indices of all active (breachable) implants — Cascade and EMP hit them all.
    fn active_implant_indices(&self) -> Vec<usize> {
        self.implants
            .iter()
            .enumerate()
            .filter(|(_, im)| im.condition.is_active())
            .map(|(i, _)| i)
            .collect()
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

    /// Digital **bandwidth** — Link floored to an integer (§7D). The hack channel
    /// is the weaker endpoint's band (`min`), which the attacker's rating averages
    /// with its Hacking. (Defense is Link-blind; zero Link is the separate hard
    /// reachability gate.)
    fn digital_band(&self) -> i32 {
        self.link.max(0)
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

/// Ticks a crit-gated knockout stun lasts. Applied in the digital phase (after
/// the action phase), so it needs ≥2 to survive decay and skip the next action.
/// Placeholder (TBD).
const KNOCKOUT_STUN: u32 = 2;

/// Fixed magnitude of the degrade-class liabilities an EMP fires — it has no
/// margin/crit, being a blunt physical pulse. Placeholder (TBD).
const EMP_MAGNITUDE: u32 = 2;

/// Cap on the meshed-PAN synergy bonus to a hack rating (§5). Placeholder (TBD).
const MESH_SYNERGY_CAP: i32 = 3;

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
        if atk.emp && self.units[target].is_alive() {
            self.apply_emp(target);
        }
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

    /// The digital activation pass (§10.3/§10.8): every unit with a hack and net
    /// presence (Link > 0) acts in **digital-initiative = Link** order, hacking
    /// the nearest reachable enemy. A unit frozen by a Crash/Seizure stun is off
    /// the net too. *(First pass: a discrete phase after the physical one; the
    /// design's fully interleaved physical+digital order is a later step.)*
    fn digital_phase(&mut self) {
        let mut order: Vec<usize> = (0..self.units.len())
            .filter(|&i| {
                let u = &self.units[i];
                u.is_alive() && u.hack.is_some() && u.link > 0
            })
            .collect();
        order.sort_by(|&a, &b| {
            let (ua, ub) = (&self.units[a], &self.units[b]);
            ub.link.cmp(&ua.link).then(ua.id.cmp(&ub.id))
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
    /// `3d6 + avg(Hacking, channel)` vs the target's `Firewall` (the TN, §13),
    /// where the **channel** is the weaker endpoint's Link bandwidth (`min`). The
    /// target's Link feeds the channel, not the wall, so a darker target is harder
    /// to hack; defense is the Firewall alone. Zero Link stays the hard
    /// reachability gate. Lands the hack's payload (margin-scaled) on success.
    pub fn resolve_hack(&mut self, attacker: usize, target: usize) -> HackResult {
        let Some(hack) = self.units[attacker].hack else {
            return HackResult::NoHack;
        };
        // Hard gate (§7D/§7F): a runner needs net presence; the target a surface.
        if self.units[attacker].link <= 0 {
            return HackResult::Offline;
        }
        if self.units[target].link <= 0 {
            return HackResult::NoSurface;
        }
        // The connection runs at the weaker endpoint's bandwidth (the channel);
        // the rating averages the attacker's Hacking with it. Firewall is the TN.
        let channel = self.units[attacker].digital_band().min(self.units[target].digital_band());
        let rating = hack_rating(self.units[attacker].skill(Skill::Hacking), channel)
            + self.units[attacker].mesh_synergy(); // meshed PAN throughput (§5)
        let tn = self.units[target].firewall;
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
        let range = me.hack.map_or(0, |h| h.range);
        self.units
            .iter()
            .enumerate()
            .filter(|(j, u)| {
                *j != i
                    && u.is_alive()
                    && u.team == me.team.enemy()
                    && u.link > 0
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
            link: 0,
            firewall: 0,
            immunity: 0,
            attack: Attack {
                damage: 10.0,
                dtype: DamageType::Piercing,
                pen: PenTier::Internal,
                range: 1,
                emp: false,
            },
            hack: None,
            implants: Vec::new(),
            pan: Pan::Meshed,
            statuses: Vec::new(),
            alive: true,
        }
    }

    /// A unit wired to hack: net presence + a Lockware deck (antenna `range`).
    /// Its hack strength comes from its own Link & Hacking — set those per test.
    fn runner(id: u32, team: Team, q: i32, range: i32) -> Unit {
        let mut u = unit(id, team, q);
        u.link = 3;
        u.hack = Some(Hack::new(range, StatusSpec::lockware(), 1, 5));
        u
    }

    /// A unit with a hackable digital surface: Link > 0 and a Firewall TN.
    fn networked(id: u32, team: Team, q: i32, firewall: i32) -> Unit {
        let mut u = unit(id, team, q);
        u.link = 2;
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
    fn hold_captures_by_round_or_by_clearing() {
        let hex = Hex::new(2, 0);
        let mut held = vec![unit(0, Team::A, 2), unit(1, Team::B, 5)]; // player on the hex
        held[0].pos = hex;
        let obj = Hold { hex, by_round: 3 };
        assert_eq!(obj.status(&held, 1, false), ObjectiveStatus::Pending); // holds, but too early
        assert_eq!(obj.status(&held, 3, false), ObjectiveStatus::Achieved); // held to the round
        let mut cleared = held.clone();
        cleared[1].alive = false; // enemy gone → captured immediately
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
        let atk = runner(0, Team::A, 0, 1);
        let mut tgt = networked(1, Team::B, 0, 8);
        tgt.link = 0; // air-gapped — no surface to reach
                        // Empty RNG: a roll here would panic, proving the gate short-circuits.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        assert_eq!(b.resolve_hack(0, 1), HackResult::NoSurface);
        assert!(b.units[1].statuses.is_empty());
    }

    #[test]
    fn offline_attacker_cannot_hack() {
        let mut atk = runner(0, Team::A, 0, 1);
        atk.link = 0; // dark — no presence to reach with
        let tgt = networked(1, Team::B, 0, 8);
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        assert_eq!(b.resolve_hack(0, 1), HackResult::Offline);
        assert!(b.units[1].statuses.is_empty());
    }

    #[test]
    fn the_channel_is_the_weaker_endpoints_link() {
        // Attacker Link 6, target Link 2 → channel 2 (the target bottlenecks it);
        // rating avg(Hacking 4, 2) = 3.
        let mut atk = runner(0, Team::A, 0, 1);
        atk.link = 6;
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = networked(1, Team::B, 0, 0);
        tgt.link = 2;
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4])); // 12
        let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
        assert_eq!(outcome.total, 12 + 3);
    }

    #[test]
    fn a_thin_runner_link_bottlenecks_the_channel() {
        // Mirror: attacker Link 2, target Link 6 → channel 2; same rating 3.
        let mut atk = runner(0, Team::A, 0, 1);
        atk.link = 2;
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = networked(1, Team::B, 0, 0);
        tgt.link = 6;
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4])); // 12
        let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
        assert_eq!(outcome.total, 12 + 3); // channel min(2, 6) = 2
    }

    #[test]
    fn a_darker_target_is_harder_to_hack() {
        // Same runner; only the target's Link (the channel) changes.
        let total_vs = |target_link: i32| {
            let mut atk = runner(0, Team::A, 0, 1);
            atk.link = 6;
            atk.skills.set(Skill::Hacking, 6);
            let mut tgt = networked(1, Team::B, 0, 0);
            tgt.link = target_link;
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
        atk.link = 4;
        atk.skills.set(Skill::Hacking, 6);
        let mut tgt = networked(1, Team::B, 0, 12);
        tgt.link = 4;
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
        atk.link = 4;
        atk.skills.set(Skill::Hacking, 6); // channel min(4, 4) = 4 → rating avg(6, 4) = 5
        let mut tgt = networked(1, Team::B, 0, 2);
        tgt.link = 4;
        // 3d6 = 9, + rating 5 = 14 vs Firewall 2 → margin 12 → 1 + 12/3 = 5 stacks.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 3]));
        assert!(b.resolve_hack(0, 1).landed());
        assert_eq!(b.units[1].statuses[0].spec.name, "Lockware");
        assert_eq!(b.units[1].statuses[0].stacks, 5);
    }

    #[test]
    fn digital_phase_hacks_the_nearest_reachable_enemy() {
        let mut atk = runner(0, Team::A, 0, 4); // antenna range 4
        atk.link = 4;
        atk.skills.set(Skill::Hacking, 4);
        let mut near = networked(1, Team::B, 2, 2); // distance 2 ≤ range 4
        near.link = 4;
        let mut far = networked(2, Team::B, 9, 2); // out of range
        far.link = 4;
        let mut b = Battle::with_rng(vec![atk, near, far], ScriptedRng::from_d6([4, 4, 4]));
        b.digital_phase();
        assert!(!b.units[1].statuses.is_empty()); // near got hacked
        assert!(b.units[2].statuses.is_empty()); // far one untouched (out of range)
    }

    #[test]
    fn a_stunned_runner_skips_its_hack() {
        let mut atk = runner(0, Team::A, 0, 4);
        atk.add_status(StatusSpec::crash(), 1, 1); // Seizure freezes the net action too
        let tgt = networked(1, Team::B, 1, 2);
        // Empty RNG: a stunned runner must not roll.
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        b.digital_phase();
        assert!(b.units[1].statuses.is_empty());
    }

    #[test]
    fn hacking_resolution_is_deterministic() {
        let setup = || {
            let mut atk = runner(0, Team::A, 0, 4);
            atk.link = 4;
            atk.skills.set(Skill::Hacking, 4);
            let mut tgt = networked(1, Team::B, 2, 4);
            tgt.link = 4;
            Battle::new(vec![atk, tgt], 99)
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

    #[test]
    fn installing_a_cyberdeck_grants_the_hack_and_folds_the_surface() {
        let mut u = unit(0, Team::A, 0); // base: no deck, Link 0, Firewall 0
        assert!(u.hack.is_none());
        u.install(Implant::cyberdeck());
        assert!(u.hack.is_some()); // benefit: the unit can now hack
        assert_eq!(u.link, 5); // folded surface
        assert_eq!(u.firewall, 2); // folded wall
    }

    #[test]
    fn a_crit_cascades_across_a_meshed_pan() {
        // A decisive hack on a meshed PAN rides to every implant, not just one.
        let mut atk = runner(0, Team::A, 0, 4);
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = unit(1, Team::B, 1);
        tgt.link = 3;
        tgt.firewall = 4;
        tgt.install(Implant::subdermal_plating());
        tgt.install(Implant::reflex_booster());
        assert_eq!(tgt.pan, Pan::Meshed); // the default
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([6, 6, 6])); // crit
        b.resolve_hack(0, 1);
        assert!(b.units[1].implants.iter().all(|im| im.condition == Condition::Offline));
    }

    #[test]
    fn a_segmented_pan_contains_the_crit() {
        // Same decisive hack, but segmentation isolates the breach to one slot.
        let mut atk = runner(0, Team::A, 0, 4);
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = unit(1, Team::B, 1);
        tgt.link = 3;
        tgt.firewall = 4;
        tgt.pan = Pan::Segmented;
        tgt.install(Implant::subdermal_plating()); // idx 0 — the targeted slot
        tgt.install(Implant::reflex_booster()); // idx 1 — contained
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([6, 6, 6])); // crit
        b.resolve_hack(0, 1);
        assert_eq!(b.units[1].implants[0].condition, Condition::Offline);
        assert_eq!(b.units[1].implants[1].condition, Condition::Online); // contained
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
            tgt.link = 5;
            let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4]));
            let HackResult::Rolled { outcome, .. } = b.resolve_hack(0, 1) else { panic!() };
            outcome.total
        };
        assert!(total_for(Pan::Meshed) > total_for(Pan::Segmented));
    }

    #[test]
    fn emp_fries_all_chrome_bypassing_firewall() {
        let mut tgt = unit(1, Team::B, 0);
        tgt.firewall = 99; // EMP ignores the wall entirely
        tgt.install(Implant::subdermal_plating()); // +6 Plating, Shed liability
        tgt.install(Implant::cyberdeck()); // Link 5, grants hack, Lockout liability
        assert!(tgt.hack.is_some());
        let mut atk = unit(0, Team::A, 0);
        atk.attack.emp = true;
        // Empty RNG: an EMP rolls nothing (a physical pulse, not a contest).
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        b.resolve_attack(0, 1);
        assert_eq!(b.units[1].implants[0].condition, Condition::Offline);
        assert_eq!(b.units[1].implants[1].condition, Condition::Offline);
        assert!(b.units[1].hack.is_none()); // deck bricked
        assert_eq!(b.units[1].defense.plating, 0.0); // plating benefit fried
    }

    #[test]
    fn emp_disables_but_does_not_knock_out() {
        // EMP is blunt — it fires degrade-class liabilities but not the stun.
        let mut tgt = unit(1, Team::B, 0);
        tgt.install(Implant::reflex_booster()); // Seizure (stun) liability
        let mut atk = unit(0, Team::A, 0);
        atk.attack.emp = true;
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        b.resolve_attack(0, 1);
        assert_eq!(b.units[1].implants[0].condition, Condition::Offline); // disabled
        assert!(!b.units[1].statuses.iter().any(|s| matches!(s.spec.effect, Effect::Stun)));
    }

    #[test]
    fn emp_is_harmless_to_unchromed_targets() {
        let tgt = unit(1, Team::B, 0); // flesh — no implants
        let mut atk = unit(0, Team::A, 0);
        atk.attack.emp = true;
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::default());
        b.resolve_attack(0, 1);
        assert!(b.units[1].implants.is_empty());
        // no chrome to fry → EMP adds no statuses (only the kinetic hit landed)
        assert!(b.units[1].statuses.is_empty());
    }

    #[test]
    fn a_degraded_implant_delivers_half_its_benefit() {
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::subdermal_plating()); // +6 Plating, Online
        assert_eq!(u.defense.plating, 6.0);
        u.degrade_implant(0); // Online → Degraded
        assert_eq!(u.implants[0].condition, Condition::Degraded);
        assert_eq!(u.defense.plating, 3.0); // half benefit
        u.degrade_implant(0); // Degraded → Offline
        assert_eq!(u.implants[0].condition, Condition::Offline);
        assert_eq!(u.defense.plating, 0.0); // none
    }

    #[test]
    fn the_ripperdoc_repairs_a_degraded_implant_to_online() {
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::subdermal_plating());
        u.degrade_implant(0); // Degraded, plating 3
        u.repair_implant(0);
        assert_eq!(u.implants[0].condition, Condition::Online);
        assert_eq!(u.defense.plating, 6.0); // restored full
    }

    #[test]
    fn wear_destroys_and_destroyed_is_terminal() {
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::subdermal_plating());
        u.degrade_implant(0); // Degraded
        u.degrade_implant(0); // Offline
        u.degrade_implant(0); // Destroyed
        assert_eq!(u.implants[0].condition, Condition::Destroyed);
        u.repair_implant(0); // terminal — a no-op
        assert_eq!(u.implants[0].condition, Condition::Destroyed);
        assert_eq!(u.defense.plating, 0.0); // stays gone
    }

    #[test]
    fn a_degraded_deck_still_hacks_on_a_thinner_surface() {
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::cyberdeck()); // Link 5, grants hack
        assert_eq!(u.link, 5);
        u.degrade_implant(0); // Degraded — still active
        assert!(u.hack.is_some()); // a degraded deck still hacks
        assert_eq!(u.link, 3); // round(0.5 × 5) = 3 — thinner surface
        u.degrade_implant(0); // Offline
        assert!(u.hack.is_none()); // now bricked
        assert_eq!(u.link, 0); // and exact — no rounding drift
    }

    #[test]
    fn firewall_suite_folds_the_wall() {
        let mut u = unit(0, Team::A, 0); // base Firewall 0
        u.install(Implant::firewall_suite());
        assert_eq!(u.firewall, 4);
        assert_eq!(u.link, 1); // a little surface comes with it
    }

    #[test]
    fn combat_stim_folds_damage_and_lifts_integrity_with_the_pump() {
        let mut u = unit(0, Team::A, 0);
        let (base_dmg, base_hp) = (u.attack.damage, u.max_integrity);
        u.install(Implant::combat_stim());
        assert_eq!(u.attack.damage, base_dmg + 4.0);
        u.install(Implant::metabolic_pump());
        assert_eq!(u.max_integrity, base_hp + 8.0);
        assert_eq!(u.integrity, base_hp + 8.0); // gained the HP too
    }

    #[test]
    fn breaching_combat_stim_fires_the_dot_on_a_solid_hit_not_the_stun() {
        // Overdose is multi-effect: the self-DoT is degrade (margin), the Crash is
        // crit-gated — so a solid, non-crit hit lands only the DoT.
        let mut atk = runner(0, Team::A, 0, 4);
        atk.skills.set(Skill::Hacking, 6);
        let mut tgt = unit(1, Team::B, 1);
        tgt.link = 4;
        tgt.firewall = 4; // big margin, no crit
        tgt.install(Implant::combat_stim());
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 3]));
        b.resolve_hack(0, 1);
        let has_dot = b.units[1].statuses.iter().any(|s| matches!(s.spec.effect, Effect::Dot { .. }));
        let has_stun = b.units[1].statuses.iter().any(|s| matches!(s.spec.effect, Effect::Stun));
        assert!(has_dot && !has_stun);
    }

    #[test]
    fn subdermal_plating_folds_into_defense() {
        let mut u = unit(0, Team::A, 0);
        let base = u.defense.plating;
        u.install(Implant::subdermal_plating());
        assert_eq!(u.defense.plating, base + 6.0);
    }

    #[test]
    fn an_offline_implant_contributes_nothing() {
        let mut u = unit(0, Team::A, 0);
        let mut deck = Implant::cyberdeck();
        deck.condition = Condition::Offline; // installed dead
        u.install(deck);
        assert!(u.hack.is_none());
        assert_eq!(u.link, 0);
    }

    #[test]
    fn disabling_drops_the_benefit_then_repair_restores_it() {
        let mut u = unit(0, Team::A, 0);
        u.install(Implant::cyberdeck());
        let effects = u.disable_implant(0); // the §6 disable floor (a breach)
        assert!(u.hack.is_none()); // bricked — lost the deck
        assert_eq!(u.link, 0); // surface folded back out
        assert!(!effects.is_empty()); // liabilities returned for the ladder (later phase)
        assert_eq!(u.implants[0].condition, Condition::Offline);
        u.repair_implant(0); // Ripperdoc
        assert!(u.hack.is_some());
        assert_eq!(u.link, 5);
        assert_eq!(u.implants[0].condition, Condition::Online);
    }

    #[test]
    fn a_marginal_hack_just_disables_the_implant() {
        // Margin 0 success → the §6 floor: disable only, no liability fired.
        let mut atk = runner(0, Team::A, 0, 4); // Link 3
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = unit(1, Team::B, 1);
        tgt.link = 2;
        tgt.install(Implant::reflex_booster()); // Seizure liability (stun)
        tgt.firewall = 12; // channel min(3,2)=2 → rating avg(4,2)=3; dice 9 → 12 = TN, margin 0
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 3]));
        assert!(b.resolve_hack(0, 1).landed());
        assert_eq!(b.units[1].implants[0].condition, Condition::Offline); // disabled
        assert!(b.units[1].statuses.is_empty()); // nothing fired (margin 0, no crit)
    }

    #[test]
    fn a_solid_hack_disables_and_fires_the_degrade_liability() {
        // Strong margin → disable + the degrade-class hack-effect, margin-scaled.
        let mut atk = runner(0, Team::A, 0, 4); // Link 3
        atk.skills.set(Skill::Hacking, 6);
        let mut tgt = unit(1, Team::B, 1);
        tgt.link = 4;
        tgt.install(Implant::subdermal_plating()); // Shed = Corrode (not stun)
        tgt.firewall = 4; // channel min(3,4)=3 → rating avg(6,3)=4; dice 9 → 13 vs 4, margin 9
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 3]));
        b.resolve_hack(0, 1);
        assert_eq!(b.units[1].implants[0].condition, Condition::Offline);
        assert_eq!(b.units[1].statuses[0].spec.name, "Corrode"); // Shed fired
        assert_eq!(b.units[1].statuses[0].stacks, 3); // margin 9 / 3
    }

    #[test]
    fn a_crit_delivers_the_knockout_stun() {
        // The stun class is crit-gated — only a decisive hack lands it.
        let mut atk = runner(0, Team::A, 0, 4);
        atk.skills.set(Skill::Hacking, 4);
        let mut tgt = unit(1, Team::B, 1);
        tgt.link = 3;
        tgt.firewall = 12;
        tgt.install(Implant::reflex_booster()); // Seizure (stun)
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([6, 6, 6])); // nat 18
        b.resolve_hack(0, 1);
        assert_eq!(b.units[1].implants[0].condition, Condition::Offline);
        assert!(b.units[1].statuses.iter().any(|s| matches!(s.spec.effect, Effect::Stun)));
    }

    #[test]
    fn breaching_a_deck_silences_the_targets_own_hacking() {
        // A netrunner whose deck is breached can't hack back (benefit unfolds).
        let mut atk = runner(0, Team::A, 0, 4);
        atk.skills.set(Skill::Hacking, 6);
        let mut tgt = unit(1, Team::B, 1);
        tgt.skills.set(Skill::Hacking, 4);
        tgt.install(Implant::cyberdeck()); // grants the hack + Link 5
        assert!(tgt.hack.is_some());
        tgt.firewall = 4;
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([3, 3, 3]));
        b.resolve_hack(0, 1);
        assert!(b.units[1].hack.is_none()); // deck bricked → no hacking back
        assert_eq!(b.units[1].implants[0].condition, Condition::Offline);
    }

    #[test]
    fn an_installed_deck_lets_a_unit_hack_in_the_digital_phase() {
        let mut atk = unit(0, Team::A, 0);
        atk.skills.set(Skill::Hacking, 4);
        atk.install(Implant::cyberdeck()); // grants hack + Link 5
        let tgt = networked(1, Team::B, 1, 9); // soft target, in deck range 6
        let mut b = Battle::with_rng(vec![atk, tgt], ScriptedRng::from_d6([4, 4, 4]));
        b.digital_phase();
        assert!(!b.units[1].statuses.is_empty()); // hacked via the installed deck
    }
}
