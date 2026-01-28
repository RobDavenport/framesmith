pub mod codegen;
pub mod commands;
pub mod mcp;
pub mod schema;

use commands::{export_character, list_characters, load_character, save_move};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            list_characters,
            load_character,
            save_move,
            export_character
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
