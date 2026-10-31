import { api } from "../api.js";
import { icon } from "../icons.js";
import { store } from "../store.js";
import { ACCENTS, BACKGROUNDS } from "../theme.js";
import { h, btn, toast, toggle, range, select, setting, debounce, confirmDialog, headCanvas } from "../ui.js";
import { startLogin, showOnboarding } from "./onboarding.js";

const SECTIONS = [
  ["appearance", "Appearance", "Theme, accent, background"],
  ["game", "Game & Java", "Memory, Java, launch behaviour"],
  ["bundle", "Salt bundle", "Performance mods we install"],
  ["accounts", "Accounts", "Microsoft accounts"],
  ["integrations", "Integrations", "CurseForge and other services"],
  ["social", "Friends", "Chat, presence, invites"],
  ["privacy", "Privacy", "What saltClient sends, and where"],
  ["developer", "Developer", "Offline test launches"],
  ["about", "About", "Version, updates, links"],
];
const RENDER = () => ({ appearance, game, bundle, accounts, integrations, social, privacy, developer, about });

export function render(root, { args }) {
  let current = SECTIONS.some(([id]) => id === args[0]) ? args[0] : "appearance";
  const body = h("div.settings-body");
  const tabs = SECTIONS.map(([id, label, desc], i) => {
    const b = h("button.lab-tab", { class: id === current ? "active" : "" }, h("span.bar"), h("span.n", String(i + 1).padStart(2, "0")), h("strong", label), h("small", desc));
    b.onclick = () => show(id);
    return b;
  });
  // Sections swap in place (no route re-render); their cards animate in via CSS.
  const show = (id) => {
    current = id;
    tabs.forEach((t, i) => t.classList.toggle("active", SECTIONS[i][0] === id));
    body.innerHTML = "";
    RENDER()[id]?.(body);
    history.replaceState(null, "", "#/settings/" + id);
  };
  root.append(
    h("div.page-head", h("div", h("div.eyebrow", "Preferences"), h("h1", "Settings"), h("p", "Make saltClient yours."))),
    h("div.settings-layout", h("div.settings-nav", tabs), body),
  );
  RENDER()[current]?.(body);
}

const S = () => store.settings;
const save = debounce((patch) => store.saveSettings(patch).catch((e) => toast("Couldn't save settings", String(e), "err")), 250);
const saveTheme = (patch) => { store.settings.theme = { ...S().theme, ...patch }; store.emit("settings", store.settings); save({ theme: store.settings.theme }); };

function appearance(root) {
  const t = S().theme;
  const sw = h("div.swatches", ACCENTS.map((c) => {
    const s = h("div.swatch", { style: { background: c }, class: c === t.accent ? "on" : "" });
    s.onclick = () => { sw.querySelectorAll(".swatch").forEach((x) => x.classList.remove("on")); s.classList.add("on"); picker.value = c; saveTheme({ accent: c }); };
    return s;
  }));
  const picker = h("input.color-input", { type: "color", value: t.accent });
  picker.oninput = () => { sw.querySelectorAll(".swatch").forEach((x) => x.classList.remove("on")); saveTheme({ accent: picker.value }); };
  sw.appendChild(picker);

  const bgs = h("div.bg-presets", { style: { width: "100%" } });
  const paintBgs = () => {
    bgs.innerHTML = "";
    for (const [id, b] of Object.entries(BACKGROUNDS)) {
      const p = h("div.bg-preset", { style: { background: b.css }, class: id === S().theme.background ? "on" : "" }, h("span", b.label));
      p.onclick = () => { saveTheme({ background: id }); paintBgs(); };
      bgs.appendChild(p);
    }
    const custom = h("label.bg-preset", { class: S().theme.background.startsWith("data:") ? "on" : "", style: S().theme.background.startsWith("data:") ? { backgroundImage: `url("${S().theme.background}")` } : { display: "grid", placeItems: "center" } }, S().theme.background.startsWith("data:") ? null : h("div", { html: icon("image") }), h("span", "Custom image"));
    const file = h("input", { type: "file", accept: "image/png,image/jpeg,image/webp", style: { display: "none" } });
    file.onchange = () => {
      const f = file.files[0];
      if (!f) return;
      if (f.size > 6 * 1024 * 1024) return toast("Image too large", "Pick an image under 6 MB.", "err");
      const r = new FileReader();
      r.onload = () => { saveTheme({ background: r.result }); paintBgs(); };
      r.readAsDataURL(f);
    };
    custom.appendChild(file);
    bgs.appendChild(custom);
  };
  paintBgs();

  root.append(
    h("div.section.glass", h("h2", "Theme"),
      setting("Mode", null, select([["dark", "Dark"], ["light", "Light"], ["system", "Match Windows"]], t.mode, (v) => saveTheme({ mode: v }))),
      setting("Accent color", null, sw),
      setting("Compact toolbar", "Icons instead of labels", toggle(t.compact, (v) => saveTheme({ compact: v }))),
    ),
    h("div.section.glass", h("h2", "Panels"), h("p.desc", "Solid by default. Add blur or transparency if you like a glass look."),
      setting("Blur", "0 = off", range(t.blur, 0, 40, 1, (v) => saveTheme({ blur: v }), (v) => v + "px")),
      setting("Panel opacity", null, range(Math.round(t.glass_opacity * 100), 10, 95, 1, (v) => saveTheme({ glass_opacity: v / 100 }), (v) => v + "%")),
      setting("Corner roundness", null, range(t.radius, 0, 26, 1, (v) => saveTheme({ radius: v }), (v) => v + "px")),
    ),
    h("div.section.glass", h("h2", "Background"), bgs, h("div", { style: { marginTop: "10px" } }, setting("Dim background image", null, range(Math.round(t.background_dim * 100), 0, 90, 1, (v) => saveTheme({ background_dim: v / 100 }), (v) => v + "%")))),
    h("div.section.glass", h("h2", "Motion"),
      setting("Animations", "Page transitions, hover effects and floating backgrounds", toggle(t.animations, (v) => saveTheme({ animations: v }))),
      setting("Animation speed", null, range(t.animation_speed, 0.4, 2, 0.1, (v) => saveTheme({ animation_speed: v }), (v) => (v < 1 ? "Faster " : v > 1 ? "Slower " : "") + v.toFixed(1) + "×")),
    ),
  );
}

function game(root) {
  const s = S();
  const java = h("input.input", { value: s.java_path || "", placeholder: "Automatic", style: { width: "320px" } });
  java.oninput = () => save({ java_path: java.value.trim() || null });
  const jvm = h("input.input", { value: s.extra_jvm_args.join(" "), placeholder: "-XX:+UseZGC", style: { width: "320px" } });
  jvm.oninput = () => save({ extra_jvm_args: jvm.value.split(/\s+/).filter(Boolean) });
  root.append(
    h("div.section.glass", h("h2", "Launching"),
      setting("Default memory", "Used for new instances", range(s.default_memory_mb, 1024, 16384, 256, (v) => save({ default_memory_mb: v }), (v) => (v / 1024).toFixed(1) + " GB")),
      setting("After launching", null, select([["keep_open", "Keep saltClient open"], ["minimize", "Minimize saltClient"], ["close", "Hide saltClient until the game closes"]], s.launch_behaviour, (v) => save({ launch_behaviour: v }))),
      setting("Parallel downloads", "Higher is faster on good connections", range(s.max_concurrent_downloads, 2, 64, 1, (v) => save({ max_concurrent_downloads: v }))),
    ),
    h("div.section.glass", h("h2", "Java"),
      h("p.desc", "saltClient downloads Eclipse Temurin automatically — Java 8 for 1.8.9, 21 for 1.21.x, 25 for 26.x. Set a path only if you want your own."),
      setting("Java executable (all instances)", "Instances can override this", java),
      setting("Extra JVM arguments", "Added to every launch", jvm),
    ),
  );
}

function bundle(root) {
  const overrides = new Set(S().bundle_overrides);
  const isOn = (slug, d) => (d ? !overrides.has(slug) : overrides.has("+" + slug));
  const set = (slug, d, on) => { if (d) on ? overrides.delete(slug) : overrides.add(slug); else on ? overrides.add("+" + slug) : overrides.delete("+" + slug); save({ bundle_overrides: [...overrides] }); };
  root.append(h("div.section.glass", h("h2", "Salt bundle"), h("p.desc", { style: { margin: 0 } }, "What every Fabric instance gets on launch. Mods are only installed when they support the instance's version, so 1.8.9 gets a smaller set than 26.3.")));
  api.bundle().then((b) => {
    root.append(h("div.section.glass", h("h2", "saltMODs"), ...b.salt.map((m) => setting(m.name, m.description, toggle(isOn(m.id, true), (v) => set(m.id, true, v)))), setting("Chlorine", b.chlorine.description, toggle(isOn("chlorine", true), (v) => set("chlorine", true, v)))));
    for (const [cat, title] of [["performance", "Performance"], ["library", "Libraries"], ["hosting", "World hosting"], ["visual", "Visual"], ["utility", "Utility"]]) {
      root.append(h("div.section.glass", h("h2", title), ...b.modrinth.filter((m) => m.category === cat).map((m) => setting(m.name, m.description, toggle(isOn(m.slug, m.default_on), (v) => set(m.slug, m.default_on, v))))));
    }
  });
}

function accounts(root) {
  const box = h("div.section.glass", h("h2", "Accounts"));
  const list = h("div.list");
  box.append(list, h("div.row", { style: { marginTop: "10px" } }, btn("Add account", { cls: "primary", icon: "userPlus", onclick: startLogin })));
  const draw = () => {
    list.innerHTML = "";
    if (!store.accounts.accounts.length) list.append(h("div.muted", "No accounts yet."));
    for (const a of store.accounts.accounts) {
      const active = a.uuid === store.accounts.active;
      const row = h("div.list-item", headCanvas(store.skins.get(a.uuid), 38), h("div.grow", h("div.li-title", a.username), h("div.li-sub", a.kind === "verified" ? "Verified with Minecraft · offline-mode servers only" : "Microsoft account")),
        active ? h("span.chip.accent", "Active") : btn("Use", { cls: "sm", onclick: async () => { await api.setActiveAccount(a.uuid); await store.refreshAccounts(); } }),
        btn("Sign out", { cls: "sm danger", onclick: async () => { if (await confirmDialog(`Sign out ${a.username}?`, a.kind === "verified" ? "Removes the account from Salt." : "Removes the account and deletes its saved tokens from Windows Credential Manager.", { ok: "Sign out", danger: true })) { await api.removeAccount(a.uuid); await store.refreshAccounts(); } } }),
      );
      list.appendChild(row);
    }
  };
  draw();
  root.append(box);
  return store.on("accounts", draw);
}

function integrations(root) {
  const cf = h("input.input", { type: "password", placeholder: "Paste your CurseForge API key", style: { width: "340px" } });
  const status = h("span.chip");
  const paint = () => api.hasCurseforgeKey().then((has) => { status.className = "chip " + (has ? "ok" : ""); status.textContent = has ? "Key saved" : "No key"; });
  paint();
  const ms = h("input.input", { value: S().microsoft_client_id, placeholder: "Azure application (client) ID", style: { width: "340px" } });
  ms.oninput = () => save({ microsoft_client_id: ms.value.trim() });
  root.append(
    h("div.section.glass", h("h2", "CurseForge"),
      h("p.desc", "CurseForge requires an API key. Get one free at console.curseforge.com. It's stored in Windows Credential Manager, not in a file."),
      setting("API key", null, h("div.row", cf, btn("Save", { cls: "primary", onclick: async () => { try { await api.setCurseforgeKey(cf.value); cf.value = ""; paint(); toast("CurseForge key saved", "", "ok"); } catch (e) { toast("Couldn't save key", String(e), "err"); } } }))),
      setting("Status", null, h("div.row", status, btn("Remove", { cls: "sm ghost", onclick: async () => { await api.setCurseforgeKey(""); paint(); } }), btn("Get a key", { cls: "sm", icon: "external", onclick: () => api.openUrl("https://console.curseforge.com/") }))),
    ),
    h("div.section.glass", h("h2", "Modrinth"), h("p.desc", { style: { margin: 0 } }, "Works out of the box — no key needed.")),
    h("div.section.glass", h("h2", "Microsoft sign-in"),
      h("p.desc", "The Azure app ID Salt signs in with. Builds can bake one in with SALT_MS_CLIENT_ID; see docs/MICROSOFT_LOGIN.md."),
      setting("Client ID", null, ms),
    ),
  );
}

function social(root) {
  const s = S();
  const url = h("input.input", { value: s.social_server, style: { width: "340px" } });
  url.oninput = () => save({ social_server: url.value.trim() });
  root.append(h("div.section.glass", h("h2", "Friends & chat"),
    h("p.desc", "Friends run on a small salt-social server you can host yourself (see server/README.md). It verifies you with Mojang's own join check, so it learns your username and UUID — never your login tokens."),
    setting("Enable friends", null, toggle(s.social_enabled, (v) => { save({ social_enabled: v }); v ? api.socialConnect().catch((e) => toast("Couldn't connect", String(e), "err")) : api.socialDisconnect(); })),
    setting("Share what I'm playing", "Friends see e.g. \"Playing 26.3\"", toggle(s.share_activity, (v) => save({ share_activity: v }))),
    setting("Server address", "wss://… for a hosted server", url),
  ));
}

function privacy(root) {
  const items = [
    ["Sign-in", "Microsoft device-code sign-in in your own browser. saltClient never sees your password."],
    ["What's checked", "Only that the account owns Minecraft: Java Edition (Xbox → Mojang profile). Nothing else is read from your account."],
    ["Where tokens live", "Windows Credential Manager (encrypted by Windows). accounts.json only holds your username, UUID and skin URL."],
    ["Who saltClient talks to", "Microsoft, Xbox Live and Mojang (sign-in, skins, game files), Adoptium (Java), Modrinth / CurseForge / GitHub (mods you choose), and your friends server if you turn it on."],
    ["Telemetry", "None. No analytics, no crash uploads, no ads."],
    ["Logs", "Access tokens are scrubbed from the launcher and game logs."],
  ];
  root.append(
    h("div.section.glass", h("h2", "Your data"), h("div.privacy-list", items.map(([b, t]) => h("div.pl.card", { html: icon("shield") }, h("div", h("b", b), h("span", t)))))),
    h("div.section.glass", h("h2", "Local data"), setting("Data folder", "Instances, Java, libraries, logs and settings", btn("Open", { icon: "folder", onclick: () => api.openDataFolder() }))),
  );
}

function developer(root) {
  const s = S();
  const name = h("input.input", { value: s.offline_name, maxlength: 16, style: { width: "200px" } });
  name.oninput = () => save({ offline_name: name.value.trim() || "Player" });
  root.append(h("div.section.glass", h("h2", "Developer"),
    h("div.callout.warn", { style: { marginBottom: "12px" } }, h("b", "For testing your own builds only. "), "Offline test launches skip Microsoft sign-in, so they can only play singleplayer (no servers, no skins). Keep this off in anything you share."),
    setting("Offline test launch", "Lets Play work without a Microsoft account", toggle(s.offline_test, (v) => save({ offline_test: v }))),
    setting("Test player name", "3–16 letters, numbers or _", name),
  ));
}

function about(root) {
  const info = h("div.muted");
  api.info().then((i) => info.append(`Version ${i.version} · ${i.os}/${i.arch} · ${(i.totalMemoryMb / 1024).toFixed(0)} GB RAM`, h("br"), h("span.mono.small", i.dataDir)));
  root.append(
    h("div.section.glass", h("div.row", h("img", { src: "img/logo.png", style: { width: "56px" } }), h("div", h("h2", { style: { fontSize: "20px" } }, "saltClient"), info))),
    h("div.section.glass", h("h2", "Built with"),
      h("p.desc", "Rust + Tauri. Inspired by OneClient (Polyfrost). Game files from Mojang; loaders from Fabric and Legacy Fabric; Java from Adoptium; mods from Modrinth and CurseForge; world hosting by e4mc. Not affiliated with Mojang or Microsoft."),
      h("div.row", btn("Run setup again", { icon: "refresh", onclick: () => showOnboarding() })),
    ),
  );
}
