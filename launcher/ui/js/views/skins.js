// Skin + cape changer (real Minecraft profile changes) with a 3D preview
// and a local skin library.
import { api } from "../api.js";
import { icon } from "../icons.js";
import { store } from "../store.js";
import { h, btn, toast, tabs, confirmDialog } from "../ui.js";
import { skinViewer, skinFront } from "../skin3d.js";
import { startLogin } from "./onboarding.js";

export function render(root) {
  if (!store.activeAccount()) {
    root.append(h("div.empty.glass", { html: icon("shirt") }, h("h2", "Sign in to change your skin"), h("p", "Skins and capes are saved to your Minecraft profile."), btn("Sign in", { cls: "primary", onclick: startLogin })));
    return;
  }
  let profile = null, slim = false, viewer = null;
  const stage = h("div.skin-stage.glass");
  const variantTabs = tabs([["classic", "Classic"], ["slim", "Slim"]], "classic", (v) => { slim = v === "slim"; paintViewer(); });
  const capes = h("div.capes");
  const lib = h("div.skin-lib");
  const busy = (b, fn) => async () => { b.disabled = true; try { await fn(); } catch (e) { toast("Something went wrong", String(e), "err"); } finally { b.disabled = false; } };

  async function paintViewer() {
    const skin = profile?.skins.find((s) => s.state === "ACTIVE");
    const cape = profile?.capes.find((c) => c.state === "ACTIVE");
    const [skinUrl, capeUrl] = await Promise.all([skin ? api.texture(skin.url) : null, cape ? api.texture(cape.url) : null]);
    if (skinUrl) store.skins.set(store.activeAccount().uuid, skinUrl);
    viewer.set(skinUrl, { slim, cape: capeUrl });
  }

  function paintCapes() {
    capes.innerHTML = "";
    const none = h("div.cape.card.hover", { class: profile.capes.some((c) => c.state === "ACTIVE") ? "" : "on" }, h("div", { style: { width: "40px", height: "64px", margin: "0 auto 6px", display: "grid", placeItems: "center", color: "var(--text-3)" }, html: icon("x") }), "No cape");
    none.onclick = async () => { try { profile = await api.capeSet(null); paintCapes(); paintViewer(); toast("Cape hidden", "", "ok"); } catch (e) { toast("Couldn't change cape", String(e), "err"); } };
    capes.appendChild(none);
    for (const c of profile.capes) {
      const cv = h("canvas", { width: 10, height: 16 });
      api.texture(c.url).then((d) => { const i = new Image(); i.onload = () => { const x = cv.getContext("2d"); x.imageSmoothingEnabled = false; x.drawImage(i, 1, 1, 10, 16, 0, 0, 10, 16); }; i.src = d; });
      const el = h("div.cape.card.hover", { class: c.state === "ACTIVE" ? "on" : "" }, cv, c.alias || "Cape");
      el.onclick = async () => { try { profile = await api.capeSet(c.id); paintCapes(); paintViewer(); toast("Cape equipped", c.alias, "ok"); } catch (e) { toast("Couldn't change cape", String(e), "err"); } };
      capes.appendChild(el);
    }
    if (!profile.capes.length) capes.append(h("div.small.muted", { style: { gridColumn: "1/-1" } }, "This account doesn't own any capes yet. Capes from Minecraft events, Minecon and purchases show up here."));
  }

  async function paintLibrary() {
    lib.innerHTML = "";
    const add = h("div.skin-tile.card.hover", { style: { display: "grid", placeItems: "center", minHeight: "170px", color: "var(--text-2)" } }, h("div", { html: icon("plus") }), "Add skin");
    add.onclick = async () => { const s = await api.skinLibraryAdd(slim).catch((e) => toast("Couldn't add skin", String(e), "err")); if (s) paintLibrary(); };
    lib.appendChild(add);
    for (const s of await api.skinLibrary()) {
      const del = btn("", { cls: "icon sm ghost st-del", icon: "trash", onclick: async (e) => { e.stopPropagation(); if (await confirmDialog(`Remove ${s.name}?`, "Removes it from your library (not from your profile).")) { await api.skinLibraryRemove(s.id); paintLibrary(); } } });
      const tile = h("div.skin-tile.card.hover", skinFront(s.data_url, { slim: s.slim }), h("div.st-name", s.name), h("div.small.faint", s.slim ? "Slim" : "Classic"), del);
      tile.onclick = async () => {
        tile.style.opacity = ".6";
        try { profile = await api.skinApplyLibrary(s.id); slim = s.slim; variantTabs.set(slim ? "slim" : "classic"); await paintViewer(); toast("Skin applied", s.name, "ok"); }
        catch (e) { toast("Couldn't apply skin", String(e), "err"); }
        tile.style.opacity = "";
      };
      lib.appendChild(tile);
    }
  }

  const uploadBtn = btn("Upload skin", { cls: "primary", icon: "upload" });
  uploadBtn.onclick = busy(uploadBtn, async () => { const p = await api.skinUpload(slim); if (p) { profile = p; await paintViewer(); paintLibrary(); toast("Skin updated", "Everyone will see it next time you join a server.", "ok"); } });
  const resetBtn = btn("Reset to default", { icon: "refresh" });
  resetBtn.onclick = busy(resetBtn, async () => { if (!(await confirmDialog("Reset your skin?", "Your profile goes back to a default Steve/Alex skin."))) return; profile = await api.skinReset(); await paintViewer(); });

  root.append(
    h("div.page-head", h("div", h("h1", "Skins & capes"), h("p", "Changes go to your real Minecraft profile — everyone sees them, on every server."))),
    h("div.skin-layout",
      h("div.col", stage, h("div.row", { style: { justifyContent: "center" } }, variantTabs), h("div.row", { style: { justifyContent: "center" } }, uploadBtn, resetBtn)),
      h("div",
        h("div.section.glass", h("h2", "Capes"), h("p.desc", "Capes you own. Click one to wear it."), capes),
        h("div.section.glass", h("h2", "Skin library"), h("p.desc", "Saved on this PC. Click a skin to wear it. The variant (classic/slim) is saved with each skin."), lib),
      ),
    ),
  );
  viewer = skinViewer(stage, { scale: 12 });
  api.skinProfile().then((p) => {
    profile = p;
    slim = p.skins.find((s) => s.state === "ACTIVE")?.variant === "SLIM";
    variantTabs.set(slim ? "slim" : "classic");
    paintViewer();
    paintCapes();
  }).catch((e) => { capes.append(h("div.callout.warn", String(e))); });
  paintLibrary();
  return () => viewer?.destroy();
}
