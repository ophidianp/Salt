// UI helpers: element builder, toasts, modals, menus, form controls.
import { icon } from "./icons.js";

/** h("div.card.hover", {onclick}, child, "text") — tiny hyperscript. */
export function h(tag, attrs, ...children) {
  const [name, ...classes] = tag.split(".");
  const el = document.createElement(name || "div");
  if (classes.length) el.className = classes.join(" ");
  if (attrs && (typeof attrs !== "object" || attrs instanceof Node || Array.isArray(attrs))) {
    children.unshift(attrs);
    attrs = null;
  }
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v == null || v === false) continue;
    if (k.startsWith("on")) el.addEventListener(k.slice(2), v);
    else if (k === "html") el.innerHTML = v;
    else if (k === "class") el.className += (el.className ? " " : "") + v;
    else if (k === "style" && typeof v === "object") Object.assign(el.style, v);
    else if (k in el && k !== "list" && typeof v !== "string") el[k] = v;
    else el.setAttribute(k, v === true ? "" : v);
  }
  append(el, children);
  return el;
}

function append(el, children) {
  for (const c of children.flat(Infinity)) {
    if (c == null || c === false) continue;
    el.appendChild(c instanceof Node ? c : document.createTextNode(String(c)));
  }
}

/** Element with inline SVG icon + optional text. */
export const ic = (name, cls) => h("span", { html: icon(name, cls), style: { display: "contents" } });

export function btn(label, opts = {}) {
  const { icon: ico, cls = "", onclick, title, disabled } = opts;
  const b = h(`button.btn${cls ? "." + cls.split(" ").join(".") : ""}`, { onclick, title, disabled });
  if (ico) b.insertAdjacentHTML("beforeend", icon(ico));
  if (label) b.append(label);
  return b;
}

export function esc(s) {
  return String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);
}

// ------------------------------------------------------------------- toasts
export function toast(title, msg = "", kind = "info", ms = 4200) {
  const root = document.getElementById("toasts");
  const ico = kind === "ok" ? "check" : kind === "err" ? "alert" : "info";
  const t = h("div.toast.glass." + kind, { html: icon(ico) }, h("div", h("div.t-title", title), msg ? h("div.t-msg", msg) : null));
  root.appendChild(t);
  const close = () => {
    t.classList.add("out");
    t.addEventListener("animationend", () => t.remove(), { once: true });
  };
  t.onclick = close;
  setTimeout(close, kind === "err" ? ms * 2 : ms);
}

export const toastErr = (title) => (e) => toast(title, String(e?.message || e), "err");

// ------------------------------------------------------------------ modals
export function modal(build, { wide = false } = {}) {
  const root = document.getElementById("modal-root");
  const wrap = h("div.modal-wrap");
  const box = h(`div.modal.glass${wide ? ".wide" : ""}`);
  wrap.appendChild(box);
  let closed = false;
  const close = (v) => {
    if (closed) return;
    closed = true;
    wrap.classList.add("closing");
    setTimeout(() => wrap.remove(), 250);
    document.removeEventListener("keydown", onKey);
    resolveFn?.(v);
  };
  const onKey = (e) => e.key === "Escape" && close();
  document.addEventListener("keydown", onKey);
  wrap.addEventListener("mousedown", (e) => e.target === wrap && close());
  let resolveFn;
  const done = new Promise((r) => (resolveFn = r));
  append(box, [build(close)]);
  root.appendChild(wrap);
  setTimeout(() => box.querySelector("input,select,textarea")?.focus(), 60);
  return done;
}

export function confirmDialog(title, text, { ok = "Confirm", danger = false } = {}) {
  return modal((close) => [
    h("h2", title),
    h("p.m-sub", text),
    h("div.m-actions", btn("Cancel", { onclick: () => close(false) }), btn(ok, { cls: danger ? "danger" : "primary", onclick: () => close(true) })),
  ]);
}

export function promptDialog(title, label, value = "", placeholder = "") {
  return modal((close) => {
    const input = h("input.input", { value, placeholder });
    input.addEventListener("keydown", (e) => e.key === "Enter" && close(input.value.trim()));
    return [h("h2", title), h("div.field", { style: { margin: "14px 0 0" } }, h("label", label), input), h("div.m-actions", btn("Cancel", { onclick: () => close(null) }), btn("OK", { cls: "primary", onclick: () => close(input.value.trim()) }))];
  });
}

// -------------------------------------------------------------------- menu
export function menu(anchor, items) {
  document.querySelectorAll(".menu").forEach((m) => m.remove());
  const m = h("div.menu.glass");
  for (const it of items) {
    if (it === "-") { m.appendChild(h("div.sep")); continue; }
    const b = h("button", { class: it.danger ? "danger" : "", html: icon(it.icon || "chevron") }, it.label);
    b.onclick = (e) => { e.stopPropagation(); m.remove(); it.onclick(); };
    m.appendChild(b);
  }
  document.body.appendChild(m);
  const r = anchor.getBoundingClientRect();
  const mw = m.offsetWidth, mh = m.offsetHeight;
  m.style.left = Math.min(r.left, innerWidth - mw - 10) + "px";
  m.style.top = (r.bottom + mh + 8 > innerHeight ? r.top - mh - 6 : r.bottom + 6) + "px";
  setTimeout(() => document.addEventListener("click", () => m.remove(), { once: true }), 0);
}

// ---------------------------------------------------------------- controls
export function toggle(checked, onchange) {
  const input = h("input", { type: "checkbox", checked });
  input.addEventListener("change", () => onchange(input.checked));
  return h("label.switch", input, h("span.track"), h("span.thumb"));
}

export function range(value, min, max, step, oninput, fmt = (v) => v) {
  const out = h("span.small.muted", { style: { minWidth: "64px", textAlign: "right", fontVariantNumeric: "tabular-nums" } }, fmt(value));
  const r = h("input", { type: "range", min, max, step, value });
  const paint = () => r.style.setProperty("--fill", ((r.value - min) / (max - min)) * 100 + "%");
  paint();
  r.addEventListener("input", () => { paint(); out.textContent = fmt(+r.value); oninput(+r.value); });
  return h("div.row", { style: { width: "260px" } }, r, out);
}

export function select(options, value, onchange) {
  const s = h("select.input");
  for (const o of options) {
    const [v, label] = Array.isArray(o) ? o : [o, o];
    s.appendChild(h("option", { value: v, selected: v === value }, label));
  }
  s.addEventListener("change", () => onchange(s.value));
  return s;
}

/** Segmented tabs with a sliding pill. */
export function tabs(items, active, onchange) {
  const wrap = h("div.tabs");
  const pill = h("span.pill");
  wrap.appendChild(pill);
  const buttons = items.map(([id, label, ico]) => {
    const b = h("button.tab", { html: ico ? icon(ico) : "" }, label);
    b.onclick = () => { set(id); onchange(id); };
    b.dataset.id = id;
    wrap.appendChild(b);
    return b;
  });
  const set = (id) => {
    for (const b of buttons) b.classList.toggle("active", b.dataset.id === id);
    const b = buttons.find((b) => b.dataset.id === id);
    if (b) requestAnimationFrame(() => { pill.style.left = b.offsetLeft + "px"; pill.style.width = b.offsetWidth + "px"; });
  };
  set(active);
  wrap.set = set;
  return wrap;
}

export function setting(title, desc, control) {
  return h("div.setting", h("div.s-text", h("div.s-title", title), desc ? h("div.s-desc", desc) : null), h("div.s-ctl", control));
}

// ------------------------------------------------------------------ format
export function fmtBytes(n) {
  if (!n) return "0 B";
  const u = ["B", "KB", "MB", "GB"];
  const i = Math.min(Math.floor(Math.log(n) / Math.log(1024)), 3);
  return (n / 1024 ** i).toFixed(i ? 1 : 0) + " " + u[i];
}
export function fmtNum(n) {
  return n >= 1e6 ? (n / 1e6).toFixed(1) + "M" : n >= 1e3 ? (n / 1e3).toFixed(1) + "k" : String(n);
}
export function fmtDuration(s) {
  if (!s) return "Never played";
  const h_ = Math.floor(s / 3600), m = Math.floor((s % 3600) / 60);
  return h_ ? `${h_}h ${m}m played` : `${m}m played`;
}
export function fmtAgo(iso) {
  if (!iso) return "Never";
  const d = (Date.now() - new Date(iso).getTime()) / 1000;
  if (d < 60) return "Just now";
  if (d < 3600) return Math.floor(d / 60) + "m ago";
  if (d < 86400) return Math.floor(d / 3600) + "h ago";
  if (d < 86400 * 30) return Math.floor(d / 86400) + "d ago";
  return new Date(iso).toLocaleDateString();
}
export const debounce = (fn, ms = 300) => { let t; return (...a) => { clearTimeout(t); t = setTimeout(() => fn(...a), ms); }; };

// ------------------------------------------------------------- player heads
const headCache = new Map();
/** Draws the face (+ hat layer) of a skin data URL onto a canvas. */
export function headCanvas(dataUrl, size = 32) {
  const c = h("canvas", { width: 8, height: 8 });
  c.style.width = c.style.height = size + "px";
  if (!dataUrl) return c;
  const img = new Image();
  img.onload = () => {
    const ctx = c.getContext("2d");
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(img, 8, 8, 8, 8, 0, 0, 8, 8);
    ctx.drawImage(img, 40, 8, 8, 8, 0, 0, 8, 8);
  };
  img.src = dataUrl;
  return c;
}
export async function headFor(api, uuid, size = 32) {
  if (!headCache.has(uuid)) headCache.set(uuid, api.playerSkin(uuid).catch(() => null));
  return headCanvas(await headCache.get(uuid), size);
}

export const EMOJI_ICONS = ["⛏️", "🗡️", "🏹", "🧱", "🌲", "🔥", "💎", "🧪", "🛡️", "🐉", "🌋", "❄️", "🌊", "⚡", "🍄", "🏰", "🚀", "🎯", "🧂", "🌙"];
