//! CurseForge Core API (https://docs.curseforge.com/rest-api/). Requires an
//! API key from https://console.curseforge.com — the user pastes theirs into
//! Settings → Integrations. It is sent only to api.curseforge.com.
//!
//! CurseForge lets authors block third-party downloads; for those files
//! `downloadUrl` is null and Salt shows a "download on the website" link
//! instead of working around the author's choice.

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::download::{self, Item};
use crate::http::client;
use crate::instance::{self, Content, ContentEntry, Instance, NewInstance, PackOrigin, Source};
use crate::meta::Loader;
use crate::paths::safe_join;
use crate::progress::{self, Reporter};
use crate::Paths;

const API: &str = "https://api.curseforge.com/v1";
const MINECRAFT: u32 = 432;
const FABRIC: u32 = 4;

pub fn class_id(kind: &str) -> u32 {
    match kind {
        "modpack" => 4471,
        "resourcepack" => 12,
        "shader" => 6552,
        _ => 6,
    }
}

fn class_dir(class: u32) -> &'static str {
    match class {
        12 => "resourcepacks",
        6552 => "shaderpacks",
        _ => "mods",
    }
}

async fn get<T: serde::de::DeserializeOwned>(key: &str, url: &str) -> Result<T> {
    if key.trim().is_empty() {
        bail!("Add your CurseForge API key in Settings → Integrations to use CurseForge.");
    }
    let res = client().get(url).header("x-api-key", key).header("Accept", "application/json").send().await?;
    if res.status().as_u16() == 403 {
        bail!("CurseForge rejected the API key. Check it in Settings → Integrations.");
    }
    if !res.status().is_success() {
        bail!("CurseForge request failed (HTTP {})", res.status());
    }
    Ok(res.json().await?)
}

async fn post<T: serde::de::DeserializeOwned>(key: &str, url: &str, body: serde_json::Value) -> Result<T> {
    let res = client().post(url).header("x-api-key", key).json(&body).send().await?;
    if !res.status().is_success() {
        bail!("CurseForge request failed (HTTP {})", res.status());
    }
    Ok(res.json().await?)
}

#[derive(Debug, Deserialize)]
struct Wrap<T> {
    data: T,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfMod {
    pub id: u64,
    pub name: String,
    pub slug: String,
    pub summary: String,
    pub download_count: f64,
    #[serde(default)]
    pub logo: Option<CfLogo>,
    #[serde(default)]
    pub authors: Vec<CfAuthor>,
    #[serde(default)]
    pub class_id: Option<u32>,
    #[serde(default)]
    pub links: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfLogo {
    pub thumbnail_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfAuthor {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfFile {
    pub id: u64,
    pub mod_id: u64,
    pub display_name: String,
    pub file_name: String,
    #[serde(default)]
    pub download_url: Option<String>,
    #[serde(default)]
    pub hashes: Vec<CfHash>,
    #[serde(default)]
    pub game_versions: Vec<String>,
    #[serde(default)]
    pub file_length: u64,
    #[serde(default)]
    pub dependencies: Vec<CfDependency>,
    pub file_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CfHash {
    pub value: String,
    pub algo: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CfDependency {
    pub mod_id: u64,
    /// 3 = required
    pub relation_type: u32,
}

impl CfFile {
    fn sha1(&self) -> Option<String> {
        self.hashes.iter().find(|h| h.algo == 1).map(|h| h.value.clone())
    }
}

/// Shaped like a Modrinth hit so the UI can render both with one card.
#[derive(Debug, Clone, Serialize)]
pub struct Hit {
    pub project_id: String,
    pub slug: String,
    pub title: String,
    pub description: String,
    pub author: String,
    pub icon_url: Option<String>,
    pub downloads: u64,
    pub project_type: String,
}

pub async fn search(key: &str, query: &str, kind: &str, game_version: Option<&str>, offset: u32, limit: u32) -> Result<Vec<Hit>> {
    let mut url = format!(
        "{API}/mods/search?gameId={MINECRAFT}&classId={}&searchFilter={}&sortField=2&sortOrder=desc&index={offset}&pageSize={}",
        class_id(kind),
        urlencoding::encode(query),
        limit.min(50)
    );
    if let Some(v) = game_version {
        url.push_str(&format!("&gameVersion={}", urlencoding::encode(v)));
    }
    if kind == "mod" || kind == "modpack" {
        url.push_str(&format!("&modLoaderType={FABRIC}"));
    }
    let res: Wrap<Vec<CfMod>> = get(key, &url).await.context("CurseForge search failed")?;
    Ok(res
        .data
        .into_iter()
        .map(|m| Hit {
            project_id: m.id.to_string(),
            slug: m.slug,
            title: m.name,
            description: m.summary,
            author: m.authors.first().map(|a| a.name.clone()).unwrap_or_default(),
            icon_url: m.logo.map(|l| l.thumbnail_url),
            downloads: m.download_count as u64,
            project_type: kind.to_string(),
        })
        .collect())
}

pub async fn get_mod(key: &str, id: u64) -> Result<CfMod> {
    let res: Wrap<CfMod> = get(key, &format!("{API}/mods/{id}")).await?;
    Ok(res.data)
}

pub async fn files(key: &str, mod_id: u64, game_version: Option<&str>, fabric: bool) -> Result<Vec<CfFile>> {
    let mut url = format!("{API}/mods/{mod_id}/files?pageSize=50");
    if let Some(v) = game_version {
        url.push_str(&format!("&gameVersion={}", urlencoding::encode(v)));
    }
    if fabric {
        url.push_str(&format!("&modLoaderType={FABRIC}"));
    }
    let res: Wrap<Vec<CfFile>> = get(key, &url).await?;
    Ok(res.data)
}

fn website_hint(m: &CfMod) -> String {
    m.links
        .as_ref()
        .and_then(|l| l["websiteUrl"].as_str())
        .map(|u| format!(" Download it manually from {u} and add it with \"Add file\"."))
        .unwrap_or_default()
}

pub async fn install_mod(paths: &Paths, key: &str, inst: &Instance, mod_id: u64, file_id: Option<u64>, r: &dyn Reporter) -> Result<Vec<String>> {
    let mut content = Content::load(paths, &inst.id);
    let mut installed = Vec::new();
    let mut queue = vec![(mod_id, file_id)];
    let mut seen = std::collections::HashSet::new();
    while let Some((mid, fid)) = queue.pop() {
        if !seen.insert(mid) {
            continue;
        }
        let pid = mid.to_string();
        if mid != mod_id && content.entries.iter().any(|e| e.project_id.as_deref() == Some(pid.as_str())) {
            continue;
        }
        let m = get_mod(key, mid).await?;
        let file = match fid {
            Some(fid) => get::<Wrap<CfFile>>(key, &format!("{API}/mods/{mid}/files/{fid}")).await?.data,
            None => files(key, mid, Some(&inst.mc_version), inst.loader == Loader::Fabric && m.class_id == Some(6))
                .await?
                .into_iter()
                .next()
                .ok_or_else(|| anyhow!("{} has no file for Minecraft {}", m.name, inst.mc_version))?,
        };
        let url = file
            .download_url
            .clone()
            .ok_or_else(|| anyhow!("{}'s author doesn't allow downloads from launchers.{}", m.name, website_hint(&m)))?;
        let dir = inst.game_dir(paths).join(class_dir(m.class_id.unwrap_or(6)));
        let dest = safe_join(&dir, &file.file_name).ok_or_else(|| anyhow!("bad file name"))?;
        download::fetch(&Item::hashed(url, &dest, file.sha1(), Some(file.file_length))).await?;
        content.upsert(ContentEntry {
            file_name: file.file_name.clone(),
            source: Source::Curseforge,
            project_id: Some(pid),
            version_id: Some(file.id.to_string()),
            title: Some(m.name.clone()),
            version: Some(file.display_name.clone()),
            icon_url: m.logo.as_ref().map(|l| l.thumbnail_url.clone()),
            kind: match m.class_id { Some(12) => "resourcepack", Some(6552) => "shader", _ => "mod" }.into(),
        });
        r.log("info", &format!("Installed {} ({})", m.name, file.display_name));
        installed.push(m.name.clone());
        for d in file.dependencies.iter().filter(|d| d.relation_type == 3) {
            queue.push((d.mod_id, None));
        }
    }
    content.save(paths, &inst.id)?;
    Ok(installed)
}

// ------------------------------------------------------------------ modpacks

#[derive(Deserialize)]
struct Manifest {
    name: String,
    #[serde(default)]
    version: String,
    minecraft: ManifestMc,
    files: Vec<ManifestFile>,
    #[serde(default = "default_overrides")]
    overrides: String,
}

fn default_overrides() -> String { "overrides".into() }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestMc {
    version: String,
    #[serde(default)]
    mod_loaders: Vec<ManifestLoader>,
}

#[derive(Deserialize)]
struct ManifestLoader {
    id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ManifestFile {
    #[serde(rename = "projectID")]
    #[allow(dead_code)]
    project_id: u64,
    #[serde(rename = "fileID")]
    file_id: u64,
    #[serde(default = "yes")]
    required: bool,
}

fn yes() -> bool { true }

pub async fn install_modpack(paths: &Paths, key: &str, mod_id: u64, file_id: Option<u64>, r: &dyn Reporter) -> Result<Instance> {
    let m = get_mod(key, mod_id).await?;
    let file = match file_id {
        Some(fid) => get::<Wrap<CfFile>>(key, &format!("{API}/mods/{mod_id}/files/{fid}")).await?.data,
        None => files(key, mod_id, None, false).await?.into_iter().next().ok_or_else(|| anyhow!("modpack has no files"))?,
    };
    let url = file.download_url.clone().ok_or_else(|| anyhow!("This modpack's author doesn't allow launcher downloads.{}", website_hint(&m)))?;
    let bytes = client().get(&url).send().await?.error_for_status()?.bytes().await?;
    let mut inst = install_zip(paths, key, &bytes, m.logo.as_ref().map(|l| l.thumbnail_url.clone()), r).await?;
    inst.pack = Some(PackOrigin { platform: "curseforge".into(), project_id: mod_id.to_string(), version_id: file.id.to_string(), title: m.name });
    instance::save(paths, &inst)?;
    Ok(inst)
}

pub async fn install_zip(paths: &Paths, key: &str, bytes: &[u8], icon: Option<String>, r: &dyn Reporter) -> Result<Instance> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("not a valid CurseForge modpack zip")?;
    let manifest: Manifest = {
        let mut f = zip.by_name("manifest.json").context("modpack is missing manifest.json")?;
        let mut s = String::new();
        std::io::Read::read_to_string(&mut f, &mut s)?;
        serde_json::from_str(&s)?
    };
    let loader_id = manifest.minecraft.mod_loaders.first().map(|l| l.id.clone()).unwrap_or_default();
    let (loader, loader_version) = if let Some(v) = loader_id.strip_prefix("fabric-") {
        (Loader::Fabric, Some(v.to_string()))
    } else if loader_id.is_empty() {
        (Loader::Vanilla, None)
    } else {
        bail!("This modpack uses {loader_id}. Salt currently supports Fabric and vanilla packs.");
    };
    let inst = instance::create(
        paths,
        NewInstance {
            name: format!("{} {}", manifest.name, manifest.version).trim().to_string(),
            mc_version: manifest.minecraft.version.clone(),
            loader,
            loader_version,
            icon,
            group: Some("Modpacks".into()),
            memory_mb: Some(6144),
            salt_bundle: false,
        },
    )?;
    let game = inst.game_dir(paths);

    let ids: Vec<u64> = manifest.files.iter().filter(|f| f.required).map(|f| f.file_id).collect();
    let files: Wrap<Vec<CfFile>> = post(key, &format!("{API}/mods/files"), json!({ "fileIds": ids })).await?;
    let mod_ids: Vec<u64> = files.data.iter().map(|f| f.mod_id).collect();
    let mods: Wrap<Vec<CfMod>> = post(key, &format!("{API}/mods"), json!({ "modIds": mod_ids })).await?;

    let mut items = Vec::new();
    let mut blocked = Vec::new();
    for f in &files.data {
        let class = mods.data.iter().find(|m| m.id == f.mod_id).and_then(|m| m.class_id).unwrap_or(6);
        let Some(dest) = safe_join(&game.join(class_dir(class)), &f.file_name) else { continue };
        match &f.download_url {
            Some(u) => items.push(Item::hashed(u.clone(), dest, f.sha1(), Some(f.file_length))),
            None => blocked.push(f.display_name.clone()),
        }
    }
    download::fetch_all(items, 16, r, &format!("pack-{}", inst.id), "Modpack files").await?;

    let prefix = format!("{}/", manifest.overrides.trim_end_matches('/'));
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let name = entry.name().to_string();
        let Some(rel) = name.strip_prefix(&prefix) else { continue };
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
    if !blocked.is_empty() {
        r.log("warn", &format!("{} file(s) must be downloaded manually from CurseForge: {}", blocked.len(), blocked.join(", ")));
    }
    progress::finish(r, &format!("pack-{}", inst.id), "Modpack installed");
    Ok(inst)
}
