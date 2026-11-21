#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod keys;
mod state;
mod window;

use keys::Keys;
use serde_json::{json, Value};
use state::{Action, Store};
use std::sync::Mutex;
use tauri::{Manager, State, WebviewWindow};

type SharedStore = Mutex<Store>;

#[tauri::command]
fn session(store: State<SharedStore>, keys: State<Keys>) -> Value {
    let store = store.lock().unwrap();
    let pinned = store.settings().pinned;
    json!({
        "platform": std::env::consts::OS,
        "pinned": pinned,
        "keysReady": pinned && keys.enable(),
        "data": store.snapshot(),
    })
}

#[tauri::command]
fn apply(action: Action, store: State<SharedStore>) -> Result<Value, String> {
    store.lock().unwrap().apply(action)
}

#[tauri::command]
fn pin(
    pinned: bool,
    window: WebviewWindow,
    store: State<SharedStore>,
    keys: State<Keys>,
) -> Result<Value, String> {
    let mut store = store.lock().unwrap();
    window::pin(&window, pinned).map_err(|error| error.to_string())?;
    if let Err(error) = store.save_pin(pinned) {
        let _ = window::pin(&window, store.settings().pinned);
        return Err(error);
    }
    if !pinned {
        keys::disable();
    }
    Ok(json!({ "pinned": pinned, "keysReady": pinned && keys.enable() }))
}

#[tauri::command]
fn authorize_keys(store: State<SharedStore>, keys: State<Keys>) -> bool {
    if !store.lock().unwrap().settings().pinned {
        return false;
    }
    keys::request_access();
    keys.enable()
}

#[tauri::command]
fn copy(id: String, store: State<SharedStore>) -> Result<(), String> {
    let text = store.lock().unwrap().text_to_copy(&id)?.to_owned();
    let mut clipboard = arboard::Clipboard::new().map_err(|error| error.to_string())?;
    clipboard.set_text(text).map_err(|error| error.to_string())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window::reveal(&window);
            }
        }))
        .setup(|app| {
            let path = app.path().data_dir()?.join("AnyNotes").join("state.json");
            let store = Store::open(path)?;
            let settings = store.settings().clone();
            app.manage(Mutex::new(store));

            #[cfg(target_os = "macos")]
            {
                use tauri::menu::{Menu, MenuItem, WINDOW_SUBMENU_ID};
                let menu = Menu::default(app.handle())?;
                if let Some(item) = menu.get(WINDOW_SUBMENU_ID) {
                    if let Some(submenu) = item.as_submenu() {
                        submenu.set_text("Окно")?;
                        submenu.append(&MenuItem::with_id(
                            app,
                            "recover",
                            "Вернуть окно на экран",
                            true,
                            Some("CmdOrCtrl+Shift+0"),
                        )?)?;
                    }
                }
                app.set_menu(menu)?;
            }

            let window = app
                .get_webview_window("main")
                .ok_or("Окно AnyNotes недоступно")?;
            window::restore(&window, &settings)?;
            if settings.pinned {
                window::pin(&window, true)?;
            }
            let keys = Keys::new(&window);
            if settings.pinned {
                keys.enable();
            }
            app.manage(keys);
            window::track_bounds(&window);
            window.show()?;
            Ok(())
        })
        .on_menu_event(|app, event| {
            if event.id().as_ref() == "recover" {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window::reveal(&window);
                }
            }
        })
        .on_window_event(|_, event| {
            if matches!(event, tauri::WindowEvent::Destroyed) {
                keys::disable();
            }
        })
        .invoke_handler(tauri::generate_handler![
            session,
            apply,
            pin,
            authorize_keys,
            copy
        ])
        .run(tauri::generate_context!())
        .expect("Не удалось запустить AnyNotes");
}
