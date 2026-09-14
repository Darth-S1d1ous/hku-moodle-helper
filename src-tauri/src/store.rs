use std::fs;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};

use crate::moodle::FetchResult;

const FILE_NAME: &str = "todos.json";

fn todos_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().app_data_dir().map_err(err)?;
    Ok(dir.join(FILE_NAME))
}

pub fn save(app: &AppHandle, todos: &FetchResult) -> Result<(), String> {
    let path = todos_path(app)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(err)?;
    }
    let json = serde_json::to_string(todos).map_err(err)?;
    fs::write(path, json).map_err(err)
}

pub fn load(app: &AppHandle) -> Result<Option<FetchResult>, String> {
    let path = todos_path(app)?;
    match fs::read_to_string(path) {
        Ok(json) => serde_json::from_str(&json).map(Some).map_err(err),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

fn err<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}
