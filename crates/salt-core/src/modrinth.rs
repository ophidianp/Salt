//! Modrinth API v2 (https://docs.modrinth.com/api/). No key is needed for
//! reading or downloading; requests carry the launcher User-Agent only.

use std::collections::HashSet;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};

use crate::download::{self, Item};
use crate::http::{client, get_json};
use crate::instance::{self, Content, ContentEntry, Instance, NewInstance, PackOrigin, Source};
use crate::meta::Loader;
use crate::paths::safe_join;
use crate::progress::{self, Reporter};
use crate::Paths;

pub const API: &str = "https://api.modrinth.com/v2";

/// Hosts a .mrpack may download from, per Modrinth's modpack format spec.
const ALLOWED_PACK_HOSTS: &[&str] = &["cdn.modrinth.com", "github.com", "raw.githubusercontent.com", "gitlab.com"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub downloads: u64,
    #[serde(default)]
    pub follows: u64,
    pub project_type: String,
    #[serde(default)]
    pub categories: Vec<String>,
    #[serde(default)]
    pub date_modified: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub hits: Vec<SearchHit>,
    pub offset: u32,
    pub limit: u32,
    pub total_hits: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    /// mod | modpack | resourcepack | shader
    pub kind: String,
    pub game_version: Option<String>,
    pub loader: Option<String>,
    #[serde(default)]
    pub offset: u32,
    #[serde(default = "default_limit")]
    pub limit: u32,
    /// relevance | downloads | follows | newest | updated
    #[serde(default = "default_sort")]
    pub sort: String,
}

fn default_limit() -> u32 { 20 }
fn default_sort() -> String { "relevance".into() }

pub async fn search(q: &SearchQuery) -> Result<SearchResult> {
    let mut facets: Vec<Vec<String>> = vec![vec![format!("project_type:{}", q.kind)]];
    if let Some(v) = &q.game_version {
        facets.push(vec![format!("versions:{v}")]);
    }
    if let Some(l) = &q.loader {
        if q.kind == "mod" || q.kind == "modpack" {
            facets.push(vec![format!("categories:{l}")]);
        }
    }
    let url = format!(
        "{API}/search?query={}&facets={}&index={}&offset={}&limit={}",
        urlencoding::encode(&q.query),
        urlencoding::encode(&serde_json::to_string(&facets)?),
        q.sort,
        q.offset,
        q.limit.min(100)
    );
    get_json(&url).await.context("Modrinth search failed")
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub body: String,
    #[serde(default)]
    pub icon_url: Option<String>,
    pub project_type: String,
    pub downloads: u64,
    #[serde(default)]
    pub gallery: Vec<serde_json::Value>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub loaders: Vec<String>,
}

pub async fn project(id: &str) -> Result<Project> {
    get_json(&format!("{API}/project/{}", urlencoding::encode(id))).await
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Version {
    pub id: String,
    pub project_id: String,
    pub name: String,
    pub version_number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub version_type: String,
    pub files: Vec<VersionFile>,
    #[serde(default)]
    pub dependencies: Vec<Dependency>,
    pub date_published: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionFile {
    pub url: String,
    pub filename: String,
    pub primary: bool,
    pub size: u64,
    pub hashes: std::collections::HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dependency {
    #[serde(default)]
    pub project_id: Option<String>,
    #[serde(default)]
    pub version_id: Option<String>,
    pub dependency_type: String,
}

pub async fn versions(project_id: &str, game_version: Option<&str>, loader: Option<&str>) -> Result<Vec<Version>> {
    let mut url = format!("{API}/project/{}/version?", urlencoding::encode(project_id));
    if let Some(l) = loader {
        url.push_str(&format!("loaders={}&", urlencoding::encode(&format!("[\"{l}\"]"))));
    }
    if let Some(v) = game_version {
        url.push_str(&format!("game_versions={}", urlencoding::encode(&format!("[\"{v}\"]"))));
    }
    get_json(&url).await
}

pub async fn version(id: &str) -> Result<Version> {
    get_json(&format!("{API}/version/{}", urlencoding::encode(id))).await
}

/// Newest compatible version, preferring releases over betas/alphas.
pub async fn best_version(project_id: &str, mc: &str, loader: Option<&str>) -> Result<Option<Version>> {
    let list = versions(project_id, Some(mc), loader).await?;
    Ok(list
        .iter()
        .find(|v| v.version_type == "release")
        .or_else(|| list.first())
        .cloned())
}

fn content_dir(kind: &str) -> &'static str {
    match kind {
        "resourcepack" => "resourcepacks",
        "shader" => "shaderpacks",
        _ => "mods",
    }
}

fn loader_filter(kind: &str, inst: &Instance) -> Option<&'static str> {
    match kind {
        "mod" => Some(inst.loader.api_name(&inst.mc_version)),
        "resourcepack" => Some("minecraft"),
        _ => None,
    }
}

/// Installs a project (and its required dependencies) into an instance.
pub async fn install_project(paths: &Paths, inst: &Instance, project_id: &str, version_id: Option<&str>, r: &dyn Reporter) -> Result<Vec<String>> {
    let proj = project(project_id).await?;
    let kind = proj.project_type.clone();
    let mut installed = Vec::new();
    let mut seen = HashSet::new();
    let mut queue: Vec<(String, Option<String>)> = vec![(proj.id.clone(), version_id.map(str::to_string))];
    let mut content = Content::load(paths, &inst.id);

    while let Some((pid, vid)) = queue.pop() {
        if !seen.insert(pid.clone()) {
            continue;
        }
        // Skip things already installed (common for Fabric API).
        if pid != proj.id && content.entries.iter().any(|e| e.project_id.as_deref() == Some(pid.as_str())) {
            continue;
        }
        let p = if pid == proj.id { proj.clone() } else { project(&pid).await? };
        let v = match vid {
            Some(id) => version(&id).await?,
            None => best_version(&pid, &inst.mc_version, loader_filter(&p.project_type, inst))
                .await?
                .ok_or_else(|| anyhow!("{} has no version for Minecraft {}", p.title, inst.mc_version))?,
        };
        let file = v.files.iter().find(|f| f.primary).or(v.files.first()).ok_or_else(|| anyhow!("{} has no files", p.title))?;
        let dir = inst.game_dir(paths).join(content_dir(&p.project_type));
        let dest = safe_join(&dir, &file.filename).ok_or_else(|| anyhow!("bad file name"))?;
        download::fetch(&Item::hashed(&file.url, &dest, file.hashes.get("sha1").cloned(), Some(file.size))).await?;

        // Remove the previous file for this project if the name changed.
        if let Some(old) = content.entries.iter().find(|e| e.project_id.as_deref() == Some(pid.as_str()) && e.file_name != file.filename) {
            let _ = std::fs::remove_file(dir.join(&old.file_name));
        }
        content.upsert(ContentEntry {
            file_name: file.filename.clone(),
            source: Source::Modrinth,
            project_id: Some(pid.clone()),
            version_id: Some(v.id.clone()),
            title: Some(p.title.clone()),
            version: Some(v.version_number.clone()),
            icon_url: p.icon_url.clone(),
            kind: p.project_type.clone(),
        });
        r.log("info", &format!("Installed {} {}", p.title, v.version_number));
        installed.push(p.title.clone());

        if p.project_type == "mod" {
            for d in v.dependencies.iter().filter(|d| d.dependency_type == "required") {
                if let Some(dp) = &d.project_id {
                    queue.push((dp.clone(), d.version_id.clone()));
                }
            }
        }
    }
    content.save(paths, &inst.id)?;
    let _ = kind;
    Ok(installed)
}

/// Checks every Modrinth-tracked mod for a newer compatible version.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub file_name: String,
    pub title: String,
    pub current: Option<String>,
    pub latest: String,
    pub project_id: String,
    pub version_id: String,
}

pub async fn check_updates(paths: &Paths, inst: &Instance) -> Result<Vec<UpdateInfo>> {
    let content = Content::load(paths, &inst.id);
    let mut out = Vec::new();
    for e in content.entries.iter().filter(|e| e.source == Source::Modrinth) {
        let Some(pid) = &e.project_id else { continue };
        let loader = if e.kind == "mod" { Some(inst.loader.api_name(&inst.mc_version)) } else { None };
        if let Ok(Some(v)) = best_version(pid, &inst.mc_version, loader).await {
            if Some(&v.id) != e.version_id.as_ref() {
                out.push(UpdateInfo {
                    file_name: e.file_name.clone(),
                    title: e.title.clone().unwrap_or_default(),
                    current: e.version.clone(),
                    latest: v.version_number.clone(),
                    project_id: pid.clone(),
                    version_id: v.id,
                });
            }
        }
    }
    Ok(out)
}

// ------------------------------------------------------------------ .mrpack

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MrIndex {
    name: String,
    #[serde(default)]
    version_id: String,
    files: Vec<MrFile>,
    dependencies: std::collections::HashMap<String, String>,
}

#[derive(Deserialize)]
struct MrFile {
    path: String,
    hashes: std::collections::HashMap<String, String>,
    #[serde(default)]
    env: Option<std::collections::HashMap<String, String>>,
    downloads: Vec<String>,
    #[serde(rename = "fileSize", default)]
    file_size: Option<u64>,
}

fn host_allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split('/').next().unwrap_or("");
    ALLOWED_PACK_HOSTS.iter().any(|h| host == *h)
}

/// Installs a Modrinth modpack version as a new instance.
pub async fn install_modpack_version(paths: &Paths, version_id: &str, icon: Option<String>, r: &dyn Reporter) -> Result<Instance> {
    let v = version(version_id).await?;
    let proj = project(&v.project_id).await?;
    let file = v.files.iter().find(|f| f.primary).or(v.files.first()).ok_or_else(|| anyhow!("modpack has no files"))?;
    let bytes = client().get(&file.url).send().await?.error_for_status()?.bytes().await?;
    let mut inst = install_mrpack_bytes(paths, &bytes, icon.or(proj.icon_url.clone()), r).await?;
    inst.pack = Some(PackOrigin { platform: "modrinth".into(), project_id: proj.id, version_id: v.id, title: proj.title });
    instance::save(paths, &inst)?;
    Ok(inst)
}

pub async fn install_mrpack_bytes(paths: &Paths, bytes: &[u8], icon: Option<String>, r: &dyn Reporter) -> Result<Instance> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("not a valid .mrpack")?;
    let index: MrIndex = {
        let mut f = zip.by_name("modrinth.index.json").context(".mrpack is missing modrinth.index.json")?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s)?;
        serde_json::from_str(&s)?
    };
    let mc = index.dependencies.get("minecraft").cloned().ok_or_else(|| anyhow!("modpack doesn't say which Minecraft version it needs"))?;
    let (loader, loader_version) = if let Some(v) = index.dependencies.get("fabric-loader") {
        (Loader::Fabric, Some(v.clone()))
    } else if index.dependencies.keys().any(|k| k == "forge" || k == "neoforge" || k == "quilt-loader") {
        bail!("This modpack needs Forge/NeoForge/Quilt. Salt currently supports Fabric and vanilla packs.");
    } else {
        (Loader::Vanilla, None)
    };
    let inst = instance::create(
        paths,
        NewInstance {
            name: format!("{} {}", index.name, index.version_id).trim().to_string(),
            mc_version: mc,
            loader,
            loader_version,
            icon,
            group: Some("Modpacks".into()),
            memory_mb: Some(6144),
            // Packs choose their own mods; don't add the Salt bundle on top.
            salt_bundle: false,
        },
    )?;
    let game = inst.game_dir(paths);

    let mut items = Vec::new();
    for f in &index.files {
        if f.env.as_ref().and_then(|e| e.get("client")).map(|s| s == "unsupported").unwrap_or(false) {
            continue;
        }
        let dest = safe_join(&game, &f.path).ok_or_else(|| anyhow!("modpack contains an unsafe path: {}", f.path))?;
        let url = f.downloads.iter().find(|u| host_allowed(u)).ok_or_else(|| anyhow!("{} is hosted on a domain Modrinth doesn't allow", f.path))?;
        items.push(Item::hashed(url.clone(), dest, f.hashes.get("sha1").cloned(), f.file_size));
    }
    download::fetch_all(items, 16, r, &format!("pack-{}", inst.id), "Modpack files").await?;

    // overrides/ then client-overrides/ (the latter wins).
    for prefix in ["overrides/", "client-overrides/"] {
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let name = entry.name().to_string();
            let Some(rel) = name.strip_prefix(prefix) else { continue };
            if rel.is_empty() || entry.is_dir() {
                continue;
            }
            let Some(dest) = safe_join(&game, rel) else { continue };
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut out = std::fs::File::create(&dest)?;
            std::io::copy(&mut entry, &mut out)?;
        }
    }
    progress::finish(r, &format!("pack-{}", inst.id), "Modpack installed");
    r.log("info", &format!("Installed modpack {}", index.name));
    Ok(inst)
}
