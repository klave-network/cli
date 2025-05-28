use anyhow::{Context, Result, anyhow};
use colored::*;
use dialoguer::{Confirm, theme::ColorfulTheme};
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

const KLAVE_CYAN_BG: &str = "Klave - The honest-by-design platform";

struct BuildResult {
    app: String,
    success: bool,
    app_type: String,
    time: Duration,
}

/// Check if a command is available in the PATH
async fn is_command_available(command: &str) -> bool {
    let check_cmd = if cfg!(target_os = "windows") {
        format!("where {}", command)
    } else {
        format!("which {}", command)
    };

    Command::new(if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    })
    .arg(if cfg!(target_os = "windows") {
        "/C"
    } else {
        "-c"
    })
    .arg(&check_cmd)
    .output()
    .map(|output| output.status.success())
    .unwrap_or(false)
}

/// Resolve the package manager being used in the project
fn resolve_package_manager() -> String {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    if Path::new(&cwd).join("yarn.lock").exists() {
        return "yarn".to_string();
    } else if Path::new(&cwd).join("pnpm-lock.yaml").exists() {
        return "pnpm".to_string();
    } else if Path::new(&cwd).join("package-lock.json").exists() {
        return "npm".to_string();
    } else {
        return "npm".to_string(); // Default to npm
    }
}

/// Check if dependencies are installed for the project
fn are_dependencies_installed(cwd: &Path) -> bool {
    // Basic check for node_modules directory existence
    let node_modules = cwd.join("node_modules");
    if !node_modules.exists() {
        return false;
    } else {
        return true;
    }
}

/// Install dependencies using the detected package manager
fn install_dependencies(cwd: &Path, package_manager: &str) -> Result<bool> {
    println!("Installing dependencies...");

    let (cmd, args) = match package_manager {
        "npm" => ("npm", vec!["install", "--legacy-peer-deps"]),
        "yarn" => ("yarn", vec!["install"]),
        "pnpm" => ("pnpm", vec!["install"]),
        _ => ("npm", vec!["install", "--legacy-peer-deps"]),
    };

    println!("Running: {} {}", cmd, args.join(" "));

    let status = std::process::Command::new(cmd)
        .args(&args)
        .current_dir(cwd)
        .status()
        .context(format!("Failed to run {} install", package_manager))?;

    if status.success() {
        println!("{}", "Dependencies installed successfully.".green());
        Ok(true)
    } else {
        println!("{}", "Failed to install dependencies.".red());
        Ok(false)
    }
}

/// Run command and capture output
async fn run_command(
    command: &str,
    args: &[&str],
    cwd: &Path,
    inherit_stdio: bool,
) -> Result<Output> {
    let mut cmd = Command::new(command);
    cmd.args(args).current_dir(cwd);

    if inherit_stdio {
        cmd.stdout(std::process::Stdio::inherit())
            .stderr(std::process::Stdio::inherit());
    }

    cmd.output()
        .context(format!("Failed to execute command: {} {:?}", command, args))
}

/// Main build command implementation
pub async fn execute(app: Option<String>, skip_checks: bool, verbose: bool) -> Result<()> {
    // Get current working directory
    let cwd = env::current_dir().context("Failed to get current directory")?;

    // Check if klave.json exists
    let klave_config_path = cwd.join("klave.json");
    if !klave_config_path.exists() {
        return Err(anyhow!(
            "Error: klave.json file not found. Make sure you are in a Klave project directory. Run 'klave create' to start a new project."
        ));
    }

    // Read the klave config
    let klave_config_str =
        fs::read_to_string(&klave_config_path).context("Failed to read klave.json")?;

    let klave_config: Value =
        serde_json::from_str(&klave_config_str).context("Invalid JSON in klave.json")?;

    // Get applications array
    let applications = klave_config
        .get("applications")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            anyhow!("Error: Invalid klave.json file structure. 'applications' array not found.")
        })?;

    // Filter applications based on app argument
    let apps_to_process = if let Some(app_name) = &app {
        let filtered = applications
            .iter()
            .filter(|a| {
                a.get("slug").and_then(|s| s.as_str()) == Some(app_name)
                    || a.get("name").and_then(|s| s.as_str()) == Some(app_name)
            })
            .collect::<Vec<_>>();

        if filtered.is_empty() {
            // List available apps if the specified app wasn't found
            let available_apps: Vec<String> = applications
                .iter()
                .filter_map(|a| {
                    a.get("slug")
                        .and_then(|s| s.as_str())
                        .or_else(|| a.get("name").and_then(|s| s.as_str()))
                })
                .map(|s| s.to_string())
                .collect();

            return Err(anyhow!(
                "Error: No application found with name \"{}\". Available applications: {}",
                app_name,
                available_apps.join(", ")
            ));
        }

        filtered
    } else {
        applications.iter().collect::<Vec<_>>()
    };

    if apps_to_process.is_empty() {
        return Err(anyhow!("Error: No applications found in klave.json"));
    }

    println!("\n");
    println!("{}", KLAVE_CYAN_BG.on_cyan().black().bold());
    println!(
        "Building {}",
        if let Some(app_name) = &app {
            format!("application \"{}\"", app_name)
        } else {
            format!("{} applications", apps_to_process.len())
        }
    );

    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ ")
            .template("{spinner:.blue} {msg}")
            .unwrap(),
    );

    spinner.set_message("Analyzing project structure");

    // Check project structure
    let has_package_json = cwd.join("package.json").exists();

    // Only check for tools if not skipped
    let (has_node, has_npm, has_cargo, has_cargo_component) = if !skip_checks {
        // Check tools availability
        let has_node = is_command_available("node").await;
        let has_npm = is_command_available("npm").await;
        let has_cargo = is_command_available("cargo").await;
        let has_cargo_component = if has_cargo {
            is_command_available("cargo-component").await
        } else {
            false
        };

        // Build will need these tools
        let mut missing_tools = Vec::new();

        // Check if apps need specific tools
        let needs_rust = apps_to_process.iter().any(|app| {
            let root_dir = app.get("rootDir").and_then(|s| s.as_str()).unwrap_or(".");
            let app_dir = if root_dir.starts_with('/') {
                cwd.join(&root_dir[1..])
            } else {
                cwd.join(root_dir)
            };
            app_dir.join("Cargo.toml").exists()
        });

        let needs_assemblyscript = apps_to_process.iter().any(|app| {
            let root_dir = app.get("rootDir").and_then(|s| s.as_str()).unwrap_or(".");
            let app_dir = if root_dir.starts_with('/') {
                cwd.join(&root_dir[1..])
            } else {
                cwd.join(root_dir)
            };
            app_dir.join("tsconfig.json").exists()
        });

        if needs_rust && !has_cargo {
            missing_tools.push("Rust toolchain (install from https://rustup.rs/)");
        }

        if needs_rust && !has_cargo_component {
            missing_tools.push("cargo-component (install with: cargo install cargo-component)");
        }

        if needs_assemblyscript && !has_node {
            missing_tools.push("Node.js (install from https://nodejs.org/)");
        }

        if needs_assemblyscript && !has_npm {
            missing_tools.push("npm (comes with Node.js installation)");
        }

        if !missing_tools.is_empty() {
            spinner.finish_with_message("Project analysis complete");
            eprintln!("{}", "Warning: Missing required tools".yellow());
            eprintln!("The following tools are required but not found:");

            for tool in missing_tools {
                eprintln!("  - {}", tool);
            }

            eprintln!("\nYou can continue with --skip-checks flag, but builds may fail.");

            if !Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Continue anyway?")
                .default(false)
                .interact()
                .unwrap_or(false)
            {
                return Err(anyhow!("Build aborted due to missing tools"));
            }
        }

        (has_node, has_npm, has_cargo, has_cargo_component)
    } else {
        // Skip checks, assume tools are available
        (true, true, true, true)
    };

    // Determine package manager if needed
    let package_manager = if has_package_json {
        resolve_package_manager()
    } else {
        String::new()
    };

    // Check if dependencies are installed for AssemblyScript projects
    let needs_dependencies = has_package_json
        && apps_to_process.iter().any(|app| {
            let root_dir = app.get("rootDir").and_then(|s| s.as_str()).unwrap_or(".");
            let app_dir = if root_dir.starts_with('/') {
                cwd.join(&root_dir[1..])
            } else {
                cwd.join(root_dir)
            };
            app_dir.join("tsconfig.json").exists()
        });

    if needs_dependencies && !are_dependencies_installed(&cwd) {
        spinner.finish_with_message("Project analysis complete");

        println!("{}", "Dependencies not installed".yellow());
        println!(
            "You need to install dependencies for your AssemblyScript project before building."
        );

        // Auto-install or prompt based on skip_checks
        if skip_checks {
            println!("Automatically installing dependencies due to --skip-checks...");
            if !install_dependencies(&cwd, &package_manager)? {
                return Err(anyhow!("Build aborted: failed to install dependencies"));
            }
        } else {
            if Confirm::with_theme(&ColorfulTheme::default())
                .with_prompt("Would you like to install dependencies now?")
                .default(true)
                .interact()?
            {
                if !install_dependencies(&cwd, &package_manager)? {
                    return Err(anyhow!("Build aborted: failed to install dependencies"));
                }
            } else {
                // User chose not to install dependencies
                if !Confirm::with_theme(&ColorfulTheme::default())
                    .with_prompt(
                        "Continue without installing dependencies? (build will likely fail)",
                    )
                    .default(false)
                    .interact()?
                {
                    return Err(anyhow!("Build aborted: dependencies not installed"));
                }
            }
        }

        // Reset spinner after dependency installation
        spinner.set_message("Continuing build process...");
    }

    if verbose {
        spinner.finish_with_message(format!(
            "Project analysis complete: found {} apps",
            apps_to_process.len()
        ));
    } else {
        spinner.finish_with_message("Project analysis complete");
    }

    // Track build status for summary
    let mut build_results: Vec<BuildResult> = Vec::new();

    // Build each application
    for application in apps_to_process {
        let app_slug = application
            .get("slug")
            .and_then(|s| s.as_str())
            .or_else(|| application.get("name").and_then(|s| s.as_str()))
            .unwrap_or("unknown");

        let root_dir = application
            .get("rootDir")
            .and_then(|s| s.as_str())
            .unwrap_or(".");

        let app_dir = if root_dir.starts_with('/') {
            cwd.join(&root_dir[1..])
        } else {
            cwd.join(root_dir)
        };

        if !app_dir.exists() {
            eprintln!(
                "{}",
                format!(
                    "Warning: Directory not found for app \"{}\" at {:?}",
                    app_slug, app_dir
                )
                .yellow()
            );
            build_results.push(BuildResult {
                app: app_slug.to_string(),
                success: false,
                app_type: "unknown".to_string(),
                time: Duration::from_secs(0),
            });
            continue;
        }

        // Determine app type - simplified to just rust or assemblyscript
        let app_type = if app_dir.join("Cargo.toml").exists() {
            "rust"
        } else if app_dir.join("tsconfig.json").exists() {
            "assemblyscript"
        } else {
            "unknown"
        };

        if app_type == "unknown" {
            eprintln!(
                "{}",
                format!("Warning: Could not determine app type for \"{}\"", app_slug).yellow()
            );
            build_results.push(BuildResult {
                app: app_slug.to_string(),
                success: false,
                app_type: app_type.to_string(),
                time: Duration::from_secs(0),
            });
            continue;
        }

        let start_time = Instant::now();
        spinner.set_message(format!("Building {} app \"{}\"", app_type, app_slug));

        let build_result = match app_type {
            "rust" => {
                // Check if Rust tools are available
                if !has_cargo {
                    Err(anyhow!(
                        "Rust toolchain not found. Please install Rust from https://rustup.rs/"
                    ))
                } else if !has_cargo_component {
                    Err(anyhow!(
                        "cargo-component not found. Please install with: cargo install cargo-component"
                    ))
                } else {
                    // Build Rust application
                    run_command(
                        "cargo",
                        &[
                            "component",
                            "build",
                            "--target",
                            "wasm32-unknown-unknown",
                            "--release",
                        ],
                        &app_dir,
                        true,
                    )
                    .await
                    .map(|_| ())
                }
            }
            "assemblyscript" => {
                // Check if Node.js tools are available
                if !has_node {
                    Err(anyhow!(
                        "Node.js not found. Please install Node.js from https://nodejs.org/"
                    ))
                } else if !has_npm {
                    Err(anyhow!(
                        "npm not found. It usually comes with Node.js installation."
                    ))
                } else {
                    // Build AssemblyScript application
                    let (build_command, build_args) = match package_manager.as_str() {
                        "npm" => ("npm", vec!["run", "build", "--", "--app", app_slug]),
                        "yarn" => ("yarn", vec!["build", "--app", app_slug]),
                        "pnpm" => ("pnpm", vec!["build", "--app", app_slug]),
                        _ => ("npm", vec!["run", "build"]),
                    };

                    run_command(build_command, &build_args, &cwd, true)
                        .await
                        .map(|_| ())
                }
            }
            _ => Err(anyhow!("Unknown app type")),
        };

        let elapsed = start_time.elapsed();

        match build_result {
            Ok(_) => {
                spinner.finish_with_message(
                    format!(
                        "Successfully built {} app \"{}\" in {:.2}s",
                        app_type,
                        app_slug,
                        elapsed.as_secs_f64()
                    )
                    .green()
                    .to_string(),
                );

                build_results.push(BuildResult {
                    app: app_slug.to_string(),
                    success: true,
                    app_type: app_type.to_string(),
                    time: elapsed,
                });
            }
            Err(error) => {
                spinner.finish_with_message(
                    format!("Failed to build {} app \"{}\"", app_type, app_slug)
                        .red()
                        .to_string(),
                );

                eprintln!(
                    "{}",
                    format!("Error building \"{}\": {}", app_slug, error).red()
                );

                // Provide helpful installation instructions based on error
                if app_type == "rust" {
                    if !has_cargo {
                        println!("\nTo install Rust:\n");
                        println!("    - Visit the Rust homepage: https://rustup.rs/");
                        println!(
                            "    - Add WebAssembly target: rustup target add wasm32-unknown-unknown"
                        );
                        println!("    - Install cargo-component: cargo install cargo-component");
                    } else if !has_cargo_component {
                        println!("\nTo install cargo-component:\n");
                        println!("    - Run in your terminal: cargo install cargo-component");
                        println!(
                            "    - Make sure you also have the WebAssembly target: rustup target add wasm32-unknown-unknown"
                        );
                    } else if error.to_string().contains("unknown target") {
                        println!("\nTo add the WebAssembly target:\n");
                        println!(
                            "    - Run in your terminal: rustup target add wasm32-unknown-unknown"
                        );
                    }
                } else if app_type == "assemblyscript" {
                    if !has_node {
                        println!("\nTo install Node.js:\n");
                        println!(
                            "    - Visit the Node.js homepage: https://nodejs.org/en/download/"
                        );
                    } else if error.to_string().contains("Cannot find module") {
                        println!("\nMissing dependencies detected. Try:\n");
                        println!("    - {} install", package_manager);
                    }
                }

                build_results.push(BuildResult {
                    app: app_slug.to_string(),
                    success: false,
                    app_type: app_type.to_string(),
                    time: elapsed,
                });
            }
        }
    }

    // Show summary
    let total = build_results.len();
    let successful = build_results.iter().filter(|r| r.success).count();

    let summary = format!(
        "\n{}: {} apps\n{}: {} apps\n{}",
        "Total builds".bold(),
        total,
        "Successful builds".bold(),
        successful,
        if successful < total {
            format!("{}: {} apps", "Failed builds".bold(), total - successful)
        } else {
            String::new()
        }
    );

    println!("\nBuild summary:\n{}", summary);

    // Next steps if builds succeeded
    if successful > 0 {
        println!("\n{}", "Next steps:".bold());
        println!("  1. {} your application to Klave", "Deploy".green().bold());
        println!("     Run: {}", "klave deploy (wip)".cyan());
        println!("  2. Test and monitor your application");
        println!("     Visit the Klave platform: https://app.klave.com");
    }

    println!("\nDocs: https://docs.klave.com");
    println!("Stuck? Reach out to us on Discord: https://discord.gg/klave");

    // Detailed results
    println!("\n{}", "Build details:".bold());
    for result in &build_results {
        let status = if result.success {
            "✓ Success".green()
        } else {
            "✗ Failed".red()
        };

        let time = if result.success {
            format!("({:.2}s)", result.time.as_secs_f64()).dimmed()
        } else {
            "".normal()
        };

        println!(
            "{} {} [{}] {}",
            status,
            result.app.bold(),
            result.app_type,
            time
        );
    }

    // Exit with error code if any builds failed
    if successful < total {
        std::process::exit(1);
    }

    Ok(())
}
