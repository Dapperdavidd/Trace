use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::DomainError;

/// Token quantity in the token's smallest integer unit. Never a float.
/// Serialized as a decimal string so JSON clients cannot lose precision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BaseUnits(pub u128);

impl BaseUnits {
    pub const ZERO: BaseUnits = BaseUnits(0);

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0.checked_add(other.0).map(Self)
    }

    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0.checked_sub(other.0).map(Self)
    }

    /// Render using `decimals` without going through floating point.
    pub fn to_decimal_string(self, decimals: u8) -> String {
        let d = decimals as usize;
        let raw = self.0.to_string();
        if d == 0 {
            return raw;
        }
        let padded = if raw.len() <= d {
            format!("{}{}", "0".repeat(d - raw.len() + 1), raw)
        } else {
            raw
        };
        let (int, frac) = padded.split_at(padded.len() - d);
        let frac = frac.trim_end_matches('0');
        if frac.is_empty() {
            int.to_string()
        } else {
            format!("{int}.{frac}")
        }
    }
}

impl fmt::Display for BaseUnits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::str::FromStr for BaseUnits {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        s.parse::<u128>()
            .map(Self)
            .map_err(|_| DomainError::InvalidAmount(s.to_string()))
    }
}

impl Serialize for BaseUnits {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for BaseUnits {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_rendering_is_exact() {
        assert_eq!(BaseUnits(1_500_000).to_decimal_string(6), "1.5");
        assert_eq!(BaseUnits(5).to_decimal_string(9), "0.000000005");
        assert_eq!(BaseUnits(42).to_decimal_string(0), "42");
        assert_eq!(BaseUnits(10u128.pow(18)).to_decimal_string(18), "1");
    }

    #[test]
    fn roundtrips_beyond_f64_precision() {
        let big = BaseUnits(u128::MAX);
        let json = serde_json::to_string(&big).unwrap();
        assert_eq!(json, format!("\"{}\"", u128::MAX));
        assert_eq!(serde_json::from_str::<BaseUnits>(&json).unwrap(), big);
    }

    #[test]
    fn checked_math_does_not_wrap() {
        assert!(BaseUnits(u128::MAX).checked_add(BaseUnits(1)).is_none());
        assert!(BaseUnits(1).checked_sub(BaseUnits(2)).is_none());
    }

    #[test]
    fn rejects_negative_and_float_strings() {
        assert!("-1".parse::<BaseUnits>().is_err());
        assert!("1.5".parse::<BaseUnits>().is_err());
    }
}
