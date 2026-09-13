mod auth;
mod moodle;

use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    Manager, WindowEvent,
};

// tauri::command is a macro that registers a function as callable from Frontend
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
async fn start_login(app: tauri::AppHandle) -> Result<(), String> {
    auth::start_login(app).await
}

#[tauri::command]
async fn session_status() -> Result<bool, String> {
    match auth::load_secrets()? {
        Some(secrets) => auth::session_is_alive(&secrets).await,
        None => Ok(false),
    }
}

#[tauri::command]
async fn refresh_todos() -> Result<moodle::FetchResult, String> {
    let secrets = auth::load_secrets()?.ok_or_else(|| "not logged in".to_string())?;
    moodle::fetch_todos(&secrets).await
}

fn show_main(app: &tauri::AppHandle) {
    // enum Option<T> {
    //     None,       // 没有值
    //     Some(T),    // 有一个类型为 T 的值
    // }
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}

fn probe_saved_session() {
    tauri::async_runtime::spawn(async {
        let Ok(Some(secrets)) = auth::load_secrets() else {
            return;
        };
        let _ = auth::session_is_alive(&secrets).await;
    });
}

// if mobile, then use mobile_entry_point
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() { // pub, so main.rs can call this function
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            start_login,
            session_status,
            refresh_todos
        ])
        .setup(|app| { // closure, a function called after the builder is created
            let show = MenuItem::with_id(app, "show", "Show", true, None::<&str>)?;
            let refresh = MenuItem::with_id(app, "refresh", "Refresh", true, None::<&str>)?;
            let login = MenuItem::with_id(app, "login", "Log in", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&show, &refresh, &login, &sep, &quit])?;

            let tray = app.tray_by_id("tray").expect("tray id `tray` must match tauri.conf.json");

            tray.set_menu(Some(menu))?;
            tray.on_menu_event(|app, event| match event.id.as_ref() {
                "show" => show_main(app),
                "refresh" => {}
                "login" => {
                    let app = app.clone();
                    tauri::async_runtime::spawn(async move {
                        if let Err(error) = auth::start_login(app).await {
                            eprintln!("start_login failed: {error}");
                        }
                    });
                }
                "quit" => app.exit(0),
                _ => {}
            });
            
            probe_saved_session();
            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != "main" {
                return;
            }

            // take api, ignore the rest; do not actually destory the window
            if let WindowEvent::CloseRequested {api, ..} = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .run(tauri::generate_context!()) // a macro, bundles tauri.conf.json... into a binary 
        .expect("error while running tauri application");
}