use include_dir::{Dir, include_dir};
use std::error::Error;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use walkdir::WalkDir;

// Embed templates in the binary
static ASSEMBLYSCRIPT_TEMPLATE: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates/assemblyscript");
static RUST_TEMPLATE: Dir = include_dir!("$CARGO_MANIFEST_DIR/templates/rust");

pub fn create_template(
    target_dir: &Path,
    project_name: &str,
    description: &str,
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

    // Define common placeholders for all template types
    let common_replacements = [
        ("{{KLAVE_APP_SLUG}}", project_name),
        ("{{KLAVE_APP_DESCRIPTION}}", description),
        ("{{KLAVE_APP_VERSION}}", "0.0.1"),
        ("{{KLAVE_APP_LICENSE}}", "MIT"),
        ("{{KLAVE_SDK_CURRENT_VERSION}}", "*"),
    ];

    // Process all template files at once, replacing placeholders
    process_template_files(temp_path, &common_replacements)?;

    // Copy processed template files to target directory
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

    // Rename the app directory for both template types
    let old_app_dir = target_dir.join("apps/hello_world");
    let new_app_dir = target_dir.join(format!("apps/{}", project_name));

    if old_app_dir.exists() {
        fs::rename(&old_app_dir, &new_app_dir)?;
    } else {
        return Err(format!("App directory not found: {:?}", old_app_dir).into());
    }

    println!("Template files created successfully");
    Ok(())
}

// Process all template files and replace placeholders
fn process_template_files(
    dir_path: &Path,
    replacements: &[(&str, &str)],
) -> Result<(), Box<dyn Error>> {
    for entry in WalkDir::new(dir_path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();

        // Skip directories
        if path.is_dir() {
            continue;
        }

        // Only process text files that might contain placeholders
        // Add more extensions if needed
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ["json", "toml", "rs", "ts", "wit"].contains(&extension) {
            update_file(path, replacements)?;
        }
    }

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
    // Skip if the file doesn't exist
    if !file_path.exists() {
        return Ok(());
    }

    let mut content = String::new();
    File::open(file_path)?.read_to_string(&mut content)?;

    let mut modified = false;
    for (pattern, replacement) in replacements {
        if content.contains(pattern) {
            content = content.replace(pattern, replacement);
            modified = true;
        }
    }

    // Only write back if content was modified
    if modified {
        let mut file = File::create(file_path)?;
        file.write_all(content.as_bytes())?;
    }

    Ok(())
}
