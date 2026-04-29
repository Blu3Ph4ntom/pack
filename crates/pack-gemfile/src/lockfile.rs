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

pub struct GemSpec {
    pub version: GemVersion,
    pub dependencies: Vec<GemName>,
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
