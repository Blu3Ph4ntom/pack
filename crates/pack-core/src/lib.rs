//! Pack core types shared across all crates.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PackError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("project error: {0}")]
    Project(String),

    #[error("gemfile error: {0}")]
    Gemfile(String),

    #[error("registry error: {0}")]
    Registry(String),

    #[error("resolver error: {0}")]
    Resolver(String),

    #[error("installer error: {0}")]
    Installer(String),

    #[error("cache error: {0}")]
    Cache(String),

    #[error("exec error: {0}")]
    Exec(String),
}

pub type PackResult<T> = Result<T, PackError>;

#[derive(Debug, Clone)]
pub struct Project {
    pub path: PathBuf,
    pub gemfile: Option<PathBuf>,
    pub gemfile_lock: Option<PathBuf>,
}

impl Project {
    pub fn discover() -> PackResult<Self> {
        let path = std::env::current_dir()
            .map_err(|e| PackError::Project(format!("failed to get current dir: {}", e)))?;

        let gemfile = find_file(&path, "Gemfile");
        let gemfile_lock = find_file(&path, "Gemfile.lock");

        Ok(Self {
            path,
            gemfile,
            gemfile_lock,
        })
    }
}

fn find_file(dir: &Path, name: &str) -> Option<PathBuf> {
    let path = dir.join(name);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

#[derive(Debug, Clone)]
pub struct RubyEnvironment {
    pub ruby_version: Option<String>,
    pub gem_available: bool,
    pub bundle_available: bool,
    pub gem_version: Option<String>,
    pub bundle_version: Option<String>,
}

impl RubyEnvironment {
    pub fn discover() -> Self {
        Self {
            ruby_version: Self::get_ruby_version(),
            gem_version: Self::get_gem_version(),
            gem_available: Self::is_command_available("gem"),
            bundle_version: Self::get_bundle_version(),
            bundle_available: Self::is_command_available("bundle"),
        }
    }

    fn get_ruby_version() -> Option<String> {
        std::process::Command::new("ruby")
            .arg("--version")
            .output()
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    String::from_utf8(o.stdout).ok()
                } else {
                    None
                }
            })
            .map(|s| s.trim().to_string())
    }

    fn get_gem_version() -> Option<String> {
        std::process::Command::new("gem")
            .arg("--version")
            .output()
            .or_else(|_| {
                std::process::Command::new("ruby")
                    .args(["-S", "gem", "--version"])
                    .output()
            })
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(
                        String::from_utf8(o.stdout)
                            .unwrap_or_default()
                            .trim()
                            .to_string(),
                    )
                } else {
                    None
                }
            })
    }

    fn get_bundle_version() -> Option<String> {
        std::process::Command::new("bundle")
            .arg("--version")
            .output()
            .or_else(|_| {
                std::process::Command::new("ruby")
                    .args(["-S", "bundle", "--version"])
                    .output()
            })
            .ok()
            .and_then(|o| {
                if o.status.success() {
                    Some(
                        String::from_utf8(o.stdout)
                            .unwrap_or_default()
                            .trim()
                            .to_string(),
                    )
                } else {
                    None
                }
            })
    }

    fn is_command_available(cmd: &str) -> bool {
        std::process::Command::new(cmd)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            || matches!(cmd, "gem" | "bundle")
                && std::process::Command::new("ruby")
                    .args(["-S", cmd, "--version"])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
    }

    pub fn has_ruby(&self) -> bool {
        self.ruby_version.is_some()
    }

    pub fn has_gem(&self) -> bool {
        self.gem_available
    }

    pub fn has_bundle(&self) -> bool {
        self.bundle_available
    }

    pub fn is_pack_compatible(&self) -> bool {
        self.gem_available || self.bundle_available
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct GemName(pub String);

impl GemName {
    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn to_lowercase(&self) -> String {
        self.0.to_lowercase()
    }

    pub fn starts_with(&self, prefix: &str) -> bool {
        self.0.starts_with(prefix)
    }

    pub fn ends_with(&self, suffix: &str) -> bool {
        self.0.ends_with(suffix)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct GemVersion(pub String);

impl GemVersion {
    pub fn new(version: impl Into<String>) -> Self {
        Self(version.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn major(&self) -> Option<u64> {
        self.0.split('.').next()?.parse().ok()
    }

    pub fn minor(&self) -> Option<u64> {
        self.0.split('.').nth(1)?.parse().ok()
    }

    pub fn patch(&self) -> Option<u64> {
        self.0.split('.').nth(2)?.parse().ok()
    }

    pub fn is_prerelease(&self) -> bool {
        self.0.contains("alpha") || self.0.contains("beta") || self.0.contains("rc")
    }
}

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: GemName,
    pub version: Option<GemVersion>,
    pub group: Option<String>,
}

impl Dependency {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: GemName(name.into()),
            version: None,
            group: None,
        }
    }

    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(GemVersion(version.into()));
        self
    }

    pub fn in_group(mut self, group: impl Into<String>) -> Self {
        self.group = Some(group.into());
        self
    }

    pub fn name_str(&self) -> &str {
        &self.name.0
    }

    pub fn version_str(&self) -> Option<&str> {
        self.version.as_ref().map(|v| v.0.as_str())
    }

    pub fn group_str(&self) -> Option<&str> {
        self.group.as_deref()
    }

    pub fn is_in_group(&self, group: &str) -> bool {
        self.group.as_deref() == Some(group)
    }

    pub fn matches_name(&self, name: &str) -> bool {
        self.name.0 == name
    }
}

#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub gems_to_install: Vec<Dependency>,
    pub cached_gems: Vec<Dependency>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gem_name_equality() {
        let name1 = GemName("rails".to_string());
        let name2 = GemName("rails".to_string());
        let name3 = GemName("rake".to_string());

        assert_eq!(name1, name2);
        assert_ne!(name1, name3);
    }

    #[test]
    fn test_gem_version_equality() {
        let v1 = GemVersion("7.1.0".to_string());
        let v2 = GemVersion("7.1.0".to_string());
        let v3 = GemVersion("7.0.0".to_string());

        assert_eq!(v1, v2);
        assert_ne!(v1, v3);
    }

    #[test]
    fn test_dependency() {
        let dep = Dependency {
            name: GemName("rails".to_string()),
            version: Some(GemVersion("7.1.0".to_string())),
            group: Some("test".to_string()),
        };

        assert_eq!(dep.name.0, "rails");
        assert_eq!(dep.version.unwrap().0, "7.1.0");
        assert_eq!(dep.group.unwrap(), "test");
    }

    #[test]
    fn test_dependency_no_version() {
        let dep = Dependency {
            name: GemName("rake".to_string()),
            version: None,
            group: None,
        };

        assert_eq!(dep.name.0, "rake");
        assert!(dep.version.is_none());
        assert!(dep.group.is_none());
    }

    #[test]
    fn test_install_plan() {
        let plan = InstallPlan {
            gems_to_install: vec![Dependency {
                name: GemName("rails".to_string()),
                version: Some(GemVersion("7.1.0".to_string())),
                group: None,
            }],
            cached_gems: vec![Dependency {
                name: GemName("rake".to_string()),
                version: None,
                group: None,
            }],
        };

        assert_eq!(plan.gems_to_install.len(), 1);
        assert_eq!(plan.cached_gems.len(), 1);
    }

    #[test]
    fn test_pack_error_io() {
        use std::io;
        let io_error = io::Error::new(io::ErrorKind::NotFound, "file not found");
        let pack_error = PackError::Io(io_error);
        assert!(format!("{}", pack_error).contains("file not found"));
    }

    #[test]
    fn test_pack_error_project() {
        let pack_error = PackError::Project("no gemfile found".to_string());
        assert_eq!(format!("{}", pack_error), "project error: no gemfile found");
    }

    #[test]
    fn test_pack_error_gemfile() {
        let pack_error = PackError::Gemfile("parse error".to_string());
        assert_eq!(format!("{}", pack_error), "gemfile error: parse error");
    }

    #[test]
    fn test_pack_error_registry() {
        let pack_error = PackError::Registry("network error".to_string());
        assert_eq!(format!("{}", pack_error), "registry error: network error");
    }

    #[test]
    fn test_gem_name_helpers() {
        let name = GemName::new("Rails");
        assert_eq!(name.as_str(), "Rails");
        assert_eq!(name.to_lowercase(), "rails");
        assert!(!name.is_empty());
        assert!(name.starts_with("R"));
        assert!(name.ends_with("ls"));
    }

    #[test]
    fn test_gem_version_helpers() {
        let version = GemVersion::new("7.1.3");
        assert_eq!(version.as_str(), "7.1.3");
        assert_eq!(version.major(), Some(7));
        assert_eq!(version.minor(), Some(1));
        assert_eq!(version.patch(), Some(3));
        assert!(!version.is_prerelease());

        let pre = GemVersion::new("7.1.0.alpha1");
        assert!(pre.is_prerelease());
    }

    #[test]
    fn test_dependency_builder() {
        let dep = Dependency::new("rails")
            .with_version("7.1.0")
            .in_group("test");

        assert_eq!(dep.name_str(), "rails");
        assert_eq!(dep.version_str(), Some("7.1.0"));
        assert_eq!(dep.group_str(), Some("test"));
        assert!(dep.is_in_group("test"));
        assert!(!dep.is_in_group("development"));
        assert!(dep.matches_name("rails"));
    }

    #[test]
    fn test_dependency_builder_minimal() {
        let dep = Dependency::new("rake");
        assert_eq!(dep.name_str(), "rake");
        assert!(dep.version_str().is_none());
        assert!(dep.group_str().is_none());
    }
}
