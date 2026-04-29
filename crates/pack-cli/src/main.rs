use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "pack")]
#[command(version = "0.1.0")]
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
            println!("pack doctor - not yet implemented");
        }
        Some(Commands::Exec { command, args }) => {
            println!("pack exec {} {:?}", command, args);
        }
        Some(Commands::Install) => {
            println!("pack install - not yet implemented");
        }
        Some(Commands::Add { gem, version, group }) => {
            println!("pack add {} {:?} {:?}", gem, version, group);
        }
        Some(Commands::Remove { gem }) => {
            println!("pack remove {}", gem);
        }
        Some(Commands::Update { gem }) => {
            println!("pack update {:?}", gem);
        }
        Some(Commands::Why { gem }) => {
            println!("pack why {}", gem);
        }
        None => {
            println!("pack 0.1.0");
        }
    }
}
