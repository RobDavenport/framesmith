mod apply;
mod matchers;
mod property_schema;
mod registry;
mod validate;

pub use apply::*;
pub use matchers::*;
pub use property_schema::*;
pub use registry::*;
pub use validate::*;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

const RULES_VERSION: u32 = 1;

/// Root structure for a Framesmith rules file.
/// Rules files define:
/// - a registry of known resources/events (`registry`)
/// - default values (`apply`)
/// - validation constraints (`validate`)
/// - property schemas for strict validation (`properties`)
/// - tag schemas for strict validation (`tags`)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RulesFile {
    /// Schema version. Must be 1.
    pub version: u32,
    /// Optional registry of resources and events used by this project/character.
    pub registry: Option<RulesRegistry>,
    /// Rules that set default values on matching moves.
    #[serde(default)]
    pub apply: Vec<ApplyRule>,
    /// Rules that enforce constraints on matching moves.
    #[serde(default)]
    pub validate: Vec<ValidateRule>,
    /// Property schema definitions. When present, enables strict validation
    /// requiring all properties to be declared in the schema.
    #[serde(default)]
    pub properties: Option<PropertySchema>,
    /// Tag schema definitions. When present, enables strict validation
    /// requiring all tags to be declared in the schema.
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

/// Severity level for validation rule violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// Errors indicate invalid data that must be fixed.
    Error,
    /// Warnings indicate potential issues but don't block saving.
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    pub field: String,
    pub message: String,
    pub severity: Severity,
}

/// Generates the JSON Schema for RulesFile.
pub fn generate_rules_schema() -> schemars::Schema {
    schemars::schema_for!(RulesFile)
}

#[derive(Debug)]
pub enum RulesError {
    Io(std::io::Error),
    Json(serde_json::Error),
    UnsupportedVersion(u32),
}

impl std::fmt::Display for RulesError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(err) => write!(f, "io error: {err}"),
            Self::Json(err) => write!(f, "json error: {err}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "unsupported rules version: {version}")
            }
        }
    }
}

impl std::error::Error for RulesError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            Self::Json(err) => Some(err),
            Self::UnsupportedVersion(_) => None,
        }
    }
}

impl From<std::io::Error> for RulesError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<serde_json::Error> for RulesError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

pub fn load_rules_file(path: &std::path::Path) -> Result<Option<RulesFile>, RulesError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(err) => return Err(err.into()),
    };
    let rules = serde_json::from_slice::<RulesFile>(&bytes)?;

    if rules.version != RULES_VERSION {
        return Err(RulesError::UnsupportedVersion(rules.version));
    }

    Ok(Some(rules))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_match_string_and_array() {
        let rules: RulesFile = serde_json::from_str(
            r#"{
  "version": 1,
  "apply": [
    {
      "match": { "type": ["normal", "special"], "button": "L", "input": ["5L", "2L"] },
      "set": { "hitstop": 3 }
    }
  ],
  "validate": [
    {
      "match": { "guard": "mid", "tags": ["starter", "reversal"] },
      "require": { "animation": { "exists": true } },
      "severity": "warning",
      "message": "needs animation"
    }
  ]
}"#,
        )
        .unwrap();

        assert_eq!(rules.apply.len(), 1);
        assert_eq!(rules.validate.len(), 1);

        let m = &rules.apply[0].match_spec;
        assert_eq!(
            m.r#type,
            Some(StringOrVec::Many(vec![
                "normal".to_string(),
                "special".to_string()
            ]))
        );
        assert_eq!(m.button, Some(StringOrVec::One("L".to_string())));
        assert_eq!(
            m.input,
            Some(StringOrVec::Many(vec!["5L".to_string(), "2L".to_string()]))
        );

        assert!(matches!(rules.apply[0].set, serde_json::Value::Object(_)));
        assert!(matches!(
            rules.validate[0].require,
            serde_json::Value::Object(_)
        ));

        let vm = &rules.validate[0].match_spec;
        assert_eq!(vm.guard, Some(StringOrVec::One("mid".to_string())));
        assert_eq!(
            vm.tags,
            Some(vec!["starter".to_string(), "reversal".to_string()])
        );
    }

    #[test]
    fn test_missing_sections_default_empty() {
        let rules: RulesFile = serde_json::from_str(r#"{"version": 1}"#).unwrap();
        assert!(rules.apply.is_empty());
        assert!(rules.validate.is_empty());
    }

    #[test]
    fn test_load_rules_file_unsupported_version() {
        let mut path = std::env::temp_dir();
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!(
            "framesmith_rules_unsupported_version_{unique}.json"
        ));

        std::fs::write(&path, r#"{"version": 2}"#).unwrap();

        let res = load_rules_file(&path);
        assert!(matches!(res, Err(RulesError::UnsupportedVersion(2))));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_generate_rules_schema() {
        let schema = generate_rules_schema();
        let json = serde_json::to_string_pretty(&schema).unwrap();
        // Schema should contain key type names
        assert!(json.contains("RulesFile"));
        assert!(json.contains("ApplyRule"));
        assert!(json.contains("ValidateRule"));
        assert!(json.contains("MatchSpec"));
        assert!(json.contains("Severity"));
    }
}
