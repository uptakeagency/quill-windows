mod models;
mod ai;
mod clipboard;
mod keyring_manager;
mod ocr;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            keyring_manager::get_gemini_key,
            keyring_manager::save_gemini_key,
            keyring_manager::delete_gemini_key,
            keyring_manager::get_claude_key,
            keyring_manager::save_claude_key,
            keyring_manager::delete_claude_key,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Quill");
}
