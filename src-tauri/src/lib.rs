pub mod codegen;
pub mod commands;
pub mod mcp;
pub mod rules;
pub mod schema;

use commands::{
    clone_character, create_character, create_move, create_project, delete_character,
    export_character, list_characters, load_character, load_character_assets, open_folder_dialog,
    read_character_asset_base64, save_move, validate_project,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            list_characters,
            load_character,
            load_character_assets,
            read_character_asset_base64,
            save_move,
            export_character,
            open_folder_dialog,
            validate_project,
            create_project,
            create_character,
            clone_character,
            delete_character,
            create_move,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
