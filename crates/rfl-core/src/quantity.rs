// SPDX-License-Identifier: Apache-2.0
// Copyright 2026 RFL Contributors

//! Dimensioned scalar quantities (`spec/01` § Dimensioned scalars).
//!
//! Quantities are carried as their original unit-suffixed strings so that
//! retargeting never reformats a float (the determinism lever, RD1c). A
//! magnitude and unit are parsed only when a value must be clamped.

/// A dimensioned scalar as authored: a magnitude and a unit suffix (e.g. `8 N`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Quantity(pub String);

impl Quantity {
    /// Parse the leading magnitude and the trailing unit, e.g. `"8 N"` -> `(8.0, "N")`.
    /// Returns `None` if the string is not a `<number> <unit>` quantity.
    #[must_use]
    pub fn parse(&self) -> Option<(f64, &str)> {
        let s = self.0.trim();
        let (num, unit) = s.split_once(char::is_whitespace)?;
        let value: f64 = num.trim().parse().ok()?;
        Some((value, unit.trim()))
    }

    /// Build a quantity from an SI magnitude and unit, rounding the magnitude to 6
    /// decimals for byte-deterministic emission (RD1c) and formatting it with the
    /// shortest round-trip representation (`10.0 -> "10"`, `7.5 -> "7.5"`). Used for
    /// retarget-*derived* quantities (grasp-force / acceleration), which, unlike
    /// authored quantities, are computed floats.
    #[must_use]
    pub fn from_si(value: f64, unit: &str) -> Quantity {
        Quantity(format!("{} {}", crate::canonical::round6(value), unit))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_magnitude_and_unit() {
        assert_eq!(Quantity("8 N".into()).parse(), Some((8.0, "N")));
        assert_eq!(Quantity("50 mm".into()).parse(), Some((50.0, "mm")));
        assert_eq!(Quantity("0.05 m/s".into()).parse(), Some((0.05, "m/s")));
    }

    #[test]
    fn rejects_non_quantity() {
        assert_eq!(Quantity("auto".into()).parse(), None);
    }

    #[test]
    fn from_si_formats_shortest_roundtrip() {
        assert_eq!(Quantity::from_si(10.0, "N").0, "10 N");
        assert_eq!(Quantity::from_si(7.5, "N").0, "7.5 N");
        assert_eq!(Quantity::from_si(2.9, "N").0, "2.9 N");
        // 9.80665 / 29 ≈ 0.33816034 -> round6 -> 0.33816.
        assert_eq!(
            Quantity::from_si(9.80665 / 29.0, "m/s^2").0,
            "0.33816 m/s^2"
        );
    }

    #[test]
    fn from_si_is_repeatable() {
        let v = 9.80665 / 29.0;
        assert_eq!(Quantity::from_si(v, "m/s^2"), Quantity::from_si(v, "m/s^2"));
    }
}
