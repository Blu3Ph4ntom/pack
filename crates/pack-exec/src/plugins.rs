//! Plugin system for Pack extensibility.

use clap::{Subcommand, ValueEnum};
use pack_core::{PackResult, PackError::Exec};
use std::collections::HashMap;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

#[derive(Debug, Clone)]
pub struct Plugin {
    pub name: String,
    pub version: String,
    pub description: String,
    pub commands: Vec<String>,
    pub path: PathBuf,
    pub enabled: bool,
}

impl Plugin {
    pub fn new(name: String, path: PathBuf) -> Self {
        Self {
            name,
            version: "1.0.0".to_string(),
            description: String::new(),
            commands: vec![],
            path,
            enabled: true,
        }
    }

    pub fn with_version(mut self, version: String) -> Self {
        self.version = version;
        self
    }

    pub fn with_description(mut self, description: String) -> Self {
        self.description = description;
        self
    }

    pub fn with_commands(mut self, commands: Vec<String>) -> Self {
        self.commands = commands;
        self
    }

    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn is_executable(&self) -> bool {
        fs::metadata(&self.path)
            .map(|m| m.is_file() && {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    m.permissions().mode() & 0o111 != 0
                }
                #[cfg(not(unix))]
                { true }
            })
            .unwrap_or(false)
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn description(&self) -> &str {
        &self.description
    }

    pub fn commands(&self) -> &[String] {
        &self.commands
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn supports_command(&self, cmd: &str) -> bool {
        self.commands.is_empty() || self.commands.iter().any(|c| c == cmd)
    }

    pub fn execute(&self, args: &[String]) -> PackResult<Output> {
        if !self.is_executable() {
            return Err(Exec(format!(
                "plugin '{}' is not executable: {}",
                self.name, self.path.display()
            )));
        }

        let output = Command::new(&self.path)
            .args(args)
            .output()
            .map_err(|e| Exec(format!("failed to execute plugin {}: {}", self.name, e)))?;

        Ok(output)
    }

    pub fn execute_with_input(&self, args: &[String], input: Option<&str>) -> PackResult<Output> {
        if !self.is_executable() {
            return Err(Exec(format!(
                "plugin '{}' is not executable: {}",
                self.name, self.path.display()
            )));
        }

        let mut cmd = Command::new(&self.path);
        cmd.args(args);

        if let Some(input) = input {
            use std::process::Stdio;
            cmd.stdin(Stdio::piped());
            cmd.stdout(Stdio::piped());
            cmd.stderr(Stdio::piped());

            let child = cmd.spawn()
                .map_err(|e| Exec(format!("failed to spawn plugin {}: {}", self.name, e)))?;

            use std::io::Write;
            if let Some(ref stdin) = child.stdin {
                let mut w = stdin;
                w.write_all(input.as_bytes())
                    .map_err(|e| Exec(format!("failed to write to plugin stdin: {}", e)))?;
            }

            child.wait_with_output()
                .map_err(|e| Exec(format!("failed to wait on plugin: {}", e)))
        } else {
            cmd.output()
                .map_err(|e| Exec(format!("failed to execute plugin {}: {}", self.name, e)))
        }
    }
}

impl fmt::Display for Plugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} v{} - {}", self.name, self.version, self.description)
    }
}

#[derive(Debug, Clone, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Quiet,
}

#[derive(Debug, Clone, Subcommand)]
pub enum PluginAction {
    /// List installed plugins
    List {
        /// Output format
        #[arg(short, long, value_enum, default_value_t = OutputFormat::Table)]
        format: OutputFormat,
    },
    /// Load plugins from directory
    Load {
        /// Path to plugins directory
        #[arg(value_name = "PATH")]
        path: Option<String>,
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Reload all plugins
    Reload {
        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },
    /// Run a plugin command directly
    Run {
        /// Plugin name
        #[arg(value_name = "PLUGIN")]
        plugin: String,
        /// Arguments to pass to the plugin
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Search for plugins in known locations
    Search {
        /// Plugin name pattern to search for
        #[arg(value_name = "PATTERN")]
        pattern: Option<String>,
    },
    /// Validate plugin installations
    Validate {
        /// Fix any issues found
        #[arg(short, long)]
        fix: bool,
    },
    /// Create a new plugin scaffold
    Init {
        /// Plugin name
        #[arg(value_name = "NAME")]
        name: String,
        /// Plugin directory path
        #[arg(short, long)]
        path: Option<String>,
        /// Plugin template type
        #[arg(short, long, value_enum, default_value_t = PluginTemplate::Binary)]
        template: PluginTemplate,
    },
    /// Uninstall a plugin
    Uninstall {
        /// Plugin name
        #[arg(value_name = "PLUGIN")]
        name: String,
        /// Remove plugin data and configs
        #[arg(short, long)]
        purge: bool,
    },
    /// Show plugin information
    Info {
        /// Plugin name
        #[arg(value_name = "PLUGIN")]
        name: String,
    },
}

#[derive(Debug, Clone, ValueEnum)]
pub enum PluginTemplate {
    Binary,
    Script,
    Docker,
    Custom,
}

pub struct PluginManager {
    plugins: HashMap<String, Plugin>,
    plugin_dirs: Vec<PathBuf>,
    config_dir: PathBuf,
}

impl PluginManager {
    pub fn new() -> Self {
        let config_dir = Self::default_config_dir();
        let mut manager = Self {
            plugins: HashMap::new(),
            plugin_dirs: vec![],
            config_dir,
        };
        manager.add_default_plugin_dirs();
        manager
    }

    pub fn with_config_dir(mut self, config_dir: PathBuf) -> Self {
        self.config_dir = config_dir;
        self
    }

    fn default_config_dir() -> PathBuf {
        env::var("PACK_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                env::var("HOME")
                    .map(|h| PathBuf::from(h).join(".pack"))
                    .unwrap_or_else(|_| PathBuf::from(".pack"))
            })
    }

    fn add_default_plugin_dirs(&mut self) {
        if let Ok(home) = env::var("HOME") {
            self.plugin_dirs.push(PathBuf::from(&home).join(".pack").join("plugins"));
        }
        self.plugin_dirs.push(PathBuf::from(".pack").join("plugins"));
        if let Ok(pack_dir) = env::var("PACK_PLUGIN_DIR") {
            self.plugin_dirs.push(PathBuf::from(pack_dir));
        }
    }

    pub fn add_plugin_dir(&mut self, path: PathBuf) {
        if !self.plugin_dirs.contains(&path) {
            self.plugin_dirs.push(path);
        }
    }

    pub fn set_plugin_dirs(&mut self, dirs: Vec<PathBuf>) {
        self.plugin_dirs = dirs;
    }

    pub fn register(&mut self, plugin: Plugin) {
        self.plugins.insert(plugin.name.clone(), plugin);
    }

    pub fn unregister(&mut self, name: &str) -> Option<Plugin> {
        self.plugins.remove(name)
    }

    pub fn get(&self, name: &str) -> Option<&Plugin> {
        self.plugins.get(name)
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut Plugin> {
        self.plugins.get_mut(name)
    }

    pub fn list(&self) -> Vec<&Plugin> {
        self.plugins.values().filter(|p| p.enabled).collect()
    }

    pub fn list_all(&self) -> Vec<&Plugin> {
        self.plugins.values().collect()
    }

    pub fn list_disabled(&self) -> Vec<&Plugin> {
        self.plugins.values().filter(|p| !p.enabled).collect()
    }

    pub fn list_commands(&self) -> Vec<String> {
        let mut commands = vec![];
        for plugin in self.plugins.values().filter(|p| p.enabled) {
            for cmd in &plugin.commands {
                commands.push(format!("{}:{}", plugin.name, cmd));
            }
        }
        commands
    }

    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    pub fn enabled_count(&self) -> usize {
        self.plugins.values().filter(|p| p.enabled).count()
    }

    pub fn has_command(&self, cmd: &str) -> bool {
        for plugin in self.plugins.values().filter(|p| p.enabled) {
            if plugin.commands.contains(&cmd.to_string()) {
                return true;
            }
            let prefixed = format!("{}:{}", plugin.name, cmd);
            if self.plugins.values().any(|p| p.commands.iter().any(|c| c == &prefixed)) {
                return true;
            }
        }
        false
    }

    pub fn find_command(&self, cmd: &str) -> Option<&Plugin> {
        for plugin in self.plugins.values().filter(|p| p.enabled) {
            if plugin.commands.contains(&cmd.to_string()) {
                return Some(plugin);
            }
            let prefixed = format!("{}:{}", plugin.name, cmd);
            if plugin.commands.iter().any(|c| c == &prefixed) {
                return Some(plugin);
            }
        }
        None
    }

    pub fn execute_command(&self, name: &str, args: &[String]) -> PackResult<Output> {
        let plugin = self.plugins.get(name)
            .ok_or_else(|| Exec(format!("plugin '{}' not found", name)))?;

        if !plugin.enabled {
            return Err(Exec(format!("plugin '{}' is disabled", name)));
        }

        plugin.execute(args)
    }

    pub fn execute_plugin(&self, plugin_name: &str, args: &[String]) -> PackResult<Output> {
        let plugin = self.plugins.get(plugin_name)
            .ok_or_else(|| Exec(format!("plugin '{}' not found", plugin_name)))?;

        if !plugin.enabled {
            return Err(Exec(format!("plugin '{}' is disabled", plugin_name)));
        }

        plugin.execute(args)
    }

    pub fn plugins_count(&self) -> usize {
        self.plugins.len()
    }

    pub fn disabled_count(&self) -> usize {
        self.plugins.values().filter(|p| !p.enabled).count()
    }

    pub fn load_from_dir(&self, dir: &PathBuf) -> PackResult<Vec<Plugin>> {
        let mut loaded = vec![];

        if !dir.exists() {
            return Ok(loaded);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(name) = path.file_name() {
                    let name_str = name.to_string_lossy().to_string();

                    if name_str.starts_with('.') || !name_str.ends_with(".pack-plugin") {
                        continue;
                    }

                    if let Ok(metadata) = fs::metadata(&path) {
                        if metadata.permissions().readonly() {
                            continue;
                        }

                        #[cfg(unix)]
                        {
                            use std::os::unix::fs::PermissionsExt;
                            if metadata.permissions().mode() & 0o111 == 0 {
                                continue;
                            }
                        }
                    }

                    let plugin = Plugin::new(
                        name_str.replace(".pack-plugin", ""),
                        path.clone(),
                    );
                    loaded.push(plugin);
                }
            }

            if path.is_dir() {
                let plugin_path = path.join("pack-plugin");
                if plugin_path.exists() {
                    let name = path.file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let mut plugin = Plugin::new(name.clone(), plugin_path);

                    let manifest_path = path.join("manifest.json");
                    if manifest_path.exists() {
                        if let Ok(content) = fs::read_to_string(&manifest_path) {
                            if let Ok(manifest) = serde_json::from_str::<PluginManifest>(&content) {
                                plugin = plugin
                                    .with_version(manifest.version)
                                    .with_description(manifest.description)
                                    .with_commands(manifest.commands);
                            }
                        }
                    }

                    loaded.push(plugin);
                }
            }
        }

        Ok(loaded)
    }

    pub fn load_all(&mut self) -> PackResult<Vec<Plugin>> {
        let mut all_loaded = vec![];

        for dir in &self.plugin_dirs {
            if dir.exists() {
                let loaded = self.load_from_dir(dir)?;
                for plugin in loaded {
                    let name = plugin.name.clone();
                    self.plugins.insert(name, plugin.clone());
                    all_loaded.push(plugin);
                }
            }
        }

        Ok(all_loaded)
    }

    pub fn reload(&mut self) -> PackResult<Vec<Plugin>> {
        self.plugins.clear();
        self.load_all()
    }

    pub fn validate_plugins(&self) -> Vec<PluginValidationResult> {
        let mut results = vec![];

        for plugin in self.plugins.values() {
            let result = self.validate_plugin(plugin);
            results.push(result);
        }

        results
    }

    fn validate_plugin(&self, plugin: &Plugin) -> PluginValidationResult {
        let mut issues = vec![];

        if !plugin.path.exists() {
            issues.push(PluginIssue::MissingFile(plugin.path.display().to_string()));
        } else if !plugin.is_executable() {
            issues.push(PluginIssue::NotExecutable(plugin.path.display().to_string()));
        }

        if plugin.version.is_empty() || plugin.version == "1.0.0" {
            issues.push(PluginIssue::DefaultVersion(plugin.name.clone()));
        }

        if plugin.description.is_empty() {
            issues.push(PluginIssue::NoDescription(plugin.name.clone()));
        }

        if issues.is_empty() {
            PluginValidationResult {
                plugin_name: plugin.name.clone(),
                valid: true,
                issues: vec![],
            }
        } else {
            PluginValidationResult {
                plugin_name: plugin.name.clone(),
                valid: false,
                issues,
            }
        }
    }

    pub fn search(&self, pattern: Option<&str>) -> Vec<&Plugin> {
        let pattern = pattern.map(|p| p.to_lowercase());

        self.plugins.values()
            .filter(|p| {
                if let Some(ref pattern) = pattern {
                    p.name.to_lowercase().contains(pattern)
                        || p.description.to_lowercase().contains(pattern)
                        || p.commands.iter().any(|c| c.to_lowercase().contains(pattern))
                } else {
                    true
                }
            })
            .collect()
    }

    pub fn disable_plugin(&mut self, name: &str) -> bool {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = false;
            true
        } else {
            false
        }
    }

    pub fn enable_plugin(&mut self, name: &str) -> bool {
        if let Some(plugin) = self.plugins.get_mut(name) {
            plugin.enabled = true;
            true
        } else {
            false
        }
    }

    pub fn init_plugin(&self, name: &str, path: &Path, template: PluginTemplate) -> PackResult<Plugin> {
        let plugin_path = path.join(name);

        fs::create_dir_all(&plugin_path)
            .map_err(|e| Exec(format!("failed to create plugin directory: {}", e)))?;

        match template {
            PluginTemplate::Binary => {
                let binary_path = plugin_path.join("pack-plugin");
                let content = format!("#!/bin/bash\n# Pack plugin: {}\n\necho 'Plugin {} initialized'\n", name, name);
                fs::write(&binary_path, content)
                    .map_err(|e| Exec(format!("failed to create plugin script: {}", e)))?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&binary_path)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&binary_path, perms)?;
                }

                let manifest = PluginManifest {
                    name: name.to_string(),
                    version: "0.1.0".to_string(),
                    description: format!("Pack plugin: {}", name),
                    commands: vec![],
                };

                let manifest_path = plugin_path.join("manifest.json");
                let manifest_json = serde_json::to_string_pretty(&manifest)
                    .map_err(|e| Exec(format!("failed to serialize manifest: {}", e)))?;
                fs::write(&manifest_path, manifest_json)
                    .map_err(|e| Exec(format!("failed to write manifest: {}", e)))?;
            }
            PluginTemplate::Script => {
                let script_path = plugin_path.join("pack-plugin");
                let content = format!("#!/bin/bash\n# Pack plugin: {}\n\necho 'Plugin {} initialized'\n", name, name);
                fs::write(&script_path, content)
                    .map_err(|e| Exec(format!("failed to create plugin script: {}", e)))?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = fs::metadata(&script_path)?.permissions();
                    perms.set_mode(0o755);
                    fs::set_permissions(&script_path, perms)?;
                }

                let manifest_path = plugin_path.join("manifest.json");
                let manifest = PluginManifest {
                    name: name.to_string(),
                    version: "0.1.0".to_string(),
                    description: format!("Pack plugin: {}", name),
                    commands: vec![],
                };
                let manifest_json = serde_json::to_string_pretty(&manifest)
                    .map_err(|e| Exec(format!("failed to serialize manifest: {}", e)))?;
                fs::write(&manifest_path, manifest_json)
                    .map_err(|e| Exec(format!("failed to write manifest: {}", e)))?;
            }
            PluginTemplate::Docker => {
                let dockerfile_path = plugin_path.join("Dockerfile");
                fs::write(&dockerfile_path, &format!(r#"FROM ubuntu:22.04
LABEL maintainer="pack@plugin"
LABEL com.pack.plugin="{}"

WORKDIR /plugin
COPY pack-plugin /usr/local/bin/pack-plugin
RUN chmod +x /usr/local/bin/pack-plugin

ENTRYPOINT ["/usr/local/bin/pack-plugin"]
"#, name))
                    .map_err(|e| Exec(format!("failed to create Dockerfile: {}", e)))?;

                let script_path = plugin_path.join("pack-plugin");
                let content = format!("#!/bin/bash\n# Pack plugin: {}\n\necho 'Docker plugin {} initialized'\n", name, name);
                fs::write(&script_path, content)
                    .map_err(|e| Exec(format!("failed to create plugin script: {}", e)))?;
            }
            PluginTemplate::Custom => {
                let plugin_path_str = plugin_path.join("pack-plugin");
                let content = format!("#!/bin/bash\n# Pack plugin: {}\n\n# Add your plugin code here\n", name);
                fs::write(&plugin_path_str, content)
                    .map_err(|e| Exec(format!("failed to create plugin script: {}", e)))?;
            }
        }

        Ok(Plugin::new(name.to_string(), plugin_path.join("pack-plugin")))
    }

    pub fn uninstall_plugin(&mut self, name: &str, purge: bool) -> PackResult<bool> {
        if let Some(plugin) = self.plugins.remove(name) {
            if purge {
                let plugin_dir = plugin.path.parent();
                if let Some(dir) = plugin_dir {
                    if dir.exists() && dir.to_string_lossy().contains(".pack") {
                        fs::remove_dir_all(dir)
                            .map_err(|e| Exec(format!("failed to remove plugin directory: {}", e)))?;
                    }
                }
            }
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn plugin_dirs(&self) -> &[PathBuf] {
        &self.plugin_dirs
    }

    pub fn config_dir(&self) -> &Path {
        &self.config_dir
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct PluginManifest {
    name: String,
    version: String,
    description: String,
    commands: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PluginValidationResult {
    pub plugin_name: String,
    pub valid: bool,
    pub issues: Vec<PluginIssue>,
}

#[derive(Debug, Clone)]
pub enum PluginIssue {
    MissingFile(String),
    NotExecutable(String),
    DefaultVersion(String),
    NoDescription(String),
}

impl fmt::Display for PluginIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginIssue::MissingFile(path) => write!(f, "missing file: {}", path),
            PluginIssue::NotExecutable(path) => write!(f, "not executable: {}", path),
            PluginIssue::DefaultVersion(name) => write!(f, "using default version ({}): {}", name, "consider updating"),
            PluginIssue::NoDescription(name) => write!(f, "no description for plugin: {}", name),
        }
    }
}

pub struct PluginOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl From<Output> for PluginOutput {
    fn from(output: Output) -> Self {
        Self {
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            exit_code: output.status.code().unwrap_or(-1),
        }
    }
}

impl PluginOutput {
    pub fn is_success(&self) -> bool {
        self.exit_code == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_new() {
        let manager = PluginManager::new();
        assert!(manager.list().is_empty());
    }

    #[test]
    fn test_register_plugin() {
        let mut manager = PluginManager::new();
        let plugin = Plugin::new("test-plugin".to_string(), PathBuf::from("/bin/true"));
        manager.register(plugin);
        assert!(manager.get("test-plugin").is_some());
    }

    #[test]
    fn test_unregister_plugin() {
        let mut manager = PluginManager::new();
        let plugin = Plugin::new("test-plugin".to_string(), PathBuf::from("/bin/true"));
        manager.register(plugin);
        let removed = manager.unregister("test-plugin");
        assert!(removed.is_some());
        assert!(manager.get("test-plugin").is_none());
    }

    #[test]
    fn test_list_commands() {
        let mut manager = PluginManager::new();
        let plugin = Plugin::new("my-plugin".to_string(), PathBuf::from("/bin/true"))
            .with_commands(vec!["install".to_string(), "list".to_string()]);
        manager.register(plugin);

        let commands = manager.list_commands();
        assert!(commands.contains(&"my-plugin:install".to_string()));
        assert!(commands.contains(&"my-plugin:list".to_string()));
    }

    #[test]
    fn test_plugin_builder() {
        let plugin = Plugin::new("rails".to_string(), PathBuf::from("/usr/bin/rails"))
            .with_version("2.0.0".to_string())
            .with_description("Rails plugin".to_string())
            .with_commands(vec!["console".to_string(), "generate".to_string()])
            .with_enabled(true);

        assert_eq!(plugin.name, "rails");
        assert_eq!(plugin.version, "2.0.0");
        assert_eq!(plugin.description, "Rails plugin");
        assert_eq!(plugin.commands.len(), 2);
        assert!(plugin.enabled);
    }

    #[test]
    fn test_has_command() {
        let mut manager = PluginManager::new();
        let plugin = Plugin::new("test".to_string(), PathBuf::from("/bin/true"))
            .with_commands(vec!["deploy".to_string()]);
        manager.register(plugin);

        assert!(manager.has_command("deploy"));
        assert!(!manager.has_command("nonexistent"));
    }

    #[test]
    fn test_default_plugin_dirs() {
        let manager = PluginManager::new();
        assert!(!manager.plugin_dirs.is_empty());
    }

    #[test]
    fn test_disable_enable_plugin() {
        let mut manager = PluginManager::new();
        let plugin = Plugin::new("test".to_string(), PathBuf::from("/bin/true"));
        manager.register(plugin);

        assert!(manager.disable_plugin("test"));
        assert!(manager.disabled_count() == 1);
        assert!(manager.enabled_count() == 0);

        assert!(manager.enable_plugin("test"));
        assert!(manager.enabled_count() == 1);
    }

    #[test]
    fn test_search() {
        let mut manager = PluginManager::new();
        manager.register(Plugin::new("deploy-plugin".to_string(), PathBuf::from("/bin/true"))
            .with_description("Deploys applications".to_string()));
        manager.register(Plugin::new("ci-plugin".to_string(), PathBuf::from("/bin/true"))
            .with_description("CI integration".to_string()));

        let results = manager.search(Some("deploy"));
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "deploy-plugin");
    }

    #[test]
    fn test_plugin_output() {
        let output = PluginOutput {
            stdout: "hello".to_string(),
            stderr: "".to_string(),
            exit_code: 0,
        };
        assert!(output.is_success());
    }
}
