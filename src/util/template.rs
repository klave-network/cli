use std::path::Path;
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use include_dir::{include_dir, Dir};

// Embed templates in the binary
static ASSEMBLYSCRIPT_TEMPLATE: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates/assemblyscript");
static RUST_TEMPLATE: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates/rust");

pub fn create_template(
    target_dir: &Path,
    project_name: &str,
    description: &str,
    author_name: &str,
    author_email: &str,
    author_url: &str,
    repo_url: &str,
    template_type: &str,
) -> Result<(), Box<dyn Error>> {
    println!("Creating template files...");

    // Create a temporary extraction directory
    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    // Extract the appropriate template
    if template_type == "rust" {
        RUST_TEMPLATE.extract(temp_path)?;
    } else {
        ASSEMBLYSCRIPT_TEMPLATE.extract(temp_path)?;
    }

    // Copy template files to the target directory manually
    for entry in fs::read_dir(temp_path)? {
        let entry = entry?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dest_path = target_dir.join(&file_name);

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }

    // Update klave.json
    update_file(
        &target_dir.join("klave.json"),
        &[
            ("{{KLAVE_APP_SLUG}}", project_name),
            ("{{KLAVE_APP_DESCRIPTION}}", description),
            ("{{KLAVE_APP_VERSION}}", "0.0.1"),
        ],
    )?;

    // Update package.json if it exists (AssemblyScript template)
    let package_json_path = target_dir.join("package.json");
    if package_json_path.exists() {
        println!("AssemblyScript template detected");

        update_file(
            &package_json_path,
            &[
                ("{{KLAVE_APP_SLUG}}", project_name),
                ("{{KLAVE_APP_DESCRIPTION}}", description),
                ("{{KLAVE_APP_VERSION}}", "0.0.1"),
                ("{{KLAVE_APP_AUTHOR}}", &format!("{} <{}> ({})", author_name, author_email, author_url)),
                ("{{KLAVE_APP_LICENSE}}", "MIT"),
                ("{{KLAVE_APP_REPO}}", repo_url),
                ("{{KLAVE_SDK_CURRENT_VERSION}}", "*"), // Replace with latest version logic
            ],
        )?;
    }

    // Handle Rust-specific updates
    if template_type == "rust" {
        println!("Rust template detected");

        // Update Cargo.toml
        let cargo_toml_path = target_dir.join("Cargo.toml");
        if cargo_toml_path.exists() {
            let content = fs::read_to_string(&cargo_toml_path)?;
            let updated_content = content.replace(
                "members = [\"apps/rust-template\"]",
                &format!("members = [\"apps/{}\"]", project_name),
            );
            fs::write(cargo_toml_path, updated_content)?;
        }

        // Update app Cargo.toml
        let app_cargo_path = target_dir.join("apps/rust-template/Cargo.toml");
        if app_cargo_path.exists() {
            let content = fs::read_to_string(&app_cargo_path)?;
            let updated_content = content
                .replace("name = \"rust-template\"", &format!("name = \"{}\"", project_name))
                .replace(
                    "package = \"component:rust-template\"",
                    &format!("package = \"component:{}\"", project_name),
                );
            fs::write(app_cargo_path, updated_content)?;
        }

        // Update bindings.rs
        let bindings_path = target_dir.join("apps/rust-template/src/bindings.rs");
        if bindings_path.exists() {
            let content = fs::read_to_string(&bindings_path)?;
            let updated_content = content
                .replace(
                    "component:rust-template/rust-template",
                    &format!("component:{}/{}", project_name, project_name),
                )
                .replace("\x0brust-template", &format!("\x0b{}", project_name));
            fs::write(bindings_path, updated_content)?;
        }

        // Rename the app directory
        let old_app_dir = target_dir.join("apps/rust-template");
        let new_app_dir = target_dir.join(format!("apps/{}", project_name));

        if old_app_dir.exists() {
            fs::rename(&old_app_dir, &new_app_dir)?;
        } else {
            return Err(format!("App directory not found: {:?}", old_app_dir).into());
        }
    } else {
        // For AssemblyScript, rename the app directory
        let old_app_dir = target_dir.join("apps/hello_world");
        let new_app_dir = target_dir.join(format!("apps/{}", project_name));

        if old_app_dir.exists() {
            fs::rename(&old_app_dir, &new_app_dir)?;
        } else {
            return Err(format!("App directory not found: {:?}", old_app_dir).into());
        }
    }

    println!("Template files created successfully");
    Ok(())
}

// Helper function to recursively copy directories
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), Box<dyn Error>> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dst_path = dst.join(file_name);

        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

fn update_file(file_path: &Path, replacements: &[(&str, &str)]) -> Result<(), Box<dyn Error>> {
    if !file_path.exists() {
        return Err(format!("File not found: {:?}", file_path).into());
    }

    let mut content = String::new();
    match File::open(file_path) {
        Ok(mut file) => {
            file.read_to_string(&mut content)?;
        }
        Err(e) => {
            return Err(format!("Failed to open file {:?}: {}", file_path, e).into());
        }
    }

    for (pattern, replacement) in replacements {
        content = content.replace(pattern, replacement);
    }

    match File::create(file_path) {
        Ok(mut file) => {
            file.write_all(content.as_bytes())?;
        }
        Err(e) => {
            return Err(format!("Failed to write file {:?}: {}", file_path, e).into());
        }
    }

    Ok(())
}
