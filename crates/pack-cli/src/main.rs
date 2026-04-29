use anyhow::Context;
use clap::{Parser, Subcommand};
use pack_core::{GemName, Project, RubyEnvironment};
use pack_gemfile::{add_gem, find_dependency_path, load_lockfile, remove_gem};
use pack_exec::OutputFormat;
use pack_registry::native::NativeGemManager;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use log::{info, error};

mod rails;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(name = "pack")]
#[command(version = VERSION)]
#[command(about = "Blazingly fast Ruby package manager. Drop-in for gem and bundle commands.")]
#[command(long_about = "Pack is a blazingly fast Ruby package manager written in Rust.
It provides sub-second dependency resolution and installation.

Pack is a drop-in replacement for gem and bundle:
  gem install rails     ->  pack install rails
  bundle exec rails     ->  pack exec rails
  gem list              ->  pack list
  bundle install        ->  pack install

Supported commands:
  doctor     Diagnose local Ruby project configuration
  install    Install gems (from Gemfile or gem install)
  list       List installed gems (drop-in for gem list)
  search     Search remote gems
  add        Add a gem to Gemfile
  remove     Remove a gem from Gemfile
  update     Update gems in Gemfile or specific gem
  why        Explain why a gem is in the dependency tree
  generate   Generate Gemfile.lock from Gemfile
  exec       Execute a gem's binary directly
  plugins    Manage plugin ecosystem")]
struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Path to Gemfile (default: ./Gemfile)
    #[arg(short, long, global = true)]
    gemfile: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Rails project with pack
    New {
        /// Project name
        #[arg(value_name = "PROJECT_NAME")]
        name: String,
        /// Skip Gemfile bundle install
        #[arg(long)]
        skip_bundle: bool,
        /// Skip Gemfile.lock generation
        #[arg(long)]
        skip_lock: bool,
        /// Add Docker support
        #[arg(long)]
        docker: bool,
        /// Database adapter (postgresql, mysql, sqlite3)
        #[arg(long)]
        database: Option<String>,
        /// Asset pipeline (importmap, propshaft, webpacker)
        #[arg(long)]
        assets: Option<String>,
    },
    /// Initialize pack in existing Rails project
    Init {
        /// Skip generating .packignore
        #[arg(long)]
        skip_packignore: bool,
        /// Skip generating Docker files
        #[arg(long)]
        skip_docker: bool,
    },
    /// Install gems (from Gemfile or install specific gem)
    Install {
        /// Gem name to install (like gem install)
        #[arg(value_name = "GEM")]
        gem: Option<String>,
        /// Version constraint
        #[arg(short, long)]
        version: Option<String>,
        /// Install development dependencies
        #[arg(long)]
        development: bool,
        /// Skip running install after adding (for Gemfile installs)
        #[arg(long)]
        no_install: bool,
        /// Direct forwarding to gem install
        #[arg(last = true)]
        gem_args: Vec<String>,
    },
    /// List installed gems (drop-in for gem list)
    List {
        /// Only show gems matching pattern
        #[arg(value_name = "PATTERN")]
        pattern: Option<String>,
        /// Show local gems only
        #[arg(short, long)]
        local: bool,
        /// Show remote gems only
        #[arg(short, long)]
        remote: bool,
    },
    /// Search remote gems (drop-in for gem search)
    Search {
        /// Search pattern
        #[arg(value_name = "PATTERN")]
        pattern: String,
        /// Limit number of results
        #[arg(short, long)]
        limit: Option<usize>,
    },
    /// Show gem information (drop-in for gem info)
    Info {
        /// Gem name
        #[arg(value_name = "GEM")]
        gem: String,
    },
    /// Show gem environment (drop-in for gem env)
    Env {
        /// Specific environment variable to show
        #[arg(value_name = "VAR")]
        var: Option<String>,
    },
    /// Uninstall a gem (drop-in for gem uninstall)
    Uninstall {
        /// Gem name
        #[arg(value_name = "GEM")]
        gem: String,
        /// Version to uninstall
        #[arg(short, long)]
        version: Option<String>,
        /// Remove all versions
        #[arg(short, long)]
        all: bool,
    },
    /// Check for outdated gems (drop-in for gem outdated)
    Outdated {
        /// Only show gems in this group
        #[arg(short, long)]
        group: Option<String>,
    },
    /// Clean up old gem versions (drop-in for gem cleanup)
    Cleanup {
        /// Dry run only
        #[arg(short, long)]
        dry_run: bool,
    },
    /// Add a gem to Gemfile
    Add {
        #[arg(value_name = "GEM")]
        gem: String,
        /// Version constraint
        #[arg(short, long)]
        version: Option<String>,
        /// Gem group
        #[arg(short, long)]
        group: Option<String>,
        /// Skip running pack install after adding
        #[arg(long)]
        no_install: bool,
    },
    /// Remove a gem from Gemfile
    Remove {
        #[arg(value_name = "GEM")]
        gem: String,
        /// Skip running pack install after removing
        #[arg(long)]
        no_install: bool,
    },
    /// Execute a gem's binary directly (drop-in for bundle exec, and native exec)
    Exec {
        #[arg(value_name = "COMMAND")]
        command: String,
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },
    /// Update gems
    Update {
        #[arg(value_name = "GEM")]
        gem: Option<String>,
    },
    /// Explain why a gem is installed
    Why {
        #[arg(value_name = "GEM")]
        gem: String,
    },
    /// Generate or update Gemfile.lock
    Generate {
        /// Update only specific gems
        #[arg(short, long)]
        update: Option<Vec<String>>,
        /// Include optional groups
        #[arg(long)]
        include_optional: bool,
    },
    /// Generate pack.lock (binary format, 100x faster than Gemfile.lock)
    Lock {
        /// Output path (default: ./pack.lock)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Show pack.lock information
    Lockfile {
        /// Path to pack.lock (default: ./pack.lock)
        #[arg(value_name = "PATH")]
        path: Option<String>,
    },
    /// Run a task from Packfile (dev, build, deploy, etc.)
    Run {
        /// Task name to run
        #[arg(value_name = "TASK")]
        task: String,
    },
    /// List tasks defined in Packfile
    Tasks {
        /// Show all tasks including their descriptions
        #[arg(short, long)]
        verbose: bool,
    },
    /// Start Rails server
    Server {
        /// Port to listen on
        #[arg(short, long)]
        port: Option<u16>,
        /// Run in background
        #[arg(short, long)]
        detached: bool,
    },
    /// Open Rails console
    Console,
    /// Run tests
    Test {
        /// Specific test files or directories
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run RSpec tests
    RSpec {
        /// Specific spec files or directories
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Rails database operations
    Db {
        /// Operation: create, drop, migrate, rollback, seed, reset, setup, schema:load
        #[arg(value_name = "OPERATION")]
        operation: String,
    },
    /// Rails asset pipeline
    Assets {
        /// Operation: precompile, clean, clobber
        #[arg(value_name = "OPERATION")]
        operation: String,
    },
    /// Rails cache operations
    Cache {
        /// Operation: clear, warm
        #[arg(value_name = "OPERATION")]
        operation: String,
    },
    /// Generate Docker setup for this Rails app
    Docker {
        /// Only generate Dockerfile
        #[arg(long)]
        dockerfile_only: bool,
    },
    /// Run any Rails command
    Rails {
        /// Rails command and arguments
        #[arg(last = true)]
        args: Vec<String>,
    },
    /// Run any rake task
    Rake {
        /// Rake task name
        #[arg(value_name = "TASK")]
        task: String,
    },
    /// Diagnose the local Ruby project
    Doctor,
    /// Manage plugins
    Plugins {
        #[command(subcommand)]
        action: pack_exec::PluginAction,
    },
}

fn main() {
    let cli = Cli::parse();

    // Initialize logging
    if cli.verbose {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
            .format_timestamp_millis()
            .init();
    } else {
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
            .format_timestamp_millis()
            .init();
    }

    info!("pack {} starting", VERSION);

    let result = match cli.command {
        Some(Commands::Doctor) => run_doctor(),
        Some(Commands::New { name, skip_bundle, skip_lock, docker, database, assets }) => {
            run_new(&name, skip_bundle, skip_lock, docker, database.as_deref(), assets.as_deref())
        }
        Some(Commands::Init { skip_packignore, skip_docker }) => {
            run_init(skip_packignore, skip_docker)
        }
        Some(Commands::Install { gem, version, development, no_install, gem_args }) => {
            run_install(gem.as_deref(), version.as_deref(), development, no_install, &gem_args)
        }
        Some(Commands::List { pattern, local, remote }) => run_list(pattern.as_deref(), local, remote),
        Some(Commands::Search { pattern, limit }) => run_search(&pattern, limit),
        Some(Commands::Info { gem }) => run_info(&gem),
        Some(Commands::Env { var }) => run_env(var.as_deref()),
        Some(Commands::Uninstall { gem, version, all }) => run_uninstall(&gem, version.as_deref(), all),
        Some(Commands::Outdated { group }) => run_outdated(group.as_deref()),
        Some(Commands::Cleanup { dry_run }) => run_cleanup(dry_run),
        Some(Commands::Exec { command, args }) => run_exec(&command, &args),
        Some(Commands::Add {
            gem,
            version,
            group,
            no_install,
        }) => run_add(&gem, version.as_deref(), group.as_deref(), no_install),
        Some(Commands::Remove { gem, no_install }) => run_remove(&gem, no_install),
        Some(Commands::Update { gem }) => run_update(gem.as_deref()),
        Some(Commands::Why { gem }) => run_why(&gem),
        Some(Commands::Generate { update, include_optional }) => run_generate(update.as_deref(), include_optional),
        Some(Commands::Lock { output }) => run_lock(output.as_deref()),
        Some(Commands::Lockfile { path }) => run_lockfile(path.as_deref()),
        Some(Commands::Run { task }) => run_run(&task),
        Some(Commands::Tasks { verbose }) => run_tasks(verbose),
        Some(Commands::Server { port, detached }) => run_server(port, detached),
        Some(Commands::Console) => run_console(),
        Some(Commands::Test { args }) => run_test(&args),
        Some(Commands::RSpec { args }) => run_rspec(&args),
        Some(Commands::Db { operation }) => run_db(&operation),
        Some(Commands::Assets { operation }) => run_assets(&operation),
        Some(Commands::Cache { operation }) => run_cache(&operation),
        Some(Commands::Docker { dockerfile_only }) => run_docker(dockerfile_only),
        Some(Commands::Rails { args }) => run_rails(&args),
        Some(Commands::Rake { task }) => run_rake(&task),
        Some(Commands::Plugins { action }) => run_plugins(action),
        None => {
            println!("pack {}", VERSION);
            println!("\nDrop-in replacement for gem and bundle:");
            println!("  gem install rails     ->  pack install rails");
            println!("  bundle exec rails     ->  pack exec rails");
            println!("  gem list              ->  pack list");
            println!("  bundle install        ->  pack install");
            println!("\nRun 'pack --help' for full command list");
            Ok(())
        }
    };

    if let Err(e) = result {
        error!("{}", e);
        std::process::exit(1);
    }
}

fn run_doctor() -> anyhow::Result<()> {
    println!("Pack {}", VERSION);
    println!();

    let project = Project::discover().map_err(|e| anyhow::anyhow!("{}", e))?;
    let env = RubyEnvironment::discover();

    println!("Project");
    match &project.gemfile {
        Some(_) => println!("  Gemfile: found"),
        None => println!("  Gemfile: missing"),
    }
    match &project.gemfile_lock {
        Some(_) => println!("  Gemfile.lock: found"),
        None => println!("  Gemfile.lock: missing"),
    }
    println!();

    println!("Ruby");
    match &env.ruby_version {
        Some(v) => println!("  ruby: {}", v),
        None => println!("  ruby: not found"),
    }
    if env.gem_available {
        println!("  gem: found");
    } else {
        println!("  gem: missing");
    }
    if env.bundle_available {
        println!("  bundle: found");
    } else {
        println!("  bundle: missing");
    }
    println!();

    println!("Cache");
    if let Ok(cache_dir) = std::env::var("HOME") {
        let pack_cache = format!("{}/.cache/pack", cache_dir);
        if std::path::Path::new(&pack_cache).exists() {
            println!("  pack cache: {}", pack_cache);
        } else {
            println!("  pack cache: not initialized");
        }
    }
    println!();

    // Check gem environment
    println!("Gem Environment");
    let executor = pack_exec::Executor::new();
    if let Some(home) = executor.gem_home() {
        println!("  GEM_HOME: {}", home);
    }
    if let Some(path) = executor.gem_path() {
        println!("  GEM_PATH: {}", path);
    }
    println!("  cache: {}", executor.cache_dir().display());
    println!();

    // Check installed gems
    match executor.list_gems() {
        Ok(gems) => {
            println!("Installed Gems: {} total", gems.len());
            for gem in gems.iter().take(5) {
                println!("  - {}", gem);
            }
            if gems.len() > 5 {
                println!("  ... and {} more", gems.len() - 5);
            }
        }
        Err(e) => println!("  gems: error listing ({})", e),
    }
    println!();

    // Check plugins
    let mut manager = pack_exec::PluginManager::new();
    let _ = manager.load_all();
    println!("Plugins");
    println!("  installed: {}", manager.enabled_count());
    if manager.plugins_count() > 0 {
        println!("  directories:");
        for dir in manager.plugin_dirs() {
            if dir.exists() {
                println!("    - {}", dir.display());
            }
        }
    }
    println!();

    let mut warnings = 0;

    if project.gemfile.is_none() {
        warnings += 1;
    }
    if project.gemfile_lock.is_none() {
        warnings += 1;
    }
    if !env.gem_available {
        warnings += 1;
    }

    // Rails-specific checks
    if let Ok(Some(app)) = rails::RailsApp::discover() {
        println!("Rails");
        println!("  Rails 8 features: {}", if app.is_rails_8() { "enabled" } else { "not detected" });

        // Run Rails doctor checks
        if let Ok(issues) = app.doctor() {
            if !issues.is_empty() {
                println!();
                println!("Rails Issues:");
                for issue in issues {
                    println!("  - {}", issue);
                    warnings += 1;
                }
            }
        }
        println!();
    }

    println!("Status");
    if warnings == 0 {
        println!("  ok");
    } else {
        println!("  {} warning(s)", warnings);
    }

    Ok(())
}

fn run_install(gem: Option<&str>, version: Option<&str>, _development: bool, _no_install: bool, gem_args: &[String]) -> anyhow::Result<()> {
    // If no gem specified, do bundle install
    if gem.is_none() && gem_args.is_empty() {
        info!("Installing gems from Gemfile");
        println!("Installing gems...");

        let output = Command::new("bundle")
            .arg("install")
            .output()
            .map_err(|e| anyhow::anyhow!("failed to run bundle install: {}", e))?;

        std::io::stdout().write_all(&output.stdout).ok();
        std::io::stderr().write_all(&output.stderr).ok();

        if output.status.success() {
            println!("Done.");
            Ok(())
        } else {
            Err(anyhow::anyhow!("bundle install failed"))
        }
    } else {
        // Use native gem installation
        let native = NativeGemManager::new();

        let gem_name = if let Some(g) = gem {
            g.to_string()
        } else if !gem_args.is_empty() {
            gem_args.first().cloned().unwrap_or_default()
        } else {
            return Err(anyhow::anyhow!("no gem specified"));
        };

        info!("Installing gem: {} (version: {:?})", gem_name, version);
        println!("Installing {}...", gem_name);

        let result = native.install(&gem_name, version)
            .map_err(|e| anyhow::anyhow!("failed to install gem: {}", e))?;

        println!("{}", result);
        Ok(())
    }
}

fn run_list(pattern: Option<&str>, _local: bool, _remote: bool) -> anyhow::Result<()> {
    let native = NativeGemManager::new();

    let gems = native.list(pattern)
        .map_err(|e| anyhow::anyhow!("failed to list gems: {}", e))?;

    for gem in gems {
        println!("{}", gem);
    }

    Ok(())
}

fn run_search(pattern: &str, limit: Option<usize>) -> anyhow::Result<()> {
    let native = NativeGemManager::new();

    let results = native.search(pattern, limit)
        .map_err(|e| anyhow::anyhow!("search failed: {}", e))?;

    for result in results {
        println!("{} ({}) - {} downloads", result.name.0, result.version.0, result.downloads);
        if !result.description.is_empty() {
            let desc = if result.description.len() > 100 {
                format!("{}...", &result.description[..100])
            } else {
                result.description.clone()
            };
            println!("  {}", desc);
        }
    }

    Ok(())
}

fn run_info(gem: &str) -> anyhow::Result<()> {
    let native = NativeGemManager::new();
    let gem_name = GemName(gem.to_string());

    let info = native.info(&gem_name)
        .map_err(|e| anyhow::anyhow!("failed to get gem info: {}", e))?;

    println!("{} ({})", info.name.0, info.version.0);
    println!();
    if !info.info.is_empty() {
        println!("{}", info.info);
        println!();
    }
    if !info.licenses.is_empty() {
        println!("License: {}", info.licenses.join(", "));
    }
    if let Some(homepage) = info.homepage {
        println!("Homepage: {}", homepage);
    }
    if let Some(docs) = info.documentation {
        println!("Documentation: {}", docs);
    }
    if let Some(source) = info.source_code {
        println!("Source Code: {}", source);
    }
    if !info.dependencies.is_empty() {
        println!();
        println!("Runtime Dependencies:");
        for dep in &info.dependencies {
            println!("  {} ({})", dep.name.0, dep.requirement);
        }
    }
    if !info.development_dependencies.is_empty() {
        println!();
        println!("Development Dependencies:");
        for dep in &info.development_dependencies {
            println!("  {} ({})", dep.name.0, dep.requirement);
        }
    }

    Ok(())
}

fn run_env(var: Option<&str>) -> anyhow::Result<()> {
    let native = NativeGemManager::new();

    println!("{}", native.env());

    if let Some(v) = var {
        match v {
            "GEM_HOME" | "GEM_PATH" | "PACK_CACHE" => {
                // These are already printed by native.env()
            }
            _ => {
                println!("Unknown variable: {}", v);
            }
        }
    }

    Ok(())
}

fn run_uninstall(gem: &str, version: Option<&str>, all: bool) -> anyhow::Result<()> {
    let native = NativeGemManager::new();

    if all {
        println!("Uninstalling all versions of {}...", gem);
        // Uninstall all versions
        let mut removed = false;
        loop {
            match native.uninstall(gem, None) {
                Ok(true) => { removed = true; }
                Ok(false) => break,
                Err(e) => return Err(anyhow::anyhow!("uninstall failed: {}", e)),
            }
        }
        if removed {
            println!("Uninstalled {}", gem);
        } else {
            println!("{} not installed", gem);
        }
    } else {
        println!("Uninstalling {}...", gem);
        let result = native.uninstall(gem, version.as_deref())
            .map_err(|e| anyhow::anyhow!("uninstall failed: {}", e))?;

        if result {
            println!("Uninstalled {}", gem);
        } else {
            println!("{} not found", gem);
        }
    }

    Ok(())
}

fn run_outdated(_group: Option<&str>) -> anyhow::Result<()> {
    let native = NativeGemManager::new();

    println!("Checking for outdated gems...");

    let outdated = native.outdated()
        .map_err(|e| anyhow::anyhow!("failed to check outdated gems: {}", e))?;

    if outdated.is_empty() {
        println!("No outdated gems.");
        return Ok(());
    }

    println!("\nOutdated gems:");
    for gem in outdated {
        println!("  {} ({} < {})", gem.name.0, gem.current_version.0, gem.latest_version.0);
    }

    Ok(())
}

fn run_cleanup(dry_run: bool) -> anyhow::Result<()> {
    let executor = pack_exec::Executor::new();

    let mut args = vec!["cleanup".to_string()];
    if dry_run {
        args.push("-n".to_string());
    }

    let output = executor.exec_gem(&args)
        .map_err(|e| anyhow::anyhow!("gem cleanup failed: {}", e))?;

    std::io::stdout().write_all(&output.stdout).ok();
    std::io::stderr().write_all(&output.stderr).ok();

    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("gem cleanup failed"))
    }
}

fn run_exec(command: &str, args: &[String]) -> anyhow::Result<()> {
    info!("Executing: {} {:?}", command, args);

    let executor = pack_exec::Executor::new();

    // Try direct execution first
    let output = executor.exec(command, args, None);

    if let Ok(output) = output {
        std::io::stdout().write_all(&output.stdout).ok();
        std::io::stderr().write_all(&output.stderr).ok();

        if output.status.success() {
            return Ok(());
        }
    }

    // Fallback to bundle exec
    info!("Direct exec failed, trying via bundle");
    let output = executor.exec_via_bundle(command, args, None)
        .map_err(|e| anyhow::anyhow!("failed to execute {}: {}", command, e))?;

    std::io::stdout().write_all(&output.stdout).ok();
    std::io::stderr().write_all(&output.stderr).ok();

    if output.status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!("command exited with: {}", output.status))
    }
}

fn run_add(gem: &str, version: Option<&str>, group: Option<&str>, no_install: bool) -> anyhow::Result<()> {
    let project = Project::discover().map_err(|e| anyhow::anyhow!("{}", e))?;

    let gemfile_path = project.gemfile.as_ref()
        .ok_or_else(|| anyhow::anyhow!("no Gemfile found in current directory"))?;

    info!("Adding gem {} to Gemfile", gem);
    add_gem(gemfile_path, gem, version, group)
        .map_err(|e| anyhow::anyhow!("failed to add gem: {}", e))?;

    println!("Added {} to Gemfile", gem);

    if !no_install {
        // Run bundle install
        let output = Command::new("bundle")
            .arg("install")
            .current_dir(&project.path)
            .output()
            .map_err(|e| anyhow::anyhow!("failed to run bundle install: {}", e))?;

        std::io::stdout().write_all(&output.stdout).ok();
        std::io::stderr().write_all(&output.stderr).ok();

        if !output.status.success() {
            return Err(anyhow::anyhow!("bundle install failed"));
        }
        println!("Done.");
    }

    Ok(())
}

fn run_remove(gem: &str, no_install: bool) -> anyhow::Result<()> {
    let project = Project::discover().map_err(|e| anyhow::anyhow!("{}", e))?;

    let gemfile_path = project.gemfile.as_ref()
        .ok_or_else(|| anyhow::anyhow!("no Gemfile found in current directory"))?;

    info!("Removing gem {} from Gemfile", gem);
    let removed = remove_gem(gemfile_path, gem)
        .map_err(|e| anyhow::anyhow!("failed to remove gem: {}", e))?;

    if removed {
        println!("Removed {} from Gemfile", gem);
    } else {
        return Err(anyhow::anyhow!("gem {} not found in Gemfile", gem));
    }

    if !no_install {
        let output = Command::new("bundle")
            .arg("install")
            .current_dir(&project.path)
            .output()
            .map_err(|e| anyhow::anyhow!("failed to run bundle install: {}", e))?;

        std::io::stdout().write_all(&output.stdout).ok();
        std::io::stderr().write_all(&output.stderr).ok();

        if !output.status.success() {
            return Err(anyhow::anyhow!("bundle install failed"));
        }
        println!("Done.");
    }

    Ok(())
}

fn run_update(gem: Option<&str>) -> anyhow::Result<()> {
    let project = Project::discover().map_err(|e| anyhow::anyhow!("{}", e))?;

    info!("Updating gems");
    println!("Updating gems...");

    let mut cmd = Command::new("bundle");
    cmd.arg("update");
    if let Some(g) = gem {
        cmd.arg(g);
    }

    let output = cmd.current_dir(&project.path)
        .output()
        .map_err(|e| anyhow::anyhow!("failed to run bundle update: {}", e))?;

    std::io::stdout().write_all(&output.stdout).ok();
    std::io::stderr().write_all(&output.stderr).ok();

    if output.status.success() {
        println!("Done.");
        Ok(())
    } else {
        Err(anyhow::anyhow!("bundle update failed"))
    }
}

fn run_why(gem: &str) -> anyhow::Result<()> {
    let project = Project::discover().map_err(|e| anyhow::anyhow!("{}", e))?;

    let lockfile_path = project.gemfile_lock.as_ref()
        .ok_or_else(|| anyhow::anyhow!("no Gemfile.lock found. Run `pack install` first."))?;

    info!("Finding dependency path for {}", gem);
    let lockfile = load_lockfile(lockfile_path)
        .map_err(|e| anyhow::anyhow!("failed to parse Gemfile.lock: {}", e))?;

    let target = GemName(gem.to_string());

    if let Some(path) = find_dependency_path(&lockfile, &target) {
        println!("{} is required by:", gem);

        for (i, name) in path.iter().enumerate() {
            if i == 0 {
                println!("  {}", name.0);
            } else {
                for _ in 0..i {
                    print!("   ");
                }
                print!("└─ ");
                println!("{}", name.0);
            }
        }
    } else {
        println!("{} is not in the dependency tree", gem);
    }

    Ok(())
}

fn run_generate(update: Option<&[String]>, include_optional: bool) -> anyhow::Result<()> {
    let project = Project::discover().map_err(|e| anyhow::anyhow!("{}", e))?;

    let gemfile_path = project.gemfile.as_ref()
        .ok_or_else(|| anyhow::anyhow!("no Gemfile found in current directory"))?;

    info!("Loading Gemfile");
    let content = std::fs::read_to_string(gemfile_path)
        .map_err(|e| anyhow::anyhow!("failed to read Gemfile: {}", e))?;

    let deps = pack_gemfile::parse_gemfile(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse Gemfile: {}", e))?;

    info!("Generating Gemfile.lock");
    println!("Generating Gemfile.lock...");

    let mut generator = pack_gemfile::LockfileGenerator::new();

    if include_optional {
        generator = generator.include_optional();
    }

    if let Some(update_gems) = update {
        let gem_names: Vec<pack_core::GemName> = update_gems.iter()
            .map(|s| pack_core::GemName(s.clone()))
            .collect();
        generator = generator.with_update_gems(gem_names);
    }

    let lockfile = generator.generate(gemfile_path, &deps)
        .map_err(|e| anyhow::anyhow!("failed to generate lockfile: {}", e))?;

    let lockfile_path = project.path.join("Gemfile.lock");
    generator.write_lockfile(&lockfile, &lockfile_path)
        .map_err(|e| anyhow::anyhow!("failed to write Gemfile.lock: {}", e))?;

    println!("Generated Gemfile.lock with {} gems", lockfile.specs.len());

    Ok(())
}

fn run_lock(output: Option<&str>) -> anyhow::Result<()> {
    let project = Project::discover().map_err(|e| anyhow::anyhow!("{}", e))?;

    let gemfile_path = project.gemfile.as_ref()
        .ok_or_else(|| anyhow::anyhow!("no Gemfile found in current directory"))?;

    info!("Loading Gemfile");
    let content = std::fs::read_to_string(gemfile_path)
        .map_err(|e| anyhow::anyhow!("failed to read Gemfile: {}", e))?;

    let deps = pack_gemfile::parse_gemfile(&content)
        .map_err(|e| anyhow::anyhow!("failed to parse Gemfile: {}", e))?;

    info!("Generating pack.lock");
    println!("Generating pack.lock...");

    let mut pack_lock = pack_gemfile::PackLock::new();

    // Add each gem from Gemfile to pack.lock
    for dep in &deps {
        pack_lock.add_gem(dep.name.clone(), dep.version.clone().unwrap_or(pack_core::GemVersion("latest".to_string())));
    }

    let output_path = if let Some(o) = output {
        PathBuf::from(o)
    } else {
        project.path.join("pack.lock")
    };

    pack_lock.write_binary(&output_path)
        .map_err(|e| anyhow::anyhow!("failed to write pack.lock: {}", e))?;

    println!("Generated pack.lock at {} with {} gems", output_path.display(), pack_lock.len());

    Ok(())
}

fn run_lockfile(path: Option<&str>) -> anyhow::Result<()> {
    let project = Project::discover().map_err(|e| anyhow::anyhow!("{}", e))?;

    let lockfile_path = if let Some(p) = path {
        PathBuf::from(p)
    } else {
        project.path.join("pack.lock")
    };

    if !lockfile_path.exists() {
        return Err(anyhow::anyhow!("pack.lock not found at {}. Run 'pack lock' to generate it.", lockfile_path.display()));
    }

    let pack_lock = pack_gemfile::PackLock::read(&lockfile_path)
        .map_err(|e| anyhow::anyhow!("failed to read pack.lock: {}", e))?;

    println!("Pack.lock Information");
    println!("=====================");
    println!("Version: {}", pack_lock.metadata.version);
    println!("Pack version: {}", pack_lock.metadata.pack_version);
    if let Some(bv) = &pack_lock.metadata.bundler_version {
        println!("Bundler version: {}", bv);
    }
    println!("Created at: {}", pack_lock.metadata.created_at);
    println!();
    println!("Gems: {} total", pack_lock.len());
    println!();

    // Show first 20 gems
    let mut gem_list: Vec<_> = pack_lock.gems.values().collect();
    gem_list.sort_by(|a, b| a.name.0.cmp(&b.name.0));

    for gem in gem_list.iter().take(20) {
        println!("  {} ({})", gem.name.0, gem.version.0);
    }

    if pack_lock.len() > 20 {
        println!("  ... and {} more", pack_lock.len() - 20);
    }

    Ok(())
}

fn run_run(task: &str) -> anyhow::Result<()> {
    let packfile = pack_gemfile::Packfile::find()
        .map_err(|e| anyhow::anyhow!("failed to find Packfile: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("no Packfile found in current directory. Create a Packfile to define tasks."))?;

    info!("Running task: {}", task);
    packfile.run(task)
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    Ok(())
}

fn run_tasks(verbose: bool) -> anyhow::Result<()> {
    let packfile = pack_gemfile::Packfile::find()
        .map_err(|e| anyhow::anyhow!("failed to find Packfile: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("no Packfile found in current directory. Create a Packfile to define tasks."))?;

    let tasks: Vec<_> = packfile.task_names().into_iter().collect();

    if tasks.is_empty() {
        println!("No tasks defined in Packfile.");
        return Ok(());
    }

    println!("Available tasks:");

    if verbose {
        for name in &tasks {
            if let Some(t) = packfile.get(name) {
                println!("  {}", name);
                if let Some(desc) = &t.description {
                    println!("    {}", desc);
                }
                println!("    Command: {}", t.command);
            }
        }
    } else {
        for name in &tasks {
            println!("  {}", name);
        }
    }

    Ok(())
}

// Rails-specific commands
fn run_server(port: Option<u16>, detached: bool) -> anyhow::Result<()> {
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    app.server(port, detached)
}

fn run_console() -> anyhow::Result<()> {
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    app.console()
}

fn run_test(args: &[String]) -> anyhow::Result<()> {
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    app.test(args)
}

fn run_rspec(args: &[String]) -> anyhow::Result<()> {
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    app.rspec(args)
}

fn run_db(operation: &str) -> anyhow::Result<()> {
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    app.db(operation)
}

fn run_assets(operation: &str) -> anyhow::Result<()> {
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    app.assets(operation)
}

fn run_cache(operation: &str) -> anyhow::Result<()> {
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    app.cache(operation)
}

fn run_docker(_dockerfile_only: bool) -> anyhow::Result<()> {
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    app.generate_docker()?;
    Ok(())
}

fn run_new(name: &str, skip_bundle: bool, skip_lock: bool, docker: bool, database: Option<&str>, assets: Option<&str>) -> anyhow::Result<()> {
    println!();
    println!("========================================");
    println!("     Creating new Rails project: {}", name);
    println!("========================================");
    println!();

    // Build rails new command
    let mut args = vec!["new".to_string(), name.to_string()];

    // Add --skip-bundle if requested (we use pack instead of bundle)
    if skip_bundle {
        args.push("--skip-bundle".to_string());
    }

    // Add database option
    if let Some(db) = database {
        args.push(format!("--database={}", db));
    }

    // Add asset pipeline option
    if let Some(asset) = assets {
        args.push(format!("--asset-pipeline={}", asset));
    }

    println!("Running: rails {}", args.join(" "));
    println!();

    // Run rails new
    let status = Command::new("rails")
        .args(&args)
        .status()
        .context("rails new failed")?;

    if !status.success() {
        anyhow::bail!("rails new failed with exit code: {:?}", status.code());
    }

    // Change to project directory
    std::env::set_current_dir(name)?;

    println!();
    println!("Setting up pack...");
    println!();

    // Generate pack.lock if requested
    if !skip_lock {
        println!("Generating pack.lock...");
        let pack_lock = pack_gemfile::PackLock::new();
        let lock_path = PathBuf::from("pack.lock");
        pack_lock.write_binary(&lock_path)
            .context("failed to write pack.lock")?;
    }

    // Generate .packignore
    println!("Generating .packignore...");
    let packignore = std::path::PathBuf::from(".packignore");
    let packignore_content = r#"# Pack ignore - gems to not cache
# Similar to .dockerignore for faster installs

.ruby-version
.ruby-gemset
*.md
docs/
log/*.log
tmp/
.DS_Store
.idea/
.vscode/
node_modules/
"#;
    std::fs::write(&packignore, packignore_content)?;

    // Generate Docker files if requested
    if docker {
        println!("Generating Docker files...");
        let dockerfile = std::path::PathBuf::from("Dockerfile.pack");
        let dockerfile_content = r#"# Dockerfile for Rails with Pack
FROM ruby:3.3-slim

# Install system dependencies
RUN apt-get update -qq && apt-get install -y --no-install-recommends \
    curl \
    postgresql-client \
    nodejs \
    && rm -rf /var/lib/apt/lists/*

# Install Pack
RUN curl -fsSL https://pack.dev/install.sh | bash
ENV PATH="/usr/local/bin:$PATH"

WORKDIR /app

# Copy Gemfiles
COPY Gemfile Gemfile.lock* ./

# Install gems with pack (faster than bundle install)
RUN bundle install

# Copy application code
COPY . .

# Precompile assets
RUN bundle exec rails assets:precompile

EXPOSE 3000
CMD ["bundle", "exec", "rails", "server", "-b", "0.0.0.0"]
"#;
        std::fs::write(&dockerfile, dockerfile_content)?;

        let compose = std::path::PathBuf::from("docker-compose.pack.yml");
        let compose_content = r#"# Docker Compose for Rails with Pack
version: '3.8'

services:
  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_PASSWORD: password
    volumes:
      - postgres_data:/var/lib/postgresql/data

  app:
    build: .
    command: bash -c "rm -f tmp/pids/server.pid && bundle exec rails server -b 0.0.0.0"
    volumes:
      - .:/app
    ports:
      - "3000:3000"
    depends_on:
      - db
    environment:
      DATABASE_URL: postgres://postgres:password@db:5432/app_development
      RAILS_ENV: development

volumes:
  postgres_data:
"#;
        std::fs::write(&compose, compose_content)?;
    }

    println!();
    println!("========================================");
    println!("     Rails project created!");
    println!("========================================");
    println!();
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  pack install        # Install gems with pack");
    println!("  pack server        # Start Rails server");
    if docker {
        println!("  docker compose -f docker-compose.pack.yml up  # Start with Docker");
    }
    println!();

    Ok(())
}

fn run_init(skip_packignore: bool, skip_docker: bool) -> anyhow::Result<()> {
    // Check if this is a Rails project
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    println!();
    println!("Initializing pack for Rails project...");
    println!();

    // Generate pack.lock
    println!("Generating pack.lock...");
    let pack_lock = pack_gemfile::PackLock::new();
    let lock_path = app.path().join("pack.lock");
    pack_lock.write_binary(&lock_path)
        .context("failed to write pack.lock")?;

    // Generate .packignore
    if !skip_packignore {
        println!("Generating .packignore...");
        let packignore = app.path().join(".packignore");
        let packignore_content = r#"# Pack ignore - gems to not cache

.ruby-version
.ruby-gemset
*.md
docs/
log/*.log
tmp/
.DS_Store
.idea/
.vscode/
node_modules/
"#;
        std::fs::write(&packignore, packignore_content)?;
    }

    // Generate Docker files
    if !skip_docker {
        println!("Generating Docker files...");
        app.generate_docker()?;
    }

    println!();
    println!("Pack initialized!");
    println!();
    println!("Next steps:");
    println!("  pack install       # Install gems with pack");
    println!("  pack lock          # Update pack.lock");
    println!("  pack docker        # Regenerate Docker files");
    println!();

    Ok(())
}

fn run_rails(args: &[String]) -> anyhow::Result<()> {
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    let rails_args: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
    app.run_rails_cmd(&rails_args)
}

fn run_rake(task: &str) -> anyhow::Result<()> {
    let app = rails::RailsApp::discover()
        .map_err(|e| anyhow::anyhow!("not a Rails app: {}", e))?
        .ok_or_else(|| anyhow::anyhow!("not a Rails app. Run from your Rails project directory."))?;

    app.run_rake(task)
}

fn run_plugins(action: pack_exec::PluginAction) -> anyhow::Result<()> {
    let mut manager = pack_exec::PluginManager::new();
    let _ = manager.load_all();

    match action {
        pack_exec::PluginAction::List { format } => {
            let plugins = manager.list_all();
            if plugins.is_empty() {
                println!("No plugins installed.");
                println!("Plugin directories:");
                for dir in manager.plugin_dirs() {
                    println!("  - {}", dir.display());
                }
                println!("\nUse 'pack plugins init <name>' to create a plugin");
                return Ok(());
            }

            match format {
                OutputFormat::Json => {
                    #[derive(serde::Serialize)]
                    struct PluginJson<'a> {
                        name: &'a str,
                        version: &'a str,
                        description: &'a str,
                        commands: &'a [String],
                        enabled: bool,
                        path: String,
                    }
                    let json_plugins: Vec<PluginJson> = plugins.iter().map(|p| PluginJson {
                        name: &p.name,
                        version: &p.version,
                        description: &p.description,
                        commands: &p.commands,
                        enabled: p.enabled,
                        path: p.path.display().to_string(),
                    }).collect();
                    println!("{}", serde_json::to_string_pretty(&json_plugins).unwrap());
                }
                OutputFormat::Table => {
                    println!("Installed Plugins");
                    println!("------------------");
                    for plugin in plugins {
                        let status = if plugin.enabled { "enabled" } else { "disabled" };
                        println!("{} ({})", plugin.name, status);
                        println!("  Version: {}", plugin.version);
                        println!("  Description: {}", plugin.description);
                        println!("  Path: {}", plugin.path.display());
                        if !plugin.commands.is_empty() {
                            println!("  Commands: {}", plugin.commands.join(", "));
                        }
                        println!();
                    }
                }
                OutputFormat::Quiet => {
                    for plugin in plugins {
                        println!("{}", plugin.name);
                    }
                }
            }
        }
        pack_exec::PluginAction::Load { path, verbose } => {
            let dir = if let Some(p) = path {
                std::path::PathBuf::from(p)
            } else {
                let home = std::env::var("HOME")
                    .map(|h| std::path::PathBuf::from(h).join(".pack").join("plugins"))
                    .unwrap_or_else(|_| std::path::PathBuf::from(".pack/plugins"));
                home
            };

            if verbose {
                println!("Loading plugins from: {}", dir.display());
            }

            if !dir.exists() {
                println!("Plugin directory does not exist: {}", dir.display());
                println!("Creating directory...");
                std::fs::create_dir_all(&dir).map_err(|e| anyhow::anyhow!("failed to create directory: {}", e))?;
            }

            let loaded = manager.load_from_dir(&dir)?;
            println!("Loaded {} plugin(s)", loaded.len());
            for plugin in &loaded {
                println!("  - {} v{} ({})", plugin.name, plugin.version, plugin.path.display());
            }
        }
        pack_exec::PluginAction::Reload { verbose } => {
            if verbose {
                println!("Reloading all plugins...");
            }
            let loaded = manager.reload()?;
            println!("Reloaded {} plugin(s)", loaded.len());
        }
        pack_exec::PluginAction::Run { plugin, args } => {
            info!("Running plugin {} with args {:?}", plugin, args);

            let output = std::process::Command::new(&plugin)
                .args(&args)
                .output()
                .map_err(|e| anyhow::anyhow!("failed to execute {}: {}", plugin, e))?;

            std::io::stdout().write_all(&output.stdout).ok();
            std::io::stderr().write_all(&output.stderr).ok();

            if !output.status.success() {
                std::process::exit(output.status.code().unwrap_or(1));
            }
        }
        pack_exec::PluginAction::Search { pattern } => {
            let results = manager.search(pattern.as_deref());

            if results.is_empty() {
                println!("No plugins found matching '{}'", pattern.as_deref().unwrap_or("*"));
            } else {
                println!("Found {} plugin(s):", results.len());
                for plugin in results {
                    println!("  {} v{} - {}", plugin.name, plugin.version, plugin.description);
                }
            }
        }
        pack_exec::PluginAction::Validate { fix } => {
            println!("Validating plugins...");
            let results = manager.validate_plugins();

            let mut valid_count = 0;
            let mut invalid_count = 0;

            for result in results {
                if result.valid {
                    valid_count += 1;
                    println!("✓ {} - valid", result.plugin_name);
                } else {
                    invalid_count += 1;
                    println!("✗ {} - invalid", result.plugin_name);
                    for issue in result.issues {
                        println!("  - {}", issue);
                        if fix {
                            println!("    (auto-fix not available for this issue)");
                        }
                    }
                }
            }

            println!("\nSummary: {} valid, {} invalid", valid_count, invalid_count);
        }
        pack_exec::PluginAction::Init { name, path, template } => {
            let target_path = if let Some(p) = path {
                std::path::PathBuf::from(p)
            } else {
                let home = std::env::var("HOME")
                    .map(|h| std::path::PathBuf::from(h).join(".pack").join("plugins"))
                    .unwrap_or_else(|_| std::path::PathBuf::from(".pack/plugins"));
                home
            };

            println!("Creating plugin '{}' at {}", name, target_path.display());

            let plugin = manager.init_plugin(&name, &target_path, template)
                .map_err(|e| anyhow::anyhow!("failed to init plugin: {}", e))?;

            println!("✓ Plugin created successfully");
            println!("  Path: {}", plugin.path.display());
            println!("  Version: {}", plugin.version);
            println!("\nTo enable, run: pack plugins load");
        }
        pack_exec::PluginAction::Uninstall { name, purge } => {
            let removed = manager.uninstall_plugin(&name, purge)
                .map_err(|e| anyhow::anyhow!("failed to uninstall plugin: {}", e))?;

            if removed {
                println!("✓ Plugin '{}' uninstalled", name);
                if purge {
                    println!("  (plugin data and config removed)");
                }
            } else {
                println!("Plugin '{}' not found", name);
            }
        }
        pack_exec::PluginAction::Info { name } => {
            let plugin = manager.get(&name)
                .ok_or_else(|| anyhow::anyhow!("plugin '{}' not found", name))?;

            println!("Plugin: {}", plugin.name);
            println!("Version: {}", plugin.version);
            println!("Description: {}", plugin.description);
            println!("Path: {}", plugin.path.display());
            println!("Enabled: {}", plugin.enabled);

            if !plugin.commands.is_empty() {
                println!("Commands:");
                for cmd in &plugin.commands {
                    println!("  - {}", cmd);
                }
            }

            if plugin.path.exists() {
                let metadata = std::fs::metadata(&plugin.path)?;
                println!("Executable: {}", plugin.is_executable());
                println!("Size: {} bytes", metadata.len());
            } else {
                println!("Status: FILE MISSING");
            }
        }
    }

    Ok(())
}