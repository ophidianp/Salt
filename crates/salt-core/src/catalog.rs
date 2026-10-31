//! The Salt bundle: everything a fresh instance gets out of the box.
//!
//! * **Salt mods** — saltHUD, saltZoom, saltFreelook… built from `mods/` in
//!   this repo and copied to `salt-mods/<mc version>/` (see mods/README.md).
//! * **Chlorine** — fetched from its GitHub releases for 26.x.
//! * **Performance + essentials** — well-known Modrinth projects. Each one is
//!   only installed if Modrinth has a build for the instance's version and
//!   loader, so the same list works for 1.8.9, 1.21.x and 26.x.
//!
//! Users can switch any entry off per instance in the mod manager.

use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::download::{self, Item};
use crate::http::get_json;
use crate::instance::{Content, ContentEntry, Instance, Source};
use crate::progress::Reporter;
use crate::{modrinth, Paths};

#[derive(Debug, Clone, Serialize)]
pub struct BundleMod {
    pub slug: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub default_on: bool,
}

const fn m(slug: &'static str, name: &'static str, category: &'static str, description: &'static str, default_on: bool) -> BundleMod {
    BundleMod { slug, name, category, description, default_on }
}

/// Modrinth projects in the bundle.
pub const MODRINTH_BUNDLE: &[BundleMod] = &[
    // libraries
    m("fabric-api", "Fabric API", "library", "Core hooks most Fabric mods need", true),
    m("legacy-fabric-api", "Legacy Fabric API", "library", "Fabric API for 1.8.9", true),
    m("cloth-config", "Cloth Config", "library", "Config screens (needed by Chlorine)", true),
    m("modmenu", "Mod Menu", "library", "Not needed: saltConfig is the mod list (opt in if you still want it)", false),
    // performance
    m("sodium", "Sodium", "performance", "Modern rendering engine, huge FPS gains", true),
    m("lithium", "Lithium", "performance", "Game logic and tick optimizations", true),
    m("ferrite-core", "FerriteCore", "performance", "Cuts memory usage", true),
    m("entityculling", "EntityCulling", "performance", "Skips rendering hidden entities", true),
    m("immediatelyfast", "ImmediatelyFast", "performance", "Faster HUD, text and entity rendering", true),
    m("modernfix", "ModernFix", "performance", "Faster startup, less memory", true),
    m("moreculling", "More Culling", "performance", "Culls more block faces", true),
    m("dynamic-fps", "Dynamic FPS", "performance", "Lowers FPS when the window is in the background", true),
    m("scalablelux", "ScalableLux", "performance", "Faster lighting engine", true),
    m("c2me-fabric", "C2ME", "performance", "Parallel chunk loading in singleplayer", true),
    m("vmp-fabric", "Very Many Players", "performance", "Faster entity tracking and chunk sending", true),
    m("sodium-extra", "Sodium Extra", "performance", "Extra video options for Sodium", true),
    m("debugify", "Debugify", "performance", "Fixes many vanilla bugs", true),
    // hosting / social
    m("e4mc", "e4mc", "hosting", "Open to LAN → instant public address for friends", true),
    // optional extras
    m("iris", "Iris Shaders", "visual", "Shader pack support", false),
    m("continuity", "Continuity", "visual", "Connected textures", false),
    m("lambdynamiclights", "LambDynamicLights", "visual", "Held items light up the world", false),
    m("no-chat-reports", "No Chat Reports", "utility", "Makes chat messages unreportable (may block some servers)", false),
];

/// Salt-built mods. The jars come from `salt-mods/<mc>/` (built by
/// `mods/build-all.ps1`).
pub const SALT_MODS: &[(&str, &str, &str)] = &[
    ("saltconfig", "saltConfig", "One settings screen for every Salt mod (Right Shift). Required by the others."),
    ("salthud", "saltHUD", "FPS, CPS, keystrokes, coords, armor, potions, ping, clock and more"),
    ("saltmenu", "saltMenu", "Clean black Salt title screen"),
    ("saltzoom", "saltZoom", "Smooth scroll-to-zoom"),
    ("saltfreelook", "saltFreelook", "Look around without turning"),
    ("salttogglesprint", "saltToggleSprint", "Toggle sprint and sneak"),
    ("saltfullbright", "saltFullbright", "Full brightness toggle"),
    ("saltsocial", "saltSocial", "Friends + chat overlay connected to the launcher"),
];

pub fn bundle_listing() -> serde_json::Value {
    serde_json::json!({
        "modrinth": MODRINTH_BUNDLE,
        "salt": SALT_MODS.iter().map(|(id, name, desc)| serde_json::json!({"id": id, "name": name, "description": desc})).collect::<Vec<_>>(),
        "chlorine": { "name": "Chlorine", "description": "CPU-side performance + power management (26.x)" }
    })
}

/// Remembers which projects have no build for a given version, so launches
/// don't re-ask Modrinth every time. Entries expire after a day.
#[derive(Default, Serialize, Deserialize)]
struct Unavailable {
    checked: HashMap<String, i64>,
}

fn unavailable_path(paths: &Paths, mc: &str, loader: &str) -> std::path::PathBuf {
    paths.cache().join(format!("bundle-unavailable-{mc}-{loader}.json"))
}

/// Installs whatever enabled bundle entries are missing. Never fails the
/// launch: problems are logged and the game starts without that mod.
/// Bundle mods whose newest build claims a version it crashes on, pinned to
/// one that works there: (slug, Minecraft version, version number). An empty
/// version number leaves the mod out on that Minecraft version.
const BUNDLE_PINS: &[(&str, &str, &str)] = &[];

pub async fn sync(paths: &Paths, inst: &Instance, disabled: &[String], r: &dyn Reporter) -> Result<()> {
    if !inst.salt_bundle || inst.loader != crate::meta::Loader::Fabric {
        return Ok(());
    }
    let loader = inst.loader.api_name(&inst.mc_version);
    let content = Content::load(paths, &inst.id);
    let un_path = unavailable_path(paths, &inst.mc_version, loader);
    let mut unavailable: Unavailable = std::fs::read(&un_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let now = chrono::Utc::now().timestamp();

    for entry in MODRINTH_BUNDLE {
        let wanted = if entry.default_on { !disabled.iter().any(|d| d == entry.slug) } else { disabled.iter().any(|d| d == &format!("+{}", entry.slug)) };
        if !wanted {
            continue;
        }
        let pin = BUNDLE_PINS.iter().find(|(slug, mc, _)| *slug == entry.slug && *mc == inst.mc_version).map(|p| p.2);
        let installed = content.entries.iter().find(|e| e.project_id.as_deref() == Some(entry.slug) || e.title.as_deref() == Some(entry.name));
        if pin == Some("") {
            if let Some(e) = installed {
                let _ = crate::instance::set_mod_enabled(paths, inst, &e.file_name, false);
            }
            continue;
        }
        if installed.is_some_and(|e| pin.is_none() || e.version.as_deref() == pin) {
            continue;
        }
        if unavailable.checked.get(entry.slug).map(|t| now - t < 86_400).unwrap_or(false) {
            continue;
        }
        let found = match pin {
            Some(pin) => modrinth::versions(entry.slug, Some(&inst.mc_version), Some(loader)).await.map(|vs| vs.into_iter().find(|v| v.version_number == pin)),
            None => modrinth::best_version(entry.slug, &inst.mc_version, Some(loader)).await,
        };
        match found {
            Ok(Some(v)) => {
                if let Err(e) = modrinth::install_project(paths, inst, entry.slug, Some(&v.id), r).await {
                    r.log("warn", &format!("Couldn't install {}: {e:#}", entry.name));
                }
            }
            Ok(None) => {
                unavailable.checked.insert(entry.slug.to_string(), now);
            }
            Err(e) => r.log("warn", &format!("Couldn't check {}: {e:#}", entry.name)),
        }
    }
    let _ = std::fs::write(&un_path, serde_json::to_vec(&unavailable)?);

    install_salt_jars(paths, inst, disabled, r)?;
    if !disabled.iter().any(|d| d == "+modmenu") {
        retire_mod_menu(paths, inst, r);
    }
    if !disabled.iter().any(|d| d == "chlorine") {
        if let Err(e) = install_chlorine(paths, inst, r).await {
            r.log("warn", &format!("Chlorine: {e:#}"));
        }
    }
    Ok(())
}

/// saltConfig replaces Mod Menu (it ships Mod Menu's API, so Cloth Config
/// screens open from it). Older bundles installed Mod Menu; switch it off once
/// this instance has a saltConfig that carries the replacement, so the two
/// don't both claim the same API classes.
fn retire_mod_menu(paths: &Paths, inst: &Instance, r: &dyn Reporter) {
    let mods_dir = inst.mods_dir(paths);
    let Ok(rd) = std::fs::read_dir(&mods_dir) else { return };
    let names: Vec<String> = rd.flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
    let replaces = names.iter().filter(|n| n.to_lowercase().starts_with("saltconfig") && n.ends_with(".jar")).any(|n| {
        std::fs::File::open(mods_dir.join(n))
            .ok()
            .and_then(|f| zip::ZipArchive::new(f).ok())
            .map(|mut z| z.by_name("com/terraformersmc/modmenu/api/ModMenuApi.class").is_ok())
            .unwrap_or(false)
    });
    if !replaces {
        return;
    }
    for n in names.iter().filter(|n| n.to_lowercase().starts_with("modmenu") && n.ends_with(".jar")) {
        if crate::instance::set_mod_enabled(paths, inst, n, false).is_ok() {
            r.log("info", &format!("Turned off {n}: saltConfig is the mod list now."));
        }
    }
}

/// Locally built Salt mod jars (and Chlorine) for a version. Looked up in the
/// data folder (`salt-mods/<mc>/`, filled by mods/build-all.ps1) and in a
/// `salt-mods/<mc>/` folder next to the launcher exe (how a portable or
/// installed build ships them). The data folder wins on name clashes.
pub fn local_jars(paths: &Paths, mc: &str) -> Vec<std::path::PathBuf> {
    let mut dirs = vec![paths.salt_mods().join(mc)];
    if let Some(exe_dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf())) {
        dirs.push(exe_dir.join("salt-mods").join(mc));
    }
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for d in dirs {
        let Ok(rd) = std::fs::read_dir(&d) else { continue };
        for e in rd.flatten() {
            let name = e.file_name().to_string_lossy().to_lowercase();
            if name.ends_with(".jar") && !name.ends_with("-sources.jar") && seen.insert(name) {
                out.push(e.path());
            }
        }
    }
    out
}

fn same_file(a: &std::path::Path, b: &std::path::Path) -> bool {
    match (std::fs::metadata(a), std::fs::metadata(b)) {
        (Ok(ma), Ok(mb)) if ma.len() == mb.len() => crate::download::sha1_file(a).ok() == crate::download::sha1_file(b).ok(),
        _ => false,
    }
}

fn install_salt_jars(paths: &Paths, inst: &Instance, disabled: &[String], r: &dyn Reporter) -> Result<()> {
    let jars = local_jars(paths, &inst.mc_version);
    if !jars.iter().any(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase().starts_with("salt")).unwrap_or(false)) {
        r.log("info", &format!("No Salt mod builds for {} yet (salt-mods/{}). Build them with mods/build-all.ps1.", inst.mc_version, inst.mc_version));
        return Ok(());
    }
    let mods_dir = inst.mods_dir(paths);
    std::fs::create_dir_all(&mods_dir)?;
    let mut content = Content::load(paths, &inst.id);
    for path in jars {
        let name = path.file_name().unwrap_or_default().to_string_lossy().to_string();
        if !name.ends_with(".jar") || name.ends_with("-sources.jar") {
            continue;
        }
        let Some((id, title, _)) = SALT_MODS.iter().find(|(id, _, _)| name.to_lowercase().starts_with(id)) else { continue };
        // saltConfig can't be switched off: every other Salt mod needs it.
        if *id != "saltconfig" && disabled.iter().any(|d| d == id) {
            continue;
        }
        // Replace older builds of the same Salt mod.
        for old in content.entries.iter().filter(|c| c.source == Source::Salt && c.project_id.as_deref() == Some(id) && c.file_name != name) {
            let _ = std::fs::remove_file(mods_dir.join(&old.file_name));
            let _ = std::fs::remove_file(mods_dir.join(format!("{}.disabled", old.file_name)));
        }
        let dest = mods_dir.join(&name);
        let disabled_copy = mods_dir.join(format!("{name}.disabled"));
        // Rebuilt jars keep the same name, so compare contents, not names.
        if disabled_copy.exists() {
            if !same_file(&path, &disabled_copy) {
                std::fs::copy(&path, &disabled_copy)?;
            }
        } else if !dest.exists() || !same_file(&path, &dest) {
            std::fs::copy(&path, &dest)?;
            r.log("info", &format!("Installed {name}"));
        }
        content.upsert(ContentEntry {
            file_name: name.clone(),
            source: Source::Salt,
            project_id: Some(id.to_string()),
            version_id: None,
            title: Some(title.to_string()),
            version: None,
            icon_url: None,
            kind: "mod".into(),
        });
    }
    content.save(paths, &inst.id)?;
    Ok(())
}

#[derive(Deserialize)]
struct GhRelease {
    name: Option<String>,
    tag_name: String,
    body: Option<String>,
    assets: Vec<GhAsset>,
    #[serde(default)]
    prerelease: bool,
}

#[derive(Deserialize)]
struct GhAsset {
    name: String,
    browser_download_url: String,
}

const CHLORINE_RELEASES: &str = "https://api.github.com/repos/ophidianp/chlorine/releases";

/// Releases whose notes don't name their version cleanly (1.0.2's notes
/// mention 26.2 but it's a 26.3 build), pinned by tag.
const CHLORINE_PINS: &[(&str, &str)] = &[("26.2", "1.0.0")];

/// Chlorine releases say "Minecraft 26.x" in their name/tag/body (see the
/// Chlorine repo's CLAUDE.md), which is how the right build is picked. A jar
/// dropped into `salt-mods/<mc>/` (e.g. a local dev build) wins over GitHub.
pub async fn install_chlorine(paths: &Paths, inst: &Instance, r: &dyn Reporter) -> Result<()> {
    if !inst.mc_version.starts_with("26.") {
        return Ok(()); // Chlorine targets 26.x only.
    }
    let mut content = Content::load(paths, &inst.id);
    let local = local_jars(paths, &inst.mc_version)
        .into_iter()
        .find(|p| p.file_name().map(|n| n.to_string_lossy().to_lowercase().starts_with("chlorine")).unwrap_or(false));
    let mods_dir = inst.mods_dir(paths);

    let (file_name, version) = if let Some(local) = local {
        let name = local.file_name().unwrap_or_default().to_string_lossy().to_string();
        if !same_file(&local, &mods_dir.join(&name)) {
            std::fs::copy(&local, mods_dir.join(&name))?;
        }
        (name, None)
    } else {
        let pin = CHLORINE_PINS.iter().find(|(mc, _)| *mc == inst.mc_version).map(|(_, tag)| *tag);
        let installed = content.entries.iter().find(|e| e.project_id.as_deref() == Some("chlorine"));
        if installed.is_some_and(|e| pin.is_none() || e.version.as_deref() == pin) {
            return Ok(());
        }
        let releases: Vec<GhRelease> = get_json(CHLORINE_RELEASES).await?;
        let needle = format!("Minecraft {}", inst.mc_version);
        let found = match pin {
            Some(tag) => releases.iter().find(|r| r.tag_name == tag),
            None => releases.iter().filter(|r| !r.prerelease).find(|r| {
                r.name.as_deref().unwrap_or("").contains(&needle) || r.tag_name.contains(&needle) || r.body.as_deref().unwrap_or("").contains(&needle)
            }),
        };
        let Some(rel) = found else {
            r.log("info", &format!("No Chlorine release for Minecraft {} yet", inst.mc_version));
            return Ok(());
        };
        let Some(asset) = rel.assets.iter().find(|a| a.name.ends_with(".jar") && !a.name.ends_with("-sources.jar")) else { return Ok(()) };
        download::fetch(&Item::new(&asset.browser_download_url, mods_dir.join(&asset.name))).await?;
        r.log("info", &format!("Installed Chlorine {}", rel.tag_name));
        (asset.name.clone(), Some(rel.tag_name.clone()))
    };
    // Drop older Chlorine jars.
    for old in content.entries.iter().filter(|c| c.project_id.as_deref() == Some("chlorine") && c.file_name != file_name) {
        let _ = std::fs::remove_file(mods_dir.join(&old.file_name));
    }
    content.upsert(ContentEntry {
        file_name,
        source: Source::Salt,
        project_id: Some("chlorine".into()),
        version_id: None,
        title: Some("Chlorine".into()),
        version,
        icon_url: None,
        kind: "mod".into(),
    });
    content.save(paths, &inst.id)?;
    Ok(())
}
