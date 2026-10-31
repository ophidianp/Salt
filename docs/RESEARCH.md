# Research notes: how other launchers/clients do it

Notes taken while designing Salt, mostly from the OneClient install on this
PC plus public knowledge of other launchers.

## OneClient (Polyfrost) — the main reference

Observed from `%APPDATA%\Polyfrost\OneClient` and the installed app:

* **Rust + Tauri desktop app** (single `oneclient_app.exe`, MSVC runtime DLLs
  bundled). Salt uses the same stack.
* **"Clusters" = instances**, one folder per version+loader (`26.3 Fabric`,
  `1.21.11 Fabric`, `1.8.9 Fabric` …). 1.8.9 runs on **Legacy Fabric**, not
  Forge — Salt does the same.
* **Shared `metadata/`** folder for assets, libraries, natives, Java and
  package caches, reused by every cluster → Salt's `meta/`.
* **`settings.json`** with `global_game_settings` (memory, java path, JVM/game
  args, hooks), `launch_behaviour`, theme/accent, `animations_enabled`,
  `seen_onboarding`, `modrinth_api_key`/`curseforge_api_key`. Salt mirrors
  most of these and adds glass/blur/background controls.
* Onboarding is tracked (`seen_onboarding`, TOS/privacy versions) → Salt's
  separate onboarding flow.
* Its 26.3 cluster ships a big curated mod set (Sodium, Lithium,
  FerriteCore, ModernFix, ImmediatelyFast, EntityCulling, C2ME, VMP,
  ScalableLux, Debugify, Iris, Continuity…). Salt's bundle picks the
  performance core of that list and keeps visuals opt-in.

## Other launchers

* **Prism / MultiMC** — instance folders with per-instance overrides; the
  "inherit from global unless set" model Salt uses for Java/memory.
* **Modrinth App (Theseus)** — Rust core + Tauri, `.mrpack` modpacks,
  Modrinth API without keys. Salt's `modrinth.rs` follows the official
  `.mrpack` spec (allowed download hosts, `overrides/` + `client-overrides/`).
* **Lunar / Badlion / Feather** — closed clients with HUD mods, cosmetics,
  friends, and world hosting. Salt's take: saltHUD + saltMODs as open Fabric
  mods, friends via a self-hostable server, hosting via e4mc.

## World hosting

* **e4mc** — "Open to LAN" gets a public `*.e4mc.link` address through e4mc's
  relay (QUIC). No port forwarding, works for most versions on Fabric, and is
  what several clients bundle. Salt installs it by default.
* Alternatives considered: running a dedicated relay (needs infrastructure
  and abuse handling), UPnP (unreliable, exposes your IP), playit.gg
  (separate agent). e4mc is the fastest path today.

## Auth

* Microsoft device code flow → XBL → XSTS → `login_with_xbox`. New Azure apps
  need Mojang approval (see MICROSOFT_LOGIN.md).
* Friends server identity uses Mojang's `join`/`hasJoined` like a game server,
  so no third party ever gets an access token.
