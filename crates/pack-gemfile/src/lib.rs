//! Gemfile and Gemfile.lock parsing.

use pack_core::{Dependency, GemName, GemVersion, PackError, PackResult};
use std::path::PathBuf;

pub mod generate;
pub mod lockfile;
pub mod pack_lock;
pub mod packfile;

pub use generate::{GemSpecGen, GeneratedLockfile, LockfileGenerator};
pub use lockfile::{find_dependency_path, load_lockfile, GemSpec, Lockfile};
pub use pack_lock::{LockedGem, PackLock, PackLockMetadata};
pub use packfile::{Packfile, PackfileTask};

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
    let mut in_group = false;
    let mut current_group: Option<String> = None;

    for line in content.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("group ") {
            in_group = true;
            let rest = trimmed.strip_prefix("group").unwrap().trim();
            if let Some(end) = rest.find(" do") {
                let group_name = rest[..end].trim();
                let clean_group = group_name.strip_prefix(':').unwrap_or(group_name);
                current_group = Some(clean_group.to_string());
            } else {
                current_group = Some(rest.trim_end_matches(':').to_string());
            }
        } else if trimmed == "end" && in_group {
            in_group = false;
            current_group = None;
        } else if trimmed.starts_with("gem ")
            || trimmed.starts_with("gem'")
            || trimmed.starts_with("gem\"")
        {
            if let Some(dep) = parse_gem_line_with_group(trimmed, current_group.as_deref()) {
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
                let group_name = rest[..end].trim();
                let clean_group = group_name.strip_prefix(':').unwrap_or(group_name);
                current_group = Some(clean_group.to_string());
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

fn parse_gem_line_with_group(line: &str, group: Option<&str>) -> Option<Dependency> {
    let content = line.strip_prefix("gem")?.trim();

    let (name, rest) = if let Some(stripped) = content.strip_prefix('\'') {
        if let Some(end) = stripped.find('\'') {
            (&content[1..end + 1], &content[end + 2..])
        } else {
            return None;
        }
    } else if let Some(stripped) = content.strip_prefix('"') {
        if let Some(end) = stripped.find('"') {
            (&content[1..end + 1], &content[end + 2..])
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

    let gem_line = build_gem_line(name, version, group);

    let new_content = if content.ends_with('\n') {
        format!("{}{}", content, gem_line)
    } else {
        format!("{}\n{}", content, gem_line)
    };

    std::fs::write(path, new_content)
        .map_err(|e| PackError::Gemfile(format!("failed to write Gemfile: {}", e)))?;

    Ok(())
}

fn build_gem_line(name: &str, version: Option<&str>, group: Option<&str>) -> String {
    if let Some(v) = version {
        if let Some(g) = group {
            format!("gem \"{}\", \"~> {}\"  # {}", name, v, g)
        } else {
            format!("gem \"{}\", \"~> {}\"", name, v)
        }
    } else {
        if let Some(g) = group {
            format!("gem '{}'  # {}", name, g)
        } else {
            format!("gem '{}'", name)
        }
    }
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

pub fn update_gem(path: &PathBuf, name: &str, new_version: &str) -> PackResult<bool> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackError::Gemfile(format!("failed to read Gemfile: {}", e)))?;

    let mut updated = false;
    let mut new_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        let is_target = trimmed.starts_with(&format!("gem \"{}\"", name))
            || trimmed.starts_with(&format!("gem '{}'", name))
            || trimmed.starts_with(&format!("gem {}", name));

        if is_target && !updated {
            let new_line = format!("gem \"{}\", \"~> {}\"", name, new_version);
            new_lines.push(new_line);
            updated = true;
        } else {
            new_lines.push(line.to_string());
        }
    }

    if updated {
        let new_content = new_lines.join("\n");
        std::fs::write(path, new_content)
            .map_err(|e| PackError::Gemfile(format!("failed to write Gemfile: {}", e)))?;
    }

    Ok(updated)
}

pub fn find_gem(path: &PathBuf, name: &str) -> PackResult<Option<String>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackError::Gemfile(format!("failed to read Gemfile: {}", e)))?;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(&format!("gem \"{}\"", name))
            || trimmed.starts_with(&format!("gem '{}'", name))
            || trimmed.starts_with(&format!("gem {}", name))
        {
            return Ok(Some(trimmed.to_string()));
        }
    }

    Ok(None)
}

pub fn list_gems(path: &PathBuf) -> PackResult<Vec<(String, Option<String>)>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| PackError::Gemfile(format!("failed to read Gemfile: {}", e)))?;

    let deps = parse_gemfile(&content)?;
    Ok(deps
        .into_iter()
        .map(|d| (d.name.0, d.version.map(|v| v.0)))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_gems() {
        let content = r#"
source 'https://rubygems.org'

gem 'rake'
gem 'rspec'
"#;
        let deps = parse_gemfile(content).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name.0, "rake");
        assert_eq!(deps[1].name.0, "rspec");
    }

    #[test]
    fn test_parse_gems_with_version() {
        let content = r#"
gem 'rails', '~> 7.1.0'
gem 'puma'
"#;
        let deps = parse_gemfile(content).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name.0, "rails");
        assert_eq!(deps[0].version.as_ref().unwrap().0, "~> 7.1.0");
        assert_eq!(deps[1].name.0, "puma");
    }

    #[test]
    fn test_parse_double_quoted_gems() {
        let content = r#"
gem "rake"
gem "rspec", "~> 3.12"
"#;
        let deps = parse_gemfile(content).unwrap();
        assert_eq!(deps.len(), 2);
        assert_eq!(deps[0].name.0, "rake");
        assert_eq!(deps[1].name.0, "rspec");
        assert_eq!(deps[1].version.as_ref().unwrap().0, "~> 3.12");
    }

    #[test]
    fn test_parse_groups() {
        let content = r#"
source 'https://rubygems.org'

gem 'rake'

group :test do
  gem 'rspec'
  gem 'factory_bot'
end
"#;
        let deps = parse_gemfile(content).unwrap();
        assert_eq!(deps.len(), 3);

        let test_gems: Vec<_> = deps
            .iter()
            .filter(|d| d.group.as_deref() == Some("test"))
            .collect();
        assert_eq!(test_gems.len(), 2);
    }

    #[test]
    fn test_add_gem() {
        let temp_dir = std::env::temp_dir();
        let gemfile_path = temp_dir.join("test_add_gem_Gemfile");
        std::fs::write(&gemfile_path, "gem 'rake'\n").unwrap();

        add_gem(&gemfile_path, "rspec", None, None).unwrap();

        let content = std::fs::read_to_string(&gemfile_path).unwrap();
        assert!(content.contains("gem 'rspec'"));

        std::fs::remove_file(&gemfile_path).ok();
    }

    #[test]
    fn test_add_gem_with_version() {
        let temp_dir = std::env::temp_dir();
        let gemfile_path = temp_dir.join("test_add_version_Gemfile");
        std::fs::write(&gemfile_path, "gem 'rake'\n").unwrap();

        add_gem(&gemfile_path, "rails", Some("7.1.0"), None).unwrap();

        let content = std::fs::read_to_string(&gemfile_path).unwrap();
        assert!(content.contains("gem \"rails\", \"~> 7.1.0\""));

        std::fs::remove_file(&gemfile_path).ok();
    }

    #[test]
    fn test_remove_gem() {
        let temp_dir = std::env::temp_dir();
        let gemfile_path = temp_dir.join("test_remove_gem_Gemfile");
        std::fs::write(&gemfile_path, "gem 'rake'\ngem 'rspec'\n").unwrap();

        let removed = remove_gem(&gemfile_path, "rake").unwrap();
        assert!(removed);

        let content = std::fs::read_to_string(&gemfile_path).unwrap();
        assert!(!content.contains("rake"));
        assert!(content.contains("rspec"));

        std::fs::remove_file(&gemfile_path).ok();
    }

    #[test]
    fn test_remove_gem_not_found() {
        let temp_dir = std::env::temp_dir();
        let gemfile_path = temp_dir.join("test_remove_missing_Gemfile");
        std::fs::write(&gemfile_path, "gem 'rake'\n").unwrap();

        let removed = remove_gem(&gemfile_path, "nonexistent").unwrap();
        assert!(!removed);

        std::fs::remove_file(&gemfile_path).ok();
    }
}
