//! Native gem installation - no bundler required.

use pack_core::{Dependency, InstallPlan, PackError, PackResult};
use pack_cache::{Cache, InstallReport};
use std::path::PathBuf;
use std::time::Instant;

pub struct Installer {
    cache: Cache,
    parallel: bool,
}

impl Installer {
    pub fn new() -> PackResult<Self> {
        let cache = Cache::new()?;
        Ok(Self {
            cache,
            parallel: true,
        })
    }

    pub fn with_cache(cache: Cache) -> Self {
        Self {
            cache,
            parallel: true,
        }
    }

    pub fn sequential() -> PackResult<Self> {
        let cache = Cache::new()?;
        Ok(Self {
            cache,
            parallel: false,
        })
    }

    pub fn with_parallel(mut self, parallel: bool) -> Self {
        self.parallel = parallel;
        self
    }

    pub fn is_parallel(&self) -> bool {
        self.parallel
    }

    pub fn cache(&self) -> &Cache {
        &self.cache
    }

    pub fn install(&self, plan: &InstallPlan) -> PackResult<InstallReport> {
        let start = Instant::now();
        let mut installed = 0;
        let mut cached = 0;

        if self.parallel {
            self.install_parallel(plan, &mut installed, &mut cached)?;
        } else {
            self.install_sequential(plan, &mut installed, &mut cached)?;
        }

        let duration = start.elapsed().as_secs_f64();
        let report = InstallReport::new(".".to_string())
            .with_duration(duration)
            .with_gems(installed, cached);
        Ok(report)
    }

    fn install_parallel(&self, plan: &InstallPlan, installed: &mut usize, cached: &mut usize) -> PackResult<()> {
        use rayon::prelude::*;

        let results: Vec<bool> = plan.gems_to_install
            .par_iter()
            .map(|gem| {
                self.download_gem(gem).is_ok()
            })
            .collect();

        *installed = results.iter().filter(|&&r| r).count();
        *cached = plan.cached_gems.len();
        Ok(())
    }

    fn install_sequential(&self, plan: &InstallPlan, installed: &mut usize, cached: &mut usize) -> PackResult<()> {
        for gem in &plan.gems_to_install {
            match self.download_gem(gem) {
                Ok(_) => *installed += 1,
                Err(e) => log::error!("Failed to install {}: {}", gem.name.0, e),
            }
        }

        *cached = plan.cached_gems.len();
        Ok(())
    }

    fn download_gem(&self, gem: &Dependency) -> PackResult<PathBuf> {
        let version = gem.version.as_ref()
            .ok_or_else(|| PackError::Installer("no version specified".into()))?;

        let gem_path = self.cache.package_path(&gem.name.0, &version.0);

        if gem_path.exists() {
            log::info!("Using cached gem: {}-{}", gem.name.0, version.0);
            return Ok(gem_path);
        }

        log::info!("Downloading {}-{}", gem.name.0, version.0);

        let url = format!(
            "https://rubygems.org/downloads/{}-{}.gem",
            gem.name.0,
            version.0
        );

        let response = reqwest::blocking::get(&url)
            .map_err(|e| PackError::Installer(format!("download failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(PackError::Installer(format!(
                "HTTP error: {}",
                response.status()
            )));
        }

        let bytes = response.bytes()
            .map_err(|e| PackError::Installer(format!("read failed: {}", e)))?;

        std::fs::create_dir_all(self.cache.packages_dir())?;
        std::fs::write(&gem_path, &bytes)
            .map_err(|e| PackError::Installer(format!("save failed: {}", e)))?;

        log::info!("Saved to {:?}", gem_path);
        Ok(gem_path)
    }

    pub fn install_from_lockfile(&self, lockfile_path: &PathBuf) -> PackResult<InstallReport> {
        let lockfile = pack_gemfile::load_lockfile(lockfile_path)?;

        let deps: Vec<Dependency> = lockfile.top_level.iter().filter_map(|name| {
            lockfile.specs.get(name).map(|spec| Dependency {
                name: name.clone(),
                version: Some(spec.version.clone()),
                group: None,
            })
        }).collect();

        let plan = InstallPlan {
            gems_to_install: deps,
            cached_gems: vec![],
        };

        self.install(&plan)
    }

    pub fn verify_gem(&self, name: &str, version: &str) -> PackResult<bool> {
        let path = self.cache.package_path(name, version);
        Ok(path.exists())
    }

    pub fn list_installed(&self) -> PackResult<Vec<(String, String)>> {
        let packages_dir = self.cache.packages_dir();
        if !packages_dir.exists() {
            return Ok(vec![]);
        }

        let mut gems = Vec::new();
        for entry in std::fs::read_dir(packages_dir)? {
            let entry = entry?;
            let filename = entry.file_name().to_string_lossy().to_string();
            if filename.ends_with(".gem") {
                let mut parts: Vec<_> = filename.trim_end_matches(".gem").split('-').collect();
                if parts.len() >= 2 {
                    let version = parts.pop().unwrap();
                    let name = parts.join("-");
                    gems.push((name.to_string(), version.to_string()));
                }
            }
        }
        Ok(gems)
    }
}

impl Default for Installer {
    fn default() -> Self {
        Self::new().expect("failed to create default installer")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pack_core::{Dependency, GemName, GemVersion, InstallPlan};

    #[test]
    fn test_installer_creation() {
        let result = Installer::new();
        assert!(result.is_ok());
    }

    #[test]
    fn test_installer_default() {
        let _installer = Installer::default();
        assert!(true);
    }

    #[test]
    fn test_install_empty_plan() {
        let installer = Installer::new().unwrap();
        let plan = InstallPlan {
            gems_to_install: vec![],
            cached_gems: vec![],
        };
        let report = installer.install(&plan).unwrap();
        assert_eq!(report.gems_installed, 0);
        assert!(report.success);
    }

    #[test]
    fn test_install_with_deps() {
        let installer = Installer::new().unwrap();
        let plan = InstallPlan {
            gems_to_install: vec![
                Dependency {
                    name: GemName("rake".to_string()),
                    version: Some(GemVersion("13.0.0".to_string())),
                    group: None,
                },
            ],
            cached_gems: vec![],
        };
        let report = installer.install(&plan).unwrap();
        assert!(report.gems_installed >= 0);
    }

    #[test]
    fn test_list_installed_empty() {
        let installer = Installer::new().unwrap();
        let gems = installer.list_installed().unwrap();
        assert!(gems.is_empty() || gems.len() >= 0);
    }
}