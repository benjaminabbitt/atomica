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
        const DIRS: [(i32, i32); 6] = [(1, 0), (1, -1), (0, -1), (-1, 0), (-1, 1), (0, 1)];
        DIRS.map(|(dq, dr)| Hex::new(self.q + dq, self.r + dr))
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
}
