//! Layered, typed configuration.
//!
//! Precedence (later wins): **defaults < files (in the order added) < environment**.
//!
//! Environment variables use a prefix and double underscores as separators:
//! `CATALOG__SERVER__ADDR=0.0.0.0:8080` sets `server.addr` when the prefix is
//! `CATALOG`. Keys are lower-cased. Values stay strings and are coerced to
//! numbers/booleans only when the target field asks for it.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::coerce::ValueDe;
use crate::validate::{Validate, ValidationErrors};

/// Errors produced while loading or reading configuration.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigError {
    /// A required configuration file could not be read.
    #[error("failed to read config file `{path}`: {source}")]
    Read {
        /// File that failed.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A configuration file is not valid TOML/JSON.
    #[error("failed to parse config file `{path}`: {message}")]
    Parse {
        /// File that failed.
        path: PathBuf,
        /// Parser message (without the file contents).
        message: String,
    },
    /// The file extension is neither `.toml` nor `.json`.
    #[error("unsupported config file `{path}`: expected a .toml or .json extension")]
    UnsupportedFormat {
        /// Offending file.
        path: PathBuf,
    },
    /// A value has the wrong shape or type for the target struct.
    #[error("invalid configuration at `{path}`: {message}")]
    Invalid {
        /// Dotted path of the offending value.
        path: String,
        /// What went wrong.
        message: String,
    },
    /// The section deserialized but failed its [`Validate`] rules.
    #[error("invalid configuration in `{section}`: {errors}")]
    Validation {
        /// Section path.
        section: String,
        /// Every rule that failed.
        errors: ValidationErrors,
    },
}

/// Merged configuration tree.
///
/// Read typed sections with [`Config::section`] or
/// [`Config::section_validated`]. `Debug` lists only top-level keys so values
/// (which may contain secrets) never end up in logs.
#[derive(Clone, Default)]
pub struct Config {
    root: Value,
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let keys: Vec<&str> = match &self.root {
            Value::Object(map) => map.keys().map(String::as_str).collect(),
            _ => Vec::new(),
        };
        f.debug_struct("Config").field("sections", &keys).finish()
    }
}

impl Config {
    /// Builds a configuration from an already-merged JSON tree.
    pub fn from_value(root: Value) -> Self {
        Self { root }
    }

    /// Returns the raw value at a dotted path (`"server.addr"`), if present.
    /// An empty path returns the root.
    pub fn raw(&self, path: &str) -> Option<&Value> {
        if path.is_empty() {
            return Some(&self.root);
        }
        path.split('.')
            .try_fold(&self.root, |node, key| node.as_object()?.get(key))
    }

    /// Returns `true` if a value exists at `path`.
    pub fn contains(&self, path: &str) -> bool {
        self.raw(path).is_some()
    }

    /// Deserializes the section at `path` into `T`.
    ///
    /// A missing section is treated as an empty table, so structs using
    /// `#[serde(default)]` get their defaults and structs with required
    /// fields report exactly which field is missing.
    pub fn section<T: DeserializeOwned>(&self, path: &str) -> Result<T, ConfigError> {
        let value = self
            .raw(path)
            .cloned()
            .unwrap_or_else(|| Value::Object(Map::new()));
        serde_path_to_error::deserialize(ValueDe(value)).map_err(|err| {
            let inner = err.path().to_string();
            let full = match (path.is_empty(), inner.as_str()) {
                (true, _) => inner.clone(),
                (false, "." | "") => path.to_owned(),
                (false, _) => format!("{path}.{inner}"),
            };
            ConfigError::Invalid {
                path: full,
                message: err.into_inner().to_string(),
            }
        })
    }

    /// Like [`Config::section`], then runs the type's [`Validate`] rules.
    pub fn section_validated<T: DeserializeOwned + Validate>(
        &self,
        path: &str,
    ) -> Result<T, ConfigError> {
        let value: T = self.section(path)?;
        value.validate().map_err(|errors| ConfigError::Validation {
            section: path.to_owned(),
            errors,
        })?;
        Ok(value)
    }
}

#[derive(Debug, Clone)]
struct FileSource {
    path: PathBuf,
    required: bool,
}

/// Builder that merges configuration sources in a fixed, documented order.
#[derive(Debug, Clone, Default)]
pub struct ConfigLoader {
    defaults: Option<Value>,
    files: Vec<FileSource>,
    env_prefix: Option<String>,
    env_override: Option<Vec<(String, String)>>,
}

impl ConfigLoader {
    /// Creates an empty loader (no sources).
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the lowest-precedence defaults.
    pub fn defaults(mut self, defaults: Value) -> Self {
        self.defaults = Some(defaults);
        self
    }

    /// Adds a file that must exist. Format is chosen by extension (`.toml`/`.json`).
    pub fn file(mut self, path: impl AsRef<Path>) -> Self {
        self.files.push(FileSource {
            path: path.as_ref().to_path_buf(),
            required: true,
        });
        self
    }

    /// Adds a file that is skipped when it does not exist.
    pub fn optional_file(mut self, path: impl AsRef<Path>) -> Self {
        self.files.push(FileSource {
            path: path.as_ref().to_path_buf(),
            required: false,
        });
        self
    }

    /// Reads environment variables starting with `{prefix}__`.
    pub fn env_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.env_prefix = Some(prefix.into());
        self
    }

    /// Replaces the process environment with an explicit list (useful in tests).
    /// Only has an effect together with [`ConfigLoader::env_prefix`].
    pub fn env_vars<I, K, V>(mut self, vars: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        self.env_override = Some(
            vars.into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        );
        self
    }

    /// Loads and merges every source.
    pub fn load(self) -> Result<Config, ConfigError> {
        let mut root = Value::Object(Map::new());
        if let Some(defaults) = self.defaults {
            merge(&mut root, defaults);
        }
        for source in &self.files {
            if let Some(value) = read_file(source)? {
                merge(&mut root, value);
            }
        }
        if let Some(prefix) = &self.env_prefix {
            let vars: Vec<(String, String)> = match self.env_override {
                Some(vars) => vars,
                None => std::env::vars().collect(),
            };
            merge(&mut root, env_tree(prefix, vars));
        }
        Ok(Config { root })
    }
}

fn read_file(source: &FileSource) -> Result<Option<Value>, ConfigError> {
    let path = &source.path;
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound && !source.required => {
            return Ok(None);
        }
        Err(source) => {
            return Err(ConfigError::Read {
                path: path.clone(),
                source,
            });
        }
    };
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    let parsed = match ext.as_deref() {
        Some("toml") => toml::from_str::<Value>(&text).map_err(|e| e.message().to_owned()),
        Some("json") => serde_json::from_str::<Value>(&text).map_err(|e| e.to_string()),
        _ => {
            return Err(ConfigError::UnsupportedFormat { path: path.clone() });
        }
    };
    parsed.map(Some).map_err(|message| ConfigError::Parse {
        path: path.clone(),
        message,
    })
}

fn env_tree(prefix: &str, vars: Vec<(String, String)>) -> Value {
    let marker = format!("{}__", prefix.to_ascii_uppercase());
    let mut root = Value::Object(Map::new());
    let mut vars: Vec<_> = vars
        .into_iter()
        .filter_map(|(key, value)| {
            let rest = key.to_ascii_uppercase().strip_prefix(&marker)?.to_owned();
            let segments: Vec<String> = rest.split("__").map(str::to_ascii_lowercase).collect();
            if segments.iter().any(String::is_empty) {
                return None;
            }
            Some((segments, value))
        })
        .collect();
    // Deterministic result regardless of environment ordering.
    vars.sort();
    for (segments, value) in vars {
        insert_path(&mut root, &segments, value);
    }
    root
}

fn insert_path(node: &mut Value, segments: &[String], value: String) {
    let Some((first, rest)) = segments.split_first() else {
        return;
    };
    if !node.is_object() {
        *node = Value::Object(Map::new());
    }
    let Value::Object(map) = node else {
        return;
    };
    if rest.is_empty() {
        map.insert(first.clone(), Value::String(value));
    } else {
        let child = map
            .entry(first.clone())
            .or_insert_with(|| Value::Object(Map::new()));
        insert_path(child, rest, value);
    }
}

/// Deep-merges `overlay` into `base`; objects merge recursively, anything else replaces.
fn merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                match base_map.get_mut(&key) {
                    Some(existing) => merge(existing, value),
                    None => {
                        base_map.insert(key, value);
                    }
                }
            }
        }
        (base, overlay) => *base = overlay,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Debug, Deserialize, PartialEq)]
    #[serde(default, deny_unknown_fields)]
    struct Server {
        port: u16,
        debug: bool,
        name: String,
    }

    impl Default for Server {
        fn default() -> Self {
            Self {
                port: 3000,
                debug: false,
                name: "app".into(),
            }
        }
    }

    impl Validate for Server {
        fn validate(&self) -> Result<(), ValidationErrors> {
            let mut errors = ValidationErrors::new();
            errors.check(self.port != 0, "port", "range", "port must not be 0");
            errors.into_result()
        }
    }

    fn temp_file(name: &str, contents: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("liga-core-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        std::fs::write(&path, contents).unwrap();
        path
    }

    #[test]
    fn missing_section_uses_defaults() {
        let config = ConfigLoader::new().load().unwrap();
        assert_eq!(
            config.section::<Server>("server").unwrap(),
            Server::default()
        );
    }

    #[test]
    fn precedence_is_defaults_then_file_then_env() {
        let file = temp_file(
            "precedence.toml",
            "[server]\nport = 4000\nname = \"from-file\"\n",
        );
        let config = ConfigLoader::new()
            .defaults(json!({"server": {"port": 1, "debug": true}}))
            .file(&file)
            .env_prefix("APP")
            .env_vars([("APP__SERVER__PORT", "5000"), ("OTHER__SERVER__PORT", "9")])
            .load()
            .unwrap();
        let server: Server = config.section("server").unwrap();
        assert_eq!(
            server,
            Server {
                port: 5000,
                debug: true,
                name: "from-file".into()
            }
        );
    }

    #[test]
    fn env_strings_coerce_to_target_types() {
        let config = ConfigLoader::new()
            .env_prefix("APP")
            .env_vars([
                ("APP__SERVER__DEBUG", "true"),
                ("APP__SERVER__NAME", "12345"),
            ])
            .load()
            .unwrap();
        let server: Server = config.section("server").unwrap();
        assert!(server.debug);
        assert_eq!(server.name, "12345");
    }

    #[test]
    fn invalid_value_reports_full_path() {
        let config = ConfigLoader::new()
            .env_prefix("APP")
            .env_vars([("APP__SERVER__PORT", "not-a-port")])
            .load()
            .unwrap();
        let err = config.section::<Server>("server").unwrap_err();
        match err {
            ConfigError::Invalid { path, .. } => assert_eq!(path, "server.port"),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let config = Config::from_value(json!({"server": {"prot": 1}}));
        assert!(matches!(
            config.section::<Server>("server"),
            Err(ConfigError::Invalid { .. })
        ));
    }

    #[test]
    fn validation_runs_after_deserialization() {
        let config = Config::from_value(json!({"server": {"port": 0}}));
        let err = config.section_validated::<Server>("server").unwrap_err();
        assert!(matches!(err, ConfigError::Validation { .. }));
        assert!(err.to_string().contains("port must not be 0"));
    }

    #[test]
    fn required_file_must_exist_but_optional_may_not() {
        let missing = std::env::temp_dir().join("liga-definitely-missing.toml");
        assert!(matches!(
            ConfigLoader::new().file(&missing).load(),
            Err(ConfigError::Read { .. })
        ));
        assert!(ConfigLoader::new().optional_file(&missing).load().is_ok());
    }

    #[test]
    fn parse_errors_do_not_echo_file_contents() {
        let file = temp_file("broken.toml", "token = \"super-secret\nport = ");
        let err = ConfigLoader::new().file(&file).load().unwrap_err();
        assert!(matches!(err, ConfigError::Parse { .. }));
        assert!(!err.to_string().contains("super-secret"));
    }

    #[test]
    fn debug_does_not_print_values() {
        let config = Config::from_value(json!({"db": {"password": "hunter2"}}));
        let debug = format!("{config:?}");
        assert!(debug.contains("db"));
        assert!(!debug.contains("hunter2"));
    }
}
