//! Native RubyGems client - no gem binary required!
//!
//! Pack communicates directly with RubyGems.org API to:
//! - Search for gems
//! - Fetch gem info and versions
//! - Download and cache gems
//! - Resolve dependencies
//!
//! This makes Pack a true drop-in replacement - install pack and you don't
//! need Ruby or gem installed at all!

pub mod native;

pub use native::OutdatedGem;

use pack_core::{GemName, GemVersion, PackError, PackResult};
use serde::Deserialize;
use std::path::PathBuf;

pub struct Registry {
    client: reqwest::Client,
    base_url: String,
    cache_dir: PathBuf,
}

impl Registry {
    pub fn new() -> Self {
        let cache_dir = Self::default_cache_dir();

        Self {
            client: reqwest::Client::builder()
                .user_agent("Pack/0.1.8")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: "https://rubygems.org".to_string(),
            cache_dir,
        }
    }

    fn default_cache_dir() -> PathBuf {
        std::env::var("PACK_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                std::env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".cache").join("pack"))
                    .unwrap_or_else(|_| PathBuf::from(".cache/pack"))
            })
    }

    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Pack/0.1.8")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: "https://rubygems.org".to_string(),
            cache_dir,
        }
    }

    pub fn with_base_url(base_url: impl Into<String>) -> Self {
        Self {
            client: reqwest::Client::builder()
                .user_agent("Pack/0.1.8")
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: base_url.into(),
            cache_dir: Self::default_cache_dir(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn cache_dir(&self) -> &PathBuf {
        &self.cache_dir
    }

    pub fn set_cache_dir(&mut self, cache_dir: PathBuf) {
        self.cache_dir = cache_dir;
    }

    /// Search for gems by name pattern
    pub async fn search(&self, query: &str, limit: Option<usize>) -> PackResult<Vec<GemSearchResult>> {
        let url = format!("{}/api/v1/search.json?query={}", self.base_url, query);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PackError::Registry(format!("search failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(PackError::Registry(format!("search failed with status: {}", resp.status())));
        }

        #[derive(Deserialize)]
        struct SearchResult {
            name: String,
            version: String,
            downloads: Option<u64>,
            description: Option<String>,
        }

        let results: Vec<SearchResult> = resp
            .json()
            .await
            .map_err(|e| PackError::Registry(format!("failed to parse search: {}", e)))?;

        let limit = limit.unwrap_or(30);
        Ok(results.into_iter().take(limit).map(|r| GemSearchResult {
            name: GemName(r.name),
            version: GemVersion(r.version),
            downloads: r.downloads.unwrap_or(0),
            description: r.description.unwrap_or_default(),
        }).collect())
    }

    /// Get all versions for a gem
    pub async fn versions(&self, name: &GemName) -> PackResult<Vec<GemVersion>> {
        let url = format!("{}/api/v1/versions/{}.json", self.base_url, name.0);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PackError::Registry(format!("versions failed: {}", e)))?;

        if !resp.status().is_success() {
            return Err(PackError::Registry(format!(
                "versions request failed with HTTP {}",
                resp.status()
            )));
        }

        #[derive(Deserialize)]
        struct VersionRecord {
            number: String,
            #[allow(dead_code)]
            prerelease: bool,
            #[allow(dead_code)]
            created: String,
        }

        let body = resp
            .text()
            .await
            .map_err(|e| PackError::Registry(format!("failed to read versions response: {}", e)))?;

        if let Ok(versions) = serde_json::from_str::<Vec<VersionRecord>>(&body) {
            return Ok(versions.into_iter().map(|v| GemVersion(v.number)).collect());
        }

        // Fallback: some mirrors/proxies return a different payload for versions.
        // Use the gem info endpoint to at least resolve the latest version.
        let latest = self.info(name).await?;
        Ok(vec![latest.version])
    }

    /// Get latest gem info
    pub async fn info(&self, name: &GemName) -> PackResult<GemInfo> {
        let url = format!("{}/api/v1/gems/{}.json", self.base_url, name.0);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PackError::Registry(format!("info failed: {}", e)))?;

        #[derive(Deserialize)]
        struct GemInfoResponse {
            name: String,
            version: String,
            info: Option<String>,
            description: Option<String>,
            licenses: Option<Vec<String>>,
            homepage_uri: Option<String>,
            documentation_uri: Option<String>,
            source_code_uri: Option<String>,
            dependencies: Option<Dependencies>,
        }

        #[derive(Deserialize)]
        struct Dependencies {
            development: Option<Vec<Dep>>,
            runtime: Option<Vec<Dep>>,
        }

        #[derive(Deserialize)]
        struct Dep {
            name: String,
            requirements: String,
        }

        let gem_info: GemInfoResponse = resp
            .json()
            .await
            .map_err(|e| PackError::Registry(format!("failed to parse gem info: {}", e)))?;

        let deps = gem_info.dependencies.unwrap_or(Dependencies {
            development: None,
            runtime: None,
        });

        Ok(GemInfo {
            name: GemName(gem_info.name),
            version: GemVersion(gem_info.version),
            info: gem_info.info.or(gem_info.description).unwrap_or_default(),
            licenses: gem_info.licenses.unwrap_or_default(),
            homepage: gem_info.homepage_uri,
            documentation: gem_info.documentation_uri,
            source_code: gem_info.source_code_uri,
            dependencies: deps.runtime.map(|d| d.into_iter().map(|dep| DependencySpec {
                name: GemName(dep.name),
                requirement: dep.requirements,
            }).collect()).unwrap_or_default(),
            development_dependencies: deps.development.map(|d| d.into_iter().map(|dep| DependencySpec {
                name: GemName(dep.name),
                requirement: dep.requirements,
            }).collect()).unwrap_or_default(),
        })
    }

    /// Get the gem specification (.gemspec) content
    pub async fn spec(&self, name: &GemName, version: &GemVersion) -> PackResult<String> {
        // RubyGems spec API format: /api/v1/specs/{gem_name}/{version}.gemspec
        let url = format!("{}/api/v1/specs/{}/{}.gemspec",
            self.base_url, name.0, version.0);

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PackError::Registry(format!("spec fetch failed: {}", e)))?;

        resp.text()
            .await
            .map_err(|e| PackError::Registry(format!("failed to read spec: {}", e)))
    }

    /// Download a gem to local cache
    pub async fn download(&self, name: &GemName, version: &GemVersion) -> PackResult<PathBuf> {
        let gem_file = format!("{}-{}.gem", name.0, version.0);
        let url = format!("{}/downloads/{}", self.base_url, gem_file);

        // Check if already cached
        let cache_path = self.cache_dir.join("gems").join(&gem_file);
        if cache_path.exists() {
            return Ok(cache_path);
        }

        // Download
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PackError::Registry(format!("download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(PackError::Registry(format!(
                "download failed with status: {}", response.status()
            )));
        }

        // Ensure cache directory exists
        std::fs::create_dir_all(cache_path.parent().unwrap())?;

        // Write to cache
        let bytes = response.bytes()
            .await
            .map_err(|e| PackError::Registry(format!("failed to read download: {}", e)))?;

        std::fs::write(&cache_path, &bytes)?;

        Ok(cache_path)
    }

    /// Check if gem is installed locally (from cache)
    pub fn is_cached(&self, name: &GemName, version: &GemVersion) -> bool {
        let gem_file = format!("{}-{}.gem", name.0, version.0);
        self.cache_dir.join("gems").join(&gem_file).exists()
    }

    /// List all cached gems
    pub fn cached_gems(&self) -> PackResult<Vec<CachedGem>> {
        let gems_dir = self.cache_dir.join("gems");
        if !gems_dir.exists() {
            return Ok(vec![]);
        }

        let mut gems = vec![];
        for entry in std::fs::read_dir(gems_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map(|e| e == "gem").unwrap_or(false) {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if let Some(stripped) = filename.strip_suffix(".gem") {
                        if let Some((n, ver)) = stripped.rsplit_once('-') {
                            gems.push(CachedGem {
                                name: GemName(n.to_string()),
                                version: GemVersion(ver.to_string()),
                                path,
                            });
                        }
                    }
                }
            }
        }
        Ok(gems)
    }

    /// Get list of most downloaded gems (popular gems)
    pub async fn popular(&self, limit: usize) -> PackResult<Vec<GemSearchResult>> {
        // RubyGems doesn't have a direct popular API, but we can search with no query
        // to get latest gems and sort by downloads (if available)
        let url = format!("{}/api/v1/search.json?query=", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| PackError::Registry(format!("popular failed: {}", e)))?;

        #[derive(Deserialize)]
        struct SearchResult {
            name: String,
            version: String,
            downloads: Option<u64>,
            description: Option<String>,
        }

        let mut results: Vec<SearchResult> = resp
            .json()
            .await
            .map_err(|e| PackError::Registry(format!("failed to parse popular: {}", e)))?;

        // Sort by downloads descending
        results.sort_by(|a, b| b.downloads.cmp(&a.downloads));

        Ok(results.into_iter().take(limit).map(|r| GemSearchResult {
            name: GemName(r.name),
            version: GemVersion(r.version),
            downloads: r.downloads.unwrap_or(0),
            description: r.description.unwrap_or_default(),
        }).collect())
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GemInfo {
    pub name: GemName,
    pub version: GemVersion,
    pub info: String,
    pub licenses: Vec<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub source_code: Option<String>,
    pub dependencies: Vec<DependencySpec>,
    pub development_dependencies: Vec<DependencySpec>,
}

impl GemInfo {
    pub fn name_str(&self) -> &str {
        &self.name.0
    }

    pub fn version_str(&self) -> &str {
        &self.version.0
    }

    pub fn has_homepage(&self) -> bool {
        self.homepage.is_some()
    }

    pub fn has_documentation(&self) -> bool {
        self.documentation.is_some()
    }

    pub fn has_source_code(&self) -> bool {
        self.source_code.is_some()
    }

    pub fn license_string(&self) -> String {
        if self.licenses.is_empty() {
            "None".to_string()
        } else {
            self.licenses.join(", ")
        }
    }

    pub fn total_dependencies(&self) -> usize {
        self.dependencies.len() + self.development_dependencies.len()
    }

    pub fn runtime_dep_count(&self) -> usize {
        self.dependencies.len()
    }

    pub fn dev_dep_count(&self) -> usize {
        self.development_dependencies.len()
    }
}

#[derive(Debug, Clone)]
pub struct DependencySpec {
    pub name: GemName,
    pub requirement: String,
}

#[derive(Debug, Clone)]
pub struct GemSearchResult {
    pub name: GemName,
    pub version: GemVersion,
    pub downloads: u64,
    pub description: String,
}

impl GemSearchResult {
    pub fn name(&self) -> &str {
        &self.name.0
    }

    pub fn version(&self) -> &str {
        &self.version.0
    }

    pub fn downloads_formatted(&self) -> String {
        if self.downloads >= 1_000_000 {
            format!("{:.1}M", self.downloads as f64 / 1_000_000.0)
        } else if self.downloads >= 1_000 {
            format!("{:.1}K", self.downloads as f64 / 1_000.0)
        } else {
            self.downloads.to_string()
        }
    }

    pub fn has_description(&self) -> bool {
        !self.description.is_empty()
    }
}

#[derive(Debug)]
pub struct CachedGem {
    pub name: GemName,
    pub version: GemVersion,
    pub path: PathBuf,
}

// Sync versions of async methods for non-async use
impl Registry {
    /// Search for gems (sync wrapper)
    pub fn search_sync(&self, query: &str, limit: Option<usize>) -> PackResult<Vec<GemSearchResult>> {
        // For synchronous use, we need to use a runtime
        // This is a simple blocking wrapper
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PackError::Registry(format!("failed to create runtime: {}", e)))?;
        rt.block_on(self.search(query, limit))
    }

    /// Get gem info (sync wrapper)
    pub fn info_sync(&self, name: &GemName) -> PackResult<GemInfo> {
        let rt = tokio::runtime::Runtime::new()
            .map_err(|e| PackError::Registry(format!("failed to create runtime: {}", e)))?;
        rt.block_on(self.info(name))
    }

    /// List installed gems from cache (sync, no gem binary needed)
    pub fn list_sync(&self) -> PackResult<Vec<String>> {
        let gems = self.cached_gems()?;
        Ok(gems.into_iter().map(|g| format!("{} ({})", g.name.0, g.version.0)).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = Registry::new();
        assert_eq!(registry.base_url, "https://rubygems.org");
    }

    #[test]
    fn test_registry_default() {
        let registry = Registry::default();
        assert_eq!(registry.base_url, "https://rubygems.org");
    }

    #[test]
    fn test_cached_gems_empty() {
        let registry = Registry::with_cache_dir(PathBuf::from("/tmp/nonexistent-pack-cache"));
        let gems = registry.cached_gems().unwrap();
        assert!(gems.is_empty());
    }
}
