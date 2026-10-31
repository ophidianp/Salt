//! Java runtimes. Each Minecraft version declares the Java major it needs
//! (8 for 1.8.9, 21 for 1.21.x, 25 for 26.x); Salt downloads an Eclipse
//! Temurin runtime from Adoptium for each one on first use and keeps it in
//! `meta/java/<major>/`.

use std::path::{Path, PathBuf};

use anyhow::{anyhow, bail, Context, Result};
use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::http::{client, get_json};
use crate::progress::{self, Reporter};
use crate::Paths;

#[derive(Deserialize)]
struct AdoptiumRelease {
    binary: AdoptiumBinary,
    release_name: String,
}

#[derive(Deserialize)]
struct AdoptiumBinary {
    package: AdoptiumPackage,
}

#[derive(Deserialize)]
struct AdoptiumPackage {
    link: String,
    checksum: String,
    name: String,
}

fn exe_name() -> &'static str {
    // javaw has no console window, so the game doesn't pop a black box.
    if cfg!(windows) { "javaw.exe" } else { "java" }
}

fn adoptium_os() -> &'static str {
    match std::env::consts::OS {
        "macos" => "mac",
        other => other,
    }
}

fn adoptium_arch() -> &'static str {
    match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        _ => "x64",
    }
}

/// Searches a folder (recursively, a few levels) for `bin/javaw.exe`.
fn find_java_in(dir: &Path) -> Option<PathBuf> {
    let direct = dir.join("bin").join(exe_name());
    if direct.exists() {
        return Some(direct);
    }
    for entry in std::fs::read_dir(dir).ok()?.flatten() {
        let p = entry.path();
        if p.is_dir() {
            let candidate = p.join("bin").join(exe_name());
            if candidate.exists() {
                return Some(candidate);
            }
            // macOS bundles: <jdk>/Contents/Home/bin/java
            let mac = p.join("Contents").join("Home").join("bin").join(exe_name());
            if mac.exists() {
                return Some(mac);
            }
        }
    }
    None
}

pub fn installed(paths: &Paths, major: u32) -> Option<PathBuf> {
    find_java_in(&paths.java().join(major.to_string()))
}

pub async fn ensure(paths: &Paths, major: u32, reporter: &dyn Reporter) -> Result<PathBuf> {
    if let Some(p) = installed(paths, major) {
        return Ok(p);
    }
    install(paths, major, reporter).await
}

pub async fn install(paths: &Paths, major: u32, reporter: &dyn Reporter) -> Result<PathBuf> {
    let task = format!("java-{major}");
    reporter.log("info", &format!("Installing Java {major} (Eclipse Temurin)…"));
    let mut release = None;
    for image in ["jre", "jdk"] {
        let url = format!(
            "https://api.adoptium.net/v3/assets/latest/{major}/hotspot?architecture={}&image_type={image}&os={}&vendor=eclipse",
            adoptium_arch(),
            adoptium_os()
        );
        if let Ok(list) = get_json::<Vec<AdoptiumRelease>>(&url).await {
            if let Some(r) = list.into_iter().next() {
                release = Some(r);
                break;
            }
        }
    }
    let release = release.ok_or_else(|| anyhow!("Adoptium has no Java {major} build for this system"))?;
    if !release.binary.package.name.ends_with(".zip") {
        bail!("Automatic Java install only supports .zip packages here. Set a Java path in Settings → Java.");
    }

    progress::step(reporter, &task, &format!("Downloading Java {major}"), 0, 2);
    let bytes = client()
        .get(&release.binary.package.link)
        .send()
        .await?
        .error_for_status()?
        .bytes()
        .await?;
    let got = hex::encode(Sha256::digest(&bytes));
    if !got.eq_ignore_ascii_case(&release.binary.package.checksum) {
        bail!("Java download was corrupted (checksum mismatch). Try again.");
    }

    progress::step(reporter, &task, &format!("Extracting Java {major}"), 1, 2);
    let target = paths.java().join(major.to_string());
    let target2 = target.clone();
    tokio::task::spawn_blocking(move || extract_zip(&bytes, &target2)).await??;
    progress::finish(reporter, &task, &format!("Java {major} ready"));
    reporter.log("info", &format!("Installed {}", release.release_name));

    installed(paths, major).ok_or_else(|| anyhow!("Java {major} was extracted but no java executable was found"))
}

pub fn extract_zip(bytes: &[u8], target: &Path) -> Result<()> {
    let _ = std::fs::remove_dir_all(target);
    std::fs::create_dir_all(target)?;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).context("not a valid zip")?;
    for i in 0..zip.len() {
        let mut file = zip.by_index(i)?;
        let Some(out) = file.enclosed_name().map(|p| target.join(p)) else { continue };
        if file.is_dir() {
            std::fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut f = std::fs::File::create(&out)?;
            std::io::copy(&mut file, &mut f)?;
        }
    }
    Ok(())
}

/// Picks the Java major version a game version needs.
pub fn required_major(java_version: Option<&crate::meta::JavaVersion>) -> u32 {
    java_version.map(|j| j.major_version).unwrap_or(8)
}
