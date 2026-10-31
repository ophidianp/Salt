//! Version metadata: Mojang's manifest + version JSONs, and Fabric / Legacy
//! Fabric loader profiles merged on top of them.

use std::collections::HashMap;

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};

use crate::http::{client, get_json};
use crate::version::{self, McVersion};
use crate::Paths;

pub const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
pub const FABRIC_META: &str = "https://meta.fabricmc.net";
pub const LEGACY_FABRIC_META: &str = "https://meta.legacyfabric.net";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Loader {
    Vanilla,
    Fabric,
}

impl Loader {
    /// Loader name used by the Modrinth/CurseForge APIs for this game version.
    pub fn api_name(&self, mc: &str) -> &'static str {
        match self {
            Loader::Vanilla => "minecraft",
            Loader::Fabric if McVersion::parse(mc).map(|v| v.is_legacy()).unwrap_or(false) => "legacy-fabric",
            Loader::Fabric => "fabric",
        }
    }
}

pub fn fabric_meta_for(mc: &str) -> &'static str {
    if McVersion::parse(mc).map(|v| v.is_legacy()).unwrap_or(false) {
        LEGACY_FABRIC_META
    } else {
        FABRIC_META
    }
}

// ------------------------------------------------------------- manifest types

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    pub latest: HashMap<String, String>,
    pub versions: Vec<ManifestEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(rename = "releaseTime")]
    pub release_time: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VersionJson {
    pub id: String,
    #[serde(default)]
    pub inherits_from: Option<String>,
    #[serde(default)]
    pub main_class: Option<String>,
    #[serde(default)]
    pub arguments: Option<Arguments>,
    #[serde(default)]
    pub minecraft_arguments: Option<String>,
    #[serde(default)]
    pub libraries: Vec<Library>,
    #[serde(default)]
    pub asset_index: Option<AssetIndexRef>,
    #[serde(default)]
    pub assets: Option<String>,
    #[serde(default)]
    pub downloads: HashMap<String, Download>,
    #[serde(default)]
    pub java_version: Option<JavaVersion>,
    #[serde(default)]
    pub logging: Option<serde_json::Value>,
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Arguments {
    #[serde(default)]
    pub game: Vec<Argument>,
    #[serde(default)]
    pub jvm: Vec<Argument>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Argument {
    Plain(String),
    Ruled { rules: Vec<Rule>, value: OneOrMany },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OneOrMany {
    One(String),
    Many(Vec<String>),
}

impl OneOrMany {
    pub fn to_vec(&self) -> Vec<String> {
        match self {
            OneOrMany::One(s) => vec![s.clone()],
            OneOrMany::Many(v) => v.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rule {
    pub action: String,
    #[serde(default)]
    pub os: Option<OsRule>,
    #[serde(default)]
    pub features: Option<HashMap<String, bool>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRule {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub arch: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Library {
    pub name: String,
    #[serde(default)]
    pub downloads: Option<LibraryDownloads>,
    /// Maven repository base (Fabric-style libraries).
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub natives: Option<HashMap<String, String>>,
    #[serde(default)]
    pub rules: Option<Vec<Rule>>,
    #[serde(default)]
    pub extract: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LibraryDownloads {
    #[serde(default)]
    pub artifact: Option<Download>,
    #[serde(default)]
    pub classifiers: Option<HashMap<String, Download>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Download {
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub sha1: Option<String>,
    #[serde(default)]
    pub size: Option<u64>,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetIndexRef {
    pub id: String,
    pub sha1: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JavaVersion {
    #[serde(default)]
    pub component: Option<String>,
    pub major_version: u32,
}

// ------------------------------------------------------------------ rule eval

pub fn os_name() -> &'static str {
    match std::env::consts::OS {
        "macos" => "osx",
        other => other,
    }
}

/// Feature flags the launcher supports (all rule-gated arguments whose
/// features aren't listed here are skipped, e.g. demo mode / quick play).
#[derive(Debug, Clone, Default)]
pub struct Features {
    pub custom_resolution: bool,
}

pub fn rules_allow(rules: &[Rule], features: &Features) -> bool {
    if rules.is_empty() {
        return true;
    }
    let mut allowed = false;
    for rule in rules {
        let mut matches = true;
        if let Some(os) = &rule.os {
            if let Some(name) = &os.name {
                matches &= name == os_name();
            }
            if let Some(arch) = &os.arch {
                matches &= arch == "x86" && cfg!(target_pointer_width = "32");
            }
        }
        if let Some(f) = &rule.features {
            for (key, want) in f {
                let have = match key.as_str() {
                    "has_custom_resolution" => features.custom_resolution,
                    _ => false,
                };
                matches &= have == *want;
            }
        }
        if matches {
            allowed = rule.action == "allow";
        }
    }
    allowed
}

// ---------------------------------------------------------------- fetching

pub async fn manifest() -> Result<Manifest> {
    get_json(MANIFEST_URL).await.context("Couldn't download the Minecraft version list")
}

/// Releases Salt supports, newest first.
pub async fn supported_versions() -> Result<Vec<ManifestEntry>> {
    let m = manifest().await?;
    let mut v: Vec<ManifestEntry> = m
        .versions
        .into_iter()
        .filter(|e| e.kind == "release" && version::is_supported(&e.id))
        .collect();
    v.sort_by(|a, b| version::compare(&b.id, &a.id));
    Ok(v)
}

async fn cached_json<T: serde::de::DeserializeOwned + Serialize>(path: std::path::PathBuf, url: &str) -> Result<T> {
    if let Ok(bytes) = tokio::fs::read(&path).await {
        if let Ok(v) = serde_json::from_slice(&bytes) {
            return Ok(v);
        }
    }
    let value: T = get_json(url).await?;
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(&path, serde_json::to_vec(&value)?).await?;
    Ok(value)
}

pub async fn vanilla_json(paths: &Paths, id: &str) -> Result<VersionJson> {
    let path = paths.versions().join(id).join(format!("{id}.json"));
    if path.exists() {
        if let Ok(v) = serde_json::from_slice(&tokio::fs::read(&path).await?) {
            return Ok(v);
        }
    }
    let m = manifest().await?;
    let entry = m
        .versions
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| anyhow!("Minecraft {id} isn't in Mojang's version list"))?;
    cached_json(path, &entry.url).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderVersion {
    pub version: String,
    pub stable: bool,
}

pub async fn fabric_loaders(mc: &str) -> Result<Vec<LoaderVersion>> {
    #[derive(Deserialize)]
    struct Entry {
        loader: LoaderVersion,
    }
    let url = format!("{}/v2/versions/loader/{}", fabric_meta_for(mc), mc);
    let list: Vec<Entry> = get_json(&url).await.with_context(|| format!("No Fabric loader is available for {mc}"))?;
    Ok(list.into_iter().map(|e| e.loader).collect())
}

pub async fn latest_fabric_loader(mc: &str) -> Result<String> {
    let loaders = fabric_loaders(mc).await?;
    loaders
        .iter()
        .find(|l| l.stable)
        .or(loaders.first())
        .map(|l| l.version.clone())
        .ok_or_else(|| anyhow!("No Fabric loader is available for {mc}"))
}

pub async fn fabric_json(paths: &Paths, mc: &str, loader: &str) -> Result<VersionJson> {
    let id = format!("fabric-loader-{loader}-{mc}");
    let path = paths.versions().join(&id).join(format!("{id}.json"));
    let url = format!("{}/v2/versions/loader/{}/{}/profile/json", fabric_meta_for(mc), mc, loader);
    cached_json(path, &url).await
}

/// The final, merged version JSON for an instance: vanilla + loader.
pub async fn resolve(paths: &Paths, mc: &str, loader: Loader, loader_version: Option<&str>) -> Result<VersionJson> {
    let vanilla = vanilla_json(paths, mc).await?;
    match loader {
        Loader::Vanilla => Ok(vanilla),
        Loader::Fabric => {
            let lv = match loader_version {
                Some(v) => v.to_string(),
                None => latest_fabric_loader(mc).await?,
            };
            let child = fabric_json(paths, mc, &lv).await?;
            Ok(merge(vanilla, child))
        }
    }
}

/// Library identity without version, so a loader's newer ASM replaces
/// vanilla's older one instead of both ending up on the classpath.
fn lib_key(name: &str) -> String {
    let parts: Vec<&str> = name.split(':').collect();
    match parts.len() {
        0..=2 => name.to_string(),
        3 => format!("{}:{}", parts[0], parts[1]),
        _ => format!("{}:{}:{}", parts[0], parts[1], parts[3]),
    }
}

pub fn merge(parent: VersionJson, child: VersionJson) -> VersionJson {
    // Only the loader's libraries override vanilla ones. Vanilla itself lists
    // the same library several times with different OS rules (LWJGL on
    // 1.8.9), so it must not be de-duplicated against itself.
    let child_keys: std::collections::HashSet<String> = child.libraries.iter().map(|l| lib_key(&l.name)).collect();
    let mut libraries = child.libraries;
    libraries.extend(parent.libraries.into_iter().filter(|l| !child_keys.contains(&lib_key(&l.name))));
    let arguments = match (parent.arguments, child.arguments) {
        (Some(mut p), Some(c)) => {
            p.game.extend(c.game);
            p.jvm.extend(c.jvm);
            Some(p)
        }
        (p, c) => p.or(c),
    };
    VersionJson {
        id: child.id,
        inherits_from: None,
        main_class: child.main_class.or(parent.main_class),
        arguments,
        minecraft_arguments: child.minecraft_arguments.or(parent.minecraft_arguments),
        libraries,
        asset_index: parent.asset_index,
        assets: parent.assets,
        downloads: parent.downloads,
        java_version: parent.java_version,
        logging: parent.logging,
        kind: parent.kind,
    }
}

/// Used by the update checker for the version list in the UI.
pub async fn ping() -> bool {
    client().head(MANIFEST_URL).send().await.map(|r| r.status().is_success()).unwrap_or(false)
}
