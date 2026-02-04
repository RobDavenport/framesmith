use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Property schema definitions for character and state properties.
/// Property names in this schema become IDs (indices) in the exported FSPK,
/// eliminating duplicate string storage across states.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize, JsonSchema)]
pub struct PropertySchema {
    /// Character-level property names (e.g., "health", "walkSpeed", "dashSpeed").
    /// Index in this array becomes the schema ID for that property.
    #[serde(default)]
    pub character: Vec<String>,
    /// State-level property names (e.g., "startup", "active", "damage").
    /// Index in this array becomes the schema ID for that property.
    #[serde(default)]
    pub state: Vec<String>,
}

use super::RulesFile;

/// Merge project + character property schemas.
///
/// Semantics:
/// - Character schema extends project schema (character names appended after project).
/// - Duplicate names are preserved (character entries keep their position relative to project).
/// - Returns None if neither project nor character defines a property schema.
pub fn merged_property_schema(
    project: Option<&RulesFile>,
    character: Option<&RulesFile>,
) -> Option<PropertySchema> {
    let project_schema = project.and_then(|r| r.properties.as_ref());
    let char_schema = character.and_then(|r| r.properties.as_ref());

    match (project_schema, char_schema) {
        (None, None) => None,
        (Some(p), None) => Some(p.clone()),
        (None, Some(c)) => Some(c.clone()),
        (Some(p), Some(c)) => {
            // Merge: project properties first, then character (preserving order for IDs)
            let char_set: std::collections::HashSet<&String> = p.character.iter().collect();
            let state_set: std::collections::HashSet<&String> = p.state.iter().collect();

            // Start with project properties
            let mut character_props = p.character.clone();
            let mut state_props = p.state.clone();

            // Add character-level properties not already in project
            for name in &c.character {
                if !char_set.contains(name) {
                    character_props.push(name.clone());
                }
            }

            // Add state-level properties not already in project
            for name in &c.state {
                if !state_set.contains(name) {
                    state_props.push(name.clone());
                }
            }

            Some(PropertySchema {
                character: character_props,
                state: state_props,
            })
        }
    }
}

/// Merge project + character tag schemas.
///
/// Semantics:
/// - Character tags extend project tags (character names appended after project).
/// - Duplicate names are preserved in their original position.
/// - Returns None if neither project nor character defines a tag schema.
pub fn merged_tag_schema(
    project: Option<&RulesFile>,
    character: Option<&RulesFile>,
) -> Option<Vec<String>> {
    let project_tags = project.and_then(|r| r.tags.as_ref());
    let char_tags = character.and_then(|r| r.tags.as_ref());

    match (project_tags, char_tags) {
        (None, None) => None,
        (Some(p), None) => Some(p.clone()),
        (None, Some(c)) => Some(c.clone()),
        (Some(p), Some(c)) => {
            // Merge: project tags first, then character (preserving order for IDs)
            let tag_set: std::collections::HashSet<&String> = p.iter().collect();
            let mut tags = p.clone();
            for name in c {
                if !tag_set.contains(name) {
                    tags.push(name.clone());
                }
            }
            Some(tags)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rules_with_property_schema(props: PropertySchema) -> RulesFile {
        RulesFile {
            version: 1,
            registry: None,
            apply: Vec::new(),
            validate: Vec::new(),
            properties: Some(props),
            tags: None,
        }
    }

    fn rules_with_tags(tags: Vec<String>) -> RulesFile {
        RulesFile {
            version: 1,
            registry: None,
            apply: Vec::new(),
            validate: Vec::new(),
            properties: None,
            tags: Some(tags),
        }
    }

    #[test]
    fn test_merged_property_schema_none_when_neither_defined() {
        let merged = merged_property_schema(None, None);
        assert!(merged.is_none());
    }

    #[test]
    fn test_merged_property_schema_project_only() {
        let project = rules_with_property_schema(PropertySchema {
            character: vec!["health".to_string(), "walkSpeed".to_string()],
            state: vec!["startup".to_string(), "active".to_string()],
        });

        let merged = merged_property_schema(Some(&project), None);
        assert!(merged.is_some());
        let schema = merged.unwrap();
        assert_eq!(schema.character, vec!["health", "walkSpeed"]);
        assert_eq!(schema.state, vec!["startup", "active"]);
    }

    #[test]
    fn test_merged_property_schema_character_only() {
        let character = rules_with_property_schema(PropertySchema {
            character: vec!["health".to_string()],
            state: vec!["damage".to_string()],
        });

        let merged = merged_property_schema(None, Some(&character));
        assert!(merged.is_some());
        let schema = merged.unwrap();
        assert_eq!(schema.character, vec!["health"]);
        assert_eq!(schema.state, vec!["damage"]);
    }

    #[test]
    fn test_merged_property_schema_combines_without_duplicates() {
        let project = rules_with_property_schema(PropertySchema {
            character: vec!["health".to_string(), "walkSpeed".to_string()],
            state: vec!["startup".to_string(), "active".to_string()],
        });
        let character = rules_with_property_schema(PropertySchema {
            character: vec!["walkSpeed".to_string(), "jumpHeight".to_string()], // walkSpeed is dupe
            state: vec!["damage".to_string()],
        });

        let merged = merged_property_schema(Some(&project), Some(&character));
        let schema = merged.unwrap();
        // Project order preserved, character additions appended (without dupes)
        assert_eq!(schema.character, vec!["health", "walkSpeed", "jumpHeight"]);
        assert_eq!(schema.state, vec!["startup", "active", "damage"]);
    }

    #[test]
    fn test_merged_tag_schema_none_when_neither_defined() {
        let merged = merged_tag_schema(None, None);
        assert!(merged.is_none());
    }

    #[test]
    fn test_merged_tag_schema_project_only() {
        let project = rules_with_tags(vec!["normal".to_string(), "special".to_string()]);

        let merged = merged_tag_schema(Some(&project), None);
        assert!(merged.is_some());
        assert_eq!(merged.unwrap(), vec!["normal", "special"]);
    }

    #[test]
    fn test_merged_tag_schema_character_only() {
        let character = rules_with_tags(vec!["super".to_string()]);

        let merged = merged_tag_schema(None, Some(&character));
        assert!(merged.is_some());
        assert_eq!(merged.unwrap(), vec!["super"]);
    }

    #[test]
    fn test_merged_tag_schema_combines_without_duplicates() {
        let project = rules_with_tags(vec![
            "normal".to_string(),
            "special".to_string(),
            "super".to_string(),
        ]);
        let character = rules_with_tags(vec![
            "special".to_string(), // dupe
            "ex".to_string(),
        ]);

        let merged = merged_tag_schema(Some(&project), Some(&character));
        assert!(merged.is_some());
        // Project order preserved, character additions appended (without dupes)
        assert_eq!(merged.unwrap(), vec!["normal", "special", "super", "ex"]);
    }

    #[test]
    fn test_property_schema_deserializes_from_json() {
        let json = r#"{
            "version": 1,
            "properties": {
                "character": ["health", "walkSpeed", "dashSpeed"],
                "state": ["startup", "active", "recovery", "damage"]
            },
            "tags": ["normal", "special", "super"]
        }"#;

        let rules: RulesFile = serde_json::from_str(json).unwrap();
        assert!(rules.properties.is_some());
        let props = rules.properties.unwrap();
        assert_eq!(props.character, vec!["health", "walkSpeed", "dashSpeed"]);
        assert_eq!(props.state, vec!["startup", "active", "recovery", "damage"]);

        assert!(rules.tags.is_some());
        assert_eq!(rules.tags.unwrap(), vec!["normal", "special", "super"]);
    }
}
