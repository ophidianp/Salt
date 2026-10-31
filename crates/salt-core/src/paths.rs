use std::path::{Path, PathBuf};

/// Every folder the launcher uses, rooted at one data directory
/// (`%APPDATA%\SaltLauncher` on Windows). Shared game files (libraries,
/// assets, Java) are stored once and reused by every instance, the same way
/// OneClient keeps a shared `metadata/` folder next to its clusters.
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self { root: root.into() }
    }

    pub fn default_root() -> PathBuf {
        dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("SaltLauncher")
    }

    pub fn ensure(&self) -> std::io::Result<()> {
        for dir in [
            self.instances(),
            self.libraries(),
            self.assets().join("indexes"),
            self.assets().join("objects"),
            self.versions(),
            self.java(),
            self.cache(),
            self.logs(),
            self.skins(),
            self.salt_mods(),
        ] {
            std::fs::create_dir_all(dir)?;
        }
        Ok(())
    }

    pub fn instances(&self) -> PathBuf { self.root.join("instances") }
    pub fn instance(&self, id: &str) -> PathBuf { self.instances().join(id) }
    pub fn meta(&self) -> PathBuf { self.root.join("meta") }
    pub fn libraries(&self) -> PathBuf { self.meta().join("libraries") }
    pub fn assets(&self) -> PathBuf { self.meta().join("assets") }
    pub fn versions(&self) -> PathBuf { self.meta().join("versions") }
    pub fn natives(&self, version_id: &str) -> PathBuf { self.meta().join("natives").join(version_id) }
    pub fn java(&self) -> PathBuf { self.meta().join("java") }
    pub fn cache(&self) -> PathBuf { self.meta().join("cache") }
    pub fn logs(&self) -> PathBuf { self.root.join("logs") }
    pub fn skins(&self) -> PathBuf { self.root.join("skins") }
    /// Locally built / bundled Salt mods (saltHUD, saltZoom, Chlorine…), one
    /// folder per Minecraft version.
    pub fn salt_mods(&self) -> PathBuf { self.root.join("salt-mods") }
    pub fn settings_file(&self) -> PathBuf { self.root.join("settings.json") }
    pub fn accounts_file(&self) -> PathBuf { self.root.join("accounts.json") }

    pub fn library_path(&self, relative: &str) -> PathBuf {
        self.libraries().join(relative)
    }
}

/// Turns a Maven coordinate (`group:artifact:version[:classifier][@ext]`) into
/// the relative path used in a Maven repository.
pub fn maven_path(coord: &str) -> Option<String> {
    let (coord, ext) = match coord.split_once('@') {
        Some((c, e)) => (c, e),
        None => (coord, "jar"),
    };
    let parts: Vec<&str> = coord.split(':').collect();
    if parts.len() < 3 {
        return None;
    }
    let group = parts[0].replace('.', "/");
    let artifact = parts[1];
    let version = parts[2];
    let file = match parts.get(3) {
        Some(classifier) => format!("{artifact}-{version}-{classifier}.{ext}"),
        None => format!("{artifact}-{version}.{ext}"),
    };
    Some(format!("{group}/{artifact}/{version}/{file}"))
}

/// Rejects paths from untrusted archives (modpacks) that try to escape the
/// target folder with `..` or absolute paths ("zip slip").
pub fn safe_join(base: &Path, relative: &str) -> Option<PathBuf> {
    let rel = Path::new(relative);
    if rel.is_absolute() {
        return None;
    }
    for comp in rel.components() {
        match comp {
            std::path::Component::Normal(_) | std::path::Component::CurDir => {}
            _ => return None,
        }
    }
    Some(base.join(rel))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maven_paths() {
        assert_eq!(
            maven_path("net.fabricmc:fabric-loader:0.19.5").unwrap(),
            "net/fabricmc/fabric-loader/0.19.5/fabric-loader-0.19.5.jar"
        );
        assert_eq!(
            maven_path("org.lwjgl:lwjgl:3.3.3:natives-windows").unwrap(),
            "org/lwjgl/lwjgl/3.3.3/lwjgl-3.3.3-natives-windows.jar"
        );
    }

    #[test]
    fn zip_slip_rejected() {
        let base = Path::new("C:/x");
        assert!(safe_join(base, "../evil").is_none());
        assert!(safe_join(base, "mods/a.jar").is_some());
    }
}
