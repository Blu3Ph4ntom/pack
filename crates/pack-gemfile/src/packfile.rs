//! Packfile - Project task runner
//!
//! Packfile lets you define project-specific tasks like dev, build, deploy
//! for Rails and other Ruby projects.
//!
//! Example Packfile:
//! ```toml
//! [tasks.dev]
//! command = "rails server"
//! description = "Start the Rails development server"
//!
//! [tasks.build]
//! command = "rails assets:precompile"
//! description = "Build Rails assets"
//!
//! [tasks.test]
//! command = "rspec"
//! description = "Run tests"
//!
//! [tasks.deploy]
//! command = "cap production deploy"
//! description = "Deploy to production"
//! ```

use pack_core::{PackError, PackResult};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct Packfile {
    pub path: PathBuf,
    pub tasks: HashMap<String, PackfileTask>,
}

#[derive(Debug, Clone)]
pub struct PackfileTask {
    pub name: String,
    pub command: String,
    pub description: Option<String>,
    pub env: HashMap<String, String>,
}

impl Packfile {
    /// Load Packfile from path
    pub fn load(path: &PathBuf) -> PackResult<Self> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| PackError::Gemfile(format!("failed to read Packfile: {}", e)))?;

        Self::parse(&content)
            .map_err(|e| PackError::Gemfile(format!("failed to parse Packfile: {}", e)))
    }

    /// Find Packfile in current directory
    pub fn find() -> PackResult<Option<Self>> {
        let path = std::env::current_dir()
            .map_err(|e| PackError::Gemfile(format!("failed to get current dir: {}", e)))?;

        let packfile_path = path.join("Packfile");
        if packfile_path.exists() {
            Ok(Some(Self::load(&packfile_path)?))
        } else {
            Ok(None)
        }
    }

    /// Create an empty Packfile
    pub fn empty() -> Self {
        Self {
            path: PathBuf::from("Packfile"),
            tasks: HashMap::new(),
        }
    }

    /// Check if Packfile has no tasks
    pub fn is_empty(&self) -> bool {
        self.tasks.is_empty()
    }

    /// Number of tasks in Packfile
    pub fn len(&self) -> usize {
        self.tasks.len()
    }

    /// Check if a task exists
    pub fn has_task(&self, name: &str) -> bool {
        self.tasks.contains_key(name)
    }

    /// Parse Packfile content (TOML format)
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut tasks = HashMap::new();

        // Simple TOML parser for Packfile
        // Format:
        // [tasks.dev]
        // command = "rails server"
        // description = "Start dev server"

        let mut current_task: Option<PackfileTask> = None;

        for line in content.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse section headers like [tasks.dev]
            if line.starts_with('[') && line.ends_with(']') {
                // Save previous task if any
                if let Some(task) = current_task.take() {
                    tasks.insert(task.name.clone(), task);
                }

                let section = &line[1..line.len() - 1];
                if section.starts_with("tasks.") {
                    let task_name = section
                        .strip_prefix("tasks.")
                        .unwrap_or(section)
                        .to_string();
                    current_task = Some(PackfileTask {
                        name: task_name,
                        command: String::new(),
                        description: None,
                        env: HashMap::new(),
                    });
                }
            } else if let Some(ref mut task) = current_task {
                // Parse key = "value" lines
                if let Some(eq_pos) = line.find('=') {
                    let key = line[..eq_pos].trim();
                    let value = line[eq_pos + 1..].trim();

                    // Remove quotes from value
                    let value = if (value.starts_with('"') && value.ends_with('"'))
                        || (value.starts_with('\'') && value.ends_with('\''))
                    {
                        &value[1..value.len() - 1]
                    } else {
                        value
                    };

                    match key {
                        "command" => task.command = value.to_string(),
                        "description" => task.description = Some(value.to_string()),
                        _ => {}
                    }
                }
            }
        }

        // Save last task
        if let Some(task) = current_task {
            tasks.insert(task.name.clone(), task);
        }

        Ok(Self {
            path: PathBuf::from("Packfile"),
            tasks,
        })
    }

    /// Get a task by name
    pub fn get(&self, name: &str) -> Option<&PackfileTask> {
        self.tasks.get(name)
    }

    /// List all task names
    pub fn task_names(&self) -> Vec<&String> {
        self.tasks.keys().collect()
    }

    /// Execute a task
    pub fn run(&self, name: &str) -> PackResult<()> {
        let task = self
            .get(name)
            .ok_or_else(|| PackError::Gemfile(format!("task '{}' not found in Packfile", name)))?;

        println!(
            "Running task '{}': {}",
            name,
            task.description.as_deref().unwrap_or(&task.command)
        );

        // Parse the command (simple shell-like parsing)
        let parts: Vec<&str> = task.command.split_whitespace().collect();
        if parts.is_empty() {
            return Err(PackError::Gemfile("empty command in task".to_string()));
        }

        let mut cmd = Command::new(parts[0]);
        if parts.len() > 1 {
            cmd.args(&parts[1..]);
        }

        // Set environment variables
        for (key, value) in &task.env {
            cmd.env(key, value);
        }

        let status = cmd
            .status()
            .map_err(|e| PackError::Gemfile(format!("failed to execute task '{}': {}", name, e)))?;

        if !status.success() {
            return Err(PackError::Gemfile(format!(
                "task '{}' failed with exit code: {:?}",
                name,
                status.code()
            )));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_packfile() {
        let content = r#"
[tasks.dev]
command = "rails server"
description = "Start dev server"

[tasks.build]
command = "rails assets:precompile"
"#;

        let packfile = Packfile::parse(content).unwrap();
        assert_eq!(packfile.tasks.len(), 2);

        let dev = packfile.get("dev").unwrap();
        assert_eq!(dev.command, "rails server");
        assert_eq!(dev.description.as_deref().unwrap(), "Start dev server");
    }

    #[test]
    fn test_parse_packfile_without_description() {
        let content = r#"
[tasks.test]
command = "rspec"
"#;

        let packfile = Packfile::parse(content).unwrap();
        assert_eq!(packfile.tasks.len(), 1);

        let test = packfile.get("test").unwrap();
        assert_eq!(test.command, "rspec");
        assert!(test.description.is_none());
    }
}
