# Salt Launcher

A clean, glassy Minecraft launcher written in **Rust** (Tauri), with its own
HUD mod (**saltHUD**), a set of **saltMODs**, the usual performance mods out of
the box, your **Chlorine**, Modrinth + CurseForge, skins & capes, friends &
chat, e4mc world hosting, and a live log viewer.

```
salt-launcher/
├─ crates/salt-core/   Rust engine: auth, versions, downloads, Java, instances, launch,
│                      Modrinth, CurseForge, Salt bundle, skins, settings, social protocol
├─ launcher/           Tauri desktop app (src/ = commands, ui/ = HTML/CSS/JS, no bundler)
├─ server/             salt-social: self-hostable friends/chat/invites server
├─ mods/               saltHUD, saltZoom, saltFreelook, saltToggleSprint, saltFullbright
└─ docs/               MICROSOFT_LOGIN.md, RESEARCH.md
```

## Features

* **Onboarding** — welcome → privacy → Microsoft sign-in → theme → memory → first instance.
* **Sign-in that only proves ownership** — device-code flow; tokens in Windows
  Credential Manager; nothing sent anywhere but Microsoft/Xbox/Mojang. See
  [PRIVACY.md](PRIVACY.md).
* **Instances** — any release from **1.21 to 26.3, plus 1.8 / 1.8.9**, vanilla or
  Fabric (Legacy Fabric for 1.8.9). Per-instance memory, Java, JVM/game args,
  resolution, icon, group; duplicate, open folders, worlds list, playtime.
* **Automatic Java** — Temurin 8 / 21 / 25 picked per version, SHA-256 verified.
* **Salt bundle** — on launch, installs whatever supports the instance's version:
  saltMODs + Chlorine + Fabric API, Sodium, Lithium, FerriteCore, EntityCulling,
  ImmediatelyFast, ModernFix, More Culling, Dynamic FPS, ScalableLux, C2ME, VMP,
  Sodium Extra, Debugify, Mod Menu, e4mc. Iris/Continuity/etc. are opt-in.
* **Mod manager** — enable/disable/remove, source badges (saltMOD / Modrinth /
  CurseForge / local), update checks, add local jars.
* **Discover** — Modrinth and CurseForge search for mods, modpacks, resource
  packs and shaders; dependencies resolved; `.mrpack` / CurseForge zip import.
* **saltHUD manager** — drag 15 HUD elements on a 16:9 preview, per-element
  options, colors, sizes; also editable in game with Right Shift. Changes sync
  both ways (newest wins).
* **Skins & capes** — 3D preview, upload, reset, switch owned capes, local skin library.
* **Friends** — friend requests, presence, DMs and world invites via `salt-social`.
* **World hosting** — e4mc: Open to LAN → public address → invite a friend.
* **Live logs** — launcher + per-game logs with level filters, search, auto-scroll, copy.
* **Customizable glass UI** — accent, blur, opacity, corner radius, background
  presets or your own image, light/dark, animation speed, compact sidebar.

## Building

Prereqs (already installed on this PC): Rust (MSVC), Visual Studio C++ Build
Tools, and a Java 25+ JDK for the mods.

```powershell
# launcher (debug)
cargo run -p salt-launcher

# release build with your approved Microsoft client ID baked in
$env:SALT_MS_CLIENT_ID = "<your Azure client id>"
cargo build --release -p salt-launcher

# salt mods (+ your Chlorine checkout) -> %APPDATA%\SaltLauncher\salt-mods
powershell -ExecutionPolicy Bypass -File mods\build-all.ps1 -Chlorine "<path to your Chlorine checkout>"

# friends server
cargo run -p salt-social --release -- 127.0.0.1:7878

# engine tests + end-to-end launch test (opens a Minecraft window)
cargo test -p salt-core
cargo run -p salt-core --example smoke -- 26.3 90
```

Build output goes to `%USERPROFILE%\SaltLauncherBuild` (outside OneDrive, which
locks build files) — copy `.cargo/config.toml.example` to `.cargo/config.toml` and see `mods/26.3/build.gradle`.

An installer (`.exe`, NSIS) can be produced with the Tauri CLI:
`cargo install tauri-cli --version "^2" --locked` then `cargo tauri build` in `launcher/`.

## Before you ship

1. **Microsoft sign-in** needs an Azure app approved by Mojang —
   [docs/MICROSOFT_LOGIN.md](docs/MICROSOFT_LOGIN.md). Without it, sign-in fails
   at the Minecraft step.
2. **CurseForge** needs an API key (Settings → Integrations); Modrinth needs none.
3. **Friends** need a hosted `salt-social` ([server/README.md](server/README.md)).
4. **Mods** build for every release from 1.21 to 26.3; the 1.8.9 versions are
   still being written. See [mods/README.md](mods/README.md).

Not affiliated with Mojang, Microsoft, or Polyfrost. Design notes from OneClient
and other launchers are in [docs/RESEARCH.md](docs/RESEARCH.md).
