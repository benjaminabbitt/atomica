//! Board **terrain** — the per-encounter tactical map that turns the bare hex grid into
//! a place with structure (`docs/combat.md` §7B board).
//!
//! The engine grid is otherwise featureless and *unbounded*, which makes a fleeing unit
//! uncatchable (a kiter on an open plane never gets cornered) and removes the positional
//! game. A [`Terrain`] gives a battle three things:
//!
//! - **Bounds** — a hard outer extent ([`Bounds`]); off-board hexes are impassable, so a
//!   retreat eventually hits a wall and the kiter is run down. Default is `None`
//!   (unbounded), so engine tests and an ad-hoc battle behave exactly as before.
//! - **Blockers** — impassable [`Tile::Blocked`] hexes (rubble / wall) movement must
//!   route *around* (see the BFS step in `lib.rs`).
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

/// A hard rectangular **extent** in axial coords — `[min_q, max_q] × [min_r, max_r]`,
/// inclusive. Off-board hexes are impassable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bounds {
    pub min_q: i32,
    pub max_q: i32,
    pub min_r: i32,
    pub max_r: i32,
}

impl Bounds {
    /// Is `h` inside the extent (inclusive)?
    pub fn contains(self, h: Hex) -> bool {
        (self.min_q..=self.max_q).contains(&h.q) && (self.min_r..=self.max_r).contains(&h.r)
    }
}

/// A battle's **board**: an optional hard extent plus sparse terrain. The default is an
/// open, unbounded plane (no bounds, every hex [`Tile::Open`]) — the engine's prior
/// behavior, so existing battles are unchanged until they opt into a map.
#[derive(Clone, Debug, Default)]
pub struct Terrain {
    bounds: Option<Bounds>,
    tiles: HashMap<Hex, Tile>,
}

impl Terrain {
    /// An **arena**: a bounded `depth × frontage` rectangle (columns `0..depth`, rows
    /// `0..frontage`) of open ground, ready to have terrain placed on it.
    pub fn arena(depth: i32, frontage: i32) -> Self {
        Self {
            bounds: Some(Bounds {
                min_q: 0,
                max_q: (depth - 1).max(0),
                min_r: 0,
                max_r: (frontage - 1).max(0),
            }),
            tiles: HashMap::new(),
        }
    }

    /// The board's hard extent, if any (`None` = unbounded).
    pub fn bounds(&self) -> Option<Bounds> {
        self.bounds
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
    /// field). Off-board hexes are ignored, so a map can over-paint freely.
    pub fn fill(mut self, hexes: impl IntoIterator<Item = Hex>, tile: Tile) -> Self {
        for h in hexes {
            if self.in_bounds(h) {
                self = self.set(h, tile);
            }
        }
        self
    }

    /// The tile at `hex` — `Open` for any unlisted (or off-board) hex.
    pub fn tile(&self, hex: Hex) -> Tile {
        self.tiles.get(&hex).copied().unwrap_or_default()
    }

    /// Is `hex` within the board's extent? (Always true when unbounded.)
    pub fn in_bounds(&self, hex: Hex) -> bool {
        self.bounds.is_none_or(|b| b.contains(hex))
    }

    /// Can a unit stand on / move through `hex` — on the board **and** not a blocker?
    pub fn passable(&self, hex: Hex) -> bool {
        self.in_bounds(hex) && self.tile(hex).passable()
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
    fn an_open_unbounded_board_is_all_passable() {
        let t = Terrain::default();
        assert!(t.passable(Hex::new(100, -50))); // no bounds — anywhere goes
        assert_eq!(t.cover_tn(Hex::new(0, 0)), 0);
        assert!(t.hazard(Hex::new(0, 0)).is_none());
    }

    #[test]
    fn an_arena_bounds_the_play_area() {
        let t = Terrain::arena(8, 6); // cols 0..=7, rows 0..=5
        assert!(t.passable(Hex::new(0, 0)));
        assert!(t.passable(Hex::new(7, 5)));
        assert!(!t.passable(Hex::new(8, 0))); // past the depth edge — off-board
        assert!(!t.passable(Hex::new(0, -1))); // past the frontage edge
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
