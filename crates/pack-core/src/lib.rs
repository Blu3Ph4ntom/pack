//! Pack core types shared across all crates.

use std::path::PathBuf;
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

fn find_file(dir: &PathBuf, name: &str) -> Option<PathBuf> {
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GemName(pub String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
