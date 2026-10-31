// First-run onboarding (its own full-screen flow) + the reusable sign-in dialog.
import { api } from "../api.js";
import { icon } from "../icons.js";
import { store } from "../store.js";
import { ACCENTS, BACKGROUNDS, applyTheme } from "../theme.js";
import { h, btn, toast, toggle, range, modal, headCanvas } from "../ui.js";

const PRIVACY = [
  ["Your password never touches saltClient", "You sign in on microsoft.com in your own browser. saltClient only receives a sign-in token."],
  ["Only ownership is checked", "saltClient asks Xbox and Mojang one question: does this account own Minecraft: Java Edition? Nothing else is read."],
  ["Tokens stay on this PC, encrypted", "Sign-in tokens live in Windows Credential Manager — never in plain files, never in logs."],
  ["No analytics, no tracking", "saltClient sends no telemetry. It only contacts Microsoft, Mojang, and the mod sites you browse."],
];

export function showOnboarding() {
  const root = document.getElementById("onboarding-root");
  const theme = { ...store.settings.theme };
  const state = { step: 0, dir: 1, memory: store.settings.default_memory_mb, bundle: true, version: null, totalMb: 8192 };
  api.info().then((i) => { state.totalMb = i.totalMemoryMb; });
  const card = h("div.onboard-card.glass");
  root.appendChild(h("div.onboard", card));

  const steps = [welcome, privacy, signIn, look, perf, firstInstance];

  function render() {
    card.innerHTML = "";
    const step = h("div.onboard-step" + (state.dir < 0 ? ".back" : ""));
    steps[state.step](step);
    const dots = h("div.steps-dots", steps.map((_, i) => h("i", { class: i === state.step ? "on" : "" })));
    const back = btn("Back", { cls: "ghost", onclick: () => nav(-1) });
    if (state.step === 0) back.style.visibility = "hidden";
    card.append(step, h("div.onboard-nav", back, dots, step.next || h("span")));
  }
  function nav(d) {
    state.dir = d;
    state.step = Math.max(0, Math.min(steps.length - 1, state.step + d));
    render();
  }
  const next = (label = "Continue", onclick = () => nav(1)) => btn(label, { cls: "primary", onclick });

  function welcome(el) {
    el.append(
      h("img.welcome-logo", { src: "img/logo.png", alt: "" }),
      h("h1", "Welcome to saltClient"),
      h("p.lead", "A clean, fast Minecraft launcher with saltHUD, performance mods out of the box, Modrinth + CurseForge, skins, friends and world hosting. Let's get you set up — it takes about a minute."),
    );
    el.next = next("Get started");
  }

  function privacy(el) {
    el.append(
      h("h1", "Your account stays yours"),
      h("p.lead", "Here's exactly what saltClient does when you sign in."),
      h("div.privacy-list", PRIVACY.map(([b, s]) => h("div.pl.card", { html: icon("shield") }, h("div", h("b", b), h("span", s))))),
    );
    el.next = next("Sounds good");
  }

  function signIn(el) {
    const a = store.activeAccount();
    el.append(h("h1", "Sign in"), h("p.lead", "Prove you own Minecraft: Java Edition. saltClient never sees your password."));
    const body = h("div.col", { style: { gap: "14px" } });
    el.appendChild(body);
    if (a) {
      const head = h("div.row");
      api.texture(a.skin_url || "").then((s) => head.prepend(headCanvas(s, 40))).catch(() => {});
      head.append(h("div", h("b", a.username), h("div.small.muted", "Signed in — ownership verified")));
      body.append(h("div.card", { style: { padding: "14px" } }, head));
      el.next = next();
    } else {
      body.append(loginPanel(() => render()));
      el.next = btn("Skip for now", { cls: "ghost", onclick: () => nav(1) });
    }
  }

  function look(el) {
    const preview = () => applyTheme(theme);
    el.append(h("h1", "Make it yours"), h("p.lead", "Everything here can be changed later in Settings → Appearance."));
    const sw = h("div.swatches", ACCENTS.map((c) => {
      const s = h("div.swatch", { style: { background: c }, class: c === theme.accent ? "on" : "" });
      s.onclick = () => { theme.accent = c; sw.querySelectorAll(".swatch").forEach((x) => x.classList.remove("on")); s.classList.add("on"); preview(); };
      return s;
    }));
    const bgs = h("div.bg-presets", Object.entries(BACKGROUNDS).map(([id, b]) => {
      const p = h("div.bg-preset", { style: { background: b.css }, class: id === theme.background ? "on" : "" }, h("span", b.label));
      p.onclick = () => { theme.background = id; bgs.querySelectorAll(".bg-preset").forEach((x) => x.classList.remove("on")); p.classList.add("on"); preview(); };
      return p;
    }));
    el.append(
      h("div.col", { style: { gap: "18px" } },
        h("div.field", h("label", "Accent"), sw),
        h("div.field", h("label", "Background"), bgs),
        h("div.row", { style: { gap: "30px" } },
          h("div.field", h("label", "Glass blur"), range(theme.blur, 0, 48, 1, (v) => { theme.blur = v; preview(); }, (v) => v + "px")),
          h("div.field", h("label", "Animations"), toggle(theme.animations, (v) => { theme.animations = v; preview(); })),
        ),
      ),
    );
    el.next = next("Continue", async () => { await store.saveSettings({ theme }); nav(1); });
  }

  function perf(el) {
    const rec = Math.min(8192, Math.max(2048, Math.floor(state.totalMb / 4 / 512) * 512));
    if (state.memory === 4096 && rec !== 4096) state.memory = rec;
    el.append(
      h("h1", "Performance"),
      h("p.lead", `Your PC has ${(state.totalMb / 1024).toFixed(0)} GB of RAM. We recommend giving Minecraft ${(rec / 1024).toFixed(1)} GB.`),
      h("div.col", { style: { gap: "18px" } },
        h("div.field", h("label", "Memory for the game"), range(state.memory, 1024, Math.min(16384, state.totalMb - 1024), 256, (v) => (state.memory = v), (v) => (v / 1024).toFixed(1) + " GB")),
        h("div.card", { style: { padding: "14px" } },
          h("div.row",
            h("div.grow", h("b", "Install the Salt bundle"), h("div.small.muted", "saltHUD + saltMODs, Sodium, Lithium, FerriteCore, EntityCulling, ImmediatelyFast, ModernFix, e4mc, Chlorine and more — only the ones that support your version.")),
            toggle(state.bundle, (v) => (state.bundle = v)),
          ),
        ),
      ),
    );
    el.next = next("Continue", async () => { await store.saveSettings({ default_memory_mb: state.memory }); nav(1); });
  }

  function firstInstance(el) {
    el.append(h("h1", "Your first instance"), h("p.lead", "Pick a version to start with. You can make as many instances as you like later."));
    const grid = h("div.version-pick");
    el.appendChild(grid);
    const choices = [];
    store.loadVersions().then((vs) => {
      const latest = vs[0]?.id;
      const latest121 = vs.find((v) => v.id.startsWith("1.21"))?.id;
      const picks = [[latest, "Latest", "Newest release"], [latest121, "1.21", "Huge mod library"], ["1.8.9", "1.8.9", "Classic PvP"]].filter(([v]) => v);
      state.version = state.version || latest;
      for (const [v, title, sub] of picks) {
        const c = h("button.vp.card.hover", { class: v === state.version ? "on" : "" }, h("b", v), h("div.small.muted", `${title} · ${sub}`));
        c.onclick = () => { state.version = v; choices.forEach((x) => x.classList.remove("on")); c.classList.add("on"); };
        choices.push(c);
        grid.appendChild(c);
      }
    }).catch(() => grid.append(h("div.callout.warn", "Couldn't load the version list — check your connection. You can create an instance later.")));
    const finish = async () => {
      try {
        if (state.version && !store.instances.some((i) => i.mc_version === state.version && i.loader === "fabric")) {
          await api.createInstance({ name: `Salt ${state.version}`, mc_version: state.version, loader: "fabric", icon: "🧂", memory_mb: state.memory, salt_bundle: state.bundle });
          await store.refreshInstances();
        }
        await store.saveSettings({ onboarded: true });
        const root = document.getElementById("onboarding-root");
        root.firstChild.style.transition = "opacity .35s";
        root.firstChild.style.opacity = "0";
        setTimeout(() => (root.innerHTML = ""), 360);
        toast("You're all set", "Hit Play whenever you're ready.", "ok");
        location.hash = "#/home";
      } catch (e) { toast("Couldn't finish setup", String(e), "err"); }
    };
    el.next = h("div.row", btn("Skip", { cls: "ghost", onclick: () => { state.version = null; finish(); } }), next("Finish", finish));
  }

  render();
}

/**
 * Sign-in panel. Defaults to "Sign in with Minecraft": the player joins a
 * local server from their normal Minecraft and Mojang confirms they own it
 * (see salt-core verify.rs). Microsoft device-code sign-in is the fallback.
 * Calls onDone() after a successful login.
 */
export function loginPanel(onDone) {
  const box = h("div.col", { style: { gap: "14px" } });
  const minecraft = async () => {
    box.innerHTML = "";
    box.append(h("div.row", h("div.spinner"), h("span.muted", "Starting sign-in…")));
    let address;
    try {
      ({ address } = await api.verifyStart());
    } catch (e) {
      box.innerHTML = "";
      box.append(h("div.callout.warn", String(e)), h("div.row", btn("Try again", { cls: "primary", onclick: minecraft })));
      return;
    }
    const copy = () => navigator.clipboard.writeText(address).then(() => toast("Address copied", "", "ok", 1500));
    box.innerHTML = "";
    box.append(
      h("ol.signin-steps",
        h("li", "Open your normal Minecraft (any version from the official launcher)."),
        h("li", "Go to ", h("b", "Multiplayer → Direct Connection"), " and join this address:"),
      ),
      h("div.device-code.signin-address", address),
      h("div.row",
        btn("Copy address", { cls: "primary", icon: "copy", onclick: copy }),
        btn("Use Microsoft instead", { cls: "ghost", onclick: () => { api.verifyCancel(); microsoft(); } }),
      ),
      h("div.row.small.muted", h("div.spinner"), "Waiting for you to join… you'll be kicked with a \"Signed in\" message."),
      h("div.callout", h("b", "What saltClient sees: "), "only your public name, UUID and skin, confirmed by Mojang. No password or token. ",
        "Verified accounts play singleplayer, LAN and offline-mode servers; online-mode servers need Microsoft sign-in."),
    );
    const off = store.on("login", (r) => {
      off();
      if (r.ok) onDone?.(r.account);
      else {
        box.innerHTML = "";
        box.append(h("div.callout.warn", r.error), h("div.row", btn("Try again", { cls: "primary", onclick: minecraft })));
      }
    });
  };
  const microsoft = () => {
    box.innerHTML = "";
    box.append(msPanel(onDone), h("div.row", btn("Back to Sign in with Minecraft", { cls: "ghost sm", onclick: minecraft })));
  };
  minecraft();
  return box;
}

/** Microsoft device-code sign-in. */
function msPanel(onDone) {
  const box = h("div.col", { style: { gap: "14px" } });
  const start = async () => {
    box.innerHTML = "";
    if (!store.settings.microsoft_client_id) {
      const input = h("input.input", { placeholder: "00000000-0000-0000-0000-000000000000" });
      box.append(
        h("div.callout.warn", h("b", "Microsoft sign-in isn't configured yet. "), "saltClient needs an Azure app (client) ID approved for Minecraft. See docs/MICROSOFT_LOGIN.md in the project, then paste the ID here."),
        h("div.field", h("label", "Azure application (client) ID"), input),
        h("div.row", btn("Save and continue", { cls: "primary", onclick: async () => {
          if (!/^[0-9a-f-]{36}$/i.test(input.value.trim())) return toast("That doesn't look like a client ID", "It's a 36-character GUID.", "err");
          await store.saveSettings({ microsoft_client_id: input.value.trim() });
          start();
        } })),
      );
      return;
    }
    box.append(h("div.row", h("div.spinner"), h("span.muted", "Contacting Microsoft…")));
    try {
      const code = await api.loginStart();
      box.innerHTML = "";
      const copy = () => navigator.clipboard.writeText(code.user_code).then(() => toast("Code copied", "", "ok", 1500));
      box.append(
        h("div.muted", "1. Copy this code"),
        h("div.device-code", code.user_code),
        h("div.muted", `2. Open ${code.verification_uri} and paste it`),
        h("div.row",
          btn("Copy code & open Microsoft", { cls: "primary", icon: "external", onclick: () => { copy(); api.openUrl(code.verification_uri); } }),
          btn("Copy code", { icon: "copy", onclick: copy }),
        ),
        h("div.row.small.muted", h("div.spinner"), "Waiting for you to finish signing in…"),
      );
      const off = store.on("login", (r) => {
        off();
        if (r.ok) onDone?.(r.account);
        else {
          box.innerHTML = "";
          box.append(h("div.callout.warn", r.error), h("div.row", btn("Try again", { cls: "primary", onclick: start })));
        }
      });
    } catch (e) {
      box.innerHTML = "";
      box.append(h("div.callout.warn", String(e)), h("div.row", btn("Try again", { cls: "primary", onclick: start })));
    }
  };
  start();
  return box;
}

export function startLogin() {
  modal((close) => [
    h("h2", "Add an account"),
    h("p.m-sub", "saltClient only checks that the account owns Minecraft. Your password never reaches Salt."),
    loginPanel(() => close()),
    h("div.m-actions", btn("Close", { onclick: () => { api.verifyCancel(); close(); } })),
  ]);
}
