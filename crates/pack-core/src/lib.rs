//! Pack core types shared across all crates.

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
    pub path: std::path::PathBuf,
    pub has_gemfile: bool,
    pub has_gemfile_lock: bool,
}

#[derive(Debug, Clone)]
pub struct RubyEnvironment {
    pub ruby_version: Option<String>,
    pub gem_available: bool,
    pub bundle_available: bool,
    pub gem_version: Option<String>,
    pub bundle_version: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GemName(pub String);

#[derive(Debug, Clone, PartialEq, Eq)]
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
