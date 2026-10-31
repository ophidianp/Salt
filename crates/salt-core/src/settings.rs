//! Launcher settings, saved as `settings.json`. Secrets (the CurseForge API
//! key) are kept in the OS credential store instead — see `secrets`.

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::Paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Theme {
    /// "dark" | "light" | "system"
    pub mode: String,
    /// Accent colour as a CSS colour.
    pub accent: String,
    /// Backdrop blur of glass panels, in px.
    pub blur: u32,
    /// Opacity of glass panels, 0.0–1.0.
    pub glass_opacity: f32,
    /// Corner radius in px.
    pub radius: u32,
    pub animations: bool,
    /// 0.5 = twice as fast, 2.0 = twice as slow.
    pub animation_speed: f32,
    /// Built-in background name, or a data URL for a custom image.
    pub background: String,
    pub background_dim: f32,
    pub font: String,
    pub compact: bool,
    /// Bumped when the default look changes so old saved themes get the new
    /// defaults once (see `Settings::load`). Missing in old files = 0.
    #[serde(default)]
    pub style_version: u32,
}

pub const STYLE_VERSION: u32 = 3;

impl Default for Theme {
    fn default() -> Self {
        Self {
            mode: "dark".into(),
            accent: "#7cc8ff".into(),
            blur: 0,
            glass_opacity: 1.0,
            radius: 14,
            animations: true,
            animation_speed: 1.0,
            background: "black".into(),
            background_dim: 0.35,
            font: "Inter".into(),
            compact: false,
            style_version: STYLE_VERSION,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub onboarded: bool,
    /// Azure application (client) ID used for Microsoft sign-in. See
    /// docs/MICROSOFT_LOGIN.md for how to get one.
    pub microsoft_client_id: String,
    pub theme: Theme,
    pub java_path: Option<String>,
    pub default_memory_mb: u32,
    pub extra_jvm_args: Vec<String>,
    pub max_concurrent_downloads: usize,
    /// "keep_open" | "minimize" | "close"
    pub launch_behaviour: String,
    /// Salt bundle entries the user switched off (slug or salt mod id).
    /// Entries that are off by default are switched on with a `+slug` entry.
    pub bundle_overrides: Vec<String>,
    /// Friends/chat server (self-hostable, see server/README.md).
    pub social_server: String,
    pub social_enabled: bool,
    /// Show friends what you're playing.
    pub share_activity: bool,
    pub show_snapshots: bool,
    pub language: String,
    /// Developer option: launch without a Microsoft account (offline test
    /// profile, singleplayer only). Off by default; for testing your own
    /// builds before Microsoft sign-in is set up. Don't ship it enabled.
    pub offline_test: bool,
    pub offline_name: String,
    /// Default Salt instance versions already created (see
    /// `instance::seed_defaults`).
    pub seeded_versions: Vec<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            onboarded: false,
            // Set SALT_MS_CLIENT_ID when building to ship your approved
            // Azure app ID; users can still override it in Settings.
            microsoft_client_id: option_env!("SALT_MS_CLIENT_ID").unwrap_or("").to_string(),
            theme: Theme::default(),
            java_path: None,
            default_memory_mb: 4096,
            extra_jvm_args: vec![],
            max_concurrent_downloads: 16,
            launch_behaviour: "keep_open".into(),
            bundle_overrides: vec![],
            social_server: "ws://127.0.0.1:7878/ws".into(),
            social_enabled: false,
            share_activity: true,
            show_snapshots: false,
            language: "en".into(),
            offline_test: false,
            offline_name: "Player".into(),
            seeded_versions: vec![],
        }
    }
}

impl Settings {
    pub fn load(paths: &Paths) -> Self {
        let mut s: Settings = std::fs::read(paths.settings_file()).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
        if s.theme.style_version < STYLE_VERSION {
            // v3: black backdrop and solid panels (no glass). Keep the
            // user's accent, mode and other choices.
            let d = Theme::default();
            s.theme.blur = d.blur;
            s.theme.glass_opacity = d.glass_opacity;
            if !s.theme.background.starts_with("data:") {
                s.theme.background = d.background;
            }
            s.theme.style_version = STYLE_VERSION;
        }
        s
    }

    pub fn save(&self, paths: &Paths) -> Result<()> {
        std::fs::create_dir_all(&paths.root)?;
        let tmp = paths.settings_file().with_extension("tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        std::fs::rename(tmp, paths.settings_file())?;
        Ok(())
    }
}

pub fn curseforge_key() -> String {
    crate::secrets::get("curseforge:api-key").unwrap_or_default()
}

pub fn set_curseforge_key(key: &str) -> Result<()> {
    if key.trim().is_empty() {
        crate::secrets::delete("curseforge:api-key");
        Ok(())
    } else {
        crate::secrets::set("curseforge:api-key", key.trim())
    }
}

// ------------------------------------------------------------------ saltHUD

/// The saltHUD layout, edited in the launcher's HUD manager and written to
/// `config/salthud.json` in each instance before launch. The mod reads the
/// same file and also lets you drag elements in-game.
pub fn hud_path(paths: &Paths) -> std::path::PathBuf {
    paths.root.join("salthud.json")
}

pub fn load_hud(paths: &Paths) -> serde_json::Value {
    std::fs::read(hud_path(paths))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_else(default_hud)
}

pub fn save_hud(paths: &Paths, hud: &serde_json::Value) -> Result<()> {
    let mut hud = hud.clone();
    // Both the launcher and the in-game editor stamp `modified`; whichever
    // edit is newer wins when they're synced on launch.
    hud["modified"] = serde_json::json!(chrono::Utc::now().timestamp_millis());
    std::fs::write(hud_path(paths), serde_json::to_vec_pretty(&hud)?)?;
    Ok(())
}

fn modified(v: &serde_json::Value) -> i64 {
    v["modified"].as_i64().unwrap_or(0)
}

/// Syncs the HUD layout with an instance before launch. If the layout was
/// changed in game (Right Shift editor) more recently than in the launcher,
/// the in-game version is kept and becomes the launcher's layout too;
/// otherwise the launcher's layout is copied in. An instance can opt out
/// entirely with a `config/salthud.local` marker file.
pub fn sync_hud_into(paths: &Paths, game_dir: &std::path::Path) -> Result<()> {
    let config = game_dir.join("config");
    if config.join("salthud.local").exists() {
        return Ok(());
    }
    let target = config.join("salthud.json");
    let global = load_hud(paths);
    let local: Option<serde_json::Value> = std::fs::read(&target).ok().and_then(|b| serde_json::from_slice(&b).ok());
    if let Some(local) = local {
        if modified(&local) > modified(&global) {
            std::fs::write(hud_path(paths), serde_json::to_vec_pretty(&local)?)?;
            return Ok(());
        }
    }
    std::fs::create_dir_all(&config)?;
    std::fs::write(target, serde_json::to_vec_pretty(&global)?)?;
    Ok(())
}

/// The launcher's look for the Salt mods: `config/salt-theme.json` in each
/// instance. Written before launch and whenever settings are saved; the game
/// re-reads it while running, so a new accent shows up in game right away.
pub fn sync_theme_into(theme: &Theme, game_dir: &std::path::Path) -> Result<()> {
    let config = game_dir.join("config");
    let json = serde_json::json!({ "accent": theme.accent, "radius": theme.radius });
    let target = config.join("salt-theme.json");
    let bytes = serde_json::to_vec_pretty(&json)?;
    // Unchanged = don't touch it, so the game's poll doesn't reload for nothing.
    if std::fs::read(&target).ok().as_deref() == Some(&bytes[..]) {
        return Ok(());
    }
    std::fs::create_dir_all(&config)?;
    std::fs::write(target, bytes)?;
    Ok(())
}

/// Element positions are fractions of the screen (0–1) so a layout looks
/// the same at any resolution. `anchor` picks which corner they're measured
/// from, so right-aligned elements stay right-aligned.
pub fn default_hud() -> serde_json::Value {
    serde_json::json!({
        "version": 1,
        "global": { "textShadow": true, "background": true, "backgroundColor": "#66000000", "textColor": "#FFFFFFFF", "accentColor": "#FF7CC8FF", "scale": 1.0, "padding": 4, "rounded": true },
        "elements": [
            { "id": "fps",         "enabled": true,  "x": 0.005, "y": 0.010, "anchor": "top_left",     "scale": 1.0, "options": { "label": "FPS" } },
            { "id": "cps",         "enabled": true,  "x": 0.005, "y": 0.045, "anchor": "top_left",     "scale": 1.0, "options": { "showRight": true } },
            { "id": "ping",        "enabled": true,  "x": 0.005, "y": 0.080, "anchor": "top_left",     "scale": 1.0, "options": {} },
            { "id": "coordinates", "enabled": true,  "x": 0.005, "y": 0.115, "anchor": "top_left",     "scale": 1.0, "options": { "showBiome": true, "showDirection": true } },
            { "id": "keystrokes",  "enabled": true,  "x": 0.010, "y": 0.250, "anchor": "bottom_left",  "scale": 1.0, "options": { "showMouse": true, "showSpace": true, "showCps": true } },
            { "id": "armor",       "enabled": true,  "x": 0.005, "y": 0.300, "anchor": "top_right",    "scale": 1.0, "options": { "showDurability": true, "vertical": true } },
            { "id": "potions",     "enabled": true,  "x": 0.005, "y": 0.010, "anchor": "top_right",    "scale": 1.0, "options": { "showIcons": true } },
            { "id": "clock",       "enabled": false, "x": 0.500, "y": 0.010, "anchor": "top_center",   "scale": 1.0, "options": { "twentyFour": true } },
            { "id": "speed",       "enabled": false, "x": 0.005, "y": 0.150, "anchor": "top_left",     "scale": 1.0, "options": { "unit": "bps" } },
            { "id": "memory",      "enabled": false, "x": 0.005, "y": 0.185, "anchor": "top_left",     "scale": 1.0, "options": {} },
            { "id": "server",      "enabled": false, "x": 0.005, "y": 0.220, "anchor": "top_left",     "scale": 1.0, "options": {} },
            { "id": "combo",       "enabled": false, "x": 0.500, "y": 0.600, "anchor": "top_center",   "scale": 1.0, "options": {} },
            { "id": "reach",       "enabled": false, "x": 0.500, "y": 0.640, "anchor": "top_center",   "scale": 1.0, "options": {} },
            { "id": "pack",        "enabled": false, "x": 0.005, "y": 0.010, "anchor": "bottom_right", "scale": 1.0, "options": {} },
            { "id": "crosshair",   "enabled": false, "x": 0.500, "y": 0.500, "anchor": "center",       "scale": 1.0, "options": { "style": "plus", "gap": 2, "color": "#FFFFFFFF" } }
        ]
    })
}
