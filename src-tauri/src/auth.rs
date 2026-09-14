use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::webview::{PageLoadEvent, WebviewWindowBuilder};
use tauri::{AppHandle, Emitter, Manager};
use url::Url;

const MOODLE_ORIGIN: &str = "https://moodle.hku.hk";
const MY_URL: &str = "https://moodle.hku.hk/my/";
const KEYRING_SERVICE: &str = "com.johnlyu.hku-moodle-helper";
const KEYRING_USER: &str = "moodle-cookies";
const LOGIN_LABEL: &str = "login";

// SSO (Single Sign-On) is used to authenticate users
static HARVEST_IN_FLIGHT: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredCookie {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub secure: bool,
    pub http_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSecrets {
    pub sesskey: String,
    pub cookies: Vec<StoredCookie>,
}

pub fn load_secrets() -> Result<Option<SessionSecrets>, String> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER).map_err(err)?;
    match entry.get_password() {
        Ok(json) => serde_json::from_str(&json).map(Some).map_err(err),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn save_secrets(secrets: &SessionSecrets) -> Result<(), String> {
    let json = serde_json::to_string(secrets).map_err(err)?;
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(err)?
        .set_password(&json)
        .map_err(err)
}

pub fn cookie_header(secrets: &SessionSecrets) -> String {
    secrets
        .cookies
        .iter()
        .map(|cookie| format!("{}={}", cookie.name, cookie.value))
        .collect::<Vec<_>>()
        .join("; ")
}

pub async fn session_is_alive(secrets: &SessionSecrets) -> Result<bool, String> {
    let response = reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(err)?
        .get(MY_URL)
        .header(reqwest::header::COOKIE, cookie_header(secrets))
        .send()
        .await
        .map_err(err)?;
    Ok(response.status().is_success())
}

pub async fn start_login(app: AppHandle) -> Result<(), String> {
    HARVEST_IN_FLIGHT.store(false, Ordering::SeqCst);

    if let Some(existing) = app.get_webview_window(LOGIN_LABEL) {
        existing.set_focus().map_err(err)?;
        return Ok(());
    }

    let config = app
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == LOGIN_LABEL)
        .cloned()
        .ok_or_else(|| "login window missing from tauri.conf.json".to_string())?;

    let handle = app.clone();
    WebviewWindowBuilder::from_config(&app, &config)
        .map_err(err)?
        .on_navigation(allow_login_navigation)
        .on_page_load(move |_window, payload| {
            if payload.event() != PageLoadEvent::Finished {
                return;
            }
            if !is_post_login_moodle(payload.url()) {
                return;
            }
            let app = handle.clone();
            tauri::async_runtime::spawn(async move {
                let _ = harvest_after_login(app).await;
            });
        })
        .build()
        .map_err(err)?;

    Ok(())
}

fn allow_login_navigation(url: &Url) -> bool {
    // HKU Portal CAS (`/cas/aad`) continues to Microsoft Entra ID.
    // Restricting this window to `*.hku.hk` makes LOG IN look like a no-op.
    // The login webview has no Tauri IPC, so allowing the HTTPS SSO chain is OK.
    let allowed = matches!(url.scheme(), "https" | "http" | "about");
    if !allowed {
        eprintln!("blocked login navigation: {url}");
    }
    allowed
}

fn is_post_login_moodle(url: &Url) -> bool {
    url.host_str() == Some("moodle.hku.hk") && !url.path().starts_with("/login")
}

async fn harvest_after_login(app: AppHandle) -> Result<(), String> {
    if HARVEST_IN_FLIGHT.swap(true, Ordering::SeqCst) {
        return Ok(());
    }

    let outcome = harvest_once(&app).await;
    if outcome.is_err() {
        HARVEST_IN_FLIGHT.store(false, Ordering::SeqCst);
    }
    outcome
}

async fn harvest_once(app: &AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(LOGIN_LABEL)
        .ok_or_else(|| "login window closed before harvest".to_string())?;
    let captured = tauri::async_runtime::spawn_blocking(move || capture_session(window))
        .await
        .map_err(err)??;
    if !session_is_alive(&captured).await? {
        return Err("Moodle session cookie was rejected by /my/".into());
    }
    save_secrets(&captured)?;
    if let Some(login) = app.get_webview_window(LOGIN_LABEL) {
        login.close().map_err(err)?;
    }
    let _ = app.emit("logged-in", ());

    Ok(())
}

fn capture_session(window: tauri::WebviewWindow) -> Result<SessionSecrets, String> {
    let moodle = Url::parse(MOODLE_ORIGIN).map_err(err)?;
    let cookies = window.cookies_for_url(moodle).map_err(err)?;
    let has_session = cookies.iter().any(|cookie| cookie.name() == "MoodleSession");
    if !has_session {
        return Err("MoodleSession cookie not present yet".into());
    }
    let stored = cookies
        .iter()
        .map(|cookie| StoredCookie {
            name: cookie.name().to_string(),
            value: cookie.value().to_string(),
            domain: cookie.domain().map(str::to_string),
            path: cookie.path().map(str::to_string),
            secure: cookie.secure().unwrap_or(true),
            http_only: cookie.http_only().unwrap_or(true),
        })
        .collect();

    let (tx, rx) = mpsc::channel();
    window
        .eval_with_callback(
            r#"(function () {
                try { return (M && M.cfg && M.cfg.sesskey) ? M.cfg.sesskey : ""; }
                catch (e) { return ""; }
            })()"#,
            move |raw| {
                let _ = tx.send(raw);
            },
        )
        .map_err(err)?;

    let raw = rx
        .recv_timeout(Duration::from_secs(5))
        .map_err(|_| "timed out waiting for sesskey".to_string())?;
    let sesskey: String = serde_json::from_str(&raw).unwrap_or_default();
    if sesskey.is_empty() {
        return Err("M.cfg.sesskey was empty".into());
    }

    Ok(SessionSecrets {
        sesskey,
        cookies: stored,
    })
}

fn err<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}