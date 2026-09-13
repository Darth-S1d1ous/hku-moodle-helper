import type { TodoItem } from "./types";

/** Local stand-ins until the Moodle client exists. */
export const MOCK_TODOS: TodoItem[] = [
  {
    id: "1",
    title: "Essay 1: Close reading",
    course: "ENGL1014",
    kind: "assignment",
    deadline: "2026-09-20T23:59:00+08:00",
    url: "https://moodle.hku.hk/mod/assign/view.php?id=1",
  },
  {
    id: "2",
    title: "Lab report 2",
    course: "COMP2396",
    kind: "assignment",
    deadline: "2026-09-18T17:00:00+08:00",
    url: "https://moodle.hku.hk/mod/assign/view.php?id=2",
  },
  {
    id: "3",
    title: "Research paper draft",
    course: "HIST2103",
    kind: "turnitin",
    deadline: "2026-09-22T12:00:00+08:00",
    url: "https://moodle.hku.hk/mod/turnitintooltwo/view.php?id=3",
  },
  {
    id: "4",
    title: "Add/drop remainder",
    course: "SIS",
    kind: "registration",
    deadline: "2026-09-15T16:00:00+08:00",
    url: "https://moodle.hku.hk/calendar/view.php",
  },
  {
    id: "5",
    title: "Midterm quiz",
    course: "STAT1601",
    kind: "assessment",
    deadline: "2026-09-25T14:30:00+08:00",
    url: "https://moodle.hku.hk/mod/quiz/view.php?id=5",
  },
  {
    id: "6",
    title: "Tutorial participation log",
    course: "POLI1003",
    kind: "assignment",
    deadline: "2026-10-02T23:59:00+08:00",
    url: "https://moodle.hku.hk/mod/assign/view.php?id=6",
  },
];
