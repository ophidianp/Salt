//! Skin + cape changer, using the official Minecraft profile API. Changes
//! are real (everyone on every server sees them), unlike cosmetic-only capes.
//!
//! The local skin library (`skins/library.json` + PNGs) lets users keep
//! several skins and swap between them in one click.

use anyhow::{anyhow, bail, Result};
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{fetch_profile, McProfile, MC_PROFILE};
use crate::http::client;
use crate::Paths;

pub async fn profile(token: &str) -> Result<McProfile> {
    fetch_profile(token).await?.ok_or_else(|| anyhow!("No Minecraft profile on this account"))
}

fn check(res: &reqwest::Response, what: &str) -> Result<()> {
    match res.status().as_u16() {
        200..=299 => Ok(()),
        401 => bail!("Your session expired — sign in again to change your {what}."),
        429 => bail!("Too many {what} changes. Wait a minute and try again."),
        s => bail!("Changing your {what} failed (HTTP {s})."),
    }
}

/// Minecraft skins are 64×64 (or legacy 64×32) PNGs.
pub fn validate_png(bytes: &[u8]) -> Result<(u32, u32)> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        bail!("That file isn't a PNG image.");
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if !(w == 64 && (h == 64 || h == 32)) {
        bail!("Skins must be 64×64 or 64×32 pixels (this one is {w}×{h}).");
    }
    Ok((w, h))
}

pub async fn upload_skin(token: &str, png: Vec<u8>, slim: bool) -> Result<McProfile> {
    validate_png(&png)?;
    let part = reqwest::multipart::Part::bytes(png).file_name("skin.png").mime_str("image/png")?;
    let form = reqwest::multipart::Form::new().text("variant", if slim { "slim" } else { "classic" }).part("file", part);
    let res = client().post(format!("{MC_PROFILE}/skins")).bearer_auth(token).multipart(form).send().await?;
    check(&res, "skin")?;
    profile(token).await
}

pub async fn reset_skin(token: &str) -> Result<McProfile> {
    let res = client().delete(format!("{MC_PROFILE}/skins/active")).bearer_auth(token).send().await?;
    check(&res, "skin")?;
    profile(token).await
}

pub async fn set_cape(token: &str, cape_id: Option<&str>) -> Result<McProfile> {
    let res = match cape_id {
        Some(id) => client().put(format!("{MC_PROFILE}/capes/active")).bearer_auth(token).json(&json!({ "capeId": id })).send().await?,
        None => client().delete(format!("{MC_PROFILE}/capes/active")).bearer_auth(token).send().await?,
    };
    check(&res, "cape")?;
    profile(token).await
}

/// Loads a skin/cape texture as a data URL so the 3D preview can read its
/// pixels (textures.minecraft.net doesn't send CORS headers). Only Mojang's
/// texture host is allowed.
pub async fn texture_data_url(url: &str) -> Result<String> {
    let ok = url.starts_with("https://textures.minecraft.net/") || url.starts_with("http://textures.minecraft.net/");
    if !ok {
        bail!("Only textures.minecraft.net textures can be loaded");
    }
    let url = url.replacen("http://", "https://", 1);
    let bytes = client().get(&url).send().await?.error_for_status()?.bytes().await?;
    Ok(format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(&bytes)))
}

/// Any player's current skin texture, from Mojang's public session server
/// (used for friend avatars, so no third-party avatar service sees who your
/// friends are).
pub async fn skin_data_url_for(uuid: &str) -> Result<String> {
    let id: String = uuid.chars().filter(|c| c.is_ascii_hexdigit()).collect();
    if id.len() != 32 {
        bail!("invalid uuid");
    }
    let v: serde_json::Value = client()
        .get(format!("https://sessionserver.mojang.com/session/minecraft/profile/{id}"))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let encoded = v["properties"]
        .as_array()
        .and_then(|p| p.iter().find(|p| p["name"] == "textures"))
        .and_then(|p| p["value"].as_str())
        .ok_or_else(|| anyhow!("no textures"))?;
    let decoded: serde_json::Value = serde_json::from_slice(&base64::engine::general_purpose::STANDARD.decode(encoded)?)?;
    let url = decoded["textures"]["SKIN"]["url"].as_str().ok_or_else(|| anyhow!("no skin"))?;
    texture_data_url(url).await
}

// ---------------------------------------------------------------- library

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LibrarySkin {
    pub id: String,
    pub name: String,
    pub slim: bool,
    pub file: String,
    pub added: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Library {
    pub skins: Vec<LibrarySkin>,
}

impl Library {
    pub fn load(paths: &Paths) -> Self {
        std::fs::read(paths.skins().join("library.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }

    pub fn save(&self, paths: &Paths) -> Result<()> {
        std::fs::create_dir_all(paths.skins())?;
        std::fs::write(paths.skins().join("library.json"), serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LibrarySkinView {
    #[serde(flatten)]
    pub skin: LibrarySkin,
    pub data_url: String,
}

pub fn library_list(paths: &Paths) -> Vec<LibrarySkinView> {
    Library::load(paths)
        .skins
        .into_iter()
        .filter_map(|s| {
            let bytes = std::fs::read(paths.skins().join(&s.file)).ok()?;
            Some(LibrarySkinView { data_url: format!("data:image/png;base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)), skin: s })
        })
        .collect()
}

pub fn library_add(paths: &Paths, name: &str, slim: bool, png: &[u8]) -> Result<LibrarySkin> {
    validate_png(png)?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let file = format!("{id}.png");
    std::fs::create_dir_all(paths.skins())?;
    std::fs::write(paths.skins().join(&file), png)?;
    let skin = LibrarySkin { id, name: name.chars().take(48).collect(), slim, file, added: chrono::Utc::now() };
    let mut lib = Library::load(paths);
    lib.skins.insert(0, skin.clone());
    lib.save(paths)?;
    Ok(skin)
}

pub fn library_remove(paths: &Paths, id: &str) -> Result<()> {
    let mut lib = Library::load(paths);
    if let Some(s) = lib.skins.iter().find(|s| s.id == id) {
        let _ = std::fs::remove_file(paths.skins().join(&s.file));
    }
    lib.skins.retain(|s| s.id != id);
    lib.save(paths)
}

pub fn library_png(paths: &Paths, id: &str) -> Result<(Vec<u8>, bool)> {
    let lib = Library::load(paths);
    let s = lib.skins.iter().find(|s| s.id == id).ok_or_else(|| anyhow!("skin not found"))?;
    Ok((std::fs::read(paths.skins().join(&s.file))?, s.slim))
}

pub fn decode_data_url(data_url: &str) -> Result<Vec<u8>> {
    let b64 = data_url.split_once(",").map(|(_, b)| b).unwrap_or(data_url);
    Ok(base64::engine::general_purpose::STANDARD.decode(b64)?)
}
