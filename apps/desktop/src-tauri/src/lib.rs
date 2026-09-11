//! Tauri shell. Deliberately thin: the React UI talks to `renewal-server` over HTTP,
//! so the same frontend runs on Windows and (later) Android. The Rust side only
//! provides what a webview cannot do on its own — a platform secure store for the
//! server URL and session token, and (in later phases) native notifications, tray,
//! updater and deep links.

mod secure;

#[tauri::command]
fn secure_get(key: String) -> Result<Option<String>, String> {
    secure::get(&key).map_err(|e| e.to_string())
}

#[tauri::command]
fn secure_set(key: String, value: String) -> Result<(), String> {
    secure::set(&key, &value).map_err(|e| e.to_string())
}

#[tauri::command]
fn secure_delete(key: String) -> Result<(), String> {
    secure::delete(&key).map_err(|e| e.to_string())
}

/// "windows" | "android" | "linux" | "macos" — lets the UI adapt (touch targets, tray, updater).
#[tauri::command]
fn platform() -> &'static str {
    std::env::consts::OS
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            secure_get,
            secure_set,
            secure_delete,
            platform
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
