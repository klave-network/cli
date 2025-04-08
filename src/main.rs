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
