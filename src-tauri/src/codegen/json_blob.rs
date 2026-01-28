use crate::commands::CharacterData;

/// Export character data as a single minified JSON blob
pub fn export_json_blob(character_data: &CharacterData) -> Result<String, String> {
    serde_json::to_string(character_data).map_err(|e| e.to_string())
}

/// Export character data as a pretty-printed JSON blob
pub fn export_json_blob_pretty(character_data: &CharacterData) -> Result<String, String> {
    serde_json::to_string_pretty(character_data).map_err(|e| e.to_string())
}
