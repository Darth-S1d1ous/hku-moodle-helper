# Implementation plan

Status: agreed. Architecture **A (session-cookie sidecar) + C (iCal fallback)**. Stack stays **Tauri 2**.

This is the working checklist. Product intent lives in `README.md`.

## Why Tauri stays

A + C needs three things at once: a real browser for HKU Portal SSO, a cheap background process for polling, and a small desktop UI with a system tray.


| Need                     | Tauri 2                                | Notes                                 |
| ------------------------ | -------------------------------------- | ------------------------------------- |
| Portal SSO + 2FA         | Login `WebviewWindow`                  | Cannot be done with `reqwest` alone   |
| HttpOnly `MoodleSession` | `Webview::cookies` / `cookies_for_url` | Includes HttpOnly and Secure cookies  |
| Fetch after login        | Rust `reqwest` + cookie jar            | Login webview is destroyed afterwards |
| Tray while “asleep”      | `tray-icon` + hide on close            | macOS menu bar, Windows tray          |
| Open Moodle links        | `tauri-plugin-opener`                  | Already in the repo                   |
| Low memory               | System webview, destroy login window   | Aligns with README                    |


Electron would also do SSO, but the webview stays heavier in the tray. A pure Swift/WinUI split would be smaller still, and is overkill for this prototype. A browser extension is the old helper, and DOM injection is what broke.

**Keep Tauri.** The caveats below are implementation rules, not reasons to change stack.

### Caveats (do not ignore)

1. **Read cookies from async commands only.** `cookies()` / `cookies_for_url()` deadlock on Windows (and can on macOS) if called from a sync command or event handler. Never poll cookies on a worker that also `eval`s the same webview.
2. **Copy cookies into** `reqwest`**; do not scrape** `document.cookie`**.** `MoodleSession` is HttpOnly.
3. **Cookie copy is a snapshot.** When the session expires, open the login window again. Do not try to replay Portal SSO with stored UID/PIN.
4. **Hidden main window still costs RSS.** Prototype: hide on close. If memory is still high: destroy the main webview in tray, recreate on show.
5. **Android cookie API is empty.** Fine — this project is desktop-only for now.
6. **Do not ship Moodle Web Services (**`token.php`**).** HKU does not support the Moodle App. One spike in Step 0 is enough to confirm, then delete that path.



## Target shape

```
SSO WebView  →  cookie harvest  →  destroy login window
                                      ↓
Tray timer  →  Rust MoodleClient  →  JSON cache  →  vanilla UI
                 ├─ primary: session AJAX / calendar JSON
                 └─ fallback: iCal export
                                      ↓
                               system notifications
```

UI stays vanilla TypeScript (current Vite scaffold). No React.

Suggested modules when code starts:

```
src/
  main.ts
  ui/          sidebar, splitter, search, cards
  state/       filters + search over invoke("list_todos")

src-tauri/src/
  lib.rs
  auth.rs      login window, async cookie harvest
  moodle.rs    TodoSource: AjaxSession + Ical
  store.rs     last-known todos JSON
  tray.rs
  scheduler.rs
  commands.rs
```

Domain object:

```ts
type TodoKind =
  | "assignment"
  | "turnitin"
  | "registration"
  | "assessment"
  | "quiz"
  | "other";

interface TodoItem {
  id: string;
  title: string;
  course?: string;
  kind: TodoKind;
  deadline: string; // ISO 8601
  url: string;
}
```

`kind` mapping (AJAX `modulename` first; iCal guesses from URL, else `other`):

- `assign` → `assignment`
- `turnitintooltwo` → `turnitin`
- `quiz` / similar graded activities → `assessment`
- enrolment / registration events → `registration`



## Steps

Each step has an exit criterion. Do not start the next step until the previous one is true.

### 0. Spike endpoints (no UI)

**Do**

- Log into `https://moodle.hku.hk` in a normal browser.
- Record cookie names (`MoodleSession` and anything else required).
- From the logged-in page, copy `M.cfg.sesskey`.
- Probe, in order:
  1. `POST /lib/ajax/service.php` calendar / timeline methods (same payload the site uses).
  2. Calendar HTML or export: `/calendar/export.php` (or the ics URL from calendar settings).
  3. Once: `GET /login/token.php?service=moodle_mobile_app` — expect failure; do not build on it.

**Exit:** A short note in this file or a comment in `moodle.rs`: which AJAX method works, what JSON fields exist (`name`, `timestart`, `url`, `modulename`), and whether iCal is reachable with the same cookies. Token path marked dead.

### 1. App shell: tray + layout

**Do**

- Enable `tray-icon` on the `tauri` crate; add hide/show window permissions.
- Intercept `CloseRequested`: hide the main window, keep the process.
- Tray menu: Show, Refresh (stub), Quit.
- Replace the greet demo with the README layout: left filter checkboxes, togglable vertical splitter, search bar, empty card list. Follow the Figma file for spacing/visuals when it is available.
- Keep using mock `TodoItem[]` in the frontend.

**Exit:** Closing the window leaves a tray icon. Reopening shows the layout. Quit from the tray fully exits.

### 2. Login window + cookie harvest

**Do**

- Second window: `label = "login"`, URL `https://moodle.hku.hk`, no local UI CSP.
- Local main window CSP stays tight (`asset:` / `tauri:` only). Login window is the only one that may load HKU origins.
- After navigation, `cookies_for_url(https://moodle.hku.hk)` in an **async** command.
- Success = `MoodleSession` present and a follow-up request to `/my/` (or equivalent) is not a login redirect.
- Copy cookies into an in-memory `reqwest` jar; persist only a “logged in” flag + cache of todos, not the password.
- Close/destroy the login webview immediately after harvest.
- `sesskey`: one native `eval("M.cfg.sesskey")` on the Moodle origin after login, then never eval that page again.

**Exit:** After one SSO, Rust can `GET https://moodle.hku.hk/my/` with the copied jar and get a 200 HTML that is not the login form. Login window is gone.

### 3. MoodleClient: AJAX primary, iCal fallback

**Do**

- Trait `TodoSource { fn fetch(&self) -> Result<Vec<TodoItem>> }`.
- `AjaxSession`: sesskey + cookies → calendar/timeline JSON → classify `kind`.
- `Ical`: same cookies → export URL → parse `SUMMARY` / `DTEND` / `URL`. Weak `kind`.
- Orchestrator: try AJAX; if empty or parse error, use iCal; surface `source: "ajax" | "ical"` for debugging.
- Write `todos.json` under the app data dir so the UI can open offline.

**Exit:** `invoke("refresh_todos")` returns real items with title, deadline, url. Unplugging AJAX (force error) still returns iCal items.

### 4. Bind UI

**Do**

- `invoke("list_todos")` on launch; `refresh_todos` from a button and from the tray.
- Checkbox filters AND with the search query (title + course).
- Card: title, deadline, course; click-through uses `plugin-opener` (already allowed).
- Logged-out state: CTA that opens the login window.

**Exit:** Filters, search, and opener work against live data. No greet demo left.

### 5. Background poll + notifications

**Do**

- `tauri-plugin-notification`.
- Tokio interval 15–30 minutes, independent of UI visibility.
- Notify when a todo is due within a threshold (start with 24h). Deduplicate by `id`.
- On auth failure (cookie dead): tray tooltip “login required”; do not spin the login window by itself.

**Exit:** Hide the app, wait for one interval (or a debug “poll now”), cache updates, a due-soon item notifies. Login webview is not running while asleep.

### 6. Memory pass

**Do**

- Confirm login webview is destroyed, not hidden.
- Measure RSS: window shown vs tray-only.
- If tray-only is still dominated by the main webview, destroy it on hide and recreate on tray Show.
- Leave Release profile as-is (`lto`, `strip`, `panic = abort`).

**Exit:** Tray-only does not keep a second (login) webview. RSS note recorded (even a one-line comment is enough).

## Config to touch (when the matching step starts)

- `src-tauri/Cargo.toml`: `tauri` feature `tray-icon`; later `reqwest`, cookie jar, ical parser, `tauri-plugin-notification`.
- `src-tauri/tauri.conf.json`: tray icon; keep `opener`; do not leave `csp: null` on the main window.
- `src-tauri/capabilities/default.json`: `core:window:allow-hide`, `allow-show`, `allow-close`; notification permission; login window gets a separate capability that may navigate to `moodle.hku.hk` and Portal SSO hosts.
- Frontend: stay on `@tauri-apps/api` v2. `withGlobalTauri` can remain, but prefer module imports.



## Out of scope until the steps above are done

- Moodle App / `wstoken` client
- Persistent hidden Moodle webview scraper
- Auto-start on login
- Mobile (`crate-type` already has `staticlib`; ignore)
- Storing Portal UID/PIN
- Course enrolment (“adding courses of this semester”) — README background only

