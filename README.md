# hku moodle helper client

This is a hku moodle client for grasping to-dos from [HKU Moodle](https://moodle.hku.hk/) so students can check their timetable anywhere they want without missing assignments, deadlines, etc.

## Background

After an update of the hku moodle site, the original hku moodle helper which provides the functionality of `adding courses of this semester` became unavailable, possibly because the new site prevents DOM injection.

## Design
The prototype should be as simple as possible. You can refer to the figma design for the initial UI/UX setup. The left column should be a list of checkboxes of filters, such as `assignments`, `turnitin`, `registration`, `assessments`, to name a few. The vertical divider should be togglable. The cards are to-do items and each should contains information such as to-do item title, deadline, and include a link that navigate to the resource on the website. The top searchbar should show only items relevant to the search query.   

This project is temporarily a desktop tauri client. When put into asleep, it should remain active on the system tray (macOS: top right corner; windows: bottom right corner).   

It is expected that this project should not be ultra-complicated. The goal is to minimize memory consumption and system usage.

## Implementation

Architecture is **A (session-cookie sidecar) + C (iCal fallback)** on Tauri 2. Working steps, stack rationale, and exit criteria: [docs/implementation.md](docs/implementation.md).