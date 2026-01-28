use crate::codegen::{export_json_blob, export_json_blob_pretty};
use crate::schema::{CancelTable, Character, Move};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterData {
    pub character: Character,
    pub moves: Vec<Move>,
    pub cancel_table: CancelTable,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct CharacterSummary {
    pub id: String,
    pub name: String,
    pub archetype: String,
    pub move_count: usize,
}

#[tauri::command]
pub fn list_characters(characters_dir: String) -> Result<Vec<CharacterSummary>, String> {
    let path = Path::new(&characters_dir);
    if !path.exists() {
        return Ok(vec![]);
    }

    let mut summaries = vec![];
    for entry in fs::read_dir(path).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let char_path = entry.path();
        if !char_path.is_dir() {
            continue;
        }

        let char_file = char_path.join("character.json");
        if !char_file.exists() {
            continue;
        }

        let content = fs::read_to_string(&char_file).map_err(|e| e.to_string())?;
        let character: Character = serde_json::from_str(&content).map_err(|e| e.to_string())?;

        let moves_dir = char_path.join("moves");
        let move_count = if moves_dir.exists() {
            fs::read_dir(&moves_dir)
                .map(|dir| dir.filter(|e| e.is_ok()).count())
                .unwrap_or(0)
        } else {
            0
        };

        summaries.push(CharacterSummary {
            id: character.id.clone(),
            name: character.name,
            archetype: character.archetype,
            move_count,
        });
    }

    Ok(summaries)
}

#[tauri::command]
pub fn load_character(
    characters_dir: String,
    character_id: String,
) -> Result<CharacterData, String> {
    // Validate character_id to prevent path traversal attacks
    if character_id.contains("..") || character_id.contains('/') || character_id.contains('\\') {
        return Err("Invalid character ID".to_string());
    }

    let char_path = Path::new(&characters_dir).join(&character_id);
    if !char_path.exists() {
        return Err(format!("Character '{}' not found", character_id));
    }

    // Load character.json
    let char_file = char_path.join("character.json");
    let content = fs::read_to_string(&char_file)
        .map_err(|e| format!("Failed to read character.json: {}", e))?;
    let character: Character = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid character.json format: {}", e))?;

    // Load all moves
    let moves_dir = char_path.join("moves");
    let mut moves = vec![];
    if moves_dir.exists() {
        for entry in fs::read_dir(&moves_dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let move_path = entry.path();
            if move_path.extension().map(|e| e == "json").unwrap_or(false) {
                let content = fs::read_to_string(&move_path)
                    .map_err(|e| format!("Failed to read move file {:?}: {}", move_path.file_name(), e))?;
                let mv: Move = serde_json::from_str(&content)
                    .map_err(|e| format!("Invalid move file {:?}: {}", move_path.file_name(), e))?;
                moves.push(mv);
            }
        }
    }

    // Load cancel table
    let cancel_file = char_path.join("cancel_table.json");
    let cancel_table: CancelTable = if cancel_file.exists() {
        let content = fs::read_to_string(&cancel_file)
            .map_err(|e| format!("Failed to read cancel_table.json: {}", e))?;
        serde_json::from_str(&content)
            .map_err(|e| format!("Invalid cancel_table.json format: {}", e))?
    } else {
        CancelTable {
            chains: HashMap::new(),
            special_cancels: vec![],
            super_cancels: vec![],
            jump_cancels: vec![],
        }
    };

    Ok(CharacterData {
        character,
        moves,
        cancel_table,
    })
}

#[tauri::command]
pub fn save_move(characters_dir: String, character_id: String, mv: Move) -> Result<(), String> {
    // Validate character_id to prevent path traversal
    if character_id.contains("..") || character_id.contains('/') || character_id.contains('\\') {
        return Err("Invalid character ID".to_string());
    }

    let move_path = Path::new(&characters_dir)
        .join(&character_id)
        .join("moves")
        .join(format!("{}.json", mv.input));

    let content = serde_json::to_string_pretty(&mv).map_err(|e| format!("Failed to serialize move: {}", e))?;
    fs::write(&move_path, content).map_err(|e| format!("Failed to write move file: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn export_character(
    characters_dir: String,
    character_id: String,
    adapter: String,
    output_path: String,
    pretty: bool,
) -> Result<(), String> {
    // Validate character_id
    if character_id.contains("..") || character_id.contains('/') || character_id.contains('\\') {
        return Err("Invalid character ID".to_string());
    }

    let char_data = load_character(characters_dir, character_id)?;

    let output = match adapter.as_str() {
        "json-blob" => {
            if pretty {
                export_json_blob_pretty(&char_data)?
            } else {
                export_json_blob(&char_data)?
            }
        }
        "breakpoint-rust" => {
            return Err("Breakpoint adapter not yet implemented".to_string());
        }
        _ => return Err(format!("Unknown adapter: {}", adapter)),
    };

    fs::write(&output_path, output).map_err(|e| format!("Failed to write export file: {}", e))?;
    Ok(())
}
