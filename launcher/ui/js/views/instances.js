import { api } from "../api.js";
import { icon } from "../icons.js";
import { store } from "../store.js";
import { h, btn, toast, menu, modal, confirmDialog, select, toggle, range, fmtAgo, fmtDuration, EMOJI_ICONS } from "../ui.js";
import { go, playButton } from "../app.js";

export function render(root) {
  let filter = "";
  const grid = h("div.grid.cards.stagger");
  const draw = () => {
    grid.innerHTML = "";
    const list = store.instances.filter((i) => !filter || (i.name + i.mc_version).toLowerCase().includes(filter));
    for (const inst of list) grid.appendChild(instanceCard(inst));
    const add = h("div.inst-card.card.hover.new", { html: icon("plus") }, h("div", "New instance"));
    add.onclick = () => newInstanceDialog();
    grid.appendChild(add);
  };
  const search = h("input.input", { placeholder: "Search instances", oninput: (e) => { filter = e.target.value.toLowerCase(); draw(); } });
  root.append(
    h("div.page-head",
      h("div", h("h1", "Instances"), h("p", "Separate installs with their own mods, worlds and settings.")),
      h("div.row", h("div.search", { html: icon("search"), style: { width: "220px" } }, search), btn("Import modpack", { icon: "upload", onclick: importPack }), btn("New instance", { cls: "primary", icon: "plus", onclick: () => newInstanceDialog() })),
    ),
    grid,
  );
  draw();
  const offs = [store.on("instances", draw)];
  return () => offs.forEach((o) => o());
}

async function importPack() {
  try {
    const inst = await api.importPack();
    if (inst) { await store.refreshInstances(); toast("Modpack imported", inst.name, "ok"); go("instance/" + inst.id); }
  } catch (e) { toast("Import failed", String(e), "err"); }
}

export function instanceCard(inst) {
  const iconEl = h("div.inst-icon", inst.icon?.startsWith("http") ? h("img", { src: inst.icon, alt: "" }) : inst.icon || "🧂");
  const card = h("div.inst-card.card.hover",
    h("div.ic-top", iconEl, h("div.grow", h("div.ic-name", inst.name), h("div.ic-sub", fmtAgo(inst.last_played)))),
    h("div.row.wrap", h("span.chip.accent", inst.mc_version), h("span.chip", inst.loader === "fabric" ? "Fabric" : "Vanilla"), inst.pack ? h("span.chip." + inst.pack.platform, inst.pack.platform === "modrinth" ? "Modrinth" : "CurseForge") : null),
    h("div.ic-sub", fmtDuration(inst.playtime_secs)),
    h("div.ic-play", playButton(inst.id, { small: true })),
  );
  card.classList.toggle("running", store.running.has(inst.id));
  card.onclick = () => go("instance/" + inst.id);
  card.oncontextmenu = (e) => { e.preventDefault(); instanceMenu(card, inst); };
  return card;
}

export function instanceMenu(anchor, inst) {
  menu(anchor, [
    { label: "Open", icon: "chevron", onclick: () => go("instance/" + inst.id) },
    { label: "Open folder", icon: "folder", onclick: () => api.openInstanceFolder(inst.id) },
    { label: "Add mods", icon: "compass", onclick: () => go("discover?instance=" + inst.id) },
    { label: "Duplicate", icon: "copy", onclick: async () => { await api.duplicateInstance(inst.id); await store.refreshInstances(); toast("Duplicated", inst.name, "ok"); } },
    "-",
    { label: "Delete", icon: "trash", danger: true, onclick: () => deleteInstance(inst) },
  ]);
}

export async function deleteInstance(inst) {
  const ok = await confirmDialog(`Delete ${inst.name}?`, "This removes its mods, worlds, screenshots and settings from this PC. It can't be undone.", { ok: "Delete", danger: true });
  if (!ok) return false;
  try {
    await api.deleteInstance(inst.id);
    await store.refreshInstances();
    toast("Instance deleted", inst.name, "ok");
    return true;
  } catch (e) { toast("Couldn't delete", String(e), "err"); return false; }
}

export function newInstanceDialog(preset = {}) {
  return modal((close) => {
    const st = { name: "", mc_version: preset.mc_version || "", loader: "fabric", icon: "🧂", memory_mb: store.settings.default_memory_mb, salt_bundle: true, loader_version: null };
    const nameIn = h("input.input", { placeholder: "My instance" });
    const versionWrap = h("div", h("div.skeleton", { style: { height: "38px" } }));
    const loaderVerWrap = h("div");
    const iconRow = h("div.swatches", EMOJI_ICONS.map((e) => {
      const b = h("button.swatch", { class: "emoji-swatch" + (e === st.icon ? " on" : "") }, e);
      b.onclick = () => { st.icon = e; iconRow.querySelectorAll(".swatch").forEach((x) => x.classList.remove("on")); b.classList.add("on"); };
      return b;
    }));

    const loadLoaders = async () => {
      loaderVerWrap.innerHTML = "";
      if (st.loader !== "fabric" || !st.mc_version) return;
      try {
        const ls = await api.fabricLoaders(st.mc_version);
        const opts = [["", `Latest stable (${ls.find((l) => l.stable)?.version || ls[0]?.version})`], ...ls.slice(0, 25).map((l) => [l.version, l.version + (l.stable ? "" : " (beta)")])];
        loaderVerWrap.append(h("div.field", h("label", "Fabric loader"), select(opts, "", (v) => (st.loader_version = v || null))));
      } catch { loaderVerWrap.append(h("div.callout.warn", `No Fabric loader for ${st.mc_version}.`)); }
    };

    store.loadVersions().then((vs) => {
      st.mc_version = st.mc_version || vs[0]?.id;
      versionWrap.replaceChildren(h("div.field", h("label", "Minecraft version"), select(vs.map((v) => [v.id, v.id + (v.id === vs[0].id ? "  (latest)" : "")]), st.mc_version, (v) => { st.mc_version = v; loadLoaders(); })));
      loadLoaders();
    }).catch((e) => versionWrap.replaceChildren(h("div.callout.warn", "Couldn't load versions: " + e)));

    const create = async (b) => {
      st.name = nameIn.value.trim() || `${st.loader === "fabric" ? "Fabric" : "Vanilla"} ${st.mc_version}`;
      if (!st.mc_version) return;
      b.disabled = true;
      try {
        const inst = await api.createInstance(st);
        await store.refreshInstances();
        close(inst);
        toast("Instance created", inst.name, "ok");
        go("instance/" + inst.id);
      } catch (e) { b.disabled = false; toast("Couldn't create instance", String(e), "err"); }
    };
    const createBtn = btn("Create", { cls: "primary", icon: "plus" });
    createBtn.onclick = () => create(createBtn);

    return [
      h("h2", "New instance"),
      h("p.m-sub", "Pick a version and loader. Fabric instances get the Salt bundle automatically."),
      h("div.col", { style: { gap: "14px" } },
        h("div.field", h("label", "Name"), nameIn),
        h("div.field", h("label", "Icon"), iconRow),
        h("div.grid", { style: { gridTemplateColumns: "1fr 1fr" } },
          versionWrap,
          h("div.field", h("label", "Loader"), select([["fabric", "Fabric"], ["vanilla", "Vanilla"]], st.loader, (v) => { st.loader = v; loadLoaders(); })),
        ),
        loaderVerWrap,
        h("div.field", h("label", "Memory"), range(st.memory_mb, 1024, 16384, 256, (v) => (st.memory_mb = v), (v) => (v / 1024).toFixed(1) + " GB")),
        h("div.card", { style: { padding: "12px 14px" } }, h("div.row", h("div.grow", h("b", "Salt bundle"), h("div.small.muted", "saltHUD, saltMODs, performance mods, e4mc, Chlorine")), toggle(true, (v) => (st.salt_bundle = v)))),
      ),
      h("div.m-actions", btn("Cancel", { onclick: () => close() }), createBtn),
    ];
  });
}
