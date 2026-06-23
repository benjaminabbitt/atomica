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

/// A unit's **netrunning doctrine** (`netrunning.md` §10.8) — the *digital* analog of the
/// physical behavior profiles: the **code** the player programs into a runner's deck, scripting
/// **both** who it dives (hack target preference) **and** which program it leads with on a breach
/// (best-fit single, §10.8). Like the other profiles it's a behavior `Override`, so a hostile
/// **Spoof** (`CORRUPTION` priority) can corrupt a runner's doctrine just as it corrupts its
/// targeting — the enemy hacks your script.
///
/// Each doctrine pairs a **target lean** (whom the loadout bites hardest) with a **rider priority**
/// (which offensive program leads); the resolver falls back through the rest of the loadout when the
/// lead isn't loaded or doesn't fit the target. Objective focus (a Datamine node) always comes
/// first, whatever the doctrine.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum NetDoctrine {
    /// **Disabler** (the default) — neutralize chrome. Dives the **most-chromed** enemy (more
    /// slots to trip) and leads with the lock-down riders (Crash / Lag).
    #[default]
    Disabler,
    /// **Burner** — digital *damage*. Hunts **heat-prone** chrome and leads with the burns
    /// (Meltdown / Overheat) — useless against a target running cool, so it picks its marks.
    Burner,
    /// **Saboteur** — soften & spread for the squad. Dives the **biggest threat** and leads with
    /// the openers (Breach / Decrypt / Worm) that set the team's blows up.
    Saboteur,
    /// **Controller** — hijack behavior. Dives the **biggest threat** and leads with the script
    /// corruptions (Spoof / Misfire / Lag) to take the dangerous gun off the board.
    Controller,
    /// **Defender** — hold the net. Dives **enemy runners** first (kill the active defense / the
    /// counter-hacker) and leads with lock-down (Crash / Lag); pairs with the passive wards
    /// (Honeypot / Ghost / Antivirus).
    Defender,
    /// **Sentinel** — *don't dive, screen*. The defensive posture (`netrunning.md` §3): instead of
    /// hacking, the runner spends its digital activation on an active **Guard**, walling the most
    /// exposed **covered ally** (a drone / node) with its own Hacking — single-focus, until its next
    /// turn. Trades all offense for protection; the mirror of Defender. 🔭
    Sentinel,
}
