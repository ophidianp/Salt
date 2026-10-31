//! Minecraft version numbers and which ones Salt supports.
//!
//! Mojang switched to year-based numbers in 2026 (`26.1`, `26.2`, `26.3`),
//! after `1.21.11`. Both styles parse into the same `(major, minor, patch)`
//! triple so comparisons work across the switch.

use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct McVersion(pub u32, pub u32, pub u32);

impl McVersion {
    pub fn parse(id: &str) -> Option<Self> {
        let mut it = id.split('.');
        let a = it.next()?.parse().ok()?;
        let b = it.next()?.parse().ok()?;
        let c = match it.next() {
            Some(p) => p.parse().ok()?,
            None => 0,
        };
        if it.next().is_some() {
            return None;
        }
        Some(Self(a, b, c))
    }

    /// Versions before 1.14 use Legacy Fabric (meta.legacyfabric.net) and
    /// the `legacy-fabric` loader tag on Modrinth.
    pub fn is_legacy(&self) -> bool {
        *self < McVersion(1, 14, 0)
    }
}

pub const MIN_MODERN: McVersion = McVersion(1, 21, 0);
pub const MAX_SUPPORTED: McVersion = McVersion(26, 3, 0);

/// Releases offered in the instance creator: 1.21 → 26.3, plus 1.8 / 1.8.9
/// for PvP.
pub fn is_supported(id: &str) -> bool {
    if id == "1.8.9" || id == "1.8" {
        return true;
    }
    match McVersion::parse(id) {
        Some(v) => v >= MIN_MODERN && v <= McVersion(MAX_SUPPORTED.0, MAX_SUPPORTED.1, u32::MAX),
        None => false,
    }
}

pub fn compare(a: &str, b: &str) -> Ordering {
    match (McVersion::parse(a), McVersion::parse(b)) {
        (Some(x), Some(y)) => x.cmp(&y),
        _ => a.cmp(b),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn supported_range() {
        assert!(is_supported("1.21"));
        assert!(is_supported("1.21.11"));
        assert!(is_supported("26.1.2"));
        assert!(is_supported("26.3"));
        assert!(is_supported("1.8.9"));
        assert!(!is_supported("1.20.6"));
        assert!(!is_supported("26.4"));
        assert!(!is_supported("26.3-snapshot-1"));
    }

    #[test]
    fn ordering_across_scheme_change() {
        assert_eq!(compare("1.21.11", "26.1"), Ordering::Less);
        assert!(McVersion::parse("1.8.9").unwrap().is_legacy());
    }
}
