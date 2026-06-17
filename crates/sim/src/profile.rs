//! Behavior profiles — the scripted **movement** and **targeting** a unit follows
//! each activation (§7J). *You program your units; the enemy hacks your script* —
//! these are **code**, so the digital realm corrupts *behavior* (spoof / Lockware),
//! not just stats. The defaults (`Nearest` + `Advance`) are the dumb "attack the
//! nearest, walk forward" baseline.

/// How a unit chooses its target among living enemies (§7J).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum TargetingProfile {
    /// Closest enemy — the dumb default (no smarts needed).
    #[default]
    Nearest,
    /// Lowest Integrity — finish off the wounded.
    LowestIntegrity,
    /// Biggest attack — strike the heaviest hitter.
    HighestThreat,
    /// Farthest enemy — reach past the front for the back ranks.
    Backline,
    /// Least armor (Barrier + Plating) — soft targets first.
    WeakestArmor,
}

/// How a unit positions itself each activation (§7J).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MovementProfile {
    /// Close on the target — the dumb default.
    #[default]
    Advance,
    /// Stay put (defend / hold a line).
    Hold,
    /// Back away from the nearest enemy (keep range).
    Kite,
    /// Approach the target from the side (lateral).
    Flank,
    /// Rush the nearest enemy (overwhelm), whoever the target is.
    Swarm,
    /// Spread out — move away from the nearest ally (anti-AoE / contagion).
    Disperse,
}
