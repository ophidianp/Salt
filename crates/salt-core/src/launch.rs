//! Turning an instance into a running game.
//!
//! `prepare` downloads whatever is missing (client jar, libraries, natives,
//! assets, Java, the Salt bundle) and builds the full command line. `spawn`
//! starts it with stdout/stderr piped so the UI can show a live log.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context, Result};
use serde::Serialize;

use crate::auth::Session;
use crate::download::{self, Item};
use crate::instance::Instance;
use crate::meta::{self, Argument, Features, VersionJson};
use crate::paths::maven_path;
use crate::progress::{self, Reporter};
use crate::{java, Paths};

const ASSETS_URL: &str = "https://resources.download.minecraft.net";
const MOJANG_LIBS: &str = "https://libraries.minecraft.net/";

pub struct Prepared {
    pub java: PathBuf,
    pub args: Vec<String>,
    pub game_dir: PathBuf,
    /// The access token, so log lines can be scrubbed if anything prints it.
    pub secret: String,
}

impl Prepared {
    /// The command line with the access token removed, for the launcher log.
    pub fn display_command(&self) -> String {
        let args: Vec<String> = self
            .args
            .iter()
            .map(|a| if a == &self.secret { "<access token hidden>".to_string() } else { a.clone() })
            .collect();
        format!("{} {}", self.java.display(), args.join(" "))
    }
}

pub struct PrepareOptions<'a> {
    pub concurrency: usize,
    pub global_java_path: Option<&'a str>,
    pub extra_jvm_args: &'a [String],
}

pub async fn prepare(paths: &Paths, inst: &Instance, session: &Session, opts: PrepareOptions<'_>, r: &dyn Reporter) -> Result<Prepared> {
    let task = format!("launch-{}", inst.id);
    progress::step(r, &task, "Resolving version", 0, 6);
    let version = meta::resolve(paths, &inst.mc_version, inst.loader, inst.loader_version.as_deref()).await?;
    let features = Features { custom_resolution: inst.resolution.is_some() };

    // 1. client jar
    progress::step(r, &task, "Downloading game", 1, 6);
    let client = version
        .downloads
        .get("client")
        .ok_or_else(|| anyhow!("Version {} has no client download", inst.mc_version))?;
    let client_jar = paths.versions().join(&inst.mc_version).join(format!("{}.jar", inst.mc_version));
    download::fetch_all(
        vec![Item::hashed(&client.url, &client_jar, client.sha1.clone(), client.size)],
        1,
        r,
        &format!("{task}-client"),
        "Minecraft client",
    )
    .await?;

    // 2. libraries + natives
    progress::step(r, &task, "Downloading libraries", 2, 6);
    let (classpath_libs, natives_jars, lib_items) = collect_libraries(paths, &version, &features);
    download::fetch_all(lib_items, opts.concurrency, r, &format!("{task}-libs"), "Libraries").await?;
    let natives_dir = paths.natives(&version.id);
    extract_natives(&natives_jars, &natives_dir)?;

    // 3. assets
    progress::step(r, &task, "Downloading assets", 3, 6);
    let (assets_root, asset_index_id, game_assets) = download_assets(paths, &version, opts.concurrency, r, &task).await?;
    let log_config = download_log_config(paths, &version).await.ok().flatten();

    // 4. java
    progress::step(r, &task, "Preparing Java", 4, 6);
    let java = match inst.java_path.as_deref().or(opts.global_java_path).filter(|p| !p.trim().is_empty()) {
        Some(p) => PathBuf::from(p),
        None => java::ensure(paths, java::required_major(version.java_version.as_ref()), r).await?,
    };

    // 5. arguments
    progress::step(r, &task, "Building arguments", 5, 6);
    let game_dir = inst.game_dir(paths);
    std::fs::create_dir_all(&game_dir)?;
    let sep = if cfg!(windows) { ";" } else { ":" };
    let mut cp: Vec<String> = classpath_libs.iter().map(|p| p.display().to_string()).collect();
    cp.push(client_jar.display().to_string());

    let (width, height) = inst.resolution.unwrap_or((854, 480));
    let vars: HashMap<&str, String> = HashMap::from([
        ("auth_player_name", session.account.username.clone()),
        ("version_name", inst.mc_version.clone()),
        ("game_directory", game_dir.display().to_string()),
        ("assets_root", assets_root.display().to_string()),
        ("game_assets", game_assets.display().to_string()),
        ("assets_index_name", asset_index_id.clone()),
        ("auth_uuid", session.account.uuid.clone()),
        ("auth_access_token", session.access_token.clone()),
        ("auth_session", session.access_token.clone()),
        ("auth_xuid", session.account.xuid.clone().unwrap_or_default()),
        ("clientid", String::new()),
        ("user_type", "msa".into()),
        ("user_properties", "{}".into()),
        ("version_type", "Salt".into()),
        ("natives_directory", natives_dir.display().to_string()),
        ("launcher_name", crate::LAUNCHER_NAME.into()),
        ("launcher_version", crate::LAUNCHER_VERSION.into()),
        ("classpath", cp.join(sep)),
        ("classpath_separator", sep.into()),
        ("library_directory", paths.libraries().display().to_string()),
        ("resolution_width", width.to_string()),
        ("resolution_height", height.to_string()),
    ]);
    let sub = |s: &str| substitute(s, &vars);

    let mut args = vec![format!("-Xmx{}M", inst.memory_mb), format!("-Xms{}M", (inst.memory_mb / 4).max(512))];
    if java::required_major(version.java_version.as_ref()) <= 8 {
        args.extend(["-XX:+UseG1GC".to_string(), "-XX:+UnlockExperimentalVMOptions".to_string()]);
    }
    args.extend(opts.extra_jvm_args.iter().cloned());
    args.extend(inst.jvm_args.iter().cloned());

    match &version.arguments {
        Some(a) if !a.jvm.is_empty() => args.extend(expand(&a.jvm, &features).iter().map(|s| sub(s))),
        _ => {
            args.push(sub("-Djava.library.path=${natives_directory}"));
            args.push(sub("-Dminecraft.launcher.brand=${launcher_name}"));
            args.push(sub("-Dminecraft.launcher.version=${launcher_version}"));
            args.push("-cp".into());
            args.push(sub("${classpath}"));
        }
    }
    if let Some((arg, path)) = &log_config {
        args.push(arg.replace("${path}", &path.display().to_string()));
    }
    args.push(version.main_class.clone().ok_or_else(|| anyhow!("version has no main class"))?);
    match (&version.arguments, &version.minecraft_arguments) {
        (_, Some(legacy)) => args.extend(legacy.split_whitespace().map(|s| sub(s))),
        (Some(a), None) => args.extend(expand(&a.game, &features).iter().map(|s| sub(s))),
        (None, None) => {}
    }
    if let Some((w, h)) = inst.resolution {
        if version.minecraft_arguments.is_some() {
            args.extend(["--width".into(), w.to_string(), "--height".into(), h.to_string()]);
        }
    }
    args.extend(inst.game_args.iter().cloned());

    progress::finish(r, &task, "Ready");
    Ok(Prepared { java, args, game_dir, secret: session.access_token.clone() })
}

fn substitute(s: &str, vars: &HashMap<&str, String>) -> String {
    let mut out = s.to_string();
    for (k, v) in vars {
        let key = format!("${{{k}}}");
        if out.contains(&key) {
            out = out.replace(&key, v);
        }
    }
    out
}

fn expand(list: &[Argument], features: &Features) -> Vec<String> {
    let mut out = Vec::new();
    for a in list {
        match a {
            Argument::Plain(s) => out.push(s.clone()),
            Argument::Ruled { rules, value } => {
                if meta::rules_allow(rules, features) {
                    out.extend(value.to_vec());
                }
            }
        }
    }
    out
}

fn collect_libraries(paths: &Paths, v: &VersionJson, features: &Features) -> (Vec<PathBuf>, Vec<PathBuf>, Vec<Item>) {
    let mut classpath = Vec::new();
    let mut natives = Vec::new();
    let mut items = Vec::new();
    for lib in &v.libraries {
        if let Some(rules) = &lib.rules {
            if !meta::rules_allow(rules, features) {
                continue;
            }
        }
        // Old-style natives (1.8.9): a classifier jar that gets unzipped.
        if let Some(map) = &lib.natives {
            if let Some(classifier) = map.get(meta::os_name()) {
                let classifier = classifier.replace("${arch}", if cfg!(target_pointer_width = "64") { "64" } else { "32" });
                if let Some(dl) = lib.downloads.as_ref().and_then(|d| d.classifiers.as_ref()).and_then(|c| c.get(&classifier)) {
                    let rel = dl.path.clone().or_else(|| maven_path(&format!("{}:{}", lib.name, classifier)));
                    if let Some(rel) = rel {
                        let p = paths.library_path(&rel);
                        items.push(Item::hashed(&dl.url, &p, dl.sha1.clone(), dl.size));
                        natives.push(p);
                    }
                } else if let Some(rel) = maven_path(&format!("{}:{}", lib.name, classifier)) {
                    // Maven-style natives with no download block (Legacy
                    // Fabric's own LWJGL 2 build).
                    let base = lib.url.clone().unwrap_or_else(|| MOJANG_LIBS.to_string());
                    let base = if base.ends_with('/') { base } else { format!("{base}/") };
                    let p = paths.library_path(&rel);
                    items.push(Item::hashed(format!("{base}{rel}"), &p, None, None));
                    natives.push(p);
                }
            }
            // A natives-only library has no main artifact.
            if lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()).is_none() {
                continue;
            }
        }
        match lib.downloads.as_ref().and_then(|d| d.artifact.as_ref()) {
            Some(art) => {
                let Some(rel) = art.path.clone().or_else(|| maven_path(&lib.name)) else { continue };
                let p = paths.library_path(&rel);
                if !art.url.is_empty() {
                    items.push(Item::hashed(&art.url, &p, art.sha1.clone(), art.size));
                }
                classpath.push(p);
            }
            None => {
                // Fabric-style: Maven coordinate + repository URL.
                let Some(rel) = maven_path(&lib.name) else { continue };
                let base = lib.url.clone().unwrap_or_else(|| MOJANG_LIBS.to_string());
                let base = if base.ends_with('/') { base } else { format!("{base}/") };
                let p = paths.library_path(&rel);
                items.push(Item::hashed(format!("{base}{rel}"), &p, lib.sha1.clone(), lib.size));
                classpath.push(p);
            }
        }
    }
    (classpath, natives, items)
}

fn extract_natives(jars: &[PathBuf], dir: &Path) -> Result<()> {
    if jars.is_empty() {
        return Ok(());
    }
    // The marker lists the jars that were extracted, so a changed library
    // set (e.g. a fixed natives entry) extracts again.
    let marker = dir.join(".extracted");
    let stamp = jars.iter().map(|j| j.display().to_string()).collect::<Vec<_>>().join("
");
    if std::fs::read_to_string(&marker).is_ok_and(|m| m == stamp) {
        return Ok(());
    }
    std::fs::create_dir_all(dir)?;
    for jar in jars {
        let f = std::fs::File::open(jar).with_context(|| format!("missing natives {}", jar.display()))?;
        let mut zip = zip::ZipArchive::new(f)?;
        for i in 0..zip.len() {
            let mut entry = zip.by_index(i)?;
            let Some(name) = entry.enclosed_name() else { continue };
            if name.starts_with("META-INF") || entry.is_dir() {
                continue;
            }
            let out = dir.join(name);
            if let Some(parent) = out.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut f = std::fs::File::create(&out)?;
            std::io::copy(&mut entry, &mut f)?;
        }
    }
    std::fs::write(marker, stamp)?;
    Ok(())
}

async fn download_assets(paths: &Paths, v: &VersionJson, concurrency: usize, r: &dyn Reporter, task: &str) -> Result<(PathBuf, String, PathBuf)> {
    let idx = v.asset_index.as_ref().ok_or_else(|| anyhow!("version has no asset index"))?;
    let root = paths.assets();
    let index_path = root.join("indexes").join(format!("{}.json", idx.id));
    download::fetch_all(vec![Item::hashed(&idx.url, &index_path, Some(idx.sha1.clone()), None)], 1, r, &format!("{task}-index"), "Asset index").await?;
    let index: serde_json::Value = serde_json::from_slice(&tokio::fs::read(&index_path).await?)?;
    let is_virtual = index["virtual"].as_bool().unwrap_or(false) || index["map_to_resources"].as_bool().unwrap_or(false);
    let mut items = Vec::new();
    let mut named = Vec::new();
    if let Some(objects) = index["objects"].as_object() {
        for (name, obj) in objects {
            let Some(hash) = obj["hash"].as_str() else { continue };
            let path = root.join("objects").join(&hash[..2]).join(hash);
            items.push(Item::hashed(format!("{ASSETS_URL}/{}/{hash}", &hash[..2]), &path, Some(hash.to_string()), obj["size"].as_u64()));
            named.push((name.clone(), path));
        }
    }
    download::fetch_all(items, concurrency, r, &format!("{task}-assets"), "Assets").await?;

    let virtual_dir = root.join("virtual").join(&idx.id);
    if is_virtual {
        for (name, src) in named {
            let Some(dest) = crate::paths::safe_join(&virtual_dir, &name) else { continue };
            if !dest.exists() {
                if let Some(parent) = dest.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                std::fs::copy(src, dest)?;
            }
        }
    }
    Ok((root, idx.id.clone(), if is_virtual { virtual_dir } else { paths.assets() }))
}

/// Mojang's log4j config. Besides giving structured (XML) log events, on
/// 1.8.9 this file is also the official Log4Shell mitigation, so it is
/// always used.
async fn download_log_config(paths: &Paths, v: &VersionJson) -> Result<Option<(String, PathBuf)>> {
    let Some(client) = v.logging.as_ref().and_then(|l| l.get("client")) else { return Ok(None) };
    let (Some(arg), Some(file)) = (client["argument"].as_str(), client.get("file")) else { return Ok(None) };
    let (Some(id), Some(url)) = (file["id"].as_str(), file["url"].as_str()) else { return Ok(None) };
    let path = paths.assets().join("log_configs").join(id);
    download::fetch(&Item::hashed(url, &path, file["sha1"].as_str().map(str::to_string), file["size"].as_u64())).await?;
    Ok(Some((arg.to_string(), path)))
}

pub fn spawn(p: &Prepared) -> Result<tokio::process::Child> {
    let mut cmd = tokio::process::Command::new(&p.java);
    cmd.args(&p.args)
        .current_dir(&p.game_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(false);
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd.spawn().with_context(|| format!("Couldn't start Java at {}", p.java.display()))
}

// --------------------------------------------------------------- log parsing

#[derive(Debug, Clone, Serialize)]
pub struct LogLine {
    pub level: String,
    pub thread: String,
    pub time: Option<i64>,
    pub message: String,
}

/// Turns Minecraft's log4j XML events (and plain stdout lines) into
/// structured lines for the live log viewer.
#[derive(Default)]
pub struct LogParser {
    in_event: bool,
    level: String,
    thread: String,
    time: Option<i64>,
    buf: String,
    secret: String,
}

impl LogParser {
    pub fn new(secret: &str) -> Self {
        Self { secret: secret.to_string(), ..Default::default() }
    }

    fn scrub(&self, s: &str) -> String {
        if self.secret.len() > 8 && s.contains(&self.secret) {
            s.replace(&self.secret, "<access token hidden>")
        } else {
            s.to_string()
        }
    }

    pub fn feed(&mut self, line: &str) -> Option<LogLine> {
        let trimmed = line.trim();
        if !self.in_event {
            if let Some(start) = trimmed.find("<log4j:Event") {
                let head = &trimmed[start..];
                self.in_event = true;
                self.level = attr(head, "level").unwrap_or_else(|| "INFO".into());
                self.thread = attr(head, "thread").unwrap_or_default();
                self.time = attr(head, "timestamp").and_then(|t| t.parse().ok());
                self.buf.clear();
                if head.contains("</log4j:Event>") {
                    self.buf.push_str(head);
                    return self.flush();
                }
                return None;
            }
            if trimmed.is_empty() {
                return None;
            }
            // Plain line (crash output, a mod printing to stdout…).
            let level = if trimmed.contains("ERROR") || trimmed.starts_with("Exception") || trimmed.starts_with("at ") || trimmed.starts_with("Caused by") {
                "ERROR"
            } else if trimmed.contains("WARN") {
                "WARN"
            } else {
                "INFO"
            };
            return Some(LogLine { level: level.into(), thread: String::new(), time: None, message: self.scrub(line.trim_end()) });
        }
        self.buf.push_str(line);
        self.buf.push('\n');
        if trimmed.contains("</log4j:Event>") {
            return self.flush();
        }
        None
    }

    fn flush(&mut self) -> Option<LogLine> {
        self.in_event = false;
        let mut message = String::new();
        let mut rest = self.buf.as_str();
        while let Some(s) = rest.find("<![CDATA[") {
            let after = &rest[s + 9..];
            let e = after.find("]]>").unwrap_or(after.len());
            if !message.is_empty() {
                message.push('\n');
            }
            message.push_str(after[..e].trim_end());
            rest = &after[e.min(after.len())..];
        }
        Some(LogLine { level: self.level.clone(), thread: self.thread.clone(), time: self.time, message: self.scrub(&message) })
    }
}

fn attr(s: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = s.find(&key)? + key.len();
    let end = s[start..].find('"')? + start;
    Some(s[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_log4j_event() {
        let mut p = LogParser::new("secret-token-123");
        assert!(p.feed(r#"<log4j:Event logger="net.minecraft.client.Minecraft" timestamp="1700000000000" level="INFO" thread="Render thread">"#).is_none());
        assert!(p.feed("  <log4j:Message><![CDATA[Setting user: Salt]]></log4j:Message>").is_none());
        let line = p.feed("</log4j:Event>").unwrap();
        assert_eq!(line.level, "INFO");
        assert_eq!(line.thread, "Render thread");
        assert_eq!(line.message, "Setting user: Salt");
    }

    #[test]
    fn scrubs_token_from_plain_lines() {
        let mut p = LogParser::new("secret-token-123");
        let line = p.feed("token is secret-token-123").unwrap();
        assert!(!line.message.contains("secret-token-123"));
    }
}
