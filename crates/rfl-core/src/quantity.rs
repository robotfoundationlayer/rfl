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
}
