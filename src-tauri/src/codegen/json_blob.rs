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
    use crate::schema::{CancelTable, Character, State};

    #[test]
    fn export_includes_state_ids() {
        let character = Character {
            id: "test".to_string(),
            name: "Test".to_string(),
            archetype: "rushdown".to_string(),
            health: 1000,
            walk_speed: 4.0,
            back_walk_speed: 3.0,
            jump_height: 100,
            jump_duration: 40,
            dash_distance: 80,
            dash_duration: 15,
            resources: vec![],
        };

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

        let character = Character {
            id: "test".to_string(),
            name: "Test".to_string(),
            archetype: "test".to_string(),
            health: 10000,
            walk_speed: 4.0,
            back_walk_speed: 3.0,
            jump_height: 120,
            jump_duration: 45,
            dash_distance: 80,
            dash_duration: 18,
            resources: vec![],
        };

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
