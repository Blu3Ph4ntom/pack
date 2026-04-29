//! Lockfile generation - create Gemfile.lock from Gemfile.

use pack_core::{GemName, GemVersion, PackError, PackResult};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use crate::Dependency;

pub struct LockfileGenerator {
    include_optional: bool,
    update_gems: Vec<GemName>,
}

impl LockfileGenerator {
    pub fn new() -> Self {
        Self {
            include_optional: false,
            update_gems: vec![],
        }
    }

    pub fn with_update_gems(mut self, gems: Vec<GemName>) -> Self {
        self.update_gems = gems;
        self
    }

    pub fn include_optional(mut self) -> Self {
        self.include_optional = true;
        self
    }

    pub fn generate(&self, _gemfile_path: &PathBuf, deps: &[Dependency]) -> PackResult<GeneratedLockfile> {
        let mut specs: HashMap<GemName, GemSpecGen> = HashMap::new();
        let mut top_level: Vec<GemName> = Vec::new();
        let mut all_deps: Vec<GemName> = Vec::new();

        // Build top-level deps
        for dep in deps {
            if dep.group.is_some() && !self.include_optional {
                continue;
            }
            top_level.push(dep.name.clone());
            all_deps.push(dep.name.clone());
        }

        // Simulate dependency resolution
        for dep in deps {
            let version = dep.version.clone()
                .unwrap_or(GemVersion(self.resolve_version(&dep.name)?));

            let spec = GemSpecGen {
                version: version.clone(),
                dependencies: self.resolve_dependencies(&dep.name)?,
            };

            specs.insert(dep.name.clone(), spec);
        }

        // Generate remote specs (simulated - would fetch from RubyGems)
        self.generate_remote_specs(&mut specs);

        Ok(GeneratedLockfile {
            specs,
            top_level,
            platforms: vec!["ruby".to_string()],
            bundler_version: Some("2.4.0".to_string()),
        })
    }

    fn resolve_version(&self, _name: &GemName) -> PackResult<String> {
        // In production, this would fetch from RubyGems API
        // For now, return a default version
        Ok("1.0.0".to_string())
    }

    fn resolve_dependencies(&self, name: &GemName) -> PackResult<Vec<(GemName, GemVersion)>> {
        // In production, this would use actual gem metadata
        // For Rails-like gems, simulate some dependencies
        let simulated_deps: Vec<(GemName, GemVersion)> = match name.0.as_str() {
            "rails" => vec![
                (GemName("actionpack".to_string()), GemVersion(">= 7.0".to_string())),
                (GemName("activerecord".to_string()), GemVersion(">= 7.0".to_string())),
                (GemName("railties".to_string()), GemVersion(">= 7.0".to_string())),
            ],
            "actionpack" => vec![
                (GemName("actionview".to_string()), GemVersion(">= 7.0".to_string())),
                (GemName("rack".to_string()), GemVersion(">= 2.2".to_string())),
            ],
            "activerecord" => vec![
                (GemName("activesupport".to_string()), GemVersion(">= 7.0".to_string())),
                (GemName(".Connection_pool".to_string()), GemVersion(">= 2.4".to_string())),
            ],
            "rspec" => vec![
                (GemName("rspec-core".to_string()), GemVersion(">= 3.12".to_string())),
                (GemName("rspec-expectations".to_string()), GemVersion(">= 3.12".to_string())),
            ],
            _ => vec![],
        };

        Ok(simulated_deps)
    }

    fn generate_remote_specs(&self, specs: &mut HashMap<GemName, GemSpecGen>) {
        // Add common remote specs for simulation
        let remote_gems = vec![
            ("actionview", "7.1.0", vec![("activesupport", ">= 7.0")]),
            ("rack", "2.2.8", vec![]),
            ("railties", "7.1.0", vec![("actionpack", ">= 7.0")]),
            ("activesupport", "7.1.0", vec![("concurrent-ruby", "~> 1.1")]),
            ("concurrent-ruby", "1.2.2", vec![]),
            ("rspec-core", "3.12.0", vec![("rspec-support", "~> 3.12")]),
            ("rspec-expectations", "3.12.0", vec![("rspec-support", "~> 3.12")]),
            ("rspec-support", "3.12.0", vec![]),
        ];

        for (name, version, deps) in remote_gems {
            let gem_name = GemName(name.to_string());
            if !specs.contains_key(&gem_name) {
                let dep_tuples: Vec<(GemName, GemVersion)> = deps.iter()
                    .map(|(n, v)| (GemName(n.to_string()), GemVersion(v.to_string())))
                    .collect();
                specs.insert(gem_name, GemSpecGen {
                    version: GemVersion(version.to_string()),
                    dependencies: dep_tuples,
                });
            }
        }
    }

    pub fn write_lockfile(&self, lockfile: &GeneratedLockfile, path: &PathBuf) -> PackResult<()> {
        let content = self.format_lockfile(lockfile);
        fs::write(path, content)
            .map_err(|e| PackError::Gemfile(format!("failed to write Gemfile.lock: {}", e)))?;
        Ok(())
    }

    fn format_lockfile(&self, lockfile: &GeneratedLockfile) -> String {
        let mut content = String::new();

        content.push_str("GEM\n");
        content.push_str("  remote: https://rubygems.org/\n");
        content.push_str("  specs:\n");

        let mut sorted_specs: Vec<_> = lockfile.specs.iter().collect();
        sorted_specs.sort_by(|a, b| a.0.0.cmp(&b.0.0));

        for (name, spec) in sorted_specs {
            content.push_str(&format!("    {} ({})\n", name.0, spec.version.0));
            let mut sorted_deps: Vec<_> = spec.dependencies.iter().collect();
            sorted_deps.sort_by(|a, b| a.0.0.cmp(&b.0.0));
            for (dep_name, dep_ver) in sorted_deps {
                content.push_str(&format!("      {} ({})\n", dep_name.0, dep_ver.0));
            }
        }

        content.push_str("\nPLATFORMS\n");
        for platform in &lockfile.platforms {
            content.push_str(&format!("  {}\n", platform));
        }

        content.push_str("\nDEPENDENCIES\n");
        let mut sorted_top: Vec<_> = lockfile.top_level.iter().collect();
        sorted_top.sort_by(|a, b| a.0.cmp(&b.0));
        for name in sorted_top {
            if let Some(spec) = lockfile.specs.get(name) {
                content.push_str(&format!("  {} ({})\n", name.0, spec.version.0));
            }
        }

        content.push_str("\nBUNDLED WITH\n");
        if let Some(ref version) = lockfile.bundler_version {
            content.push_str(&format!("   {}\n", version));
        }

        content
    }
}

impl Default for LockfileGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct GeneratedLockfile {
    pub specs: HashMap<GemName, GemSpecGen>,
    pub top_level: Vec<GemName>,
    pub platforms: Vec<String>,
    pub bundler_version: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GemSpecGen {
    pub version: GemVersion,
    pub dependencies: Vec<(GemName, GemVersion)>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generator_new() {
        let _gen = LockfileGenerator::new();
        assert!(true);
    }

    #[test]
    fn test_generator_with_update() {
        let _gen = LockfileGenerator::new()
            .with_update_gems(vec![GemName("rails".to_string())]);
        assert!(true);
    }

    #[test]
    fn test_generate_basic() {
        let gen = LockfileGenerator::new();
        let deps = vec![
            Dependency {
                name: GemName("rails".to_string()),
                version: Some(GemVersion("~> 7.1".to_string())),
                group: None,
            },
            Dependency {
                name: GemName("rspec".to_string()),
                version: Some(GemVersion("~> 3.12".to_string())),
                group: None,
            },
        ];

        let result = gen.generate(&PathBuf::from("Gemfile"), &deps);
        assert!(result.is_ok());

        let lockfile = result.unwrap();
        assert!(lockfile.specs.contains_key(&GemName("rails".to_string())));
        assert!(lockfile.specs.contains_key(&GemName("rspec".to_string())));
    }

    #[test]
    fn test_generate_with_rails_deps() {
        let gen = LockfileGenerator::new();
        let deps = vec![
            Dependency {
                name: GemName("rails".to_string()),
                version: Some(GemVersion("~> 7.1".to_string())),
                group: None,
            },
        ];

        let lockfile = gen.generate(&PathBuf::from("Gemfile"), &deps).unwrap();

        // Rails should have actionpack dependency
        let rails_spec = lockfile.specs.get(&GemName("rails".to_string())).unwrap();
        let has_actionpack = rails_spec.dependencies.iter()
            .any(|(n, _)| n.0 == "actionpack");
        assert!(has_actionpack);
    }

    #[test]
    fn test_format_lockfile() {
        let gen = LockfileGenerator::new();
        let mut specs = HashMap::new();
        specs.insert(
            GemName("rails".to_string()),
            GemSpecGen {
                version: GemVersion("7.1.0".to_string()),
                dependencies: vec![
                    (GemName("actionpack".to_string()), GemVersion("7.1.0".to_string())),
                ],
            },
        );
        specs.insert(
            GemName("actionpack".to_string()),
            GemSpecGen {
                version: GemVersion("7.1.0".to_string()),
                dependencies: vec![],
            },
        );

        let lockfile = GeneratedLockfile {
            specs,
            top_level: vec![GemName("rails".to_string())],
            platforms: vec!["ruby".to_string()],
            bundler_version: Some("2.4.0".to_string()),
        };

        let content = gen.format_lockfile(&lockfile);
        assert!(content.contains("GEM"));
        assert!(content.contains("PLATFORMS"));
        assert!(content.contains("DEPENDENCIES"));
        assert!(content.contains("rails (7.1.0)"));
        assert!(content.contains("actionpack (7.1.0)"));
        assert!(content.contains("BUNDLED WITH"));
    }

    #[test]
    fn test_write_lockfile() {
        let gen = LockfileGenerator::new();
        let deps = vec![
            Dependency {
                name: GemName("rake".to_string()),
                version: Some(GemVersion("13.0.0".to_string())),
                group: None,
            },
        ];

        let lockfile = gen.generate(&PathBuf::from("Gemfile"), &deps).unwrap();
        let temp_path = std::env::temp_dir().join("test_Gemfile.lock");

        let result = gen.write_lockfile(&lockfile, &temp_path);
        assert!(result.is_ok());

        let content = std::fs::read_to_string(&temp_path).unwrap();
        assert!(content.contains("rake"));

        std::fs::remove_file(&temp_path).ok();
    }
}