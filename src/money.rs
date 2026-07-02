use crate::errors::LedgerError;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// A validated monetary amount.
///
/// Invariants enforced at construction, everywhere, once:
/// - always strictly positive (zero and negative amounts are rejected)
/// - always rounded to 2 decimal places
///
/// Every `Event` variant that carries an amount stores `Money`, not a bare
/// `Decimal`, so "no negative expenses" is a type-level guarantee instead of
/// a convention every call site has to remember to uphold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(try_from = "Decimal", into = "Decimal")]
pub struct Money(Decimal);

impl Money {
    pub fn new(amount: Decimal) -> Result<Self, LedgerError> {
        if amount.is_sign_negative() {
            return Err(LedgerError::Validation(format!(
                "amount must be positive, got {amount}"
            )));
        }
        if amount.is_zero() {
            return Err(LedgerError::Validation(
                "amount must be greater than zero".to_string(),
            ));
        }
        Ok(Self(amount.round_dp(2)))
    }

    pub fn as_decimal(self) -> Decimal {
        self.0
    }
}

impl TryFrom<Decimal> for Money {
    type Error = LedgerError;
    fn try_from(value: Decimal) -> Result<Self, Self::Error> {
        Money::new(value)
    }
}

impl From<Money> for Decimal {
    fn from(m: Money) -> Decimal {
        m.0
    }
}

impl FromStr for Money {
    type Err = LedgerError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let d = Decimal::from_str(s.trim())
            .map_err(|_| LedgerError::Parse(format!("invalid amount: {s}")))?;
        Money::new(d)
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "₹{:.2}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_negative_and_zero() {
        assert!(Money::new(Decimal::new(-100, 2)).is_err());
        assert!(Money::new(Decimal::ZERO).is_err());
    }

    #[test]
    fn rounds_to_two_dp() {
        let m = Money::new(Decimal::new(123456, 3)).unwrap(); // 123.456
        assert_eq!(m.as_decimal(), Decimal::new(12346, 2)); // 123.46
    }

    #[test]
    fn from_str_rejects_garbage() {
        assert!("not-a-number".parse::<Money>().is_err());
        assert!("-5".parse::<Money>().is_err());
    }
}
