//! Board **terrain** — the per-encounter tactical map that turns the bare hex grid into
//! a place with structure (`docs/combat.md` §7B board).
//!
//! The engine grid is otherwise featureless and *unbounded*, which makes a fleeing unit
//! uncatchable (a kiter on an open plane never gets cornered) and removes the positional
//! game. A [`Terrain`] gives a battle three things:
//!
//! - **Zone + friction** — a [`Bounds`] **deployment / engagement zone** of free ground.
//!   The edge is *soft*, not a wall: a unit may step beyond it, but each hex outside costs
//!   **escalating movement** ([`Terrain::move_cost`]) — `1 + distance past the zone`. With
//!   the small move stats in play a unit can barely poke a hex past the line, so a kiter
//!   bogs down in the fringe and a pursuer (moving at cost 1 inside) runs it down — the
//!   stalemate dies without a hard wall to bump. Default is `None` (open, cost-1
//!   everywhere), so engine tests and an ad-hoc battle behave exactly as before.
//! - **Blockers** — impassable [`Tile::Blocked`] hexes (rubble / wall) movement must
//!   route *around* (see the BFS step in `lib.rs`). These *are* hard — only the map edge
//!   is soft.
//! - **Cover & hazards** — a [`Tile::Cover`] hex makes its occupant harder to hit (a
//!   to-hit TN bonus, read in the attack pipeline); a [`Tile::Hazard`] hex damages
//!   whoever stands on it each tick.
//!
//! Terrain is **sparse**: only non-`Open` hexes are stored, so an empty arena is just its
//! bounds. It carries no AI of its own — the movement/targeting profiles in `lib.rs` read
//! it (avoid hazards, prefer cover) — keeping this module pure board description.

use crate::{DamageType, Hex, PenTier};
use std::collections::HashMap;

/// What occupies a board hex besides a unit. Sparse — an unlisted hex is [`Tile::Open`].
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Tile {
    /// Empty ground — passable, no modifier.
    #[default]
    Open,
    /// **Impassable** wall / rubble: movement must route around it.
    Blocked,
    /// **Cover** — passable, but a unit standing here is harder to hit: `+tn` to an
    /// attacker's to-hit target number (§7G positioning).
    Cover(i32),
    /// **Hazard** — passable, but damages whoever stands here at tick start (fire / toxic
    /// / EMP field). The AI avoids it; you can herd enemies into it.
    Hazard { damage: f32, dtype: DamageType, pen: PenTier },
}

impl Tile {
    /// Can a unit move onto / through this tile?
    pub fn passable(self) -> bool {
        !matches!(self, Tile::Blocked)
    }

    /// The to-hit TN this tile adds to *its occupant's* defense (cover bonus; 0 otherwise).
    pub fn cover_tn(self) -> i32 {
        match self {
            Tile::Cover(tn) => tn,
            _ => 0,
        }
    }

    /// The per-tick damage this tile inflicts on its occupant, if any.
    pub fn hazard(self) -> Option<(f32, DamageType, PenTier)> {
        match self {
            Tile::Hazard { damage, dtype, pen } => Some((damage, dtype, pen)),
            _ => None,
        }
    }
}

/// A rectangular **zone** in axial coords — `[min_q, max_q] × [min_r, max_r]`, inclusive.
/// The free-movement / deployment area; outside it movement is *penalised*, not blocked.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bounds {
    pub min_q: i32,
    pub max_q: i32,
    pub min_r: i32,
    pub max_r: i32,
}

impl Bounds {
    /// Is `h` inside the zone (inclusive)?
    pub fn contains(self, h: Hex) -> bool {
        (self.min_q..=self.max_q).contains(&h.q) && (self.min_r..=self.max_r).contains(&h.r)
    }

    /// The nearest in-zone hex to `h` (clamped per axis) — `h` itself when already inside.
    fn clamp(self, h: Hex) -> Hex {
        Hex::new(h.q.clamp(self.min_q, self.max_q), h.r.clamp(self.min_r, self.max_r))
    }

    /// How far `h` lies **outside** the zone (0 when inside) — the friction distance.
    fn overshoot(self, h: Hex) -> i32 {
        h.distance(self.clamp(h))
    }
}

/// How far past the zone the engine still bothers to path/track (the friction makes a unit
/// bog down within a hex or two of the edge, so a small fringe is plenty).
const FRINGE: i32 = 3;

/// A battle's **board**: an optional engagement [`Bounds`] zone plus sparse terrain. The
/// default is an open, unbounded plane (no zone, every hex [`Tile::Open`], move-cost 1) —
/// the engine's prior behavior, so existing battles are unchanged until they opt into a map.
#[derive(Clone, Debug, Default)]
pub struct Terrain {
    zone: Option<Bounds>,
    tiles: HashMap<Hex, Tile>,
}

impl Terrain {
    /// An **arena**: a `depth × frontage` engagement zone (columns `0..depth`, rows
    /// `0..frontage`) of open ground, ready to have terrain placed on it.
    pub fn arena(depth: i32, frontage: i32) -> Self {
        Self {
            zone: Some(Bounds {
                min_q: 0,
                max_q: (depth - 1).max(0),
                min_r: 0,
                max_r: (frontage - 1).max(0),
            }),
            tiles: HashMap::new(),
        }
    }

    /// The board's engagement zone, if any (`None` = open plane).
    pub fn zone(&self) -> Option<Bounds> {
        self.zone
    }

    /// Builder: place a single `tile` at `hex` (`Open` removes any feature there).
    pub fn set(mut self, hex: Hex, tile: Tile) -> Self {
        if tile == Tile::Open {
            self.tiles.remove(&hex);
        } else {
            self.tiles.insert(hex, tile);
        }
        self
    }

    /// Builder: stamp the same `tile` across many hexes (walls, a cover line, a hazard
    /// field). Out-of-zone hexes are ignored, so a map can over-paint freely.
    pub fn fill(mut self, hexes: impl IntoIterator<Item = Hex>, tile: Tile) -> Self {
        for h in hexes {
            if self.in_zone(h) {
                self = self.set(h, tile);
            }
        }
        self
    }

    /// The tile at `hex` — `Open` for any unlisted hex.
    pub fn tile(&self, hex: Hex) -> Tile {
        self.tiles.get(&hex).copied().unwrap_or_default()
    }

    /// Is `hex` inside the engagement zone? (Always true on the open plane.)
    pub fn in_zone(&self, hex: Hex) -> bool {
        self.zone.is_none_or(|b| b.contains(hex))
    }

    /// The **movement cost** to enter `hex`: `1` inside the zone (or on the open plane),
    /// rising by `1` per hex of overshoot past the zone edge. The soft-boundary friction —
    /// venturing out is allowed but increasingly expensive, so a flee bottoms out fast.
    pub fn move_cost(&self, hex: Hex) -> i32 {
        1 + self.zone.map_or(0, |b| b.overshoot(hex))
    }

    /// Is `hex` worth pathing through — inside the zone or within the thin [`FRINGE`] past
    /// it? Bounds the BFS flow field so it always terminates (units never get further out).
    pub fn pathable(&self, hex: Hex) -> bool {
        self.zone.is_none_or(|b| b.overshoot(hex) <= FRINGE)
    }

    /// Can a unit stand on / move through `hex` — i.e. not a hard blocker? (The zone edge
    /// is soft, charged via [`move_cost`](Self::move_cost), not refused here.)
    pub fn passable(&self, hex: Hex) -> bool {
        self.tile(hex).passable()
    }

    /// The cover TN a unit standing on `hex` gains (added to attackers' to-hit TN).
    pub fn cover_tn(&self, hex: Hex) -> i32 {
        self.tile(hex).cover_tn()
    }

    /// The hazard damage a unit standing on `hex` suffers each tick, if any.
    pub fn hazard(&self, hex: Hex) -> Option<(f32, DamageType, PenTier)> {
        self.tile(hex).hazard()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_open_board_is_all_passable_at_cost_one() {
        let t = Terrain::default();
        assert!(t.passable(Hex::new(100, -50))); // no zone — anywhere goes
        assert_eq!(t.move_cost(Hex::new(100, -50)), 1); // and always cheap
        assert_eq!(t.cover_tn(Hex::new(0, 0)), 0);
        assert!(t.hazard(Hex::new(0, 0)).is_none());
    }

    #[test]
    fn the_zone_edge_is_soft_friction_not_a_wall() {
        let t = Terrain::arena(8, 6); // zone cols 0..=7, rows 0..=5
        assert_eq!(t.move_cost(Hex::new(3, 3)), 1); // inside — free
        assert!(t.passable(Hex::new(8, 0))); // off-zone is still steppable...
        assert_eq!(t.move_cost(Hex::new(8, 0)), 2); // ...but the first fringe hex costs 2
        assert_eq!(t.move_cost(Hex::new(10, 0)), 4); // and it climbs: 1 + 3 overshoot
        assert!(!t.pathable(Hex::new(20, 0))); // far out — past where the AI bothers to path
    }

    #[test]
    fn blockers_cover_and_hazards_report_their_rules() {
        let t = Terrain::arena(8, 6)
            .set(Hex::new(3, 3), Tile::Blocked)
            .set(Hex::new(4, 2), Tile::Cover(3))
            .set(Hex::new(2, 4), Tile::Hazard { damage: 5.0, dtype: DamageType::Piercing, pen: PenTier::Internal });
        assert!(!t.passable(Hex::new(3, 3))); // wall blocks movement
        assert_eq!(t.cover_tn(Hex::new(4, 2)), 3); // cover lifts the to-hit TN
        assert_eq!(t.hazard(Hex::new(2, 4)).map(|(d, _, _)| d), Some(5.0));
        assert!(t.passable(Hex::new(4, 2)) && t.passable(Hex::new(2, 4))); // cover/hazard still passable
    }
}
