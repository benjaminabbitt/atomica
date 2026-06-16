//! The 3-tier armor matrix: damage *type* vs target *armor class*.
//!
//! This is the "types ≠ statuses" rule's mitigation half — damage type governs
//! only how much a hit is reduced. The multipliers below encode the *shape* from
//! the design, not balanced numbers (all values are TBD):
//! - **Bludgeoning** (anti-rigid) — beats Plate, soaked by Padding.
//! - **Piercing** (universal penetrator) — only Plate resists.
//! - **Slashing** (anti-unarmored) — great vs Padding, poor vs Plate.

use crate::DamageType;

/// Target armor class for the matrix. (Distinct from the Plating layer's material
/// flavor — Composite/Reactive/Ablative — which is a separate, later concern.)
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ArmorClass {
    Padding,
    #[default]
    Mail,
    Plate,
}

/// Damage multiplier for `dtype` striking `armor`. Placeholder values (TBD).
pub fn matrix(dtype: DamageType, armor: ArmorClass) -> f32 {
    use ArmorClass::*;
    use DamageType::*;
    match (dtype, armor) {
        (Bludgeoning, Padding) => 0.5,
        (Bludgeoning, Mail) => 1.0,
        (Bludgeoning, Plate) => 1.5,

        (Piercing, Padding) => 1.0,
        (Piercing, Mail) => 1.0,
        (Piercing, Plate) => 0.5,

        (Slashing, Padding) => 1.5,
        (Slashing, Mail) => 1.0,
        (Slashing, Plate) => 0.5,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DamageType::*;

    #[test]
    fn shape_holds() {
        // Bludgeoning best vs Plate, worst vs Padding.
        assert!(matrix(Bludgeoning, ArmorClass::Plate) > matrix(Bludgeoning, ArmorClass::Padding));
        // Slashing best vs Padding, worst vs Plate.
        assert!(matrix(Slashing, ArmorClass::Padding) > matrix(Slashing, ArmorClass::Plate));
        // Piercing only resisted by Plate.
        assert_eq!(matrix(Piercing, ArmorClass::Padding), matrix(Piercing, ArmorClass::Mail));
        assert!(matrix(Piercing, ArmorClass::Plate) < matrix(Piercing, ArmorClass::Mail));
    }
}
