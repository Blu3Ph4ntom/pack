//! Pack core types shared across all crates.

use std::path::{Path, PathBuf};
use thiserror::Error;
use serde::{Serialize, Deserialize};

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
        let ruby_version = std::process::Command::new("ruby")
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
            .map(|s| s.trim().to_string());

        let gem_version = std::process::Command::new("gem")
            .arg("--version")
            .output()
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
            });

        let gem_available = std::process::Command::new("gem")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let bundle_version = std::process::Command::new("bundle")
            .arg("--version")
            .output()
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
            });

        let bundle_available = std::process::Command::new("bundle")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        Self {
            ruby_version,
            gem_available,
            bundle_available,
            gem_version,
            bundle_version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct GemName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Ord, PartialOrd)]
pub struct GemVersion(pub String);

#[derive(Debug, Clone)]
pub struct Dependency {
    pub name: GemName,
    pub version: Option<GemVersion>,
    pub group: Option<String>,
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
            gems_to_install: vec![
                Dependency {
                    name: GemName("rails".to_string()),
                    version: Some(GemVersion("7.1.0".to_string())),
                    group: None,
                },
            ],
            cached_gems: vec![
                Dependency {
                    name: GemName("rake".to_string()),
                    version: None,
                    group: None,
                },
            ],
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
}
