//! Netrunning — the digital attack, resolved by the core **2d10 roll-under**.
//!
//! A hack is one unit projecting onto the net against another. It runs over the
//! **connection** between them, whose bandwidth is the **weaker endpoint's Link**
//! (the bottleneck), and the attacker's rating averages its **Hacking** with that
//! channel:
//!
//! ```text
//! 2d10 ≤ avg(Hacking, min(Link_attacker, Link_target)) − Ice
//! ```
//!
//! — skill and channel each pull half the weight (a master runner on a thin pipe
//! is dragged down but not gutted). The target's Link feeds the **channel**, not
//! the wall, so a darker target is harder to hack while **defense stays the
//! Ice alone** (Link-blind). Both ends are still hard-gated by reachability —
//! **zero Link** means *no surface to reach* (immune target) or *no presence to
//! reach with* (dark attacker), §7F. Equipment arms the roll through the stats
//! (cyberdeck → Link, skill chip → Hacking, §7D/§13), so there is no separate roll
//! add-on. On a success the hack lands its **payload** — a status (a tripped
//! hack-effect or a deployed program, §7F/§10.8) — with stacks scaling on the
//! **margin** (degree of success).
//!
//! Link's *other* job is **latency → digital initiative**: a unit's own Link sets
//! when it acts on the net (high Link = sooner), independent of the channel.
//!
//! The roll lives in [`resolve_versus`](crate::resolve_versus) (roll-under, Ice as the
//! resist TN); the channel / rating and payload application happen in
//! [`Battle::resolve_hack`](crate::Battle::resolve_hack).

use crate::{RollOutcome, StatusSpec};

/// How many margin points buy one extra payload stack (degree-of-success
/// scaling, §13). Placeholder tuning value — numbers are TBD.
const MARGIN_PER_STACK: i32 = 3;

/// Extra stacks bought by the **degree of success** — the margin, floored by
/// [`MARGIN_PER_STACK`]. `0` at margin ≤ 0, so a marginal breach is a *pure
/// disable* (the §6 floor) with no liability fired.
pub fn margin_stacks(margin: i32) -> u32 {
    (margin.max(0) / MARGIN_PER_STACK) as u32
}

/// The attacker's effective hack rating: its **Hacking** averaged with the
/// **connection channel** (the weaker endpoint's Link bandwidth), floored —
/// `(hacking + channel) / 2`. Skill and channel each carry half the weight, so a
/// thin channel drags a master runner down without gutting it.
pub fn hack_rating(hacking: i32, channel: i32) -> i32 {
    (hacking + channel) / 2
}

/// A unit's **hack capability** — the digital action a deck grants (§7F): it can project onto
/// the net. *Strength* is the unit's Hacking and the connection channel; the **programs** it
/// runs on a breach are a separate loadout on the unit ([`Unit::programs`](crate::Unit)).
#[derive(Clone, Copy, Debug)]
pub struct Hack {
    /// *Legacy* nominal antenna reach. **Superseded:** reach is the unit's **Link**
    /// ([`Unit::hack_reach`](crate::Unit)). Retained on the deck spec; not read by resolution.
    pub range: i32,
    /// Stacks a landed program lands at margin 0; the margin (degree of success) adds more.
    pub base_stacks: u32,
    /// Duration of a landed program's status.
    pub duration: u32,
}

impl Hack {
    pub fn new(range: i32, base_stacks: u32, duration: u32) -> Self {
        Self { range, base_stacks, duration }
    }

    /// Stacks landed for a resolved `outcome`: base + a margin-scaled bonus + a crit bump, or
    /// `0` on failure. Margin floored at `0` so a crit-over-the-wall still lands base.
    pub fn stacks_for(&self, outcome: &RollOutcome) -> u32 {
        if !outcome.success {
            return 0;
        }
        self.base_stacks + margin_stacks(outcome.margin) + outcome.crit as u32
    }
}

/// A **netrunning program** (`netrunning.md`) — a deck-loadout slot, the runner's software. A
/// breach runs the unit's loaded programs; this is the roster a fixer / Halcyon Cybernetics would
/// vendor. Loading one **requires a cyberdeck** ([`Unit::install_program`](crate::Unit)).
///
/// Programs fall in three classes, by *when* they fire (the resolver routes each, §10.8):
/// - **Offensive riders** run on a successful breach against the hacked target — most of the
///   roster. Each is gated to keep the §6 **disable floor**: a *marginal* crack just disables;
///   a program needs a **solid** breach (Crash needs a **crit**) to deploy.
/// - **Breach modifiers** ([`Program::Cascade`] / [`Program::Logicbomb`]) reshape the breach
///   itself — its **breadth** (all implants) or **depth** (past the floor).
/// - **Defensive** programs ([`Program::Honeypot`] / [`Program::Ghost`] / [`Program::Antivirus`])
///   fire on their *own* seam — a hack-back on a repelled intruder, a passive net-defense bonus,
///   a standing worm-cleanse — not on the owner's offensive breach.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Program {
    /// **Lockware** — the lockout drain; a deck's bread-and-butter payload on a target with **no
    /// chrome** to exploit (the generic breach program). Lands an Internal DoT.
    Lockware,
    /// **Overheat** — cooks a target's **heat-prone** chrome (an Internal DoT); a *solid* breach
    /// only, against heat-prone gear.
    Overheat,
    /// **Meltdown** — a premium **heavy** burn (Internal DoT, far harder than Overheat); the
    /// runner's finisher, on any *solid* breach.
    Meltdown,
    /// **Crash** — a digital **stun**; crit-gated like every knockout (a *decisive* breach only).
    Crash,
    /// **Lag** — slows the target's initiative (a `Slow` on a *solid* breach).
    Lag,
    /// **Breach** — pries the target open: a **vulnerability** (incoming damage amped) on a solid
    /// breach, so the squad's next blows bite harder.
    Breach,
    /// **Blind** — corrupts the target's targeting optics: a **Dexterity** debuff (its shots go
    /// wide) on a solid breach.
    Blind,
    /// **Decrypt** — rots the target's **Ice** (a `Worm`-tagged corruption) on a solid
    /// breach, softening it for the next dive.
    Decrypt,
    /// **Leech** — drains the target's **Link** on a solid breach: a thinner channel and slower
    /// digital initiative, and it edges toward going dark (unreachable).
    Leech,
    /// **Worm** — deploys a **contagious** Ice-rot ([`Corruption::worm_swarm`](crate::Corruption))
    /// that rides the net to nearby surfaces; the spreader, on a solid breach.
    Worm,
    /// **Cascade** — a *breach modifier*: forces a meshed target's breach to trip **every** implant
    /// (not just on a crit), on a solid breach.
    Cascade,
    /// **Logicbomb** — a *breach modifier*: a planted bomb force-fires the tripped chrome's degrade
    /// liabilities **past the disable floor** (even a marginal breach detonates it).
    Logicbomb,
    /// **Spoof** — corrupts the target's **targeting** script (a `CORRUPTION`-priority override) for
    /// several ticks: it chases the wrong enemy. On a solid breach.
    Spoof,
    /// **Misfire** — corrupts the target's **movement** routine (a brief `CORRUPTION` override): it
    /// backs off / scatters instead of pressing. A cheap, short scramble.
    Misfire,
    /// **Honeypot** — a *defensive* counter-ICE: a repelled intruder (a hack that **fails** against
    /// this unit) gets bitten back — its deck fried by the trap.
    Honeypot,
    /// **Ghost** — a *defensive* stealth suite: the runner reads **darker**, a passive bonus to its
    /// own [net defense](crate::Battle) (harder to hack back).
    Ghost,
    /// **Antivirus** — a *defensive* ward: a standing cleanse that strips **worm** corruption off
    /// its owner each tick (the digital counterplay, loaded as a program).
    Antivirus,
}

impl Program {
    /// The full roster, in resolution order (offensive riders, then breach modifiers, then the
    /// defensive trio).
    pub const ALL: [Program; 17] = [
        Program::Lockware,
        Program::Overheat,
        Program::Meltdown,
        Program::Crash,
        Program::Lag,
        Program::Breach,
        Program::Blind,
        Program::Decrypt,
        Program::Leech,
        Program::Worm,
        Program::Cascade,
        Program::Logicbomb,
        Program::Spoof,
        Program::Misfire,
        Program::Honeypot,
        Program::Ghost,
        Program::Antivirus,
    ];
    const COUNT: usize = Self::ALL.len();

    /// The **status payload** a DoT/control rider program lands (the resolver scales its stacks /
    /// duration). Only the status-shaped programs map here; the stat-debuff, corruption, behavior,
    /// breach-modifier, and defensive programs have no `StatusSpec` and are routed directly by the
    /// resolver (§10.8) — calling this on one is a programming error.
    pub fn payload(self) -> StatusSpec {
        match self {
            Program::Lockware => StatusSpec::lockware(),
            Program::Overheat => StatusSpec::overheat(),
            Program::Meltdown => StatusSpec::meltdown(),
            Program::Crash => StatusSpec::crash(),
            Program::Lag => StatusSpec::lag(),
            Program::Breach => StatusSpec::breach(),
            other => unreachable!("{other:?} is not a status-payload program"),
        }
    }
}

/// A unit's **loaded program set** — its deck loadout (`netrunning.md`). A small `Copy` set
/// keyed by [`Program`]; build with [`Programs::with`], test with [`Programs::has`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Programs {
    loaded: [bool; Program::COUNT],
}

impl Programs {
    pub const NONE: Programs = Programs { loaded: [false; Program::COUNT] };
    /// A set of one program.
    pub const fn just(p: Program) -> Programs {
        Programs::NONE.with(p)
    }
    /// This set with `p` loaded.
    pub const fn with(mut self, p: Program) -> Programs {
        self.loaded[p as usize] = true;
        self
    }
    /// Is `p` loaded?
    pub const fn has(self, p: Program) -> bool {
        self.loaded[p as usize]
    }
    /// The loaded programs, in [`Program::ALL`] order.
    pub fn iter(self) -> impl Iterator<Item = Program> {
        Program::ALL.into_iter().filter(move |&p| self.has(p))
    }
    /// The **generic breach payload** landed on a chrome-less target — **Lockware**, the deck's
    /// bread-and-butter drain (every other program is a margin-gated rider the resolver runs
    /// separately, so only the reliable lockout lands here). `None` if no Lockware is loaded.
    pub fn breach_payload(self) -> Option<StatusSpec> {
        self.has(Program::Lockware).then(StatusSpec::lockware)
    }
}

/// The result of a [`Battle::resolve_hack`](crate::Battle::resolve_hack).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HackResult {
    /// The attacker carries no hack loadout — nothing to resolve.
    NoHack,
    /// Attacker has zero Link — it can't project onto the net (no roll, §7D).
    Offline,
    /// Target has zero Link — no surface to reach; immune (no roll, §7D/§7F).
    NoSurface,
    /// The contest was rolled. `stacks` is what landed (`0` ⇒ the hack whiffed).
    Rolled { outcome: RollOutcome, stacks: u32 },
}

impl HackResult {
    /// Did the hack succeed — i.e. **breach** the target? A success always at
    /// least *disables* a tripped implant (the §6 floor); `stacks` may still be 0
    /// (a marginal breach fires no liability).
    pub fn landed(&self) -> bool {
        matches!(self, HackResult::Rolled { outcome, .. } if outcome.success)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{resolve_versus, ScriptedRng};

    fn hack() -> Hack {
        Hack::new(2, 1, 5)
    }

    #[test]
    fn rating_averages_skill_with_the_channel_floored() {
        assert_eq!(hack_rating(6, 4), 5);
        assert_eq!(hack_rating(6, 1), 3); // 3.5 → floor 3
        assert_eq!(hack_rating(9, 2), 5); // 5.5 → floor 5
        assert_eq!(hack_rating(0, 0), 0);
    }

    #[test]
    fn margin_scales_the_stacks() {
        // Versus (resist as a modifier): target = rating 14 − ice 4 = 10; 2d10 = 7 ⇒ margin 3.
        let mut rng = ScriptedRng::from_d10([3, 4]);
        let o = resolve_versus(&mut rng, 14, 4);
        assert_eq!(o.margin, 3);
        assert_eq!(hack().stacks_for(&o), 2); // base 1 + 3/3
    }

    #[test]
    fn deeper_margin_lands_more() {
        // Lower dice ⇒ deeper margin under target 15 (rating 15, no ice); 2d10 = 5 ⇒ margin 10.
        let mut rng = ScriptedRng::from_d10([2, 3]);
        let o = resolve_versus(&mut rng, 15, 0);
        assert_eq!(o.margin, 10);
        assert_eq!(hack().stacks_for(&o), 1 + 3); // base 1 + 10/3
    }

    #[test]
    fn a_whiff_lands_nothing() {
        // target = 21 + 0 − 20 = 1; 2d10 = 6 misses (and isn't a 2–3 crit).
        let mut rng = ScriptedRng::from_d10([3, 3]);
        let o = resolve_versus(&mut rng, 0, 20);
        assert!(!o.success);
        assert_eq!(hack().stacks_for(&o), 0);
    }

    #[test]
    fn a_crit_lands_despite_the_wall_and_adds_a_stack() {
        let mut rng = ScriptedRng::from_d10([1, 2]); // natural 3 → roll-under crit
        let o = resolve_versus(&mut rng, 0, 99); // hopeless target, but a crit lands
        assert!(o.crit && o.success);
        assert_eq!(hack().stacks_for(&o), 1 + 1); // base + crit bump (margin floored at 0)
    }
}
