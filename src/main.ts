import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { openUrl } from "@tauri-apps/plugin-opener";
import { TODO_KINDS, type FetchResult, type TodoItem, type TodoKind } from "./types";

const dateFmt = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
});

let todos: TodoItem[] = [];
let needsLogin = false;

function escapeHtml(value: string): string {
  return value.replace(/[&<>"']/g, (char) => {
    switch (char) {
      case "&":
        return "&amp;";
      case "<":
        return "&lt;";
      case ">":
        return "&gt;";
      case '"':
        return "&quot;";
      default:
        return "&#39;";
    }
  });
}

function selectedKinds(): Set<TodoKind> {
  const checked = document.querySelectorAll<HTMLInputElement>(
    'input[name="kind"]:checked',
  );
  return new Set(Array.from(checked, (input) => input.value as TodoKind));
}

function matchesQuery(todo: TodoItem, query: string): boolean {
  if (!query) {
    return true;
  }
  const haystack = `${todo.title} ${todo.course ?? ""} ${todo.kind}`.toLowerCase();
  return haystack.includes(query);
}

const DAY_MS = 24 * 60 * 60 * 1000;

type Urgency = "critical" | "week" | "fortnight" | "month" | "later";

const URGENCY_LABEL: Record<Urgency, string> = {
  critical: "Due within 3 days",
  week: "Due this week",
  fortnight: "Due within 2 weeks",
  month: "Due this month",
  later: "Due later",
};

// Bands from now: [-3d, 3d] ! ; (3d, 7d) red; [7d, 14d) yellow; [14d, 30d) green; else gray.
function urgencyFor(deadline: string, now = Date.now()): Urgency {
  const delta = Date.parse(deadline) - now;
  if (Number.isNaN(delta)) {
    return "later";
  }
  if (delta >= -3 * DAY_MS && delta <= 3 * DAY_MS) {
    return "critical";
  }
  if (delta > 3 * DAY_MS && delta < 7 * DAY_MS) {
    return "week";
  }
  if (delta >= 7 * DAY_MS && delta < 14 * DAY_MS) {
    return "fortnight";
  }
  if (delta >= 14 * DAY_MS && delta < 30 * DAY_MS) {
    return "month";
  }
  return "later";
}

function deadlineMs(todo: TodoItem): number {
  const ms = Date.parse(todo.deadline);
  return Number.isNaN(ms) ? Number.POSITIVE_INFINITY : ms;
}

function isFarPast(todo: TodoItem, now = Date.now()): boolean {
  const ms = Date.parse(todo.deadline);
  return !Number.isNaN(ms) && ms < now - 3 * DAY_MS;
}

function pastHeadingHtml(): string {
  return `
    <li class="past-divider">
      <h3 class="past-divider-label">Past</h3>
    </li>
  `;
}

function urgencyHtml(urgency: Urgency): string {
  const label = URGENCY_LABEL[urgency];
  return `<span class="card-icon urgency-${urgency}" role="img" aria-label="${escapeHtml(label)}"></span>`;
}

function cardHtml(todo: TodoItem): string {
  const when = dateFmt.format(new Date(todo.deadline));
  const course = todo.course ? `${escapeHtml(todo.course)} · ` : "";
  return `
    <li class="card">
      ${urgencyHtml(urgencyFor(todo.deadline))}
      <div class="card-body">
        <h2 class="card-title">
          <a class="card-title-link" href="${escapeHtml(todo.url)}" target="_blank" rel="noopener noreferrer">${escapeHtml(todo.title)}</a>
        </h2>
        <p class="card-text">${course}Due ${escapeHtml(when)}</p>
      </div>
    </li>
  `;
}

function loginHtml(): string {
  return `
    <li class="empty">
      <button type="button" class="card-open" data-login>Log in</button>
    </li>
  `;
}

function applyResult(result: FetchResult | null): void {
  if (!result) {
    todos = [];
    needsLogin = true;
    return;
  }
  todos = result.items;
  needsLogin = false;
}

function isNotLoggedIn(error: unknown): boolean {
  return String(error).includes("not logged in");
}

async function loadCache(): Promise<void> {
  try {
    applyResult(await invoke<FetchResult | null>("list_todos"));
  } catch {
    todos = [];
  }
  render();
}

async function refresh(): Promise<void> {
  try {
    applyResult(await invoke<FetchResult>("refresh_todos"));
  } catch (error) {
    if (isNotLoggedIn(error)) {
      needsLogin = true;
    }
  }
  render();
}

function render(): void {
  const list = document.querySelector("#todo-list");
  const search = document.querySelector<HTMLInputElement>("#search");
  if (!list || !search) {
    return;
  }

  const kinds = selectedKinds();
  const query = search.value.trim().toLowerCase();
  const items = todos
    .filter((todo) => kinds.has(todo.kind) && matchesQuery(todo, query))
    .sort((a, b) => deadlineMs(a) - deadlineMs(b));

  const current = items.filter((todo) => !isFarPast(todo));
  const past = items.filter((todo) => isFarPast(todo));

  const login = needsLogin ? loginHtml() : "";
  const cards = current.map(cardHtml).join("");
  const pastSection = past.length
    ? pastHeadingHtml() + past.map(cardHtml).join("")
    : "";
  const empty =
    !cards && !pastSection && !needsLogin
      ? `<li class="empty">No matching to-dos</li>`
      : "";

  list.innerHTML = login + cards + pastSection + empty;
}

async function openExternal(url: string): Promise<void> {
  try {
    await openUrl(url);
  } catch {
    window.open(url, "_blank", "noopener,noreferrer");
  }
}

function mountFilters(): void {
  const fieldset = document.querySelector(".filters");
  if (!fieldset) {
    return;
  }

  fieldset.innerHTML = TODO_KINDS.map(
    ({ kind, label, description }) => `
      <label class="filter">
        <input type="checkbox" name="kind" value="${kind}" checked />
        <span class="filter-box">
          <img src="/src/assets/icons/check.svg" width="12.2667" height="8.93333" alt="" />
        </span>
        <span class="filter-copy">
          <span class="filter-label">${escapeHtml(label)}</span>
          <span class="filter-desc">${escapeHtml(description)}</span>
        </span>
      </label>
    `,
  ).join("");
}

window.addEventListener("DOMContentLoaded", () => {
  mountFilters();

  document.querySelector(".filters")?.addEventListener("change", render);
  document.querySelector("#search")?.addEventListener("input", render);
  document.querySelector("#refresh")?.addEventListener("click", () => {
    void refresh();
  });

  document.querySelector("#todo-list")?.addEventListener("click", (event) => {
    const target = event.target as HTMLElement;
    if (target.closest("[data-login]")) {
      void invoke("start_login");
      return;
    }

    const link = target.closest<HTMLAnchorElement>("a.card-title-link");
    if (!link || !("__TAURI_INTERNALS__" in window)) {
      return;
    }
    event.preventDefault();
    void openExternal(link.href);
  });

  const splitter = document.querySelector<HTMLButtonElement>("#splitter");
  splitter?.addEventListener("click", () => {
    const collapsed = document.querySelector(".app")?.classList.toggle("sidebar-collapsed");
    splitter.setAttribute("aria-expanded", collapsed ? "false" : "true");
    splitter.setAttribute("aria-label", collapsed ? "Show filters" : "Hide filters");
  });

  void listen("logged-in", () => {
    void refresh();
  });
  void loadCache();
});
