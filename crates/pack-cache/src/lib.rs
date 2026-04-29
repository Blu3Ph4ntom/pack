//! Cache layout and operations with offline support.

use directories::ProjectDirs;
use pack_core::{PackError, PackResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct Cache {
    root: PathBuf,
}

impl Cache {
    pub fn new() -> PackResult<Self> {
        let proj_dirs = ProjectDirs::from("com", "piper", "pack")
            .ok_or_else(|| PackError::Cache("could not determine cache directory".into()))?;

        let root = proj_dirs.cache_dir().to_path_buf();
        Ok(Self { root })
    }

    pub fn from_path(path: &Path) -> Self {
        Self { root: path.to_path_buf() }
    }

    pub fn with_root(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &PathBuf {
        &self.root
    }

    pub fn exists(&self) -> bool {
        self.root.exists()
    }

    pub fn is_initialized(&self) -> bool {
        self.spec_cache_dir().exists() && self.packages_dir().exists()
    }

    pub fn packages_dir(&self) -> PathBuf {
        self.root.join("packages")
    }

    pub fn metadata_dir(&self) -> PathBuf {
        self.root.join("metadata")
    }

    pub fn native_dir(&self) -> PathBuf {
        self.root.join("native")
    }

    pub fn installs_dir(&self) -> PathBuf {
        self.root.join("installs")
    }

    pub fn logs_dir(&self) -> PathBuf {
        self.root.join("logs")
    }

    pub fn gem_cache_dir(&self) -> PathBuf {
        self.root.join("gem_cache")
    }

    pub fn spec_cache_dir(&self) -> PathBuf {
        self.root.join("specs")
    }

    pub fn ensure_dirs(&self) -> PackResult<()> {
        std::fs::create_dir_all(self.packages_dir())?;
        std::fs::create_dir_all(self.metadata_dir())?;
        std::fs::create_dir_all(self.native_dir())?;
        std::fs::create_dir_all(self.installs_dir())?;
        std::fs::create_dir_all(self.logs_dir())?;
        std::fs::create_dir_all(self.gem_cache_dir())?;
        std::fs::create_dir_all(self.spec_cache_dir())?;
        Ok(())
    }

    pub fn is_offline(&self) -> bool {
        std::env::var("PACK_OFFLINE").is_ok()
    }

    pub fn save_install_report(&self, project_path: &str, report: &InstallReport) -> PackResult<()> {
        let file_name = sanitize_filename(project_path);
        let report_path = self.installs_dir().join(format!("{}.json", file_name));
        let content = serde_json::to_string_pretty(report)
            .map_err(|e| PackError::Cache(format!("failed to serialize report: {}", e)))?;
        std::fs::write(&report_path, content)
            .map_err(|e| PackError::Cache(format!("failed to write report: {}", e)))?;
        Ok(())
    }

    pub fn load_install_report(&self, project_path: &str) -> PackResult<Option<InstallReport>> {
        let file_name = sanitize_filename(project_path);
        let report_path = self.installs_dir().join(format!("{}.json", file_name));
        if !report_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&report_path)
            .map_err(|e| PackError::Cache(format!("failed to read report: {}", e)))?;
        let report: InstallReport = serde_json::from_str(&content)
            .map_err(|e| PackError::Cache(format!("failed to parse report: {}", e)))?;
        Ok(Some(report))
    }

    pub fn package_path(&self, gem_name: &str, version: &str) -> PathBuf {
        self.packages_dir().join(format!("{}-{}.gem", gem_name, version))
    }

    pub fn has_gem(&self, name: &str, version: &str) -> bool {
        self.package_path(name, version).exists()
    }

    pub fn list_cached_gems(&self) -> PackResult<Vec<CachedGem>> {
        let mut gems = Vec::new();
        let packages = self.packages_dir();

        if !packages.exists() {
            return Ok(gems);
        }

        for entry in std::fs::read_dir(packages)? {
            let entry = entry?;
            let filename = entry.file_name().to_string_lossy().to_string();

            if filename.ends_with(".gem") {
                let name_ver = filename.trim_end_matches(".gem");
                if let Some((name, version)) = name_ver.rsplit_once('-') {
                    gems.push(CachedGem {
                        name: name.to_string(),
                        version: version.to_string(),
                        path: entry.path(),
                        size: entry.metadata().map(|m| m.len()).unwrap_or(0),
                    });
                }
            }
        }

        Ok(gems)
    }

    pub fn save_spec(&self, name: &str, spec: &GemSpecCache) -> PackResult<()> {
        let spec_path = self.spec_cache_dir().join(format!("{}.json", name));
        let content = serde_json::to_string_pretty(spec)
            .map_err(|e| PackError::Cache(format!("failed to serialize spec: {}", e)))?;
        std::fs::write(&spec_path, content)
            .map_err(|e| PackError::Cache(format!("failed to write spec: {}", e)))?;
        Ok(())
    }

    pub fn load_spec(&self, name: &str) -> PackResult<Option<GemSpecCache>> {
        let spec_path = self.spec_cache_dir().join(format!("{}.json", name));
        if !spec_path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(&spec_path)
            .map_err(|e| PackError::Cache(format!("failed to read spec: {}", e)))?;
        let spec: GemSpecCache = serde_json::from_str(&content)
            .map_err(|e| PackError::Cache(format!("failed to parse spec: {}", e)))?;
        Ok(Some(spec))
    }

    pub fn clear(&self) -> PackResult<()> {
        if self.root.exists() {
            std::fs::remove_dir_all(&self.root)?;
        }
        self.ensure_dirs()?;
        Ok(())
    }

    pub fn size(&self) -> PackResult<u64> {
        let mut total = 0u64;
        if self.root.exists() {
            for entry in std::fs::read_dir(&self.root)?.flatten() {
                total += Self::dir_size(&entry.path())?;
            }
        }
        Ok(total)
    }

    pub fn size_human(&self) -> PackResult<String> {
        let bytes = self.size()?;
        Ok(human_readable_size(bytes))
    }

    fn dir_size(path: &PathBuf) -> PackResult<u64> {
        let mut total = 0u64;
        if path.is_dir() {
            for entry in std::fs::read_dir(path)? {
                total += Self::dir_size(&entry?.path())?;
            }
        } else if path.is_file() {
            total += path.metadata().map(|m| m.len()).unwrap_or(0);
        }
        Ok(total)
    }
}

impl Default for Cache {
    fn default() -> Self {
        Self::new().expect("failed to create default cache")
    }
}

fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            _ => c,
        })
        .collect()
}

fn human_readable_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallReport {
    pub project_path: String,
    pub timestamp: u64,
    pub duration_secs: f64,
    pub gems_installed: usize,
    pub gems_cached: usize,
    pub success: bool,
    pub error_message: Option<String>,
}

impl InstallReport {
    pub fn new(project_path: String) -> Self {
        Self {
            project_path,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            duration_secs: 0.0,
            gems_installed: 0,
            gems_cached: 0,
            success: true,
            error_message: None,
        }
    }

    pub fn with_duration(mut self, duration_secs: f64) -> Self {
        self.duration_secs = duration_secs;
        self
    }

    pub fn with_gems(mut self, installed: usize, cached: usize) -> Self {
        self.gems_installed = installed;
        self.gems_cached = cached;
        self
    }

    pub fn with_error(mut self, error: String) -> Self {
        self.success = false;
        self.error_message = Some(error);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedGem {
    pub name: String,
    pub version: String,
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GemSpecCache {
    pub name: String,
    pub version: String,
    pub dependencies: Vec<(String, String)>,
    pub platform: String,
    pub cached_at: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_dirs() {
        let cache = Cache::new().unwrap();
        assert!(cache.root().ends_with("pack"));
        assert!(cache.packages_dir().ends_with("packages"));
        assert!(cache.metadata_dir().ends_with("metadata"));
        assert!(cache.native_dir().ends_with("native"));
        assert!(cache.installs_dir().ends_with("installs"));
        assert!(cache.logs_dir().ends_with("logs"));
        assert!(cache.gem_cache_dir().ends_with("gem_cache"));
        assert!(cache.spec_cache_dir().ends_with("specs"));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("/home/user/project"), "_home_user_project");
        assert_eq!(sanitize_filename("simple"), "simple");
        assert_eq!(sanitize_filename("C:\\Users\\test"), "C__Users_test");
    }

    #[test]
    fn test_install_report_new() {
        let report = InstallReport::new("/path/to/project".to_string());
        assert_eq!(report.project_path, "/path/to/project");
        assert!(report.success);
        assert!(report.error_message.is_none());
        assert_eq!(report.gems_installed, 0);
        assert_eq!(report.gems_cached, 0);
    }

    #[test]
    fn test_install_report_builder() {
        let report = InstallReport::new("/path/to/project".to_string())
            .with_duration(5.5)
            .with_gems(10, 5);

        assert_eq!(report.duration_secs, 5.5);
        assert_eq!(report.gems_installed, 10);
        assert_eq!(report.gems_cached, 5);
    }

    #[test]
    fn test_install_report_error() {
        let report = InstallReport::new("/path/to/project".to_string())
            .with_error("something went wrong".to_string());

        assert!(!report.success);
        assert_eq!(report.error_message, Some("something went wrong".to_string()));
    }
}