//! `atomica-sim` — engine-agnostic, deterministic battle simulation for
//! **CHROME AND CODE**.
//!
//! This crate has no rendering or game-engine dependencies. The front-end reads
//! [`Battle`] state each frame and draws it; nothing here knows about a screen.
//!
//! ## Determinism
//! All randomness flows through [`Rng`] (seeded). Given the same seed and the same
//! initial [`Battle`], [`Battle::step`] always produces the same result. That is
//! what makes the design's *async / replayable auto-resolution* possible.
//!
//! ## Scope (so far)
//! This is the architectural skeleton, not the full ruleset. It models the locked
//! *shapes* — the stat line, layered defense, penetration tiers, the armor matrix,
//! initiative-ordered ticks — with placeholder values. The à-la-carte status pool,
//! both contagion families, netrunning, IFF/spoof, and Heat are intentionally
//! left as `TODO` extension points rather than stubbed with guessed numbers.

mod hex;
mod rng;

pub use hex::Hex;
pub use rng::Rng;

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
    pub id: u32,
    pub name: String,
    pub team: Team,
    pub pos: Hex,

    /// The single HP pool — all damage ultimately reduces it.
    pub integrity: f32,
    pub max_integrity: f32,
    pub defense: Defense,

    /// Physical Initiative — turn order in the world (higher acts first).
    pub initiative: f32,
    /// Digital Initiative / net presence. `0.0` ⇒ immune to all digital attack.
    pub link: f32,
    /// Resist vs Worm + hacks.
    pub firewall: f32,
    /// Resist vs Virus.
    pub immunity: f32,

    pub attack: Attack,
    pub alive: bool,
}

impl Unit {
    pub fn is_alive(&self) -> bool {
        self.alive && self.integrity > 0.0
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

/// A full battle: the units, the seeded RNG, and a tick counter.
#[derive(Clone, Debug)]
pub struct Battle {
    pub units: Vec<Unit>,
    pub tick: u32,
    rng: Rng,
}

impl Battle {
    pub fn new(units: Vec<Unit>, seed: u64) -> Self {
        Self {
            units,
            tick: 0,
            rng: Rng::new(seed),
        }
    }

    /// Advance one tick: each living unit, in physical-initiative order, acts once.
    ///
    /// Acting = attack the nearest enemy if in range, else step toward it. This is
    /// the minimal resolution loop; richer behavior profiles (Advance/Hold/Kite,
    /// targeting modes) and the status pipeline hang off this same ordering.
    pub fn step(&mut self) -> Outcome {
        if let o @ (Outcome::Winner(_) | Outcome::Draw) = self.outcome() {
            return o;
        }
        self.tick += 1;

        // Deterministic action order: initiative desc, id asc as the tiebreak.
        let mut order: Vec<usize> = (0..self.units.len())
            .filter(|&i| self.units[i].is_alive())
            .collect();
        order.sort_by(|&a, &b| {
            let (ua, ub) = (&self.units[a], &self.units[b]);
            ub.initiative
                .partial_cmp(&ua.initiative)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then(ua.id.cmp(&ub.id))
        });

        for i in order {
            if !self.units[i].is_alive() {
                continue; // died earlier this tick
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

    fn nearest_enemy(&self, i: usize) -> Option<usize> {
        let me = &self.units[i];
        self.units
            .iter()
            .enumerate()
            .filter(|(j, u)| *j != i && u.is_alive() && u.team == me.team.enemy())
            // Distance first, then id, so ties resolve deterministically.
            .min_by_key(|(_, u)| (me.pos.distance(u.pos), u.id))
            .map(|(j, _)| j)
    }

    fn resolve_attack(&mut self, attacker: usize, target: usize) {
        let atk = self.units[attacker].attack;
        // The armor matrix will scale damage by dtype-vs-material here; for now the
        // type is carried through unchanged. `rng` is threaded in so stochastic
        // statuses can hook this without changing the call sites.
        let _ = (&mut self.rng, atk.dtype);
        apply_damage(&mut self.units[target], atk.damage, atk.pen);
    }
}

/// Route `amount` through the defense layers selected by `pen`, spilling any
/// remainder inward. Pierce semantics ("drop a tier") are expressed by choosing a
/// deeper `PenTier`.
fn apply_damage(unit: &mut Unit, amount: f32, pen: PenTier) {
    let mut remaining = amount;
    if matches!(pen, PenTier::External) {
        remaining = absorb(&mut unit.defense.barrier, remaining);
    }
    if matches!(pen, PenTier::External | PenTier::Contact) {
        remaining = absorb(&mut unit.defense.plating, remaining);
    }
    unit.integrity -= remaining;
    if unit.integrity <= 0.0 {
        unit.integrity = 0.0;
        unit.alive = false;
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

    fn unit(id: u32, team: Team, q: i32, dmg: f32, init: f32) -> Unit {
        Unit {
            id,
            name: format!("U{id}"),
            team,
            pos: Hex::new(q, 0),
            integrity: 30.0,
            max_integrity: 30.0,
            defense: Defense::default(),
            initiative: init,
            link: 0.0,
            firewall: 0.0,
            immunity: 0.0,
            attack: Attack {
                damage: dmg,
                dtype: DamageType::Piercing,
                pen: PenTier::Internal,
                range: 1,
            },
            alive: true,
        }
    }

    fn demo() -> Battle {
        Battle::new(
            vec![
                unit(0, Team::A, 0, 10.0, 5.0),
                unit(1, Team::B, 3, 8.0, 4.0),
            ],
            123,
        )
    }

    #[test]
    fn layers_absorb_then_integrity() {
        let mut u = unit(0, Team::A, 0, 0.0, 0.0);
        u.defense = Defense { barrier: 5.0, plating: 5.0 };
        apply_damage(&mut u, 12.0, PenTier::External);
        // 5 barrier + 5 plating soaked, 2 reaches integrity.
        assert_eq!(u.integrity, 28.0);
        assert_eq!(u.defense.barrier, 0.0);
        assert_eq!(u.defense.plating, 0.0);
    }

    #[test]
    fn internal_bypasses_layers() {
        let mut u = unit(0, Team::A, 0, 0.0, 0.0);
        u.defense = Defense { barrier: 99.0, plating: 99.0 };
        apply_damage(&mut u, 10.0, PenTier::Internal);
        assert_eq!(u.integrity, 20.0);
        assert_eq!(u.defense.barrier, 99.0);
    }

    #[test]
    fn battle_terminates_with_a_winner() {
        let mut b = demo();
        let outcome = b.resolve(1000);
        assert!(matches!(outcome, Outcome::Winner(_)));
    }

    #[test]
    fn resolution_is_deterministic() {
        let mut x = demo();
        let mut y = demo();
        assert_eq!(x.resolve(1000), y.resolve(1000));
        assert_eq!(x.tick, y.tick);
        // State, not just the verdict, must match tick-for-tick.
        for (a, b) in x.units.iter().zip(&y.units) {
            assert_eq!(a.integrity, b.integrity);
            assert_eq!(a.pos, b.pos);
        }
    }
}
