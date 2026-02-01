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
}
