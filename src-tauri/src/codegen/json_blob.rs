use crate::commands::CharacterData;

/// Export character data as a single minified JSON blob
pub fn export_json_blob(character_data: &CharacterData) -> Result<String, String> {
    serde_json::to_string(character_data).map_err(|e| e.to_string())
}

/// Export character data as a pretty-printed JSON blob
pub fn export_json_blob_pretty(character_data: &CharacterData) -> Result<String, String> {
    serde_json::to_string_pretty(character_data).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{CancelTable, Character, PropertyValue, State};
    use std::collections::BTreeMap;

    /// Create a minimal test character.
    fn make_test_character(id: &str) -> Character {
        let mut properties = BTreeMap::new();
        properties.insert("archetype".to_string(), PropertyValue::String("rushdown".to_string()));
        properties.insert("health".to_string(), PropertyValue::Number(1000.0));
        properties.insert("walk_speed".to_string(), PropertyValue::Number(4.0));
        properties.insert("back_walk_speed".to_string(), PropertyValue::Number(3.0));
        properties.insert("jump_height".to_string(), PropertyValue::Number(100.0));
        properties.insert("jump_duration".to_string(), PropertyValue::Number(40.0));
        properties.insert("dash_distance".to_string(), PropertyValue::Number(80.0));
        properties.insert("dash_duration".to_string(), PropertyValue::Number(15.0));

        Character {
            id: id.to_string(),
            name: "Test".to_string(),
            properties,
            resources: vec![],
        }
    }

    #[test]
    fn export_includes_state_ids() {
        let character = make_test_character("test");

        let moves = vec![
            State {
                id: Some("5H".to_string()),
                input: "5H".to_string(),
                damage: 50,
                ..Default::default()
            },
            State {
                id: Some("5H~level1".to_string()),
                input: "5H".to_string(),
                damage: 80,
                ..Default::default()
            },
        ];

        let data = CharacterData {
            character,
            moves,
            cancel_table: CancelTable::default(),
        };

        let json = export_json_blob(&data).unwrap();

        assert!(json.contains(r#""id":"5H""#));
        assert!(json.contains(r#""id":"5H~level1""#));
    }

    #[test]
    fn export_includes_global_derived_states() {
        // Global states become regular states after resolution,
        // so they should be included in exports automatically.
        // This test documents that expectation.

        let states = vec![
            State {
                id: Some("5L".to_string()),
                input: "5L".to_string(),
                name: "Light Attack".to_string(),
                ..Default::default()
            },
            State {
                id: Some("burst".to_string()),
                input: "burst".to_string(),
                name: "Burst".to_string(),
                move_type: Some("system".to_string()),
                ..Default::default()
            },
        ];

        let character = make_test_character("test");

        let cancel_table = CancelTable::default();

        let char_data = CharacterData {
            character,
            moves: states,
            cancel_table,
        };

        let json = export_json_blob(&char_data).unwrap();

        // Verify both states are in the export
        assert!(json.contains("\"5L\""));
        assert!(json.contains("\"burst\""));
        assert!(json.contains("\"system\"")); // type field from global
    }
}
