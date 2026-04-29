//! Gemfile.lock parsing.

use pack_core::{GemName, GemVersion, PackError, PackResult};
use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;

pub struct Lockfile {
    pub path: PathBuf,
    pub content: String,
    pub specs: HashMap<GemName, GemSpec>,
    pub top_level: Vec<GemName>,
}

impl Lockfile {
    pub fn get_spec(&self, name: &GemName) -> Option<&GemSpec> {
        self.specs.get(name)
    }

    pub fn has_gem(&self, name: &GemName) -> bool {
        self.specs.contains_key(name)
    }

    pub fn gem_count(&self) -> usize {
        self.specs.len()
    }

    pub fn top_level_gem_count(&self) -> usize {
        self.top_level.len()
    }

    pub fn get_all_gem_names(&self) -> Vec<&GemName> {
        self.specs.keys().collect()
    }

    pub fn find_gems_with_dep(&self, dep: &GemName) -> Vec<&GemName> {
        self.specs
            .iter()
            .filter(|(_, spec)| spec.dependencies.contains(dep))
            .map(|(name, _)| name)
            .collect()
    }
}

pub struct GemSpec {
    pub version: GemVersion,
    pub dependencies: Vec<GemName>,
}

impl GemSpec {
    pub fn new(version: GemVersion) -> Self {
        Self {
            version,
            dependencies: Vec::new(),
        }
    }

    pub fn with_dep(mut self, dep: GemName) -> Self {
        self.dependencies.push(dep);
        self
    }

    pub fn add_dep(&mut self, dep: GemName) {
        if !self.dependencies.contains(&dep) {
            self.dependencies.push(dep);
        }
    }

    pub fn has_dependency(&self, name: &GemName) -> bool {
        self.dependencies.contains(name)
    }

    pub fn dep_count(&self) -> usize {
        self.dependencies.len()
    }
}

pub fn load_lockfile(path: &PathBuf) -> PackResult<Lockfile> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackError::Gemfile(format!("failed to read Gemfile.lock: {}", e)))?;

    let specs = parse_specs(&content)?;
    let top_level = parse_top_level(&content);

    Ok(Lockfile {
        path: path.clone(),
        content,
        specs,
        top_level,
    })
}

fn parse_top_level(content: &str) -> Vec<GemName> {
    let mut gems = Vec::new();
    let mut in_deps = false;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "DEPENDENCIES" {
            in_deps = true;
            continue;
        }

        if trimmed == "GEM" || trimmed == "PLATFORMS" || trimmed.starts_with("BUNDLED WITH") {
            in_deps = false;
        }

        if in_deps && !trimmed.is_empty() {
            let name = trimmed
                .split_whitespace()
                .next()
                .unwrap_or(trimmed)
                .split('(')
                .next()
                .unwrap_or(trimmed)
                .trim()
                .to_string();

            if !name.is_empty() {
                gems.push(GemName(name));
            }
        }
    }

    gems
}

fn parse_specs(content: &str) -> PackResult<HashMap<GemName, GemSpec>> {
    let mut specs = HashMap::new();
    let mut in_specs = false;
    let mut current_name: Option<GemName> = None;
    let mut current_version: Option<GemVersion> = None;
    let mut current_deps: Vec<GemName> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed == "specs:" {
            in_specs = true;
            continue;
        }

        if trimmed == "PLATFORMS"
            || trimmed == "DEPENDENCIES"
            || trimmed.starts_with("BUNDLED WITH")
        {
            if in_specs {
                if let Some(name) = current_name.take() {
                    specs.insert(
                        name,
                        GemSpec {
                            version: current_version
                                .take()
                                .unwrap_or(GemVersion("unknown".to_string())),
                            dependencies: mem::take(&mut current_deps),
                        },
                    );
                }
            }
            in_specs = false;
        }

        if in_specs {
            if trimmed.is_empty() {
                continue;
            }

            let leading_spaces = line.len() - trimmed.len();
            let is_gem_spec = leading_spaces == 4;
            let is_dependency = leading_spaces >= 6;

            if is_gem_spec {
                if let Some(name) = current_name.take() {
                    specs.insert(
                        name,
                        GemSpec {
                            version: current_version
                                .take()
                                .unwrap_or(GemVersion("unknown".to_string())),
                            dependencies: mem::take(&mut current_deps),
                        },
                    );
                }

                let (name, version) = parse_spec_name_version(trimmed);
                if let Some(n) = name {
                    current_name = Some(n);
                    current_version = version;
                }
            } else if is_dependency {
                if let Some(ref name) = current_name {
                    let dep_name = parse_dep_line(line, trimmed);
                    if let Some(ref dn) = dep_name {
                        if dn != name {
                            current_deps.push(dn.clone());
                        }
                    }
                }
            }
        }
    }

    if let Some(name) = current_name.take() {
        specs.insert(
            name,
            GemSpec {
                version: current_version
                    .take()
                    .unwrap_or(GemVersion("unknown".to_string())),
                dependencies: current_deps,
            },
        );
    }

    Ok(specs)
}

fn parse_spec_name_version(line: &str) -> (Option<GemName>, Option<GemVersion>) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return (None, None);
    }

    if trimmed == "specs:" {
        return (None, None);
    }

    let (name, rest) = if let Some(paren) = trimmed.find(" (") {
        (&trimmed[..paren], Some(&trimmed[paren..]))
    } else if let Some(stripped) = trimmed.strip_suffix(" ()") {
        (stripped, None)
    } else if let Some(paren_pos) = trimmed.find('(') {
        let name = &trimmed[..paren_pos];
        let rest = &trimmed[paren_pos..];
        if rest.ends_with(")") && !rest.contains("=") {
            (name, Some(rest))
        } else {
            (trimmed, None)
        }
    } else {
        (trimmed, None)
    };

    let name = name.trim();
    if name.is_empty()
        || name == "PLATFORMS"
        || name == "DEPENDENCIES"
        || name == "BUNDLED WITH"
        || name == "GEM"
        || name == "specs:"
    {
        return (None, None);
    }

    let version = rest.map(|r| {
        let v = r.trim_start_matches(" (").trim_end_matches(")").trim();
        GemVersion(v.to_string())
    });

    (Some(GemName(name.to_string())), version)
}

fn parse_dep_line(line: &str, trimmed: &str) -> Option<GemName> {
    if trimmed.is_empty() {
        return None;
    }

    if line.starts_with("  ") {
        let content = trimmed;
        if content.starts_with("dep ") || content.starts_with("require ") {
            let rest = content
                .strip_prefix("dep ")
                .or_else(|| content.strip_prefix("require "))
                .unwrap_or(content);

            let name = rest
                .trim()
                .trim_matches(|c| c == '"' || c == '\'' || c == '(' || c == ')')
                .split_whitespace()
                .next()
                .unwrap_or(rest.trim())
                .split('(')
                .next()
                .unwrap_or(rest.trim())
                .to_string();

            if !name.is_empty() {
                return Some(GemName(name));
            }
        } else {
            let name = content
                .split_whitespace()
                .next()
                .unwrap_or(content)
                .split('(')
                .next()
                .unwrap_or(content)
                .trim()
                .to_string();

            if !name.is_empty() && !name.starts_with("(") {
                return Some(GemName(name));
            }
        }
    }

    None
}

pub fn find_dependency_path(lockfile: &Lockfile, target: &GemName) -> Option<Vec<GemName>> {
    if lockfile.specs.contains_key(target) {
        for top in &lockfile.top_level {
            if let Some(path) = find_path_recursive(lockfile, target, top, &mut Vec::new()) {
                return Some(path);
            }
        }
        return Some(vec![target.clone()]);
    }

    for top in &lockfile.top_level {
        if let Some(path) = find_path_recursive(lockfile, target, top, &mut Vec::new()) {
            return Some(path);
        }
    }

    None
}

fn find_path_recursive(
    lockfile: &Lockfile,
    target: &GemName,
    current: &GemName,
    visited: &mut Vec<GemName>,
) -> Option<Vec<GemName>> {
    if visited.contains(current) {
        return None;
    }

    visited.push(current.clone());

    if current == target {
        return Some(vec![current.clone()]);
    }

    if let Some(spec) = lockfile.specs.get(current) {
        for dep in &spec.dependencies {
            if let Some(mut path) = find_path_recursive(lockfile, target, dep, visited) {
                path.insert(0, current.clone());
                return Some(path);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_LOCKFILE: &str = r#"GEM
  remote: https://rubygems.org/
  specs:
    actionpack (7.1.0)
      actionview (= 7.1.0)
      activesupport (= 7.1.0)
      rack (>= 2.2.4)
    actionview (7.1.0)
      activesupport (= 7.1.0)
    activesupport (7.1.0)
      concurrent-ruby (~> 1.0)
    concurrent-ruby (1.2.2)
    rack (2.2.8)
    rails (7.1.0)
      actionpack (= 7.1.0)
      bundler (>= 1.15.0)

PLATFORMS
  ruby

DEPENDENCIES
  rails (~> 7.1.0)

BUNDLED WITH
   2.4.0
"#;

    #[test]
    fn test_parse_top_level() {
        let top_level = parse_top_level(TEST_LOCKFILE);
        assert_eq!(top_level.len(), 1);
        assert_eq!(top_level[0].0, "rails");
    }

    #[test]
    fn test_parse_specs() {
        let specs = parse_specs(TEST_LOCKFILE).unwrap();
        assert_eq!(specs.len(), 6);

        assert!(specs.contains_key(&GemName("rails".to_string())));
        assert!(specs.contains_key(&GemName("actionpack".to_string())));
        assert!(specs.contains_key(&GemName("rack".to_string())));
    }

    #[test]
    fn test_rails_dependencies() {
        let specs = parse_specs(TEST_LOCKFILE).unwrap();
        let rails_spec = specs.get(&GemName("rails".to_string())).unwrap();
        assert_eq!(rails_spec.version.0, "7.1.0");
        assert_eq!(rails_spec.dependencies.len(), 2);
        let dep_names: Vec<_> = rails_spec.dependencies.iter().map(|d| d.0.clone()).collect();
        assert!(dep_names.contains(&"actionpack".to_string()));
        assert!(dep_names.contains(&"bundler".to_string()));
    }

    #[test]
    fn test_actionpack_dependencies() {
        let specs = parse_specs(TEST_LOCKFILE).unwrap();
        let actionpack_spec = specs.get(&GemName("actionpack".to_string())).unwrap();
        assert_eq!(actionpack_spec.version.0, "7.1.0");
        let dep_names: Vec<_> = actionpack_spec.dependencies.iter().map(|d| d.0.clone()).collect();
        assert!(dep_names.contains(&"actionview".to_string()));
        assert!(dep_names.contains(&"activesupport".to_string()));
        assert!(dep_names.contains(&"rack".to_string()));
    }

    #[test]
    fn test_find_dependency_path_direct() {
        let specs = parse_specs(TEST_LOCKFILE).unwrap();
        let top_level = parse_top_level(TEST_LOCKFILE);
        let lockfile = Lockfile {
            path: PathBuf::from("/fake"),
            content: TEST_LOCKFILE.to_string(),
            specs,
            top_level,
        };

        let path = find_dependency_path(&lockfile, &GemName("rails".to_string()));
        assert!(path.is_some());
        assert_eq!(path.unwrap(), vec![GemName("rails".to_string())]);
    }

    #[test]
    fn test_find_dependency_path_nested() {
        let specs = parse_specs(TEST_LOCKFILE).unwrap();
        let top_level = parse_top_level(TEST_LOCKFILE);
        let lockfile = Lockfile {
            path: PathBuf::from("/fake"),
            content: TEST_LOCKFILE.to_string(),
            specs,
            top_level,
        };

        let path = find_dependency_path(&lockfile, &GemName("rack".to_string()));
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.len(), 3);
        assert_eq!(path[0].0, "rails");
        assert_eq!(path[1].0, "actionpack");
        assert_eq!(path[2].0, "rack");
    }

    #[test]
    fn test_find_dependency_path_not_found() {
        let specs = parse_specs(TEST_LOCKFILE).unwrap();
        let top_level = parse_top_level(TEST_LOCKFILE);
        let lockfile = Lockfile {
            path: PathBuf::from("/fake"),
            content: TEST_LOCKFILE.to_string(),
            specs,
            top_level,
        };

        let path = find_dependency_path(&lockfile, &GemName("nonexistent".to_string()));
        assert!(path.is_none());
    }

    #[test]
    fn test_load_lockfile_from_content() {
        let specs = parse_specs(TEST_LOCKFILE).unwrap();
        let top_level = parse_top_level(TEST_LOCKFILE);
        let lockfile = Lockfile {
            path: PathBuf::from("/fake"),
            content: TEST_LOCKFILE.to_string(),
            specs,
            top_level,
        };

        assert_eq!(lockfile.specs.len(), 6);
        assert_eq!(lockfile.top_level.len(), 1);
    }
}
