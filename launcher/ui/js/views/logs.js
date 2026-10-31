// Live logs: the launcher's own log and each game's output, with level
// filters, search, auto-scroll and copy.
import { api } from "../api.js";
import { icon } from "../icons.js";
import { store } from "../store.js";
import { h, btn, toast, tabs, select, toggle, debounce, esc } from "../ui.js";

export function render(root, { query }) {
  let cleanup = null;
  const body = h("div");
  const ids = () => store.instances.map((i) => [i.id, i.name + (store.running.has(i.id) ? "  ●" : "")]);
  let source = query.instance ? "game" : "launcher";
  let instanceId = query.instance || [...store.running][0] || store.instances[0]?.id;
  const draw = () => {
    cleanup?.();
    body.innerHTML = "";
    cleanup = logPanel(body, { source, instanceId });
  };
  const picker = h("div");
  const paintPicker = () => {
    picker.innerHTML = "";
    if (source === "game" && store.instances.length) picker.append(select(ids(), instanceId, (v) => { instanceId = v; draw(); }));
  };
  root.append(
    h("div.page-head",
      h("div", h("h1", "Logs"), h("p", "Everything Salt and your games print, live.")),
      h("div.row", picker, tabs([["launcher", "Launcher", "terminal"], ["game", "Game", "gamepad"]], source, (s) => { source = s; paintPicker(); draw(); })),
    ),
    body,
  );
  paintPicker();
  draw();
  return () => cleanup?.();
}

const LEVELS = ["INFO", "WARN", "ERROR", "DEBUG"];

/** Renders a filterable live log into root. Returns a cleanup function. */
export function logPanel(root, { source, instanceId }) {
  const show = new Set(["INFO", "WARN", "ERROR", "FATAL", "DEBUG"]);
  let term = "", follow = true;
  const view = h("div.log-view");
  const counter = h("span.small.faint");

  const lines = () => (source === "launcher" ? store.launcherLogs : store.gameLogs.get(instanceId) || []);
  const lineEl = (l) => {
    const lvl = (l.level || "INFO").toUpperCase();
    const t = l.time ? new Date(l.time).toLocaleTimeString([], { hour12: false }) : "";
    let msg = esc(l.message);
    if (term) msg = msg.replace(new RegExp(term.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"), "gi"), (m) => `<mark>${m}</mark>`);
    return h("div.log-line." + lvl, { html: `<span class="lt">${t}</span><span class="ll">${lvl}</span>${l.thread ? `<span class="lth">${esc(l.thread)}</span>` : ""}<span>${msg}</span>` });
  };
  const visible = (l) => show.has((l.level || "INFO").toUpperCase()) && (!term || (l.message || "").toLowerCase().includes(term));
  const redraw = () => {
    const all = lines().filter(visible);
    const slice = all.slice(-3000);
    view.replaceChildren(...slice.map(lineEl));
    if (!slice.length) view.append(h("div.empty", source === "game" ? "No output yet. Launch the game to see its log here." : "Nothing logged yet."));
    counter.textContent = `${all.length} lines`;
    if (follow) view.scrollTop = view.scrollHeight;
  };
  const append = (newLines) => {
    if (view.querySelector(".empty")) view.innerHTML = "";
    const frag = document.createDocumentFragment();
    for (const l of newLines) if (visible(l)) frag.appendChild(lineEl(l));
    view.appendChild(frag);
    while (view.childElementCount > 3000) view.firstChild.remove();
    counter.textContent = `${lines().filter(visible).length} lines`;
    if (follow) view.scrollTop = view.scrollHeight;
  };
  view.addEventListener("scroll", () => { follow = view.scrollTop + view.clientHeight >= view.scrollHeight - 30; followToggle.querySelector("input").checked = follow; });

  const levelBtns = LEVELS.map((lv) => {
    const b = btn(lv[0] + lv.slice(1).toLowerCase(), { cls: "sm" });
    const paint = () => (b.style.opacity = show.has(lv) ? "1" : ".45");
    b.onclick = () => { show.has(lv) ? show.delete(lv) : show.add(lv); if (lv === "ERROR") show.has("ERROR") ? show.add("FATAL") : show.delete("FATAL"); paint(); redraw(); };
    paint();
    return b;
  });
  const search = h("input.input", { placeholder: "Filter…", style: { height: "30px" }, oninput: debounce((e) => { term = e.target.value.toLowerCase(); redraw(); }, 150) });
  const followToggle = h("label.row.small.muted", toggle(true, (v) => { follow = v; if (v) view.scrollTop = view.scrollHeight; }), "Auto-scroll");
  const copy = () => {
    const text = lines().filter(visible).map((l) => `[${(l.level || "").toUpperCase()}] ${l.message}`).join("\n");
    navigator.clipboard.writeText(text).then(() => toast("Copied log", `${text.split("\n").length} lines`, "ok", 1800));
  };
  root.append(
    h("div.row.wrap", { style: { marginBottom: "10px" } },
      h("div.search", { html: icon("search"), style: { width: "220px" } }, search),
      ...levelBtns, h("span.grow"), counter, followToggle,
      btn("Copy", { cls: "sm", icon: "copy", onclick: copy }),
      source === "game" && instanceId ? btn("Logs folder", { cls: "sm", icon: "folder", onclick: () => api.openInstanceFolder(instanceId, "logs") }) : btn("Data folder", { cls: "sm", icon: "folder", onclick: () => api.openDataFolder() }),
    ),
    view,
  );
  redraw();
  const off = source === "launcher"
    ? store.on("launcher-log", (l) => append([l]))
    : store.on("game-log", ({ id, lines: ls }) => id === instanceId && append(ls));
  const off2 = store.on("game", (ev) => ev.id === instanceId && ev.state === "preparing" && source === "game" && redraw());
  return () => { off(); off2(); };
}
