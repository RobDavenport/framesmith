pub mod codegen;
pub mod commands;
pub mod mcp;
pub mod rules;
pub mod schema;
pub mod variant;
pub mod globals;

use commands::{
    clone_character, create_character, create_move, create_project, delete_character,
    delete_global_state, export_character, get_character_fspk, get_global_state, list_characters,
    list_global_states, load_character, load_character_assets, load_rules_registry,
    open_folder_dialog, open_training_window, read_character_asset_base64, save_global_state,
    save_move, validate_project,
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
            load_rules_registry,
            read_character_asset_base64,
            save_move,
            export_character,
            get_character_fspk,
            open_folder_dialog,
            validate_project,
            create_project,
            create_character,
            clone_character,
            delete_character,
            create_move,
            open_training_window,
            list_global_states,
            get_global_state,
            save_global_state,
            delete_global_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
