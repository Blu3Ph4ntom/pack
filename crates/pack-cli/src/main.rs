use clap::{Parser, Subcommand};
use pack_core::{Project, RubyEnvironment};
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
    },
    /// Remove a gem from Gemfile
    Remove {
        #[arg(value_name = "GEM")]
        gem: String,
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
        Some(Commands::Add { gem, version, group }) => {
            run_add(&gem, version.as_deref(), group.as_deref());
        }
        Some(Commands::Remove { gem }) => {
            run_remove(&gem);
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
    println!("pack install - not yet implemented");
    std::process::exit(1);
}

fn run_add(gem: &str, version: Option<&str>, group: Option<&str>) {
    println!("pack add {} {:?} {:?}", gem, version, group);
    std::process::exit(1);
}

fn run_remove(gem: &str) {
    println!("pack remove {}", gem);
    std::process::exit(1);
}

fn run_update(gem: Option<&str>) {
    println!("pack update {:?}", gem);
    std::process::exit(1);
}

fn run_why(gem: &str) {
    println!("pack why {}", gem);
    std::process::exit(1);
}
