//! Gemfile and Gemfile.lock parsing.

use pack_core::{GemName, GemVersion, Dependency, PackResult};

pub fn find_gemfile(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let gemfile_path = path.join("Gemfile");
    if gemfile_path.exists() {
        Some(gemfile_path)
    } else {
        None
    }
}

pub fn find_gemfile_lock(path: &std::path::Path) -> Option<std::path::PathBuf> {
    let lockfile_path = path.join("Gemfile.lock");
    if lockfile_path.exists() {
        Some(lockfile_path)
    } else {
        None
    }
}

pub fn parse_gemfile(content: &str) -> PackResult<Vec<Dependency>> {
    let mut deps = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("gem ") || trimmed.starts_with("gem'") || trimmed.starts_with("gem\"") {
            if let Some(dep) = parse_gem_line(trimmed) {
                deps.push(dep);
            }
        }
    }

    Ok(deps)
}

fn parse_gem_line(line: &str) -> Option<Dependency> {
    let content = line.strip_prefix("gem")?.trim();

    let (name, rest) = if content.starts_with("'") {
        let end = content[1..].find('\'')?;
        (&content[1..end], &content[end+1..])
    } else if content.starts_with("\"") {
        let end = content[1..].find('\"')?;
        (&content[1..end], &content[end+1..])
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
        group: None,
    })
}

fn parse_version_from_rest(rest: &str) -> Option<GemVersion> {
    let rest = rest.trim_start_matches(',').trim();

    if rest.starts_with(',') {
        let rest = rest.trim_start_matches(',').trim();
        if rest.is_empty() || rest.starts_with("group") {
            return None;
        }
    }

    if rest.starts_with("version:") || rest.starts_with("\"~>") || rest.starts_with("'~>") {
        return None;
    }

    None
}
