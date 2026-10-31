// Discover: Modrinth + CurseForge search for mods, modpacks, resource packs
// and shaders, installed straight into an instance.
import { api } from "../api.js";
import { icon } from "../icons.js";
import { store } from "../store.js";
import { h, btn, toast, tabs, select, modal, fmtNum, debounce, esc } from "../ui.js";
import { go } from "../app.js";

const KINDS = [["mod", "Mods", "puzzle"], ["modpack", "Modpacks", "package"], ["resourcepack", "Resource packs", "image"], ["shader", "Shaders", "sparkles"]];

export function render(root, { query }) {
  const st = {
    platform: query.platform || "modrinth",
    kind: query.kind || "mod",
    q: "",
    sort: "relevance",
    offset: 0,
    target: query.instance || store.instances[0]?.id || null,
    loading: false,
    total: 0,
  };
  const target = () => store.instances.find((i) => i.id === st.target);
  const results = h("div.grid.wide.stagger");
  const more = h("div.row", { style: { justifyContent: "center", margin: "16px 0" } });
  const status = h("span.small.faint");
  const installed = new Set();

  const search = async (append = false) => {
    if (!append) { st.offset = 0; results.innerHTML = ""; for (let i = 0; i < 6; i++) results.append(h("div.skeleton", { style: { height: "96px" } })); }
    st.loading = true;
    const inst = target();
    const gameVersion = st.kind === "modpack" ? null : inst?.mc_version;
    try {
      let hits;
      if (st.platform === "modrinth") {
        const loader = st.kind === "mod" && inst ? (inst.loader === "fabric" ? (inst.mc_version.startsWith("1.8") ? "legacy-fabric" : "fabric") : null) : null;
        const r = await api.mrSearch({ query: st.q, kind: st.kind, game_version: gameVersion, loader, offset: st.offset, limit: 20, sort: st.sort });
        hits = r.hits; st.total = r.total_hits;
      } else {
        hits = await api.cfSearch(st.q, st.kind, gameVersion, st.offset);
        st.total = hits.length === 20 ? st.offset + 40 : st.offset + hits.length;
      }
      if (!append) results.innerHTML = "";
      for (const hit of hits) results.appendChild(card(hit));
      if (!hits.length && !append) results.append(h("div.empty", { html: icon("search") }, h("div", "Nothing found. Try another search.")));
      st.offset += hits.length;
      more.innerHTML = "";
      if (st.offset < st.total && hits.length) more.append(btn("Load more", { onclick: () => search(true) }));
      status.textContent = st.platform === "modrinth" ? `${fmtNum(st.total)} results` : "";
    } catch (e) {
      results.innerHTML = "";
      results.append(h("div.callout.warn", String(e), st.platform === "curseforge" ? h("div", { style: { marginTop: "8px" } }, btn("Open Settings", { cls: "sm", onclick: () => go("settings/integrations") })) : null));
    }
    st.loading = false;
  };

  const install = async (hit, b, versionId) => {
    const inst = target();
    b.disabled = true;
    const label = b.textContent;
    b.innerHTML = '<span class="spinner" style="width:14px;height:14px"></span> Installing';
    try {
      if (st.kind === "modpack") {
        const created = st.platform === "modrinth"
          ? await api.mrInstallPack(versionId || (await api.mrVersions(hit.project_id, null, null))[0].id)
          : await api.cfInstallPack(+hit.project_id);
        await store.refreshInstances();
        toast("Modpack installed", created.name, "ok");
        go("instance/" + created.id);
        return;
      }
      if (!inst) throw "Create an instance first.";
      const names = st.platform === "modrinth" ? await api.mrInstall(inst.id, hit.project_id, versionId || null) : await api.cfInstall(inst.id, +hit.project_id);
      installed.add(hit.project_id);
      b.innerHTML = icon("check") + " Installed";
      toast(`Installed into ${inst.name}`, names.join(", "), "ok");
    } catch (e) {
      b.disabled = false;
      b.textContent = label;
      toast("Install failed", String(e), "err");
    }
  };

  const card = (hit) => {
    const b = btn(st.kind === "modpack" ? "Install pack" : installed.has(hit.project_id) ? "Installed" : "Install", { cls: "sm primary", icon: "download" });
    b.onclick = (e) => { e.stopPropagation(); install(hit, b); };
    const c = h("div.result.card.hover",
      h("div.r-icon", hit.icon_url ? h("img", { src: hit.icon_url, loading: "lazy", alt: "" }) : null),
      h("div.grow",
        h("div.r-title", hit.title),
        h("div.small.faint", "by " + hit.author),
        h("div.r-desc", hit.description),
        h("div.row", h("span.chip", { html: icon("download") + " " + fmtNum(hit.downloads) }), h("span.chip." + st.platform, st.platform === "modrinth" ? "Modrinth" : "CurseForge"), h("span.grow"), b),
      ),
    );
    c.onclick = () => st.platform === "modrinth" ? projectDialog(hit, install) : api.openUrl(`https://www.curseforge.com/minecraft/${st.kind === "modpack" ? "modpacks" : st.kind === "resourcepack" ? "texture-packs" : st.kind === "shader" ? "shaders" : "mc-mods"}/${hit.slug}`);
    return c;
  };

  const debounced = debounce(() => search(), 300);
  const qIn = h("input.input", { placeholder: "Search Modrinth…", oninput: (e) => { st.q = e.target.value; debounced(); } });
  const targetSel = h("div");
  const paintTarget = () => {
    targetSel.innerHTML = "";
    if (st.kind === "modpack") { targetSel.append(h("span.small.muted", "Modpacks install as a new instance")); return; }
    if (!store.instances.length) { targetSel.append(btn("Create an instance first", { cls: "sm", onclick: () => go("instances") })); return; }
    targetSel.append(h("div.row", h("span.small.muted", "Into"), select(store.instances.map((i) => [i.id, `${i.name} (${i.mc_version})`]), st.target, (v) => { st.target = v; search(); })));
  };
  const sortSel = select([["relevance", "Relevance"], ["downloads", "Downloads"], ["follows", "Follows"], ["newest", "Newest"], ["updated", "Updated"]], st.sort, (v) => { st.sort = v; search(); });
  sortSel.style.width = "150px";

  root.append(
    h("div.page-head",
      h("div", h("h1", "Discover"), h("p", "Mods, modpacks, resource packs and shaders from Modrinth and CurseForge.")),
      tabs([["modrinth", "Modrinth"], ["curseforge", "CurseForge"]], st.platform, (p) => { st.platform = p; qIn.placeholder = `Search ${p === "modrinth" ? "Modrinth" : "CurseForge"}…`; sortSel.style.display = p === "modrinth" ? "" : "none"; search(); }),
    ),
    h("div.glass", { style: { padding: "14px", marginBottom: "14px" } },
      h("div.row.wrap", { style: { gap: "12px" } },
        tabs(KINDS, st.kind, (k) => { st.kind = k; paintTarget(); search(); }),
        h("div.search.grow", { html: icon("search"), style: { minWidth: "220px" } }, qIn),
        sortSel,
      ),
      h("div.row", { style: { marginTop: "12px" } }, targetSel, h("span.grow"), status),
    ),
    results,
    more,
  );
  paintTarget();
  search();
}

async function projectDialog(hit, install) {
  modal((close) => {
    const body = h("div", h("div.skeleton", { style: { height: "180px" } }));
    const versionsBox = h("div");
    (async () => {
      const p = await api.mrProject(hit.project_id);
      body.innerHTML = "";
      // Project descriptions are Markdown from third parties; show them as
      // plain text (escaped) rather than rendering untrusted HTML.
      const text = p.body.replace(/!\[[^\]]*\]\([^)]*\)/g, "").replace(/<[^>]+>/g, "").replace(/\n{3,}/g, "\n\n").trim();
      body.append(h("div.project-body", text || p.description));
      const vs = await api.mrVersions(p.id, null, null);
      versionsBox.append(h("h2", { style: { fontSize: "15px", margin: "18px 0 8px" } }, "Versions"));
      const list = h("div.list");
      for (const v of vs.slice(0, 30)) {
        const b = btn("Install", { cls: "sm" });
        b.onclick = () => install(hit, b, v.id);
        list.append(h("div.list-item", h("div.grow", h("div.li-title", v.name), h("div.li-sub", `${v.version_number} · ${v.game_versions.slice(-4).join(", ")} · ${v.loaders.join(", ")}`)), h("span.chip" + (v.version_type === "release" ? ".ok" : ".warn"), v.version_type), b));
      }
      versionsBox.append(list);
    })().catch((e) => body.replaceChildren(h("div.callout.warn", String(e))));
    return [
      h("div.row", { style: { gap: "14px", marginBottom: "14px" } }, h("div.r-icon", { style: { width: "64px", height: "64px", borderRadius: "14px", overflow: "hidden", flex: "none" } }, hit.icon_url ? h("img", { src: hit.icon_url, style: { width: "100%" } }) : null), h("div.grow", h("h2", hit.title), h("div.muted", hit.description))),
      body,
      versionsBox,
      h("div.m-actions", btn("Open on Modrinth", { icon: "external", onclick: () => api.openUrl(`https://modrinth.com/${hit.project_type}/${hit.slug}`) }), btn("Close", { onclick: () => close() })),
    ];
  }, { wide: true });
}
