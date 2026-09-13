//! Step 0 spike (2026-09-13), against a live HKU Moodle session from keyring.
//! Site is Moodle 5.1 (`PRODID` `Moodle Version 2025100600.07`; AJAX errors link
//! `docs.moodle.org/501`). Theme on `/my/` is `space`.
//!
//! Session used: `GET https://moodle.hku.hk/my/` → 200 HTML (not the login form).
//! `M.cfg` on that page includes `wwwroot`, `sesskey`, `theme`, `courseId`, `userId`.
//! Harvested cookies (names only):
//! - `MoodleSession` — required; HttpOnly; Secure; `Domain=moodle.hku.hk`; `Path=/`
//! - `MOODLEID1_` — also present (remember-username). Not enough on its own.
//! `sesskey` is a short string from `M.cfg.sesskey`. It is **not** in a cookie.
//!
//! # 1. AJAX primary — works
//!
//! `POST https://moodle.hku.hk/lib/ajax/service.php?sesskey=<sesskey>&info=<method>`
//! `Content-Type: application/json`
//! body: `[{"index":0,"methodname":"<method>","args":{...}}]`
//! HTTP status is 200 even on RPC errors. Envelope:
//! - ok: `[{"error":false,"data":{...}}]`
//! - fail: `[{"error":true,"exception":{"errorcode","message",...}}]`
//! Missing/wrong `sesskey` → `errorcode: invalidsesskey`.
//!
//! **Use `core_calendar_get_action_events_by_timesort`.** Same cookies + sesskey.
//! Useful args: `limitnum`, `timesortfrom` (unix seconds; use "now" for open
//! todos, `0` to include overdue), optional `timesortto`,
//! `limittononsuspendedevents: true`, `aftereventid` for paging.
//! `data` keys: `events`, `firstid`, `lastid`.
//!
//! Each `events[]` item has (observed): `id`, `name`, `description`,
//! `descriptionformat`, `location`, `categoryid`, `groupid`, `userid`,
//! `repeatid`, `eventcount`, `component`, `modulename`, `activityname`,
//! `activitystr`, `instance`, `eventtype`, `timestart`, `timeduration`,
//! `timesort`, `timeusermidnight`, `visible`, `timemodified`, `overdue`,
//! `icon`, `course`, `subscription`, `canedit`, `candelete`, `deleteurl`,
//! `editurl`, `viewurl`, `formattedtime`, `formattedlocation`, `isactionevent`,
//! `iscourseevent`, `iscategoryevent`, `groupname`, `normalisedeventtype`,
//! `normalisedeventtypetext`, `action`, `purpose`, `branded`, `url`.
// [
//   {
//     "error": false,
//     "data": {
//       "events": [
//         {
//           "id": 10749654,
//           "name": "Partial Report Submission - 20% - Partial Report",
//           "description": "<p><strong>Deadline: </strong>14th Oct (Wed)</p>",
//           "descriptionformat": 1,
//           "location": "",
//           "categoryid": null,
//           "groupid": null,
//           "userid": 156913,
//           "component": "mod_turnitintooltwo",
//           "modulename": "turnitintooltwo",
//           "activityname": "Partial Report Submission - 20%",
//           "activitystr": "Turnitin Assignment 2 requires action",
//           "instance": 4170576,
//           "eventtype": "due",
//           "timestart": 1791993540,
//           "timeduration": 0,
//           "timesort": 1791993540,
//           "overdue": false,
//           "course": {
//             "id": 145893,
//             "fullname": "CAES9542 Technical English for Computer Science [2026]",
//             "shortname": "_CAES9542_2026",
//             "viewurl": "https://moodle.hku.hk/course/view.php?id=145893",
//             "coursecategory": "2026-27"
//           },
//           "isactionevent": true,
//           "action": {
//             "name": "Add Submission",
//             "url": "https://moodle.hku.hk/mod/turnitintooltwo/view.php?id=4170576",
//             "itemcount": 1,
//             "actionable": true,
//             "showitemcount": false
//           },
//           "url": "https://moodle.hku.hk/mod/turnitintooltwo/view.php?id=4170576"
//         }
//       ],
//       "firstid": 10298045,
//       "lastid": 10749655
//     }
//   }
// ]
//! Map to `TodoItem`:
//! - `id` → `id` (stringify)
//! - `name` → `title`
//! - `course.shortname` / `course.fullname` → `course`
//! - `timestart` (unix) → `deadline` ISO-8601; `timesort` matches on due events
//! - `url` (and `action.url`) → `url`; paths like `/mod/assign/view.php`,
//!   `/mod/turnitintooltwo/view.php`
//! - `modulename` → `kind` (`component` is `mod_<modulename>`)
//!
//! `modulename` values seen on this account: `assign`, `turnitintooltwo`,
//! `questionnaire`. `eventtype` seen: `due`, `expectcompletionon`.
//! `quiz` / enrolment events were not in this inbox — still map `quiz` →
//! `assessment` and enrolment → `registration` when they appear.
//! `purpose` is **not** a reliable kind (`turnitintooltwo` came back as `none`).
//!
//! Other methods (do not use as primary):
//! - `core_calendar_get_calendar_monthly_view` — works, but a month grid with
//!   HTML, not a todo list.
//! - `core_calendar_get_calendar_upcoming_view` — needs `courseid`; site
//!   course `1` returned `events: []` (not user action events).
//! - `core_calendar_get_calendar_events` — `servicenotavailable` (WS disabled).
//! - `core_calendar_get_calendar_event_by_id` / `_by_courses` — work, not needed
//!   if timesort already returns the list.
//!
//! # 2. iCal fallback — reachable with the same cookies
//!
//! `GET /calendar/export.php` is an HTML form, not an `.ics`.
//! `POST /calendar/export.php` with `sesskey`, `_qf__core_calendar_export_form=1`,
//! `events[exportevents]=all`, `period[timeperiod]=…`, `export=1` follows to
//! `/calendar/export_execute.php?userid=…&authtoken=…&preset_what=all&preset_time=…`
//! `Content-Type: text/calendar`, attachment `icalexport.ics`.
//! That execute URL also returns ICS **without** cookies (authtoken is enough).
//! Prefer posting the form with the session rather than persisting authtoken.
//!
//! Period: `recentupcoming` missed later due dates on this account; `custom`
//! matched the three future AJAX events. `weeknext` / `monthnow` can be empty.
//!
//! VEVENT properties observed: `UID`, `SUMMARY`, `DESCRIPTION`, `CLASS`,
//! `LAST-MODIFIED`, `DTSTAMP`, `DTSTART`, `DTEND`, `CATEGORIES`.
//! **No `URL` property.** Guess `kind` from `SUMMARY` (and maybe description);
//! `CATEGORIES` is the course shortname. `UID` is `{eventid}@moodle.hku.hk`
//! so it lines up with AJAX `id`. Due items have `DTSTART == DTEND`.
//!
//! # 3. `token.php` — dead, do not build on it
//!
//! `GET /login/token.php?service=moodle_mobile_app` exists and returns JSON.
//! No username → `errorcode: missingparam`. Dummy Moodle user/pass →
//! `invalidlogin`. Session cookies are ignored; it wants a local Moodle
//! password, which HKU Portal SSO does not provide. Same for services
//! `moodle_webservice` and `moodle_official_mobile_app`.

use serde::{Deserialize, Serialize};

use crate::auth::{cookie_header, SessionSecrets};

const AJAX_URL: &str = "https://moodle.hku.hk/lib/ajax/service.php";
const EXPORT_URL: &str = "https://moodle.hku.hk/calendar/export.php";
const FALLBACK_URL: &str = "https://moodle.hku.hk/calendar/view.php?view=upcoming";
const AJAX_METHOD: &str = "core_calendar_get_action_events_by_timesort";
// Moodle core rejects limitnum outside 1..=50.
const PAGE_SIZE: u32 = 50;
const MAX_PAGES: u32 = 20;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TodoKind {
    Assignment,
    Turnitin,
    Registration,
    Assessment,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum TodoSourceKind {
    Ajax,
    Ical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub course: Option<String>,
    pub kind: TodoKind,
    pub deadline: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    pub source: TodoSourceKind,
    pub items: Vec<TodoItem>,
}

pub trait TodoSource {
    async fn fetch(&self) -> Result<Vec<TodoItem>, String>;
}

pub struct AjaxSession<'a> {
    secrets: &'a SessionSecrets,
}

pub struct Ical<'a> {
    secrets: &'a SessionSecrets,
}

/// Try AJAX first; empty list or hard failure falls back to iCal.
pub async fn fetch_todos(secrets: &SessionSecrets) -> Result<FetchResult, String> {
    let ajax = AjaxSession { secrets }.fetch().await;
    match ajax {
        Ok(items) if !items.is_empty() => Ok(FetchResult {
            source: TodoSourceKind::Ajax,
            items,
        }),
        ajax_outcome => {
            let ical = Ical { secrets }.fetch().await;
            match ical {
                Ok(items) => Ok(FetchResult {
                    source: TodoSourceKind::Ical,
                    items,
                }),
                Err(ical_err) => match ajax_outcome {
                    Err(ajax_err) => Err(format!("ajax: {ajax_err}; ical: {ical_err}")),
                    Ok(_) => Err(ical_err),
                },
            }
        }
    }
}

impl TodoSource for AjaxSession<'_> {
    async fn fetch(&self) -> Result<Vec<TodoItem>, String> {
        let client = http_client()?;
        let cookie = cookie_header(self.secrets);
        let mut items = Vec::new();
        let mut aftereventid: Option<i64> = None;

        for _ in 0..MAX_PAGES {
            let page = fetch_ajax_page(&client, &cookie, &self.secrets.sesskey, aftereventid).await?;
            if page.is_empty() {
                break;
            }

            let last_id = page.last().and_then(ajax_event_id);
            let page_len = page.len();
            for event in page {
                if let Some(item) = todo_from_ajax(event) {
                    items.push(item);
                }
            }
            // A short page means we reached the end; repeating lastid would loop.
            if page_len < PAGE_SIZE as usize {
                break;
            }
            match last_id {
                Some(id) if aftereventid != Some(id) => aftereventid = Some(id),
                _ => break,
            }
        }

        Ok(items)
    }
}

impl TodoSource for Ical<'_> {
    async fn fetch(&self) -> Result<Vec<TodoItem>, String> {
        let client = http_client()?;
        let form = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("sesskey", &self.secrets.sesskey)
            .append_pair("_qf__core_calendar_export_form", "1")
            .append_pair("events[exportevents]", "all")
            // Spike: recentupcoming missed later dues; custom matched AJAX.
            .append_pair("period[timeperiod]", "custom")
            .append_pair("export", "1")
            .finish();

        let response = client
            .post(EXPORT_URL)
            .header(reqwest::header::COOKIE, cookie_header(self.secrets))
            .header(reqwest::header::USER_AGENT, user_agent())
            .header(
                reqwest::header::CONTENT_TYPE,
                "application/x-www-form-urlencoded",
            )
            .body(form)
            .send()
            .await
            .map_err(err)?;

        let ctype = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .unwrap_or("")
            .to_string();

        let text = response.text().await.map_err(err)?;
        if !text.contains("BEGIN:VCALENDAR") {
            return Err(format!(
                "calendar export did not return iCal (content-type {ctype})"
            ));
        }
        Ok(parse_ics(&text))
    }
}

fn http_client() -> Result<reqwest::Client, String> {
    // Follow redirects: POST export.php lands on export_execute.php.
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::limited(10))
        .build()
        .map_err(err)
}

fn user_agent() -> &'static str {
    "Mozilla/5.0 (HKU Moodle Helper)"
}

async fn fetch_ajax_page(
    client: &reqwest::Client,
    cookie: &str,
    sesskey: &str,
    aftereventid: Option<i64>,
) -> Result<Vec<AjaxEvent>, String> {
    let mut args = serde_json::json!({
        "limitnum": PAGE_SIZE,
        // 0 keeps overdue action events visible, not only future ones.
        "timesortfrom": 0,
        "limittononsuspendedevents": true
    });
    if let Some(id) = aftereventid {
        args["aftereventid"] = serde_json::json!(id);
    }

    let payload = serde_json::json!([{
        "index": 0,
        "methodname": AJAX_METHOD,
        "args": args
    }]);

    let url = format!("{AJAX_URL}?sesskey={sesskey}&info={AJAX_METHOD}");
    let response = client
        .post(url)
        .header(reqwest::header::COOKIE, cookie)
        .header(reqwest::header::USER_AGENT, user_agent())
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .body(payload.to_string())
        .send()
        .await
        .map_err(err)?;
    let text = response.text().await.map_err(err)?;
    let envelopes: Vec<AjaxEnvelope> = serde_json::from_str(&text).map_err(err)?;
    let first = envelopes
        .into_iter()
        .next()
        .ok_or_else(|| "empty AJAX envelope".to_string())?;
    if first.error {
        let code = first
            .exception
            .as_ref()
            .and_then(|ex| ex.errorcode.as_deref())
            .unwrap_or("unknown");
        let message = first
            .exception
            .as_ref()
            .and_then(|ex| ex.message.as_deref())
            .unwrap_or("AJAX error");
        return Err(format!("{code}: {message}"));
    }
    Ok(first.data.map(|data| data.events).unwrap_or_default())
}

fn todo_from_ajax(event: AjaxEvent) -> Option<TodoItem> {
    let title = event.name.as_deref().filter(|name| !name.is_empty())?;
    let timestamp = event.timesort.or(event.timestart)?;
    let url = event
        .url
        .as_deref()
        .filter(|url| !url.is_empty())
        .or(event
            .action
            .as_ref()
            .and_then(|action| action.url.as_deref()))
        .filter(|url| !url.is_empty())?
        .to_string();
    let course = event.course.as_ref().and_then(|course| {
        course
            .shortname
            .as_deref()
            .filter(|name| !name.is_empty())
            .or(course.fullname.as_deref().filter(|name| !name.is_empty()))
            .map(str::to_string)
    });
    Some(TodoItem {
        id: event.id.unwrap_or(0).to_string(),
        title: title.to_string(),
        course,
        kind: kind_from_modulename(event.modulename.as_deref().unwrap_or("")),
        deadline: unix_to_iso8601(timestamp),
        url,
    })
}

fn ajax_event_id(event: &AjaxEvent) -> Option<i64> {
    event.id
}

fn kind_from_modulename(modulename: &str) -> TodoKind {
    match modulename {
        "assign" | "assignment" => TodoKind::Assignment,
        "turnitintooltwo" | "turnitintool" | "turnitin" => TodoKind::Turnitin,
        "quiz" | "quizattempt" => TodoKind::Assessment,
        "enrol" | "enroll" | "registration" => TodoKind::Registration,
        // Seen on this account; not a due-date assignment. Keep filterable as other
        // until the UI grows an extra checkbox.
        _ => TodoKind::Other,
    }
}

fn kind_from_ical_text(summary: &str, description: &str) -> TodoKind {
    let haystack = format!("{summary} {description}").to_ascii_lowercase();
    if haystack.contains("turnitin") {
        TodoKind::Turnitin
    } else if haystack.contains("quiz") || haystack.contains("test") || haystack.contains("exam") {
        TodoKind::Assessment
    } else if haystack.contains("enrol") || haystack.contains("registration") {
        TodoKind::Registration
    } else if haystack.contains("assign")
        || haystack.contains("submission")
        || haystack.contains("due")
    {
        TodoKind::Assignment
    } else {
        TodoKind::Other
    }
}

fn parse_ics(text: &str) -> Vec<TodoItem> {
    let unfolded = unfold_ics(text);
    let mut items = Vec::new();
    for block in unfolded.split("BEGIN:VEVENT").skip(1) {
        let event = block.split("END:VEVENT").next().unwrap_or(block);
        let Some(summary) = ics_prop(event, "SUMMARY") else {
            continue;
        };
        let Some(deadline_raw) = ics_prop(event, "DTEND").or_else(|| ics_prop(event, "DTSTART"))
        else {
            continue;
        };
        let deadline = ics_datetime_to_iso(&deadline_raw).unwrap_or(deadline_raw);
        let course = ics_prop(event, "CATEGORIES").filter(|value| !value.is_empty());
        let description = ics_prop(event, "DESCRIPTION").unwrap_or_default();
        let uid = ics_prop(event, "UID").unwrap_or_default();
        let id = uid
            .split('@')
            .next()
            .filter(|part| !part.is_empty())
            .unwrap_or(&uid)
            .to_string();
        // HKU ICS has no URL property; send the user to the upcoming calendar.
        items.push(TodoItem {
            id,
            title: unescape_ics(&summary),
            course: course.map(|value| unescape_ics(&value)),
            kind: kind_from_ical_text(&summary, &description),
            deadline,
            url: FALLBACK_URL.to_string(),
        });
    }
    items
}

fn unfold_ics(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix(' ').or_else(|| line.strip_prefix('\t')) {
            out.push_str(rest);
        } else {
            if !out.is_empty() {
                out.push('\n');
            }
            out.push_str(line);
        }
    }
    out
}

fn ics_prop(block: &str, name: &str) -> Option<String> {
    for line in block.lines() {
        let Some((head, value)) = line.split_once(':') else {
            continue;
        };
        let prop = head.split(';').next().unwrap_or(head);
        if prop.eq_ignore_ascii_case(name) {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn unescape_ics(value: &str) -> String {
    value
        .replace("\\n", "\n")
        .replace("\\,", ",")
        .replace("\\;", ";")
        .replace("\\\\", "\\")
}

/// `20261014T155900Z` or `20261014` → RFC3339 UTC.
fn ics_datetime_to_iso(raw: &str) -> Option<String> {
    let compact: String = raw.chars().filter(|ch| ch.is_ascii_alphanumeric()).collect();
    if compact.len() >= 15 {
        let year = &compact[0..4];
        let month = &compact[4..6];
        let day = &compact[6..8];
        let hour = &compact[9..11];
        let min = &compact[11..13];
        let sec = &compact[13..15];
        Some(format!("{year}-{month}-{day}T{hour}:{min}:{sec}Z"))
    } else if compact.len() >= 8 {
        let year = &compact[0..4];
        let month = &compact[4..6];
        let day = &compact[6..8];
        Some(format!("{year}-{month}-{day}T00:00:00Z"))
    } else {
        None
    }
}

fn unix_to_iso8601(secs: i64) -> String {
    if secs < 0 {
        return "1970-01-01T00:00:00Z".into();
    }
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400) as u32;
    let hour = rem / 3600;
    let min = (rem % 3600) / 60;
    let sec = rem % 60;
    let (year, month, day) = civil_from_days(days);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{min:02}:{sec:02}Z")
}

/// Howard Hinnant civil-from-days; `days` is days since 1970-01-01.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

#[derive(Debug, Deserialize)]
struct AjaxEnvelope {
    error: bool,
    #[serde(default)]
    data: Option<AjaxData>,
    #[serde(default)]
    exception: Option<AjaxException>,
}

#[derive(Debug, Deserialize)]
struct AjaxData {
    #[serde(default)]
    events: Vec<AjaxEvent>,
}

#[derive(Debug, Deserialize)]
struct AjaxException {
    errorcode: Option<String>,
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AjaxEvent {
    id: Option<i64>,
    name: Option<String>,
    modulename: Option<String>,
    timestart: Option<i64>,
    timesort: Option<i64>,
    url: Option<String>,
    course: Option<AjaxCourse>,
    action: Option<AjaxAction>,
}

#[derive(Debug, Deserialize)]
struct AjaxCourse {
    shortname: Option<String>,
    fullname: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AjaxAction {
    url: Option<String>,
}

fn err<E: std::fmt::Display>(error: E) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_known_modules() {
        assert_eq!(kind_from_modulename("assign"), TodoKind::Assignment);
        assert_eq!(kind_from_modulename("turnitintooltwo"), TodoKind::Turnitin);
        assert_eq!(kind_from_modulename("quiz"), TodoKind::Assessment);
        assert_eq!(kind_from_modulename("questionnaire"), TodoKind::Other);
    }

    #[test]
    fn unix_formats_known_instant() {
        // 2026-10-14T15:59:00Z == 1791993540, from the spike sample.
        assert_eq!(unix_to_iso8601(1_791_993_540), "2026-10-14T15:59:00Z");
    }

    #[test]
    fn parses_hk_ics_event() {
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:10749654@moodle.hku.hk\r\nSUMMARY:Partial Report\r\nDTSTART:20261014T155900Z\r\nDTEND:20261014T155900Z\r\nCATEGORIES:_CAES9542_2026\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n";
        let items = parse_ics(ics);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].id, "10749654");
        assert_eq!(items[0].title, "Partial Report");
        assert_eq!(items[0].course.as_deref(), Some("_CAES9542_2026"));
        assert_eq!(items[0].deadline, "2026-10-14T15:59:00Z");
        assert_eq!(items[0].url, FALLBACK_URL);
    }
}
