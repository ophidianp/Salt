//! Parallel, hash-verified downloads.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use futures::StreamExt;
use sha1::{Digest, Sha1};

use crate::http::client;
use crate::progress::{self, Reporter};

#[derive(Debug, Clone)]
pub struct Item {
    pub url: String,
    pub path: PathBuf,
    pub sha1: Option<String>,
    pub size: Option<u64>,
}

impl Item {
    pub fn new(url: impl Into<String>, path: impl Into<PathBuf>) -> Self {
        Self { url: url.into(), path: path.into(), sha1: None, size: None }
    }

    pub fn hashed(url: impl Into<String>, path: impl Into<PathBuf>, sha1: Option<String>, size: Option<u64>) -> Self {
        Self { url: url.into(), path: path.into(), sha1, size }
    }

    /// Cheap "already have it" check. A size match is trusted on launch;
    /// the full SHA-1 is checked whenever a file is actually downloaded.
    fn is_present(&self) -> bool {
        match std::fs::metadata(&self.path) {
            Ok(m) => match self.size {
                Some(s) => m.len() == s,
                None => m.len() > 0,
            },
            Err(_) => false,
        }
    }
}

pub fn sha1_file(path: &Path) -> Result<String> {
    let mut f = std::fs::File::open(path)?;
    let mut h = Sha1::new();
    std::io::copy(&mut f, &mut h)?;
    Ok(hex::encode(h.finalize()))
}

async fn fetch_owned(item: Item) -> Result<()> {
    fetch(&item).await
}

pub async fn fetch(item: &Item) -> Result<()> {
    let mut last_err = None;
    for attempt in 0..3 {
        match fetch_once(item).await {
            Ok(()) => return Ok(()),
            Err(e) => {
                last_err = Some(e);
                tokio::time::sleep(std::time::Duration::from_millis(400 * (attempt + 1))).await;
            }
        }
    }
    Err(last_err.unwrap()).with_context(|| format!("Download failed: {}", item.url))
}

async fn fetch_once(item: &Item) -> Result<()> {
    let res = client().get(&item.url).send().await?;
    if !res.status().is_success() {
        bail!("HTTP {}", res.status());
    }
    let bytes = res.bytes().await?;
    if let Some(expected) = &item.sha1 {
        let got = hex::encode(Sha1::digest(&bytes));
        if !got.eq_ignore_ascii_case(expected) {
            bail!("hash mismatch (expected {expected}, got {got})");
        }
    }
    if let Some(parent) = item.path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    // Write to a temp file first so a crash never leaves a half-written jar
    // that looks complete.
    let tmp = item.path.with_extension("part");
    tokio::fs::write(&tmp, &bytes).await?;
    tokio::fs::rename(&tmp, &item.path).await?;
    Ok(())
}

/// Downloads everything that's missing, `concurrency` at a time, reporting
/// progress under `task`.
pub async fn fetch_all(items: Vec<Item>, concurrency: usize, reporter: &dyn Reporter, task: &str, label: &str) -> Result<()> {
    let missing: Vec<Item> = items.into_iter().filter(|i| !i.is_present()).collect();
    let total = missing.len() as u64;
    if total == 0 {
        return Ok(());
    }
    progress::step(reporter, task, label, 0, total);
    // Each download is an owned future (no closure borrowing the reporter),
    // which keeps the whole thing `Send` for Tauri's async commands.
    let mut stream = futures::stream::iter(missing.into_iter().map(fetch_owned)).buffer_unordered(concurrency.max(1));
    let mut done = 0u64;
    let mut errors = Vec::new();
    while let Some(r) = stream.next().await {
        done += 1;
        if let Err(e) = r {
            errors.push(format!("{e:#}"));
        }
        if done % 8 == 0 || done == total {
            progress::step(reporter, task, label, done, total);
        }
    }
    if !errors.is_empty() {
        for e in errors.iter().take(5) {
            reporter.log("error", e);
        }
        bail!("{} file(s) failed to download. Check your connection and try again.", errors.len());
    }
    Ok(())
}
