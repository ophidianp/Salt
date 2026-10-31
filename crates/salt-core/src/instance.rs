//! Instances: separate Minecraft installs, each with its own mods, configs,
//! worlds and settings (OneClient calls these "clusters").
//!
//! Layout:
//! ```text
//! instances/<id>/instance.json   settings for this instance
//! instances/<id>/content.json    where each installed mod/pack came from
//! instances/<id>/minecraft/      the game directory (mods/, saves/, config/…)
//! ```

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::meta::Loader;
use crate::Paths;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub id: String,
    pub name: String,
    pub mc_version: String,
    pub loader: Loader,
    #[serde(default)]
    pub loader_version: Option<String>,
    /// Emoji or built-in icon name shown on the instance card.
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default = "default_memory")]
    pub memory_mb: u32,
    #[serde(default)]
    pub jvm_args: Vec<String>,
    #[serde(default)]
    pub game_args: Vec<String>,
    #[serde(default)]
    pub java_path: Option<String>,
    #[serde(default)]
    pub resolution: Option<(u32, u32)>,
    /// Install the Salt bundle (saltHUD, performance mods, Chlorine…).
    #[serde(default = "yes")]
    pub salt_bundle: bool,
    #[serde(default)]
    pub pack: Option<PackOrigin>,
    pub created: chrono::DateTime<chrono::Utc>,
    #[serde(default)]
    pub last_played: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(default)]
    pub playtime_secs: u64,
}

fn default_memory() -> u32 { 4096 }
fn yes() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackOrigin {
    pub platform: String,
    pub project_id: String,
    pub version_id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewInstance {
    pub name: String,
    pub mc_version: String,
    pub loader: Loader,
    #[serde(default)]
    pub loader_version: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub memory_mb: Option<u32>,
    #[serde(default = "yes")]
    pub salt_bundle: bool,
}

impl Instance {
    pub fn dir(&self, paths: &Paths) -> PathBuf { paths.instance(&self.id) }
    pub fn game_dir(&self, paths: &Paths) -> PathBuf { self.dir(paths).join("minecraft") }
    pub fn mods_dir(&self, paths: &Paths) -> PathBuf { self.game_dir(paths).join("mods") }
}

fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "instance".into() } else { s.chars().take(40).collect() }
}

pub fn list(paths: &Paths) -> Vec<Instance> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(paths.instances()) {
        for e in rd.flatten() {
            let f = e.path().join("instance.json");
            if let Ok(bytes) = std::fs::read(&f) {
                if let Ok(i) = serde_json::from_slice::<Instance>(&bytes) {
                    out.push(i);
                }
            }
        }
    }
    out.sort_by(|a, b| b.last_played.cmp(&a.last_played).then(a.name.cmp(&b.name)));
    out
}

pub fn get(paths: &Paths, id: &str) -> Result<Instance> {
    let bytes = std::fs::read(paths.instance(id).join("instance.json")).with_context(|| format!("Instance {id} not found"))?;
    Ok(serde_json::from_slice(&bytes)?)
}

/// Salt instances every install starts with, newest first.
pub const DEFAULT_VERSIONS: &[&str] = &["26.3", "26.2", "26.1.2", "1.21.11", "1.21.9", "1.8.9"];

/// Creates a "Salt <version>" Fabric instance for each default version that
/// hasn't been offered yet and isn't already installed. `seeded` records what
/// was offered, so an instance the user deletes doesn't come back.
pub fn seed_defaults(paths: &Paths, memory_mb: u32, seeded: &mut Vec<String>) -> Result<Vec<Instance>> {
    let existing = list(paths);
    let mut made = Vec::new();
    for &v in DEFAULT_VERSIONS {
        if seeded.iter().any(|s| s == v) {
            continue;
        }
        if !existing.iter().any(|i| i.mc_version == v && i.loader == Loader::Fabric) {
            made.push(create(paths, NewInstance {
                name: format!("Salt {v}"),
                mc_version: v.into(),
                loader: Loader::Fabric,
                loader_version: None,
                icon: Some("🧂".into()),
                group: None,
                memory_mb: Some(memory_mb),
                salt_bundle: true,
            })?);
        }
        seeded.push(v.into());
    }
    Ok(made)
}

pub fn save(paths: &Paths, inst: &Instance) -> Result<()> {
    let dir = inst.dir(paths);
    std::fs::create_dir_all(dir.join("minecraft").join("mods"))?;
    std::fs::write(dir.join("instance.json"), serde_json::to_vec_pretty(inst)?)?;
    Ok(())
}

pub fn create(paths: &Paths, req: NewInstance) -> Result<Instance> {
    let base = slug(&req.name);
    let mut id = base.clone();
    let mut n = 2;
    while paths.instance(&id).exists() {
        id = format!("{base}-{n}");
        n += 1;
    }
    let inst = Instance {
        id,
        name: req.name,
        mc_version: req.mc_version,
        loader: req.loader,
        loader_version: req.loader_version,
        icon: req.icon,
        group: req.group,
        memory_mb: req.memory_mb.unwrap_or(4096),
        jvm_args: vec![],
        game_args: vec![],
        java_path: None,
        resolution: None,
        salt_bundle: req.salt_bundle,
        pack: None,
        created: chrono::Utc::now(),
        last_played: None,
        playtime_secs: 0,
    };
    save(paths, &inst)?;
    Ok(inst)
}

pub fn delete(paths: &Paths, id: &str) -> Result<()> {
    let dir = paths.instance(id);
    if !dir.starts_with(paths.instances()) || id.is_empty() || id.contains("..") {
        return Err(anyhow!("refusing to delete outside the instances folder"));
    }
    std::fs::remove_dir_all(dir)?;
    Ok(())
}

pub fn duplicate(paths: &Paths, id: &str) -> Result<Instance> {
    let src = get(paths, id)?;
    let mut copy = create(
        paths,
        NewInstance {
            name: format!("{} (copy)", src.name),
            mc_version: src.mc_version.clone(),
            loader: src.loader,
            loader_version: src.loader_version.clone(),
            icon: src.icon.clone(),
            group: src.group.clone(),
            memory_mb: Some(src.memory_mb),
            salt_bundle: src.salt_bundle,
        },
    )?;
    copy_dir(&src.game_dir(paths), &copy.game_dir(paths))?;
    let content = src.dir(paths).join("content.json");
    if content.exists() {
        std::fs::copy(content, copy.dir(paths).join("content.json"))?;
    }
    copy.jvm_args = src.jvm_args;
    copy.game_args = src.game_args;
    save(paths, &copy)?;
    Ok(copy)
}

fn copy_dir(from: &std::path::Path, to: &std::path::Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for e in std::fs::read_dir(from)?.flatten() {
        let p = e.path();
        let dest = to.join(e.file_name());
        // Skip logs/crash reports — they belong to the original.
        if matches!(e.file_name().to_str(), Some("logs" | "crash-reports")) {
            continue;
        }
        if p.is_dir() {
            copy_dir(&p, &dest)?;
        } else {
            std::fs::copy(&p, &dest)?;
        }
    }
    Ok(())
}

// ------------------------------------------------------------------ content

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    Salt,
    Modrinth,
    Curseforge,
    Local,
}

/// One line of `content.json`: where a file in `mods/` came from, so updates
/// and the mod manager can show proper names and icons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentEntry {
    pub file_name: String,
    pub source: Source,
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub icon_url: Option<String>,
    /// "mod", "resourcepack", "shader"
    #[serde(default = "mod_kind")]
    pub kind: String,
}

fn mod_kind() -> String { "mod".into() }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Content {
    pub entries: Vec<ContentEntry>,
}

impl Content {
    pub fn load(paths: &Paths, id: &str) -> Self {
        std::fs::read(paths.instance(id).join("content.json"))
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, paths: &Paths, id: &str) -> Result<()> {
        std::fs::write(paths.instance(id).join("content.json"), serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    pub fn upsert(&mut self, entry: ContentEntry) {
        // Replace an older version of the same project.
        if let Some(pid) = &entry.project_id {
            self.entries.retain(|e| e.project_id.as_ref() != Some(pid) || e.source != entry.source);
        }
        self.entries.retain(|e| e.file_name != entry.file_name);
        self.entries.push(entry);
    }
}

/// What the mod manager shows for each jar.
#[derive(Debug, Clone, Serialize)]
pub struct ModView {
    pub file_name: String,
    pub enabled: bool,
    pub title: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub source: Source,
    pub project_id: Option<String>,
    pub icon_url: Option<String>,
    pub mod_id: Option<String>,
    pub size: u64,
}

#[derive(Deserialize, Default)]
struct FabricModJson {
    id: Option<String>,
    name: Option<String>,
    version: Option<String>,
    description: Option<String>,
    #[serde(default)]
    authors: Vec<serde_json::Value>,
}

fn read_fabric_mod_json(path: &std::path::Path) -> Option<FabricModJson> {
    let f = std::fs::File::open(path).ok()?;
    let mut zip = zip::ZipArchive::new(f).ok()?;
    let mut entry = zip.by_name("fabric.mod.json").ok()?;
    let mut s = String::new();
    std::io::Read::read_to_string(&mut entry, &mut s).ok()?;
    // Some mods put raw newlines inside strings; strip control chars first.
    let cleaned: String = s.chars().filter(|c| !c.is_control() || *c == '\n').collect();
    serde_json::from_str(&cleaned.replace('\n', " ")).ok()
}

pub fn list_mods(paths: &Paths, inst: &Instance) -> Vec<ModView> {
    let content = Content::load(paths, &inst.id);
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(inst.mods_dir(paths)) else { return out };
    for e in rd.flatten() {
        let file_name = e.file_name().to_string_lossy().to_string();
        let enabled = file_name.ends_with(".jar");
        if !enabled && !file_name.ends_with(".jar.disabled") {
            continue;
        }
        let base = file_name.trim_end_matches(".disabled").to_string();
        let tracked = content.entries.iter().find(|c| c.file_name == base);
        let fmj = read_fabric_mod_json(&e.path()).unwrap_or_default();
        let authors = fmj
            .authors
            .iter()
            .filter_map(|a| a.as_str().map(str::to_string).or_else(|| a["name"].as_str().map(str::to_string)))
            .collect();
        out.push(ModView {
            title: tracked
                .and_then(|t| t.title.clone())
                .or(fmj.name)
                .unwrap_or_else(|| base.trim_end_matches(".jar").to_string()),
            version: tracked.and_then(|t| t.version.clone()).or(fmj.version),
            description: fmj.description,
            authors,
            source: tracked.map(|t| t.source.clone()).unwrap_or(Source::Local),
            project_id: tracked.and_then(|t| t.project_id.clone()),
            icon_url: tracked.and_then(|t| t.icon_url.clone()),
            mod_id: fmj.id,
            size: e.metadata().map(|m| m.len()).unwrap_or(0),
            file_name: base,
            enabled,
        });
    }
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    out
}

pub fn set_mod_enabled(paths: &Paths, inst: &Instance, file_name: &str, enabled: bool) -> Result<()> {
    let dir = inst.mods_dir(paths);
    let on = dir.join(file_name);
    let off = dir.join(format!("{file_name}.disabled"));
    if enabled && off.exists() {
        std::fs::rename(off, on)?;
    } else if !enabled && on.exists() {
        std::fs::rename(on, off)?;
    }
    Ok(())
}

pub fn remove_mod(paths: &Paths, inst: &Instance, file_name: &str) -> Result<()> {
    if file_name.contains('/') || file_name.contains('\\') || file_name.contains("..") {
        return Err(anyhow!("invalid file name"));
    }
    let dir = inst.mods_dir(paths);
    let _ = std::fs::remove_file(dir.join(file_name));
    let _ = std::fs::remove_file(dir.join(format!("{file_name}.disabled")));
    let mut content = Content::load(paths, &inst.id);
    content.entries.retain(|e| e.file_name != file_name);
    content.save(paths, &inst.id)?;
    Ok(())
}

pub fn import_local_mod(paths: &Paths, inst: &Instance, src: &std::path::Path) -> Result<String> {
    let name = src.file_name().ok_or_else(|| anyhow!("no file name"))?.to_string_lossy().to_string();
    if !name.ends_with(".jar") {
        return Err(anyhow!("Only .jar mods can be added"));
    }
    std::fs::create_dir_all(inst.mods_dir(paths))?;
    std::fs::copy(src, inst.mods_dir(paths).join(&name))?;
    let mut content = Content::load(paths, &inst.id);
    content.upsert(ContentEntry {
        file_name: name.clone(),
        source: Source::Local,
        project_id: None,
        version_id: None,
        title: None,
        version: None,
        icon_url: None,
        kind: "mod".into(),
    });
    content.save(paths, &inst.id)?;
    Ok(name)
}

/// Worlds in this instance (for the world list + "host this world").
#[derive(Debug, Clone, Serialize)]
pub struct WorldView {
    pub folder: String,
    pub size: u64,
    pub modified: Option<chrono::DateTime<chrono::Utc>>,
}

pub fn list_worlds(paths: &Paths, inst: &Instance) -> Vec<WorldView> {
    let mut out = Vec::new();
    if let Ok(rd) = std::fs::read_dir(inst.game_dir(paths).join("saves")) {
        for e in rd.flatten() {
            if e.path().join("level.dat").exists() {
                let modified = e.metadata().ok().and_then(|m| m.modified().ok()).map(chrono::DateTime::<chrono::Utc>::from);
                out.push(WorldView { folder: e.file_name().to_string_lossy().into(), size: 0, modified });
            }
        }
    }
    out.sort_by(|a, b| b.modified.cmp(&a.modified));
    out
}
