import { openUrl } from "@tauri-apps/plugin-opener";
import { MOCK_TODOS } from "./mock-todos";
import { TODO_KINDS, type TodoItem, type TodoKind } from "./types";

const dateFmt = new Intl.DateTimeFormat(undefined, {
  dateStyle: "medium",
  timeStyle: "short",
});

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

function cardHtml(todo: TodoItem): string {
  const when = dateFmt.format(new Date(todo.deadline));
  const course = todo.course ? `${escapeHtml(todo.course)} · ` : "";
  return `
    <li class="card">
      <span class="card-icon">
        <img src="/src/assets/icons/info.svg" width="29.6667" height="29.6667" alt="" />
      </span>
      <div class="card-body">
        <div>
          <h2 class="card-title">${escapeHtml(todo.title)}</h2>
          <p class="card-text">${course}Due ${escapeHtml(when)}</p>
        </div>
        <a class="card-open" href="${escapeHtml(todo.url)}" target="_blank" rel="noopener noreferrer">
          Open
        </a>
      </div>
    </li>
  `;
}

function render(): void {
  const list = document.querySelector("#todo-list");
  const search = document.querySelector<HTMLInputElement>("#search");
  if (!list || !search) {
    return;
  }

  const kinds = selectedKinds();
  const query = search.value.trim().toLowerCase();
  const items = MOCK_TODOS.filter(
    (todo) => kinds.has(todo.kind) && matchesQuery(todo, query),
  );

  list.innerHTML =
    items.length > 0
      ? items.map(cardHtml).join("")
      : `<li class="empty">No matching to-dos</li>`;
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
  render();

  document.querySelector(".filters")?.addEventListener("change", render);
  document.querySelector("#search")?.addEventListener("input", render);

  document.querySelector("#todo-list")?.addEventListener("click", (event) => {
    const link = (event.target as HTMLElement).closest<HTMLAnchorElement>("a.card-open");
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
  });
});
