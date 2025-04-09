use clap::{Parser, Subcommand};
use std::error::Error;

mod commands;
mod util;

#[derive(Parser)]
#[clap(author, version, about = "Klave CLI - The honest-by-design platform")]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Klave project
    Create {
        /// The name of the project
        #[clap(value_parser)]
        name: Option<String>,

        /// The template to use (assemblyscript or rust)
        #[clap(short, long, value_parser = ["rust", "assemblyscript"])]
        template: Option<String>,

        /// Skip git initialization
        #[clap(long)]
        no_git: bool,

        /// Skip dependency installation
        #[clap(long)]
        no_install: bool,

        /// Directory to create the project in
        #[clap(short, long)]
        dir: Option<String>,
    },
    
    /// Build Klave applications
    Build {
        /// Specific application to build (builds all if not specified)
        #[clap(short, long)]
        app: Option<String>,

        /// Skip checks for required tools
        #[clap(long)]
        skip_checks: bool,

        /// Output verbose build information
        #[clap(short, long)]
        verbose: bool,
    },
}

fn run() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Create { name, template, no_git, no_install, dir } => {
            commands::create::execute(
                name.clone(),
                template.clone(),
                *no_git,
                *no_install,
                dir.clone(),
            )?;
        },
        Commands::Build { app, skip_checks, verbose } => {
            // Create a tokio runtime for the async execute function
            let rt = tokio::runtime::Runtime::new()?;
            rt.block_on(commands::build::execute(
                app.clone(),
                *skip_checks,
                *verbose,
            ))?;
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
