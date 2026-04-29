use clap::{Parser, Subcommand};
use pack_core::{Project, RubyEnvironment, GemName};
use pack_gemfile::{load_lockfile, find_dependency_path, add_gem, remove_gem};
use std::io::Write;
use std::process::Command;

const VERSION: &str = "0.1.0";

#[derive(Parser)]
#[command(name = "pack")]
#[command(version = VERSION)]
#[command(about = "Fast Ruby package management")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Install missing gems
    Install,
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
    /// Execute a command
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
    /// Diagnose the local Ruby project
    Doctor,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Doctor) => {
            run_doctor();
        }
        Some(Commands::Exec { command, args }) => {
            run_exec(&command, &args);
        }
        Some(Commands::Install) => {
            run_install();
        }
        Some(Commands::Add { gem, version, group, no_install }) => {
            run_add(&gem, version.as_deref(), group.as_deref(), no_install);
        }
        Some(Commands::Remove { gem, no_install }) => {
            run_remove(&gem, no_install);
        }
        Some(Commands::Update { gem }) => {
            run_update(gem.as_deref());
        }
        Some(Commands::Why { gem }) => {
            run_why(&gem);
        }
        None => {
            println!("pack {}", VERSION);
        }
    }
}

fn run_doctor() {
    println!("Pack {}", VERSION);
    println!();

    let project = match Project::discover() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };
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

    println!("Status");
    if warnings == 0 {
        println!("  ok");
    } else {
        println!("  {} warning(s)", warnings);
    }
}

fn run_exec(command: &str, args: &[String]) {
    let output = Command::new("bundle")
        .arg("exec")
        .arg(command)
        .args(args)
        .output();

    match output {
        Ok(o) => {
            std::io::stdout().write_all(&o.stdout).ok();
            std::io::stderr().write_all(&o.stderr).ok();
            std::process::exit(o.status.code().unwrap_or(1));
        }
        Err(e) => {
            eprintln!("Error: failed to execute bundle exec {}: {}", command, e);
            std::process::exit(1);
        }
    }
}

fn run_install() {
    let project = match Project::discover() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if project.gemfile.is_none() {
        eprintln!("Error: no Gemfile found in current directory");
        std::process::exit(1);
    }

    println!("Installing gems...");

    let output = Command::new("bundle")
        .arg("install")
        .current_dir(&project.path)
        .output();

    match output {
        Ok(o) => {
            std::io::stdout().write_all(&o.stdout).ok();
            std::io::stderr().write_all(&o.stderr).ok();
            if o.status.success() {
                println!("Done.");
            } else {
                std::process::exit(o.status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("Error: failed to run bundle install: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_add(gem: &str, version: Option<&str>, group: Option<&str>, no_install: bool) {
    let project = match Project::discover() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let gemfile_path = match &project.gemfile {
        Some(p) => p,
        None => {
            eprintln!("Error: no Gemfile found in current directory");
            std::process::exit(1);
        }
    };

    if let Err(e) = add_gem(gemfile_path, gem, version, group) {
        eprintln!("Error: failed to add gem: {}", e);
        std::process::exit(1);
    }

    println!("Added {} to Gemfile", gem);

    if !no_install {
        run_install();
    }
}

fn run_remove(gem: &str, no_install: bool) {
    let project = match Project::discover() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let gemfile_path = match &project.gemfile {
        Some(p) => p,
        None => {
            eprintln!("Error: no Gemfile found in current directory");
            std::process::exit(1);
        }
    };

    match remove_gem(gemfile_path, gem) {
        Ok(true) => println!("Removed {} from Gemfile", gem),
        Ok(false) => {
            eprintln!("Error: gem {} not found in Gemfile", gem);
            std::process::exit(1);
        }
        Err(e) => {
            eprintln!("Error: failed to remove gem: {}", e);
            std::process::exit(1);
        }
    }

    if !no_install {
        run_install();
    }
}

fn run_update(gem: Option<&str>) {
    let project = match Project::discover() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    if project.gemfile.is_none() {
        eprintln!("Error: no Gemfile found in current directory");
        std::process::exit(1);
    }

    println!("Updating gems...");

    let mut cmd = Command::new("bundle");
    cmd.arg("update");
    if let Some(g) = gem {
        cmd.arg(g);
    }

    let output = cmd.current_dir(&project.path).output();

    match output {
        Ok(o) => {
            std::io::stdout().write_all(&o.stdout).ok();
            std::io::stderr().write_all(&o.stderr).ok();
            if o.status.success() {
                println!("Done.");
            } else {
                std::process::exit(o.status.code().unwrap_or(1));
            }
        }
        Err(e) => {
            eprintln!("Error: failed to run bundle update: {}", e);
            std::process::exit(1);
        }
    }
}

fn run_why(gem: &str) {
    let project = match Project::discover() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    };

    let lockfile_path = match &project.gemfile_lock {
        Some(p) => p,
        None => {
            eprintln!("Error: no Gemfile.lock found. Run `pack install` first.");
            std::process::exit(1);
        }
    };

    let lockfile = match load_lockfile(lockfile_path) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Error: failed to parse Gemfile.lock: {}", e);
            std::process::exit(1);
        }
    };

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
}
