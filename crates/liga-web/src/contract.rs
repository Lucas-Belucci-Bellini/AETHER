//! Versioned success and error envelopes for the AETHER family HTTP contract.

use serde::{Deserialize, Serialize};

/// Metadata carried by a versioned HTTP response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiMeta {
    /// Version of the HTTP/JSON contract, for example `v1`.
    pub api_version: String,
}

/// Standard success envelope for versioned HTTP/JSON endpoints.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiEnvelope<T> {
    /// Route-specific response data.
    pub data: T,
    /// Contract metadata.
    pub meta: ApiMeta,
}

impl<T> ApiEnvelope<T> {
    /// Wrap route-specific data in a versioned success envelope.
    pub fn new(api_version: impl Into<String>, data: T) -> Self {
        Self {
            data,
            meta: ApiMeta {
                api_version: api_version.into(),
            },
        }
    }

    /// Wrap route-specific data in the current family contract version.
    pub fn v1(data: T) -> Self {
        Self::new("v1", data)
    }
}

/// Minimal machine-readable and human-readable error fields in contract v1.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorBody {
    /// Stable code intended for programmatic handling.
    pub code: String,
    /// Safe message intended for people; never include internal diagnostics.
    pub message: String,
}

/// Error envelope for versioned route adapters.
///
/// This is a wire-contract type, not a global replacement for each framework's
/// legacy error responses. Adapters should use it for errors that explicitly
/// opt into the family HTTP/JSON v1 contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiErrorEnvelope {
    /// Structured error information.
    pub error: ApiErrorBody,
    /// Contract metadata.
    pub meta: ApiMeta,
}

impl ApiErrorEnvelope {
    /// Create an error using an explicit contract version.
    pub fn new(
        api_version: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            error: ApiErrorBody {
                code: code.into(),
                message: message.into(),
            },
            meta: ApiMeta {
                api_version: api_version.into(),
            },
        }
    }

    /// Create an error using the current family contract version.
    pub fn v1(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::new("v1", code, message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn v1_serializes_to_the_family_success_envelope() {
        let response = ApiEnvelope::v1(json!({
            "message": "Olá do AETHER",
            "environment": "test"
        }));

        assert_eq!(
            serde_json::to_value(response).expect("envelope serializes"),
            json!({
                "data": {
                    "message": "Olá do AETHER",
                    "environment": "test"
                },
                "meta": {
                    "api_version": "v1"
                }
            })
        );
    }

    #[test]
    fn v1_error_serializes_to_the_family_error_envelope() {
        let response = ApiErrorEnvelope::v1("not_found", "recurso não encontrado");

        assert_eq!(
            serde_json::to_value(response).expect("error envelope serializes"),
            json!({
                "error": {
                    "code": "not_found",
                    "message": "recurso não encontrado"
                },
                "meta": {
                    "api_version": "v1"
                }
            })
        );
    }
}
