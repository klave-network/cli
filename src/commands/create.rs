use std::error::Error;
use std::path::PathBuf;
use dialoguer::{theme::ColorfulTheme, Input, Confirm, Select};
use console::style;

use crate::util::template;
use crate::util::git;

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
    println!("{}", style(" Klave - The honest-by-design platform ").black().on_cyan().bold());
    println!("Welcome to Klave. Let's create your honest application!");
    
    // Initialize tokio runtime for async operations
    let tr = tokio::runtime::Runtime::new()?;

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
        },
        Some(template) => template.clone()
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
    
    // Use git utility to get default author information
    let default_author_name = git::find_my_name();
    let default_author_email = git::find_github_email();

    // Get more project info
    let description: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("How would you describe your honest application?")
        .default("This is an honest application for the Klave Network".into())
        .interact()?;

    let author_name: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the name of the author?")
        .default(if default_author_name.is_empty() { "Your Name".into() } else { default_author_name })
        .interact()?;

    let author_email: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the email address of the author?")
        .default(if default_author_email.is_empty() { "your.email@example.com".into() } else { default_author_email })
        .interact()?;

    // Use git utility to find GitHub profile URL
    let default_author_url = tr.block_on(git::find_github_profile_url(&author_email));
    
    let author_url: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the URL to the author's GitHub profile?")
        .default(if default_author_url.is_empty() { "https://github.com/yourusername".into() } else { default_author_url })
        .interact()?;

    // Guess repository URL based on author URL and project name
    let default_repo_url = git::guess_repo_url(&author_url, &project_dir);

    let repo_url: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("What is the URL for the repository?")
        .default(if default_repo_url.is_empty() { "https://github.com/yourusername/my-repo".into() } else { default_repo_url })
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
    template::create_template(
        &target_dir,
        &project_name,
        &description,
        &author_name,
        &author_email,
        &author_url,
        &repo_url,
        &project_template,
    )?;

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
        }
    }

    // Install dependencies if requested
    if install_deps {
        println!("Installing dependencies...");
        // Determine package manager and install
        let package_manager = "npm"; // Simplified for this example

        let args = if package_manager == "npm" {
            vec!["install", "--legacy-peer-deps"]
        } else if package_manager == "yarn" {
            vec![]
        } else { // pnpm
            vec![]
        };

        let status = std::process::Command::new(package_manager)
            .args(&args)
            .current_dir(&target_dir)
            .status()?;

        if !status.success() {
            eprintln!("Warning: Failed to install dependencies");
        }
    }

    // Display next steps
    if project_template == "rust" {
        println!("
Build your Rust application:

    - Enter your project directory using cd {}
    - Make sure you have Rust toolchain installed: rustup target add wasm32-unknown-unknown
    - Make sure you have cargo-component installed: cargo install cargo-component
    - To build your application, run cargo component build --target wasm32-unknown-unknown --release
    - Log in to Klave to deploy your application

Documentation

    - Learn more about Klave here: https://docs.klave.com
    ", project_dir);
    } else {
        // AssemblyScript instructions
        let build_cmd = if install_deps { "build" } else { "run build" };

        println!("
Build your AssemblyScript application:

    - Enter your project directory using cd {}
    - To build your application, run npm {}
    - Log in to Klave to deploy your application

Documentation

    - Learn more about Klave here: https://docs.klave.com
    ", project_dir, build_cmd);
    }

    println!("Stuck? Reach out to us on Discord: https://discord.gg/klave");

    Ok(())
}
