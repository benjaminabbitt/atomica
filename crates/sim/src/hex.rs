//! Flat-top hexagonal grid math, in axial coordinates.
//!
//! Per the design: columns are depth ranks (`q`), the long edge is frontage (`r`).
//! Rendering pixel placement lives in the front-end; this module is pure topology.

/// An axial hex coordinate.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub struct Hex {
    /// Column / depth rank.
    pub q: i32,
    /// Row / frontage.
    pub r: i32,
}

/// The six axial step vectors, in order — shared by neighbours, rings and beams.
const DIRECTIONS: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];

impl Hex {
    pub const fn new(q: i32, r: i32) -> Self {
        Self { q, r }
    }

    /// Cube-coordinate distance between two hexes.
    pub fn distance(self, other: Hex) -> i32 {
        let dq = self.q - other.q;
        let dr = self.r - other.r;
        let ds = (self.q + self.r) - (other.q + other.r);
        (dq.abs() + dr.abs() + ds.abs()) / 2
    }

    /// The six adjacent hexes (degree-6 neighbourhood).
    pub fn neighbors(self) -> [Hex; 6] {
        DIRECTIONS.map(|(dq, dr)| Hex::new(self.q + dq, self.r + dr))
    }

    /// All hexes within `radius` of `self` (inclusive) — the **blast footprint**
    /// (§7G). `radius` 1 = **7** hexes, `radius` 2 = **19** (`1 + 3·r·(r+1)`).
    pub fn within(self, radius: i32) -> Vec<Hex> {
        let mut out = Vec::new();
        for dx in -radius..=radius {
            let lo = (-radius).max(-dx - radius);
            let hi = radius.min(-dx + radius);
            for dy in lo..=hi {
                let dz = -dx - dy;
                out.push(Hex::new(self.q + dx, self.r + dz));
            }
        }
        out
    }

    /// The hexes at *exactly* `radius` (the ring) — `6·radius` of them
    /// (`radius` 0 = just `self`).
    pub fn ring(self, radius: i32) -> Vec<Hex> {
        if radius <= 0 {
            return vec![self];
        }
        let mut out = Vec::with_capacity(6 * radius as usize);
        // Start `radius` steps along one direction, then walk the six sides.
        let (sq, sr) = DIRECTIONS[4];
        let mut hex = Hex::new(self.q + sq * radius, self.r + sr * radius);
        for (dq, dr) in DIRECTIONS {
            for _ in 0..radius {
                out.push(hex);
                hex = Hex::new(hex.q + dq, hex.r + dr);
            }
        }
        out
    }

    /// A straight line of `length` hexes from `self` stepping in direction
    /// `dir` (0..6) — the **beam spine** (§7G). `self` is the first hex.
    pub fn line(self, dir: usize, length: i32) -> Vec<Hex> {
        let (dq, dr) = DIRECTIONS[dir % 6];
        (0..length.max(0)).map(|i| Hex::new(self.q + dq * i, self.r + dr * i)).collect()
    }

    /// The neighbour that most reduces distance to `goal` (one step of a greedy
    /// path). Returns `self` if already adjacent-or-on the goal.
    pub fn step_toward(self, goal: Hex) -> Hex {
        if self.distance(goal) <= 1 {
            return self;
        }
        self.neighbors()
            .into_iter()
            .min_by_key(|n| n.distance(goal))
            .unwrap_or(self)
    }

    /// The neighbour (or `self`) that **maximises** distance from `threat` — one
    /// step of a retreat (Kite / Disperse). Stable tiebreak so it's deterministic.
    pub fn step_away(self, threat: Hex) -> Hex {
        std::iter::once(self)
            .chain(self.neighbors())
            .max_by_key(|h| (h.distance(threat), -h.q, -h.r))
            .unwrap_or(self)
    }

    /// A **flanking** step toward `goal`: close the distance, but break ties toward
    /// the greatest frontage (row) offset — approach from the side.
    pub fn step_flank(self, goal: Hex) -> Hex {
        std::iter::once(self)
            .chain(self.neighbors())
            .min_by_key(|h| (h.distance(goal), -(h.r - goal.r).abs(), h.q, h.r))
            .unwrap_or(self)
    }

    /// The axial **direction index** (`0..6`) whose one-hex step best closes on
    /// `goal` — the bearing a beam fires along. Ties break to the lower index for
    /// determinism.
    pub fn direction_to(self, goal: Hex) -> usize {
        (0..6)
            .min_by_key(|&d| {
                let (dq, dr) = DIRECTIONS[d];
                Hex::new(self.q + dq, self.r + dr).distance(goal)
            })
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn distance_basics() {
        assert_eq!(Hex::new(0, 0).distance(Hex::new(0, 0)), 0);
        assert_eq!(Hex::new(0, 0).distance(Hex::new(3, 0)), 3);
        for n in Hex::new(0, 0).neighbors() {
            assert_eq!(Hex::new(0, 0).distance(n), 1);
        }
    }

    #[test]
    fn step_reduces_distance() {
        let start = Hex::new(0, 0);
        let goal = Hex::new(4, -2);
        let next = start.step_toward(goal);
        assert!(next.distance(goal) < start.distance(goal));
    }

    #[test]
    fn blast_footprint_matches_the_spec() {
        let c = Hex::new(2, -1);
        // §7G: footprint 1 = 7 hexes, footprint 2 = 19.
        assert_eq!(c.within(1).len(), 7);
        assert_eq!(c.within(2).len(), 19);
        // Everything is within range, and the centre is included.
        assert!(c.within(2).iter().all(|h| c.distance(*h) <= 2));
        assert!(c.within(1).contains(&c));
    }

    #[test]
    fn ring_is_six_per_radius() {
        let c = Hex::new(-3, 1);
        assert_eq!(c.ring(0), vec![c]);
        assert_eq!(c.ring(1).len(), 6);
        assert_eq!(c.ring(2).len(), 12);
        // A ring sits at exactly its radius.
        assert!(c.ring(2).iter().all(|h| c.distance(*h) == 2));
    }

    #[test]
    fn step_away_increases_distance() {
        let pos = Hex::new(0, 0);
        let threat = Hex::new(2, 0);
        let away = pos.step_away(threat);
        assert!(away.distance(threat) > pos.distance(threat)); // retreated
        assert_eq!(pos.distance(away), 1); // exactly one step
    }

    #[test]
    fn step_flank_still_closes_in() {
        let pos = Hex::new(0, 0);
        let goal = Hex::new(4, 0);
        let flanked = pos.step_flank(goal);
        assert!(flanked.distance(goal) < pos.distance(goal)); // approaches
    }

    #[test]
    fn beam_line_walks_straight() {
        let start = Hex::new(0, 0);
        let beam = start.line(0, 4); // footprint-1 beam, 4 long
        assert_eq!(beam.len(), 4);
        assert_eq!(beam[0], start);
        // Each step is one hex farther along.
        for (i, h) in beam.iter().enumerate() {
            assert_eq!(start.distance(*h), i as i32);
        }
    }
}
