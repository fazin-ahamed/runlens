//! ULID generation for RunLens identifiers.
//!
//! ULIDs are 26-char Crockford-base32 strings encoding 48-bit time + 80-bit
//! randomness. We always emit them in canonical lowercase form.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A RunLens identifier: ULID, normalised.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Identifier(String);

impl Identifier {
    /// Create from an existing ULID string. Returns None if input is invalid.
    pub fn from_string(s: &str) -> Option<Self> {
        let normalised = s.trim().to_ascii_lowercase();
        if ulid::Ulid::from_string(&normalised).is_ok() {
            Some(Self(normalised))
        } else {
            None
        }
    }

    /// Create a new ULID for the current wall-clock instant with fresh randomness.
    pub fn now() -> Self {
        Self(ulid::Ulid::new().to_string().to_ascii_lowercase())
    }

    /// Create a monotonically increasing ULID inside a process. Useful for event
    /// ordering when generating many identifiers within the same millisecond.
    pub fn monotonic(generator: &mut ulid::Generator) -> Self {
        Self(generator.generate().unwrap_or_else(|_| ulid::Ulid::new()).to_string())
    }

    /// Borrow the canonical string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Identifier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::str::FromStr for Identifier {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_string(s).ok_or_else(|| format!("invalid identifier: {s}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let id = Identifier::now();
        let parsed = Identifier::from_string(id.as_str()).unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn rejects_invalid() {
        assert!(Identifier::from_string("not-a-ulid").is_none());
        assert!(Identifier::from_string("").is_none());
        assert!(Identifier::from_string("01ARZ3NDEKTSV4RRFFQ69G5FAVEXTRA").is_none());
    }

    #[test]
    fn normalises_case() {
        let id = "01arz3ndektsv4rrffq69g5fav";
        let parsed = Identifier::from_string(id).unwrap();
        assert_eq!(parsed.as_str(), "01arz3ndektsv4rrffq69g5fav");
        let upper = Identifier::from_string(&id.to_ascii_uppercase()).unwrap();
        assert_eq!(upper.as_str(), id);
    }

    #[test]
    fn monotonic_is_strictly_ordered_for_tiny_minute() {
        let mut g = ulid::Generator::new();
        let a = Identifier::monotonic(&mut g);
        let b = Identifier::monotonic(&mut g);
        let c = Identifier::monotonic(&mut g);
        assert!(a.as_str() <= b.as_str());
        assert!(b.as_str() <= c.as_str());
    }
}
