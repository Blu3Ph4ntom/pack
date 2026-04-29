//! Gemfile and Gemfile.lock parsing.

use pack_core::{Dependency, GemName, GemVersion, PackError, PackResult};
use std::path::PathBuf;

pub mod lockfile;

pub use lockfile::{find_dependency_path, load_lockfile, GemSpec, Lockfile};

pub struct Gemfile {
    pub path: PathBuf,
    pub content: String,
    pub dependencies: Vec<Dependency>,
    pub groups: Vec<GemGroup>,
}

pub struct GemGroup {
    pub name: String,
    pub gems: Vec<Dependency>,
}

pub fn load_gemfile(path: &PathBuf) -> PackResult<Gemfile> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackError::Gemfile(format!("failed to read Gemfile: {}", e)))?;

    let dependencies = parse_gemfile(&content)?;
    let groups = parse_groups(&content);

    Ok(Gemfile {
        path: path.clone(),
        content,
        dependencies,
        groups,
    })
}

pub fn parse_gemfile(content: &str) -> PackResult<Vec<Dependency>> {
    let mut deps = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("gem ")
            || trimmed.starts_with("gem'")
            || trimmed.starts_with("gem\"")
        {
            if let Some(dep) = parse_gem_line(trimmed) {
                deps.push(dep);
            }
        }
    }

    Ok(deps)
}

fn parse_groups(content: &str) -> Vec<GemGroup> {
    let mut groups = Vec::new();
    let mut current_group: Option<String> = None;
    let mut current_gems: Vec<Dependency> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("group ") {
            if let Some(name) = current_group.take() {
                if !current_gems.is_empty() {
                    groups.push(GemGroup {
                        name,
                        gems: std::mem::take(&mut current_gems),
                    });
                }
            }

            let rest = trimmed.strip_prefix("group").unwrap().trim();
            if let Some(end) = rest.find(" do") {
                current_group = Some(rest[..end].trim().to_string());
            } else {
                current_group = Some(rest.trim_end_matches(':').to_string());
            }
        } else if current_group.is_some() {
            if trimmed.starts_with("gem ")
                || trimmed.starts_with("gem'")
                || trimmed.starts_with("gem\"")
            {
                if let Some(dep) = parse_gem_line_with_group(trimmed, current_group.as_deref()) {
                    current_gems.push(dep);
                }
            } else if trimmed == "end" {
                if let Some(name) = current_group.take() {
                    if !current_gems.is_empty() {
                        groups.push(GemGroup {
                            name,
                            gems: std::mem::take(&mut current_gems),
                        });
                    }
                }
            }
        }
    }

    if let Some(name) = current_group {
        if !current_gems.is_empty() {
            groups.push(GemGroup {
                name,
                gems: current_gems,
            });
        }
    }

    groups
}

fn parse_gem_line(line: &str) -> Option<Dependency> {
    parse_gem_line_with_group(line, None)
}

fn parse_gem_line_with_group(line: &str, group: Option<&str>) -> Option<Dependency> {
    let content = line.strip_prefix("gem")?.trim();

    let (name, rest) = if let Some(stripped) = content.strip_prefix('\'') {
        if let Some(end) = stripped.find('\'') {
            (&content[1..end], &content[end + 1..])
        } else {
            return None;
        }
    } else if let Some(stripped) = content.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            (&content[1..end], &content[end + 1..])
        } else {
            return None;
        }
    } else {
        let end = content.find(|c: char| c.is_whitespace() || c == ',')?;
        (&content[..end], &content[end..])
    };

    let name = name.trim();
    if name.is_empty() {
        return None;
    }

    let version = parse_version_from_rest(rest);

    Some(Dependency {
        name: GemName(name.to_string()),
        version,
        group: group.map(String::from),
    })
}

fn parse_version_from_rest(rest: &str) -> Option<GemVersion> {
    let rest = rest.trim_start_matches(',').trim();

    if rest.is_empty() || rest.starts_with("group") || rest.starts_with("end") {
        return None;
    }

    if rest.starts_with("version:") {
        let rest = rest.strip_prefix("version:").unwrap().trim();
        if (rest.starts_with('"') || rest.starts_with('\'')) && rest.len() > 2 {
            let quote = rest.chars().next()?;
            let rest = &rest[1..];
            if let Some(end) = rest.find(quote) {
                return Some(GemVersion(rest[..end].to_string()));
            }
        }
        return Some(GemVersion(rest.trim().to_string()));
    }

    if (rest.starts_with("\"~>") || rest.starts_with("'~>")) && rest.len() > 4 {
        let quote = rest.chars().next()?;
        let rest = &rest[1..];
        if let Some(end) = rest.find(quote) {
            return Some(GemVersion(rest[..end].to_string()));
        }
    }

    if (rest.starts_with("\"") || rest.starts_with("'")) && rest.len() > 2 {
        let quote = rest.chars().next()?;
        let rest = &rest[1..];
        if let Some(end) = rest.find(quote) {
            return Some(GemVersion(rest[..end].to_string()));
        }
    }

    None
}

pub fn add_gem(
    path: &PathBuf,
    name: &str,
    version: Option<&str>,
    group: Option<&str>,
) -> PackResult<()> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackError::Gemfile(format!("failed to read Gemfile: {}", e)))?;

    let gem_line = if let Some(v) = version {
        if let Some(g) = group {
            format!("gem \"{}\", \"~> {}\"  # {}", name, v, g)
        } else {
            format!("gem \"{}\", \"~> {}\"", name, v)
        }
    } else {
        if let Some(g) = group {
            format!("gem \"{}\"  # {}", name, g)
        } else {
            format!("gem \"{}\"", name)
        }
    };

    let new_content = if content.trim().ends_with('\n') {
        format!("{}{}\n", content, gem_line)
    } else {
        format!("{}\n{}\n", content, gem_line)
    };

    std::fs::write(path, new_content)
        .map_err(|e| PackError::Gemfile(format!("failed to write Gemfile: {}", e)))?;

    Ok(())
}

pub fn remove_gem(path: &PathBuf, name: &str) -> PackResult<bool> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackError::Gemfile(format!("failed to read Gemfile: {}", e)))?;

    let mut removed = false;
    let mut new_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        let is_target = trimmed.starts_with(&format!("gem \"{}\"", name))
            || trimmed.starts_with(&format!("gem '{}'", name))
            || trimmed.starts_with(&format!("gem {}", name));

        if is_target && !removed {
            removed = true;
            continue;
        }
        new_lines.push(line);
    }

    if removed {
        let new_content = new_lines.join("\n");
        std::fs::write(path, new_content)
            .map_err(|e| PackError::Gemfile(format!("failed to write Gemfile: {}", e)))?;
    }

    Ok(removed)
}
