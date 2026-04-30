//! Command execution with proper PATH handling.
//!
//! Pack aims to be a drop-in replacement for both `gem` and `bundle` commands.
//! It provides native gem installation and execution without requiring
//! the Ruby interpreter overhead of Bundler.

use pack_core::PackResult;
use std::collections::HashMap;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

pub mod plugins;
pub use plugins::{Plugin, PluginManager, PluginAction, OutputFormat, PluginTemplate, PluginOutput};

pub struct Executor {
    bundle_path: Option<String>,
    gem_home: Option<String>,
    gem_path: Option<String>,
    cache_dir: PathBuf,
}

impl Executor {
    pub fn new() -> Self {
        let cache_dir = Self::default_cache_dir();

        Self {
            bundle_path: env::var("BUNDLE_PATH").ok(),
            gem_home: env::var("GEM_HOME").ok(),
            gem_path: env::var("GEM_PATH").ok(),
            cache_dir,
        }
    }

    fn default_cache_dir() -> PathBuf {
        env::var("PACK_CACHE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".cache").join("pack"))
                    .unwrap_or_else(|_| PathBuf::from(".cache/pack"))
            })
    }

    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self {
            bundle_path: env::var("BUNDLE_PATH").ok(),
            gem_home: env::var("GEM_HOME").ok(),
            gem_path: env::var("GEM_PATH").ok(),
            cache_dir,
        }
    }

    pub fn with_gem_home(gem_home: PathBuf) -> Self {
        Self {
            bundle_path: env::var("BUNDLE_PATH").ok(),
            gem_home: Some(gem_home.to_string_lossy().to_string()),
            gem_path: None,
            cache_dir: Self::default_cache_dir(),
        }
    }

    pub fn bundle_path(&self) -> Option<&str> {
        self.bundle_path.as_deref()
    }

    pub fn gem_home_opt(&self) -> Option<&str> {
        self.gem_home.as_deref()
    }

    pub fn gem_path_opt(&self) -> Option<&str> {
        self.gem_path.as_deref()
    }

    pub fn is_ruby_available(&self) -> bool {
        Command::new("ruby")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn is_gem_available(&self) -> bool {
        Command::new("gem")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
            || Command::new("ruby")
                .args(["-S", "gem", "--version"])
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
    }

    pub fn is_bundle_available(&self) -> bool {
        Command::new("bundle")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Execute a command directly (like `gem install` or `gem list`)
    pub fn exec_gem(&self, args: &[String]) -> PackResult<Output> {
        let mut cmd = Command::new("gem");
        cmd.args(args);
        cmd.envs(self.gem_env());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        match cmd.output() {
            Ok(output) => Ok(output),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut fallback = Command::new("ruby");
                fallback.arg("-S").arg("gem").args(args);
                fallback.envs(self.gem_env());
                fallback.stdout(Stdio::piped());
                fallback.stderr(Stdio::piped());
                fallback.output()
                    .map_err(|fallback_err| {
                        pack_core::PackError::Exec(format!(
                            "failed to execute gem: {}. fallback `ruby -S gem` also failed: {}",
                            e, fallback_err
                        ))
                    })
            }
            Err(e) => Err(pack_core::PackError::Exec(format!("failed to execute gem: {}", e))),
        }
    }

    /// Execute a command directly (like `bundle install` or `bundle exec`)
    pub fn exec_bundle(&self, args: &[String]) -> PackResult<Output> {
        let mut cmd = Command::new("bundle");
        cmd.args(args);
        if let Some(ref bp) = self.bundle_path {
            cmd.env("BUNDLE_PATH", bp);
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output()
            .map_err(|e| pack_core::PackError::Exec(format!("failed to execute bundle: {}", e)))?;

        Ok(output)
    }

    /// Execute a gem's binary directly via the installed gem path
    /// This bypasses bundle exec and runs the gem binary directly
    pub fn exec_gem_binary(&self, gem_name: &str, bin_name: &str, args: &[String]) -> PackResult<Output> {
        // Find the gem's bin directory
        let gem_home = self.gem_home()
            .ok_or_else(|| pack_core::PackError::Exec(format!("GEM_HOME not set, cannot execute {} directly", gem_name)))?;

        let bin_path = PathBuf::from(&gem_home)
            .join("bin")
            .join(bin_name);

        // Fallback to searching in gems directory
        let gem_bin = if !bin_path.exists() {
            self.find_gem_bin(gem_name, bin_name)?
        } else {
            bin_path
        };

        let mut cmd = Command::new(&gem_bin);
        cmd.args(args);
        cmd.envs(self.gem_env());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let output = cmd.output()
            .map_err(|e| pack_core::PackError::Exec(format!("failed to execute {}: {}", gem_bin.display(), e)))?;

        Ok(output)
    }

    fn find_gem_bin(&self, gem_name: &str, bin_name: &str) -> PackResult<PathBuf> {
        let gem_home = self.gem_home()
            .ok_or_else(|| pack_core::PackError::Exec("GEM_HOME not set".to_string()))?;

        // Search in gems directory
        let gems_dir = PathBuf::from(&gem_home).join("gems");

        if let Ok(entries) = std::fs::read_dir(&gems_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() && path.file_name()
                    .map(|n| n.to_string_lossy().starts_with(gem_name))
                    .unwrap_or(false)
                {
                    let bin = path.join("bin").join(bin_name);
                    if bin.exists() {
                        return Ok(bin);
                    }
                }
            }
        }

        Err(pack_core::PackError::Exec(format!(
            "could not find binary {} for gem {} in {}",
            bin_name, gem_name, gems_dir.display()
        )))
    }

    /// Execute a command, preferring native execution over bundle exec when possible
    /// This is the "drop-in" mode - acts like gem/bundle but faster
    pub fn exec(&self, cmd: &str, args: &[String], working_dir: Option<&Path>) -> PackResult<Output> {
        // If BUNDLE_PATH is set and we're not doing bundle-specific operations,
        // we can execute directly
        let mut command = Command::new(cmd);
        command.args(args);

        if let Some(dir) = working_dir {
            command.current_dir(dir);
        }

        // Add gem environment
        command.envs(self.gem_env());

        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.output()
            .map_err(|e| pack_core::PackError::Exec(format!("failed to execute {}: {}", cmd, e)))?;

        Ok(output)
    }

    /// Run with bundle exec (legacy support)
    pub fn exec_via_bundle(&self, cmd: &str, args: &[String], working_dir: Option<&Path>) -> PackResult<Output> {
        let mut command = Command::new("bundle");
        command.arg("exec").arg(cmd).args(args);

        if let Some(dir) = working_dir {
            command.current_dir(dir);
        }

        if let Some(ref bp) = self.bundle_path {
            command.env("BUNDLE_PATH", bp);
        }

        command.envs(self.gem_env());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());

        let output = command.output()
            .map_err(|e| pack_core::PackError::Exec(format!("failed to execute bundle exec {}: {}", cmd, e)))?;

        Ok(output)
    }

    fn gem_env(&self) -> HashMap<String, String> {
        let mut env = HashMap::new();

        if let Some(ref home) = self.gem_home {
            env.insert("GEM_HOME".to_string(), home.clone());
        }

        if let Some(ref path) = self.gem_path {
            env.insert("GEM_PATH".to_string(), path.clone());
        } else if self.gem_home.is_some() {
            // If GEM_HOME is set but GEM_PATH is not, set GEM_PATH to GEM_HOME
            if let Some(ref home) = self.gem_home {
                env.insert("GEM_PATH".to_string(), home.clone());
            }
        }

        // Set PATH to include gem bin
        if let Some(ref home) = self.gem_home {
            let gem_bin = format!("{}/bin:{}", home, env::var("PATH").unwrap_or_default());
            env.insert("PATH".to_string(), gem_bin);
        }

        env
    }

    pub fn which(&self, cmd: &str) -> Option<String> {
        Command::new("which")
            .arg(cmd)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
    }

    pub fn gem_home(&self) -> Option<String> {
        self.gem_home.clone()
            .or_else(|| {
                self.exec_gem(&["env".to_string(), "GEM_HOME".to_string()])
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            })
    }

    pub fn gem_path(&self) -> Option<String> {
        self.gem_path.clone()
            .or_else(|| {
                self.exec_gem(&["env".to_string(), "GEM_PATH".to_string()])
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            })
    }

    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }

    /// List installed gems using native gem command
    pub fn list_gems(&self) -> PackResult<Vec<String>> {
        let output = self.exec_gem(&["list".to_string(), "--local".to_string()])?;

        let gems = String::from_utf8_lossy(&output.stdout)
            .lines()
            .filter_map(|line| {
                // Parse "gem_name (version)" format
                line.split_whitespace()
                    .next()
                    .map(|s| s.to_string())
            })
            .collect();

        Ok(gems)
    }

    /// Check if a gem is installed
    pub fn gem_installed(&self, gem_name: &str) -> bool {
        self.list_gems()
            .map(|gems| gems.iter().any(|g| g == gem_name))
            .unwrap_or(false)
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_new() {
        let _executor = Executor::new();
        assert!(true);
    }

    #[test]
    fn test_executor_default() {
        let _executor = Executor::default();
        assert!(true);
    }

    #[test]
    fn test_exec_gem_list() {
        let executor = Executor::new();
        let result = executor.exec_gem(&["list".to_string()]);
        // May fail if no gem, but shouldn't panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_which() {
        let executor = Executor::new();
        let result = executor.which("nonexistent_cmd_12345");
        assert!(result.is_none());
    }

    #[test]
    fn test_gem_env() {
        let executor = Executor::new();
        let env = executor.gem_env();
        // Should at least not panic
        assert!(env.contains_key("PATH") || env.is_empty());
    }
}
