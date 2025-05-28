use colored::Colorize;
use console::style;
use dialoguer::{Confirm, Input, Select, theme::ColorfulTheme};
use std::error::Error;
use std::path::PathBuf;

use crate::util::template;

pub fn execute(
    name: Option<String>,
    template_type: Option<String>,
    no_git: bool,
    no_install: bool,
    dir: Option<String>,
) -> Result<(), Box<dyn Error>> {
    // Check if we're already in a Klave project
    let cwd = std::env::current_dir()?;
    let klave_config_path = cwd.join("klave.json");

    if klave_config_path.exists() {
        eprintln!("{}", style("Error: Found klave.json file. Looks like you already are in a Klave project. To add a new app to your project, run:

- klave add

Read more here: https://docs.klave.com/quickstart/create").red());
        return Err("Already in a Klave project".into());
    }

    println!("\n");
    println!(
        "{}",
        style(" Klave - The honest-by-design platform ")
            .black()
            .on_cyan()
            .bold()
    );
    println!("Welcome to Klave. Let's create your honest application!");

    // Determine template type
    let project_template = match &template_type {
        None => {
            let options = vec!["assemblyscript", "rust"];
            let selection = Select::with_theme(&ColorfulTheme::default())
                .with_prompt("What language would you like to use?")
                .items(&options)
                .default(0)
                .interact()?;
            options[selection].to_string()
        }
        Some(template) => template.clone(),
    };

    // Get project directory
    let project_dir = if let Some(d) = dir {
        d
    } else if name.is_none() {
        Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("Where should we create your project?")
            .default("./my-honest-app".into())
            .validate_with(|input: &String| -> Result<(), &str> {
                if !input.starts_with(".") {
                    return Err("Please enter a relative path.");
                }
                // TODO: Add more validation here
                Ok(())
            })
            .interact()?
    } else {
        format!("./{}", name.as_ref().unwrap())
    };

    // Get project name
    let project_name = if let Some(n) = name {
        n
    } else {
        Input::<String>::with_theme(&ColorfulTheme::default())
            .with_prompt("What is the name of your honest application?")
            .default("hello-world".into())
            .validate_with(|input: &String| -> Result<(), &str> {
                if input.is_empty() {
                    return Err("Project name is required");
                }
                // TODO: Add more validation here
                Ok(())
            })
            .interact()?
    };

    // Get more project info
    let description: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("How would you describe your honest application?")
        .default("This is an honest application for the Klave Network".into())
        .interact()?;

    // Initialize git
    let init_git = if no_git {
        false
    } else {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Initialize a git repository?")
            .default(true)
            .interact()?
    };

    // Install dependencies (AssemblyScript only)
    let install_deps = if no_install || project_template != "assemblyscript" {
        false
    } else {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt("Install dependencies?")
            .default(true)
            .interact()?
    };

    // Create the target directory
    let target_dir = PathBuf::from(&project_dir);
    std::fs::create_dir_all(&target_dir)?;

    // Create the project template
    template::create_template(&target_dir, &project_name, &description, &project_template)?;

    // Initialize git repository if requested
    if init_git {
        println!("Initializing git repository...");
        // Execute git init command
        let status = std::process::Command::new("git")
            .arg("init")
            .current_dir(&target_dir)
            .status()?;

        if !status.success() {
            eprintln!("Warning: Failed to initialize git repository");
        } else {
            println!("{}", "Git repository initialized successfully".green());
        }
    }

    // Install dependencies if requested
    if install_deps {
        println!("Installing dependencies...");
        // Determine package manager and install
        let package_manager = "npm"; // Simplified for this example

        // Use proper command arguments for each package manager
        let (cmd, args) = match package_manager {
            "npm" => ("npm", vec!["install", "--legacy-peer-deps"]),
            "yarn" => ("yarn", vec!["install"]),
            "pnpm" => ("pnpm", vec!["install"]),
            _ => ("npm", vec!["install", "--legacy-peer-deps"]),
        };

        println!("Running: {} {}", cmd, args.join(" "));

        let status = std::process::Command::new(cmd)
            .args(&args)
            .current_dir(&target_dir)
            .status()?;

        if !status.success() {
            eprintln!("{}", "Warning: Failed to install dependencies".yellow());
            eprintln!(
                "You can try installing them later with 'npm install' or during the build process"
            );
        } else {
            println!("{}", "Dependencies installed successfully".green());
        }
    } else if project_template == "assemblyscript" {
        println!("\n{}", "Note: Dependencies not installed".yellow());
        println!(
            "You chose to skip dependency installation. Dependencies will be installed automatically when you run 'klave build'."
        );
    }

    // Display next steps
    if project_template == "rust" {
        println!(
            "
    Build your Rust application:
    
    - Enter your project directory using cd {}
    - Make sure you have Rust toolchain installed: rustup target add wasm32-unknown-unknown
    - Make sure you have cargo-component installed: cargo install cargo-component
    - To build your application, run klave build
    - Log in to Klave to deploy your application
    
Documentation

    - Learn more about Klave here: https://docs.klave.com
    ",
            project_dir
        );
    } else {
        // AssemblyScript instructions
        println!(
            "
        Build your AssemblyScript application:
        
    - Enter your project directory using cd {}
    - To build your application, run klave build
    - Log in to Klave to deploy your application
    
Documentation

    - Learn more about Klave here: https://docs.klave.com
    ",
            project_dir
        );
    }

    println!("Stuck? Reach out to us on Discord: https://discord.gg/klave");

    Ok(())
}
