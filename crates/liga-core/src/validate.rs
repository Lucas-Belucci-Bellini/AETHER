//! Validation contract shared by configuration and request bodies.

use std::borrow::Cow;
use std::fmt;

use serde::Serialize;

/// Types that can check their own invariants after deserialization.
///
/// Implement it with an empty body (`impl Validate for MyType {}`) to state
/// explicitly that a type has no extra rules.
pub trait Validate {
    /// Returns every failed rule, or `Ok(())`.
    fn validate(&self) -> Result<(), ValidationErrors> {
        Ok(())
    }
}

/// A single failed rule.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FieldError {
    /// Field path, e.g. `"price"` or `"items[2].name"`.
    pub field: String,
    /// Stable machine-readable code, e.g. `"required"` or `"range"`.
    pub code: Cow<'static, str>,
    /// Human-readable explanation that is safe to show to clients.
    pub message: String,
}

/// A collection of failed rules.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct ValidationErrors {
    errors: Vec<FieldError>,
}

impl ValidationErrors {
    /// Creates an empty collection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a failed rule.
    pub fn add(
        &mut self,
        field: impl Into<String>,
        code: impl Into<Cow<'static, str>>,
        message: impl Into<String>,
    ) -> &mut Self {
        self.errors.push(FieldError {
            field: field.into(),
            code: code.into(),
            message: message.into(),
        });
        self
    }

    /// Records a failed rule when `ok` is `false`.
    pub fn check(
        &mut self,
        ok: bool,
        field: impl Into<String>,
        code: impl Into<Cow<'static, str>>,
        message: impl Into<String>,
    ) -> &mut Self {
        if !ok {
            self.add(field, code, message);
        }
        self
    }

    /// `true` when no rule failed.
    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    /// Number of failed rules.
    pub fn len(&self) -> usize {
        self.errors.len()
    }

    /// Iterates over the failed rules.
    pub fn iter(&self) -> impl Iterator<Item = &FieldError> {
        self.errors.iter()
    }

    /// `Ok(())` when empty, otherwise `Err(self)`.
    pub fn into_result(self) -> Result<(), Self> {
        if self.is_empty() { Ok(()) } else { Err(self) }
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, error) in self.errors.iter().enumerate() {
            if i > 0 {
                f.write_str("; ")?;
            }
            write!(f, "{}: {}", error.field, error.message)?;
        }
        Ok(())
    }
}

impl<'a> IntoIterator for &'a ValidationErrors {
    type Item = &'a FieldError;
    type IntoIter = std::slice::Iter<'a, FieldError>;

    fn into_iter(self) -> Self::IntoIter {
        self.errors.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collects_and_displays_errors() {
        let mut errors = ValidationErrors::new();
        errors
            .check(true, "ok", "never", "not recorded")
            .check(false, "name", "required", "name is required")
            .add("price", "range", "price must be positive");
        assert_eq!(errors.len(), 2);
        assert_eq!(
            errors.to_string(),
            "name: name is required; price: price must be positive"
        );
        assert!(errors.into_result().is_err());
    }
}
