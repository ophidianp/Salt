// saltClient UI entry: shell, router, titlebar, progress dock.
import { api, appWindow } from "./api.js";
import { icon } from "./icons.js";
import { store, initStore } from "./store.js";
import { applyTheme } from "./theme.js";
import { h, toast, menu, headCanvas, btn } from "./ui.js";
import { showOnboarding, startLogin } from "./views/onboarding.js";
import * as home from "./views/home.js";
import * as instances from "./views/instances.js";
import * as instanceView from "./views/instance.js";
import * as discover from "./views/discover.js";
import * as skins from "./views/skins.js";
import * as friends from "./views/friends.js";
import * as logs from "./views/logs.js";
import * as settings from "./views/settings.js";

const ROUTES = {
  home: home.render,
  instances: instances.render,
  instance: instanceView.render,
  discover: discover.render,
  skins: skins.render,
  friends: friends.render,
  logs: logs.render,
  settings: settings.render,
};

// Toolbar links (Settings lives in the round gear button on the right).
const NAV = [
  ["home", "Home", "home"], ["instances", "Instances", "boxes"], ["discover", "Discover", "compass"],
  ["skins", "Skins", "shirt"], ["friends", "Friends", "users"], ["logs", "Logs", "terminal"],
];

let cleanup = null;
let lastRoute = null;

export function go(path) {
  if (location.hash !== "#/" + path) location.hash = "#/" + path;
  else route();
}

function parseHash() {
  const raw = location.hash.replace(/^#\/?/, "") || "home";
  const [path, query = ""] = raw.split("?");
  const parts = path.split("/").map(decodeURIComponent);
  return { name: parts[0], args: parts.slice(1), query: Object.fromEntries(new URLSearchParams(query)) };
}

function route() {
  const { name, args, query } = parseHash();
  const view = ROUTES[name] || ROUTES.home;
  if (typeof cleanup === "function") cleanup();
  const root = document.getElementById("view");
  root.innerHTML = "";
  // Moving between sub-pages of the same view (e.g. Settings sections) doesn't replay the page reveal.
  const page = h("div.page" + (name === lastRoute ? ".still" : ""));
  lastRoute = name;
  root.appendChild(page);
  root.scrollTop = 0;
  cleanup = view(page, { args, query });
  // Mono eyebrow above each page title, like the site's section labels.
  const h1 = page.querySelector(".page-head h1");
  if (h1 && !h1.previousElementSibling?.classList.contains("eyebrow")) {
    const label = NAV.find(([id]) => id === (name === "instance" ? "instances" : name))?.[1];
    if (label) h1.before(h("div.eyebrow", "saltClient · " + label));
  }
  const active = name === "instance" ? "instances" : name;
  for (const b of document.querySelectorAll(".nav-link")) b.classList.toggle("active", b.dataset.route === active);
  document.getElementById("settings-btn").classList.toggle("active", name === "settings");
  movePill();
}

/* Pill that glides between toolbar links, like the Chlorine site header. */
function movePill(target) {
  const nav = document.getElementById("topnav");
  const pill = nav?.querySelector(".nav-pill");
  if (!pill) return;
  const link = target || nav.querySelector(".nav-link.active");
  if (!link) { pill.style.opacity = "0"; return; }
  pill.style.opacity = "1";
  pill.style.width = `${link.offsetWidth}px`;
  pill.style.transform = `translateX(${link.offsetLeft}px)`;
}

function buildToolbar() {
  const nav = document.getElementById("topnav");
  nav.innerHTML = "";
  nav.appendChild(h("span.nav-pill"));
  for (const [id, label, ico] of NAV) {
    const b = h("button.nav-link", { html: icon(ico), title: label }, h("span", label));
    b.dataset.route = id;
    b.onclick = () => go(id);
    b.onmouseenter = () => movePill(b);
    if (id === "friends") {
      const badge = h("span.badge", { style: { display: "none" } });
      b.appendChild(badge);
      store.on("social", () => {
        const n = [...store.social.unread.values()].reduce((a, b) => a + b, 0) + store.social.incoming.length;
        badge.textContent = n;
        badge.style.display = n ? "" : "none";
      });
    }
    nav.appendChild(b);
  }
  nav.onmouseleave = () => movePill();
  addEventListener("resize", () => movePill());

  document.getElementById("brand").onclick = () => go("home");
  const gear = document.getElementById("settings-btn");
  gear.innerHTML = icon("settings");
  gear.onclick = () => go("settings");
  document.getElementById("theme-toggle").onclick = () => {
    const t = store.settings.theme;
    const now = document.documentElement.dataset.theme === "light" ? "dark" : "light";
    store.settings.theme = { ...t, mode: now };
    store.emit("settings", store.settings);
    store.saveSettings({ theme: store.settings.theme }).catch(() => {});
  };

  // Quick play: the most recently played instance, as the toolbar's primary button.
  const quick = document.getElementById("quick-play");
  const paint = () => {
    const last = store.instances[0];
    quick.innerHTML = "";
    if (!last) { quick.append(btn("New instance", { cls: "primary sm", icon: "plus", onclick: () => go("instances") })); return; }
    const b = playButton(last.id, { small: true });
    b.title = `${last.name} · ${last.mc_version}`;
    quick.append(b);
  };
  paint();
  store.on("instances", paint);
}

/* Scroll progress bar + toolbar shadow once the page scrolls, and the card spotlight. */
function pageEffects() {
  const view = document.getElementById("view");
  const header = document.getElementById("site-header");
  const bar = document.getElementById("scroll-progress");
  view.addEventListener("scroll", () => {
    const max = view.scrollHeight - view.clientHeight;
    bar.style.setProperty("--progress", max > 0 ? String(view.scrollTop / max) : "0");
    header.classList.toggle("is-scrolled", view.scrollTop > 8);
  }, { passive: true });
  addEventListener("pointermove", (e) => {
    const card = e.target.closest?.(".card");
    if (!card) return;
    const r = card.getBoundingClientRect();
    card.style.setProperty("--mx", `${e.clientX - r.left}px`);
    card.style.setProperty("--my", `${e.clientY - r.top}px`);
  }, { passive: true });
}

/** Play/Stop button that tracks game state for one instance. */
export function playButton(id, { small = false } = {}) {
  const b = h("button.btn" + (small ? ".primary.sm" : ".play"));
  const paint = () => {
    const running = store.running.has(id), preparing = store.preparing.has(id);
    b.classList.toggle("running", running);
    b.disabled = preparing;
    b.innerHTML = preparing ? '<span class="spinner" style="width:15px;height:15px"></span> Preparing' : running ? icon("stop") + " Stop" : icon("play") + " Play";
  };
  b.onclick = async (e) => {
    e.stopPropagation();
    if (store.running.has(id)) {
      api.kill(id).catch((err) => toast("Couldn't stop the game", String(err), "err"));
      return;
    }
    if (!store.activeAccount() && !store.settings.offline_test) {
      toast("Sign in first", "Add your Microsoft account to play (or enable Settings → Developer → Offline test launch).", "info");
      startLogin();
      return;
    }
    store.preparing.add(id);
    paint();
    api.launch(id).catch((err) => {
      store.preparing.delete(id);
      paint();
      toast("Launch failed", String(err), "err");
    });
  };
  paint();
  const off = store.on("game", (ev) => ev.id === id && paint());
  // Stop listening once the button leaves the page.
  const mo = new MutationObserver(() => { if (!b.isConnected) { off(); mo.disconnect(); } });
  requestAnimationFrame(() => mo.observe(document.body, { childList: true, subtree: true }));
  return b;
}

async function paintAccountChip() {
  const chip = document.getElementById("account-chip");
  const a = store.activeAccount();
  chip.innerHTML = "";
  if (!a) { chip.append(h("span.head-ph"), h("span", "Sign in")); return; }
  const skin = store.skins.get(a.uuid) || (a.skin_url ? await api.texture(a.skin_url).catch(() => null) : null);
  if (skin) store.skins.set(a.uuid, skin);
  chip.append(headCanvas(skin, 22), h("span", a.username));
}

function accountMenu(anchor) {
  const items = store.accounts.accounts.map((a) => ({
    label: (a.uuid === store.accounts.active ? "✓ " : "") + a.username,
    icon: "user",
    onclick: async () => { await api.setActiveAccount(a.uuid); await store.refreshAccounts(); toast("Switched account", a.username, "ok"); },
  }));
  menu(anchor, [
    ...items,
    ...(items.length ? ["-"] : []),
    { label: "Add Microsoft account", icon: "userPlus", onclick: () => startLogin() },
    { label: "Skins & capes", icon: "shirt", onclick: () => go("skins") },
    ...(store.activeAccount() ? [{ label: "Sign out " + store.activeAccount().username, icon: "logout", danger: true, onclick: async () => { await api.removeAccount(store.accounts.active); await store.refreshAccounts(); } }] : []),
  ]);
}

function progressDock() {
  const dock = document.getElementById("progress-dock");
  const rows = new Map();
  store.on("progress", (p) => {
    // Sub-tasks (launch-x-assets…) show as their own rows only while busy.
    let row = rows.get(p.task);
    if (!row) {
      const fill = h("i");
      const label = h("span"), count = h("span");
      const el = h("div.ptask.glass", h("div.p-label", label, count), h("div.bar", fill));
      dock.appendChild(el);
      row = { el, fill, label, count, timer: null };
      rows.set(p.task, row);
    }
    row.label.textContent = p.label;
    row.count.textContent = p.total > 1 ? `${p.done}/${p.total}` : "";
    row.fill.style.width = (p.total ? (p.done / p.total) * 100 : 0) + "%";
    clearTimeout(row.timer);
    if (p.finished) {
      row.timer = setTimeout(() => {
        row.el.classList.add("out");
        setTimeout(() => row.el.remove(), 260);
        rows.delete(p.task);
      }, 900);
    }
  });
  store.on("game", (ev) => {
    if (ev.state === "running" || ev.state === "error") {
      for (const [k, r] of rows) if (k.includes(ev.id)) { r.el.remove(); rows.delete(k); }
    }
    if (ev.state === "running") toast("Game started", nameOf(ev.id), "ok", 2500);
    if (ev.state === "exited" && ev.crashed) toast("Game crashed", `${nameOf(ev.id)} exited with code ${ev.code}. Check Logs for details.`, "err");
  });
}

const nameOf = (id) => store.instances.find((i) => i.id === id)?.name || id;

function windowControls() {
  const w = appWindow();
  document.getElementById("wc-min").onclick = () => w.minimize();
  document.getElementById("wc-max").onclick = () => w.toggleMaximize();
  document.getElementById("wc-close").onclick = () => w.close();
  document.getElementById("titlebar").addEventListener("dblclick", (e) => {
    if (e.target.closest("[data-tauri-drag-region]") === e.target) w.toggleMaximize();
  });
  document.getElementById("account-chip").onclick = (e) => { e.stopPropagation(); accountMenu(e.currentTarget); };
}

function runningStatus() {
  const el = document.getElementById("titlebar-status");
  const paint = () => {
    const n = store.running.size;
    el.textContent = n ? `${n} game${n > 1 ? "s" : ""} running` : "";
  };
  store.on("game", paint);
  paint();
}

// Keyboard shortcuts: Ctrl+1..9 jump between pages.
function shortcuts() {
  const order = [...NAV.map(([id]) => id), "settings"];
  addEventListener("keydown", (e) => {
    if (e.ctrlKey && /^[1-9]$/.test(e.key) && order[+e.key - 1]) { e.preventDefault(); go(order[+e.key - 1]); }
  });
}

async function main() {
  // Block the browser context menu / reload keys so it feels like an app.
  addEventListener("contextmenu", (e) => { if (!e.target.closest("input,textarea,.log-view,.chat-msgs")) e.preventDefault(); });
  addEventListener("keydown", (e) => { if (e.key === "F5" || (e.ctrlKey && e.key.toLowerCase() === "r")) e.preventDefault(); });

  await initStore();
  applyTheme(store.settings.theme);
  store.on("settings", (s) => applyTheme(s.theme));
  windowControls();
  buildToolbar();
  pageEffects();
  progressDock();
  runningStatus();
  shortcuts();
  paintAccountChip();
  store.on("accounts", paintAccountChip);
  store.on("login", (r) => {
    if (r.ok) toast(`Welcome, ${r.account.username}!`, r.account.kind === "verified" ? "Verified with Minecraft." : "Signed in with Microsoft.", "ok");
  });
  addEventListener("hashchange", route);
  route();
  if (!store.settings.onboarded) showOnboarding();
  if (store.settings.social_enabled && store.activeAccount()) api.socialConnect().catch(() => {});
}

main().catch((e) => {
  document.getElementById("view").innerHTML = `<div class="empty"><h2>saltClient couldn't start</h2><p class="mono">${String(e)}</p></div>`;
  console.error(e);
});
