// One instance: header + tabs (Content / Salt bundle / Worlds / Logs / Settings).
import { api } from "../api.js";
import { icon } from "../icons.js";
import { store } from "../store.js";
import { h, btn, toast, tabs, toggle, range, setting, select, fmtBytes, fmtDuration, fmtAgo, confirmDialog, debounce, EMOJI_ICONS, menu } from "../ui.js";
import { go, playButton } from "../app.js";
import { deleteInstance } from "./instances.js";
import { logPanel } from "./logs.js";

const SOURCE = { salt: ["salt", "saltMOD"], modrinth: ["modrinth", "Modrinth"], curseforge: ["curseforge", "CurseForge"], local: ["", "Local file"] };

export function render(root, { args }) {
  const [id, initialTab = "content"] = args;
  let inst = store.instances.find((i) => i.id === id);
  let cleanupTab = null;
  if (!inst) { root.append(h("div.empty", "That instance doesn't exist anymore."), btn("Back to instances", { onclick: () => go("instances") })); return; }

  const header = h("div.inst-header.glass");
  const body = h("div");
  const drawHeader = () => {
    header.innerHTML = "";
    header.append(
      h("div.inst-icon.lg", inst.icon?.startsWith("http") ? h("img", { src: inst.icon }) : inst.icon || "🧂"),
      h("div.grow",
        h("h1", inst.name),
        h("div.row.wrap", { style: { marginTop: "6px" } }, h("span.chip.accent", inst.mc_version), h("span.chip", inst.loader === "fabric" ? "Fabric " + (inst.loader_version || "latest") : "Vanilla"), h("span.small.muted", fmtDuration(inst.playtime_secs) + " · last played " + fmtAgo(inst.last_played))),
      ),
      btn("", { cls: "icon", icon: "folder", title: "Open folder", onclick: () => api.openInstanceFolder(inst.id) }),
      btn("", { cls: "icon", icon: "more", title: "More", onclick: (e) => menu(e.currentTarget, [
        { label: "Add mods from Discover", icon: "compass", onclick: () => go("discover?instance=" + inst.id) },
        { label: "Open mods folder", icon: "folder", onclick: () => api.openInstanceFolder(inst.id, "mods") },
        { label: "Open screenshots", icon: "image", onclick: () => api.openInstanceFolder(inst.id, "screenshots") },
        { label: "Duplicate", icon: "copy", onclick: async () => { const c = await api.duplicateInstance(inst.id); await store.refreshInstances(); go("instance/" + c.id); } },
        "-",
        { label: "Delete instance", icon: "trash", danger: true, onclick: async () => { if (await deleteInstance(inst)) go("instances"); } },
      ]) }),
      playButton(inst.id),
    );
  };

  const t = tabs([["content", "Content", "puzzle"], ["bundle", "Salt bundle", "sparkles"], ["worlds", "Worlds", "globe"], ["logs", "Live log", "terminal"], ["settings", "Settings", "settings"]], initialTab, (tab) => show(tab));
  const show = (tab) => {
    cleanupTab?.();
    cleanupTab = null;
    body.innerHTML = "";
    const wrap = h("div.page");
    body.appendChild(wrap);
    history.replaceState(null, "", `#/instance/${encodeURIComponent(inst.id)}/${tab}`);
    cleanupTab = ({ content: contentTab, bundle: bundleTab, worlds: worldsTab, logs: logsTab, settings: settingsTab })[tab](wrap, inst, (updated) => { inst = updated; drawHeader(); });
  };
  drawHeader();
  root.append(header, h("div", { style: { marginBottom: "14px" } }, t), body);
  show(initialTab);
  const off = store.on("instances", () => { const n = store.instances.find((i) => i.id === id); if (n) { inst = n; drawHeader(); } });
  return () => { off(); cleanupTab?.(); };
}

// ------------------------------------------------------------------ content
function contentTab(root, inst) {
  let mods = [], filter = "", updates = new Map();
  const list = h("div.list.stagger");
  const count = h("span.muted.small");
  const draw = () => {
    list.innerHTML = "";
    const shown = mods.filter((m) => !filter || (m.title + m.file_name + (m.mod_id || "")).toLowerCase().includes(filter));
    count.textContent = `${mods.length} mods · ${mods.filter((m) => m.enabled).length} enabled`;
    if (!shown.length) list.append(h("div.empty", { html: icon("puzzle") }, h("div", mods.length ? "No mods match your search." : "No mods yet. Add some from Discover, or launch once to install the Salt bundle.")));
    for (const m of shown) {
      const [cls, label] = SOURCE[m.source] || SOURCE.local;
      const upd = updates.get(m.file_name);
      const ico = h("div.li-icon", m.icon_url ? h("img", { src: m.icon_url, loading: "lazy" }) : m.source === "salt" ? h("span", { style: { color: "var(--salt)" } }, "S") : (m.title[0] || "?").toUpperCase());
      const row = h("div.list-item" + (m.enabled ? "" : ".off"),
        ico,
        h("div.grow",
          h("div.li-title", m.title, " ", m.version ? h("span.faint.small", m.version) : null),
          h("div.li-sub", m.description || m.file_name),
        ),
        h("div.li-actions",
          upd ? btn("Update to " + upd.latest, { cls: "sm primary", icon: "download", onclick: async (e) => { e.currentTarget.disabled = true; try { await api.mrInstall(inst.id, upd.project_id, upd.version_id); updates.delete(m.file_name); load(); toast("Updated", m.title, "ok"); } catch (err) { toast("Update failed", String(err), "err"); } } }) : null,
          h("span.chip" + (cls ? "." + cls : ""), label),
          h("span.small.faint", { style: { width: "64px", textAlign: "right" } }, fmtBytes(m.size)),
          toggle(m.enabled, async (v) => { try { await api.toggleMod(inst.id, m.file_name, v); m.enabled = v; row.classList.toggle("off", !v); count.textContent = `${mods.length} mods · ${mods.filter((x) => x.enabled).length} enabled`; } catch (e) { toast("Couldn't change mod", String(e), "err"); } }),
          btn("", { cls: "icon sm ghost", icon: "trash", title: "Remove", onclick: async () => {
            if (!(await confirmDialog(`Remove ${m.title}?`, "The jar is deleted from this instance."))) return;
            await api.removeMod(inst.id, m.file_name); load();
          } }),
        ),
      );
      list.appendChild(row);
    }
  };
  const load = async () => { mods = await api.mods(inst.id); draw(); };
  const search = h("input.input", { placeholder: "Search mods", oninput: debounce((e) => { filter = e.target.value.toLowerCase(); draw(); }, 120) });
  root.append(
    h("div.row.wrap", { style: { marginBottom: "12px" } },
      h("div.search.grow", { html: icon("search"), style: { maxWidth: "320px", minWidth: "200px" } }, search),
      count, h("span.grow"),
      btn("Check updates", { icon: "refresh", onclick: async (e) => {
        const b = e.currentTarget; b.disabled = true;
        try { const u = await api.checkUpdates(inst.id); updates = new Map(u.map((x) => [x.file_name, x])); draw(); toast(u.length ? `${u.length} update(s) available` : "Everything's up to date", "", "ok"); }
        catch (err) { toast("Couldn't check updates", String(err), "err"); } finally { b.disabled = false; }
      } }),
      btn("Add file", { icon: "upload", onclick: async () => { try { const a = await api.addLocalMods(inst.id); if (a.length) { toast(`Added ${a.length} mod(s)`, a.join(", "), "ok"); load(); } } catch (e) { toast("Couldn't add mod", String(e), "err"); } } }),
      btn("Browse mods", { cls: "primary", icon: "compass", onclick: () => go("discover?instance=" + inst.id) }),
    ),
    h("div.glass", { style: { padding: "6px" } }, list),
  );
  load();
  const off = store.on("game", (ev) => ev.id === inst.id && ev.state === "running" && load());
  return off;
}

// ------------------------------------------------------------------- bundle
function bundleTab(root, inst, onUpdate) {
  const overrides = new Set(store.settings.bundle_overrides);
  const save = () => store.saveSettings({ bundle_overrides: [...overrides] });
  const isOn = (slug, defOn) => (defOn ? !overrides.has(slug) : overrides.has("+" + slug));
  const set = (slug, defOn, on) => {
    if (defOn) on ? overrides.delete(slug) : overrides.add(slug);
    else on ? overrides.add("+" + slug) : overrides.delete("+" + slug);
    save();
  };
  root.append(
    h("div.section.glass",
      h("div.row", h("div.grow", h("h2", "Salt bundle for this instance"), h("p.desc", { style: { margin: 0 } }, "Installed automatically on launch, only for mods that support " + inst.mc_version + ". Switching a mod off here stops new installs; remove it in Content to uninstall.")),
        toggle(inst.salt_bundle, async (v) => { const u = await api.updateInstance({ ...inst, salt_bundle: v }); onUpdate(u); await store.refreshInstances(); })),
    ),
  );
  const box = h("div");
  root.appendChild(box);
  api.bundle().then((b) => {
    const group = (title, desc, rows) => h("div.section.glass", h("h2", title), h("p.desc", desc), h("div", rows));
    box.append(
      group("saltMODs", "Built by saltClient for this launcher. Jars come from salt-mods/<version>/ (see mods/README.md).",
        [...b.salt.map((m) => setting(m.name, m.description, toggle(isOn(m.id, true), (v) => set(m.id, true, v)))),
          setting("Chlorine", b.chlorine.description, toggle(isOn("chlorine", true), (v) => set("chlorine", true, v)))]),
    );
    const cats = { library: "Libraries", performance: "Performance", hosting: "World hosting", visual: "Visual extras", utility: "Utility" };
    for (const [cat, title] of Object.entries(cats)) {
      const items = b.modrinth.filter((m) => m.category === cat);
      box.append(group(title, cat === "performance" ? "The usual suspects — only installed when they support this version." : "", items.map((m) => setting(m.name, m.description, toggle(isOn(m.slug, m.default_on), (v) => set(m.slug, m.default_on, v))))));
    }
  });
}

// ------------------------------------------------------------------- worlds
function worldsTab(root, inst) {
  root.append(
    h("div.section.glass",
      h("h2", "Host a world with e4mc"),
      h("p.desc", "e4mc is part of the Salt bundle. In game: open a world → Esc → Open to LAN → Start. e4mc prints a public address in chat (like something.e4mc.link) and copies it — send it to friends and they join like any server. No port forwarding needed."),
      h("div.row", btn("Invite friends", { icon: "users", onclick: () => go("friends") }), btn("Open saves folder", { icon: "folder", onclick: () => api.openInstanceFolder(inst.id, "saves") })),
    ),
  );
  const list = h("div.list.stagger");
  root.appendChild(h("div.glass", { style: { padding: "6px" } }, list));
  api.worlds(inst.id).then((ws) => {
    if (!ws.length) list.append(h("div.empty", { html: icon("globe") }, h("div", "No worlds yet. Create one in game.")));
    for (const w of ws) list.append(h("div.list-item", h("div.li-icon", { html: icon("globe") }), h("div.grow", h("div.li-title", w.folder), h("div.li-sub", "Last played " + fmtAgo(w.modified)))));
  });
}

// --------------------------------------------------------------------- logs
function logsTab(root, inst) {
  return logPanel(root, { source: "game", instanceId: inst.id });
}

// ----------------------------------------------------------------- settings
function settingsTab(root, inst, onUpdate) {
  const st = structuredClone(inst);
  const save = debounce(async () => {
    try { const u = await api.updateInstance(st); onUpdate(u); await store.refreshInstances(); }
    catch (e) { toast("Couldn't save", String(e), "err"); }
  }, 400);
  const text = (key, placeholder, transform = (v) => v) => {
    const i = h("input.input", { value: Array.isArray(st[key]) ? st[key].join(" ") : st[key] || "", placeholder, style: { width: "360px" } });
    i.oninput = () => { st[key] = transform(i.value); save(); };
    return i;
  };
  const iconRow = h("div.swatches", EMOJI_ICONS.slice(0, 12).map((e) => {
    const b = h("button.swatch", { class: "emoji-swatch" + (e === st.icon ? " on" : ""), style: { width: "30px", height: "30px", fontSize: "15px" } }, e);
    b.onclick = () => { st.icon = e; iconRow.querySelectorAll(".swatch").forEach((x) => x.classList.remove("on")); b.classList.add("on"); save(); };
    return b;
  }));
  const res = st.resolution || [0, 0];
  const resIn = (i) => { const el = h("input.input", { type: "number", value: res[i] || "", placeholder: i ? "480" : "854", style: { width: "90px" } }); el.oninput = () => { res[i] = +el.value || 0; st.resolution = res[0] && res[1] ? [res[0], res[1]] : null; save(); }; return el; };
  const loaderWrap = h("div");
  if (st.loader === "fabric") {
    api.fabricLoaders(st.mc_version).then((ls) => loaderWrap.append(select([["", "Latest stable"], ...ls.slice(0, 30).map((l) => [l.version, l.version])], st.loader_version || "", (v) => { st.loader_version = v || null; save(); }))).catch(() => {});
  }
  root.append(
    h("div.section.glass", h("h2", "General"),
      setting("Name", null, text("name", "Instance name")),
      setting("Icon", null, iconRow),
      setting("Group", "Instances with the same group are shown together.", text("group", "e.g. PvP")),
    ),
    h("div.section.glass", h("h2", "Game"),
      setting("Minecraft version", "Create a new instance (or duplicate this one) to change versions safely.", h("span.chip.accent", st.mc_version)),
      st.loader === "fabric" ? setting("Fabric loader", null, loaderWrap) : null,
      setting("Memory", "How much RAM Minecraft may use.", range(st.memory_mb, 1024, 16384, 256, (v) => { st.memory_mb = v; save(); }, (v) => (v / 1024).toFixed(1) + " GB")),
      setting("Window size", "Leave empty for the default.", h("div.row", resIn(0), "×", resIn(1))),
    ),
    h("div.section.glass", h("h2", "Java"),
      setting("Java executable", "Empty = saltClient downloads the right Java automatically.", text("java_path", "C:\\Path\\to\\javaw.exe", (v) => v.trim() || null)),
      setting("JVM arguments", "Extra flags, space-separated.", text("jvm_args", "-XX:+UseZGC", (v) => v.split(/\s+/).filter(Boolean))),
      setting("Game arguments", null, text("game_args", "--quickPlayMultiplayer play.example.net", (v) => v.split(/\s+/).filter(Boolean))),
    ),
  );
}
