//! Versioned success envelope shared by the AETHER family HTTP contract.

use serde::{Deserialize, Serialize};

/// Metadata carried by a versioned successful API response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiMeta {
    /// Version of the HTTP/JSON contract, for example `v1`.
    pub api_version: String,
}

/// Standard success envelope for versioned HTTP/JSON endpoints.
///
/// This type describes successful responses only; error compatibility remains
/// a separate migration because existing frameworks currently differ.
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
}
