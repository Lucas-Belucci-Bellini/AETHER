//! A value that must never be printed, logged or serialized.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Wraps sensitive data (API keys, passwords, tokens).
///
/// `Debug`, `Display` and `Serialize` all emit `***`. Read the value only
/// where it is needed with [`Secret::expose`].
#[derive(Clone, Default, PartialEq, Eq)]
pub struct Secret<T>(T);

impl<T> Secret<T> {
    /// Wraps a value.
    pub fn new(value: T) -> Self {
        Self(value)
    }

    /// Borrows the secret value. Keep the borrow as short as possible.
    pub fn expose(&self) -> &T {
        &self.0
    }

    /// Unwraps the secret value.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Secret(***)")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("***")
    }
}

impl<T> Serialize for Secret<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str("***")
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Secret<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Secret)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn never_leaks_through_formatting_or_serde() {
        let secret: Secret<String> = serde_json::from_str("\"sk-live-123\"").unwrap();
        assert_eq!(secret.expose(), "sk-live-123");
        assert_eq!(format!("{secret:?}"), "Secret(***)");
        assert_eq!(secret.to_string(), "***");
        assert_eq!(serde_json::to_string(&secret).unwrap(), "\"***\"");
    }
}
